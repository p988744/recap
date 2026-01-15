//! GitLab commands
//!
//! Tauri commands for GitLab integration operations.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use recap_core::auth::verify_token;
use recap_core::models::GitLabProject;
use recap_core::services::worklog;

use super::AppState;

// Types

#[derive(Debug, Deserialize)]
pub struct AddProjectRequest {
    pub gitlab_project_id: i64,
    // Optional fields - if not provided, will be fetched from GitLab API
    pub name: Option<String>,
    pub path_with_namespace: Option<String>,
    pub gitlab_url: Option<String>,
    pub default_branch: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SyncGitLabRequest {
    pub project_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncGitLabResponse {
    pub synced_commits: i64,
    pub synced_merge_requests: i64,
    pub work_items_created: i64,
}

#[derive(Debug, Deserialize)]
pub struct SearchProjectsRequest {
    pub search: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitLabProjectInfo {
    pub id: i64,
    pub name: String,
    pub path_with_namespace: String,
    pub web_url: String,
    pub default_branch: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitLabCommit {
    id: String,
    title: String,
    message: Option<String>,
    committed_date: String,
    stats: Option<CommitStats>,
}

#[derive(Debug, Deserialize)]
struct CommitStats {
    additions: i32,
    deletions: i32,
}

#[derive(Debug, Serialize)]
pub struct GitLabConfigStatus {
    pub configured: bool,
    pub gitlab_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigureGitLabRequest {
    pub gitlab_url: String,
    pub gitlab_pat: String,
}

// Commands

/// Get GitLab configuration status
#[tauri::command]
pub async fn get_gitlab_status(
    state: State<'_, AppState>,
    token: String,
) -> Result<GitLabConfigStatus, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(GitLabConfigStatus {
        configured: user.gitlab_pat.is_some(),
        gitlab_url: user.gitlab_url,
    })
}

/// Configure GitLab
#[tauri::command]
pub async fn configure_gitlab(
    state: State<'_, AppState>,
    token: String,
    request: ConfigureGitLabRequest,
) -> Result<serde_json::Value, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;
    let now = Utc::now();

    sqlx::query("UPDATE users SET gitlab_url = ?, gitlab_pat = ?, updated_at = ? WHERE id = ?")
        .bind(&request.gitlab_url)
        .bind(&request.gitlab_pat)
        .bind(now)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "message": "GitLab configured successfully" }))
}

/// Remove GitLab configuration
#[tauri::command]
pub async fn remove_gitlab_config(
    state: State<'_, AppState>,
    token: String,
) -> Result<serde_json::Value, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;
    let now = Utc::now();

    sqlx::query("UPDATE users SET gitlab_url = NULL, gitlab_pat = NULL, updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "message": "GitLab configuration removed" }))
}

/// List user's tracked GitLab projects
#[tauri::command]
pub async fn list_gitlab_projects(
    state: State<'_, AppState>,
    token: String,
) -> Result<Vec<GitLabProject>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let projects: Vec<GitLabProject> =
        sqlx::query_as("SELECT * FROM gitlab_projects WHERE user_id = ? ORDER BY name")
            .bind(&claims.sub)
            .fetch_all(&db.pool)
            .await
            .map_err(|e| e.to_string())?;

    Ok(projects)
}

/// Add a GitLab project to track
#[tauri::command]
pub async fn add_gitlab_project(
    state: State<'_, AppState>,
    token: String,
    request: AddProjectRequest,
) -> Result<GitLabProject, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Get user's GitLab config
    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    let user_gitlab_url = user
        .gitlab_url
        .ok_or("GitLab URL not configured".to_string())?;

    let gitlab_pat = user
        .gitlab_pat
        .ok_or("GitLab PAT not configured".to_string())?;

    // Fetch project details from GitLab API if not provided
    let (name, path_with_namespace, gitlab_url, default_branch) =
        if request.name.is_none() || request.path_with_namespace.is_none() {
            let client = reqwest::Client::new();
            let url = format!(
                "{}/api/v4/projects/{}",
                user_gitlab_url, request.gitlab_project_id
            );

            let response = client
                .get(&url)
                .header("PRIVATE-TOKEN", &gitlab_pat)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch project details: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("GitLab API returned: {}", response.status()));
            }

            let project_info: GitLabProjectInfo = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse project details: {}", e))?;

            (
                project_info.name,
                project_info.path_with_namespace,
                request.gitlab_url.unwrap_or(user_gitlab_url),
                request
                    .default_branch
                    .or(project_info.default_branch)
                    .unwrap_or_else(|| "main".to_string()),
            )
        } else {
            (
                request.name.unwrap(),
                request.path_with_namespace.unwrap(),
                request.gitlab_url.unwrap_or(user_gitlab_url),
                request.default_branch.unwrap_or_else(|| "main".to_string()),
            )
        };

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO gitlab_projects (id, user_id, gitlab_project_id, name, path_with_namespace,
            gitlab_url, default_branch, enabled, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?)
        ON CONFLICT(user_id, gitlab_project_id) DO UPDATE SET
            name = excluded.name,
            path_with_namespace = excluded.path_with_namespace,
            enabled = 1
        "#,
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(request.gitlab_project_id)
    .bind(&name)
    .bind(&path_with_namespace)
    .bind(&gitlab_url)
    .bind(&default_branch)
    .bind(now)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let project: GitLabProject = sqlx::query_as(
        "SELECT * FROM gitlab_projects WHERE user_id = ? AND gitlab_project_id = ?",
    )
    .bind(&claims.sub)
    .bind(request.gitlab_project_id)
    .fetch_one(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(project)
}

/// Remove a GitLab project from tracking
#[tauri::command]
pub async fn remove_gitlab_project(
    state: State<'_, AppState>,
    token: String,
    id: String,
) -> Result<serde_json::Value, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let result = sqlx::query("DELETE FROM gitlab_projects WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err("Project not found".to_string());
    }

    Ok(serde_json::json!({ "message": "Project removed" }))
}

/// Sync GitLab data to work items
#[tauri::command]
pub async fn sync_gitlab(
    state: State<'_, AppState>,
    token: String,
    request: SyncGitLabRequest,
) -> Result<SyncGitLabResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Get user's GitLab config
    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    let gitlab_url = user
        .gitlab_url
        .ok_or("GitLab URL not configured".to_string())?;

    let gitlab_pat = user
        .gitlab_pat
        .ok_or("GitLab PAT not configured".to_string())?;

    // Get projects to sync
    let projects: Vec<GitLabProject> = if let Some(project_id) = &request.project_id {
        sqlx::query_as("SELECT * FROM gitlab_projects WHERE id = ? AND user_id = ? AND enabled = 1")
            .bind(project_id)
            .bind(&claims.sub)
            .fetch_all(&db.pool)
            .await
            .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as("SELECT * FROM gitlab_projects WHERE user_id = ? AND enabled = 1")
            .bind(&claims.sub)
            .fetch_all(&db.pool)
            .await
            .map_err(|e| e.to_string())?
    };

    let mut synced_commits = 0i64;
    #[allow(unused_mut)]
    let mut synced_merge_requests = 0i64;
    let mut work_items_created = 0i64;

    let client = reqwest::Client::new();

    for project in projects {
        // Sync commits
        let commits_url = format!(
            "{}/api/v4/projects/{}/repository/commits",
            gitlab_url, project.gitlab_project_id
        );

        let commits_result = client
            .get(&commits_url)
            .header("PRIVATE-TOKEN", &gitlab_pat)
            .query(&[("per_page", "100"), ("with_stats", "true")])
            .send()
            .await;

        match commits_result {
            Ok(response) => {
                if !response.status().is_success() {
                    log::warn!(
                        "GitLab API returned status {} for project {}",
                        response.status(),
                        project.path_with_namespace
                    );
                    continue;
                }

                match response.json::<Vec<GitLabCommit>>().await {
                    Ok(commits) => {
                        // Batch fetch existing source_ids to avoid N+1 queries
                        let commit_ids: Vec<&str> = commits.iter().map(|c| c.id.as_str()).collect();
                        let short_hashes: Vec<String> = commit_ids.iter().map(|id| id.chars().take(8).collect()).collect();

                        // Check both source_id (GitLab) and commit_hash (cross-source dedup)
                        let (existing_source_ids, existing_hashes): (std::collections::HashSet<String>, std::collections::HashSet<String>) = if !commit_ids.is_empty() {
                            // Query existing GitLab source_ids
                            let placeholders = commit_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                            let query = format!(
                                "SELECT source_id FROM work_items WHERE source = 'gitlab' AND source_id IN ({})",
                                placeholders
                            );
                            let mut q = sqlx::query_as::<_, (String,)>(&query);
                            for id in &commit_ids {
                                q = q.bind(id);
                            }
                            let source_ids = q.fetch_all(&db.pool)
                                .await
                                .map_err(|e| {
                                    log::warn!("Failed to query existing commits: {}", e);
                                    e
                                })
                                .unwrap_or_default()
                                .into_iter()
                                .map(|(id,)| id)
                                .collect();

                            // Query existing commit_hash (cross-source deduplication)
                            let hash_placeholders = short_hashes.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                            let hash_query = format!(
                                "SELECT commit_hash FROM work_items WHERE commit_hash IS NOT NULL AND commit_hash IN ({})",
                                hash_placeholders
                            );
                            let mut hq = sqlx::query_as::<_, (String,)>(&hash_query);
                            for hash in &short_hashes {
                                hq = hq.bind(hash);
                            }
                            let hashes = hq.fetch_all(&db.pool)
                                .await
                                .unwrap_or_default()
                                .into_iter()
                                .map(|(h,)| h)
                                .collect();

                            (source_ids, hashes)
                        } else {
                            (std::collections::HashSet::new(), std::collections::HashSet::new())
                        };

                        for commit in commits {
                            let short_hash = commit.id.chars().take(8).collect::<String>();

                            // Skip if already exists by source_id OR commit_hash (cross-source dedup)
                            if existing_source_ids.contains(&commit.id) || existing_hashes.contains(&short_hash) {
                                continue;
                            }

                            // Create work item from commit
                            let work_item_id = Uuid::new_v4().to_string();
                            let now = Utc::now();
                            let commit_date = commit
                                .committed_date
                                .split('T')
                                .next()
                                .unwrap_or(&commit.committed_date);

                            let source_url = format!(
                                "{}/{}/-/commit/{}",
                                gitlab_url, project.path_with_namespace, commit.id
                            );

                            // Calculate hours using heuristic from diff stats
                            let (additions, deletions) = commit.stats
                                .as_ref()
                                .map(|s| (s.additions, s.deletions))
                                .unwrap_or((0, 0));
                            // Use 1 file as estimate since GitLab list doesn't give file count
                            let estimated_hours = worklog::estimate_from_diff(additions, deletions, 1);
                            // short_hash is already calculated above for dedup check

                            if let Err(e) = sqlx::query(
                                r#"
                                INSERT INTO work_items (id, user_id, source, source_id, source_url, title,
                                    description, hours, date, hours_source, hours_estimated, commit_hash, created_at, updated_at)
                                VALUES (?, ?, 'gitlab', ?, ?, ?, ?, ?, ?, 'heuristic', ?, ?, ?, ?)
                                "#,
                            )
                            .bind(&work_item_id)
                            .bind(&claims.sub)
                            .bind(&commit.id)
                            .bind(&source_url)
                            .bind(&commit.title)
                            .bind(&commit.message)
                            .bind(estimated_hours)
                            .bind(commit_date)
                            .bind(estimated_hours)
                            .bind(&short_hash)
                            .bind(now)
                            .bind(now)
                            .execute(&db.pool)
                            .await
                            {
                                log::warn!("Failed to insert GitLab commit {}: {}", commit.id, e);
                                continue;
                            }

                            synced_commits += 1;
                            work_items_created += 1;
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to parse commits JSON for project {}: {}",
                            project.path_with_namespace,
                            e
                        );
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to fetch commits for project {}: {}",
                    project.path_with_namespace,
                    e
                );
            }
        }

        // Update last_synced
        let now = Utc::now();
        if let Err(e) = sqlx::query("UPDATE gitlab_projects SET last_synced = ? WHERE id = ?")
            .bind(now)
            .bind(&project.id)
            .execute(&db.pool)
            .await
        {
            log::warn!("Failed to update last_synced for project {}: {}", project.id, e);
        }
    }

    Ok(SyncGitLabResponse {
        synced_commits,
        synced_merge_requests,
        work_items_created,
    })
}

/// Search GitLab projects
#[tauri::command]
pub async fn search_gitlab_projects(
    state: State<'_, AppState>,
    token: String,
    request: SearchProjectsRequest,
) -> Result<Vec<GitLabProjectInfo>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Get user's GitLab config
    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    let gitlab_url = user
        .gitlab_url
        .ok_or("GitLab URL not configured".to_string())?;

    let gitlab_pat = user
        .gitlab_pat
        .ok_or("GitLab PAT not configured".to_string())?;

    let client = reqwest::Client::new();

    let url = format!("{}/api/v4/projects", gitlab_url);
    let mut params = vec![("membership", "true"), ("per_page", "50")];

    let search_str;
    if let Some(search) = &request.search {
        search_str = search.clone();
        params.push(("search", &search_str));
    }

    let response = client
        .get(&url)
        .header("PRIVATE-TOKEN", &gitlab_pat)
        .query(&params)
        .send()
        .await
        .map_err(|e| format!("GitLab API error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitLab API returned: {}", response.status()));
    }

    let projects: Vec<GitLabProjectInfo> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(projects)
}
