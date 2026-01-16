//! GitLab sync commands
//!
//! Commands for syncing GitLab data to work items.

use chrono::Utc;
use std::collections::HashSet;
use tauri::State;
use uuid::Uuid;

use recap_core::auth::verify_token;
use recap_core::models::GitLabProject;
use recap_core::services::worklog;

use crate::commands::AppState;
use super::types::{GitLabCommit, SyncGitLabRequest, SyncGitLabResponse};

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
                        let (synced, created) = process_commits(
                            &db.pool,
                            &claims.sub,
                            &gitlab_url,
                            &project,
                            commits,
                        )
                        .await;
                        synced_commits += synced;
                        work_items_created += created;
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

/// Process commits and create work items
async fn process_commits(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    gitlab_url: &str,
    project: &GitLabProject,
    commits: Vec<GitLabCommit>,
) -> (i64, i64) {
    let mut synced_commits = 0i64;
    let mut work_items_created = 0i64;

    // Batch fetch existing source_ids to avoid N+1 queries
    let commit_ids: Vec<&str> = commits.iter().map(|c| c.id.as_str()).collect();
    let short_hashes: Vec<String> = commit_ids.iter().map(|id| id.chars().take(8).collect()).collect();

    // Check both source_id (GitLab) and commit_hash (cross-source dedup)
    let (existing_source_ids, existing_hashes): (HashSet<String>, HashSet<String>) = if !commit_ids.is_empty() {
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
        let source_ids = q.fetch_all(pool)
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
        let hashes = hq.fetch_all(pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(h,)| h)
            .collect();

        (source_ids, hashes)
    } else {
        (HashSet::new(), HashSet::new())
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

        if let Err(e) = sqlx::query(
            r#"
            INSERT INTO work_items (id, user_id, source, source_id, source_url, title,
                description, hours, date, hours_source, hours_estimated, commit_hash, created_at, updated_at)
            VALUES (?, ?, 'gitlab', ?, ?, ?, ?, ?, ?, 'heuristic', ?, ?, ?, ?)
            "#,
        )
        .bind(&work_item_id)
        .bind(user_id)
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
        .execute(pool)
        .await
        {
            log::warn!("Failed to insert GitLab commit {}: {}", commit.id, e);
            continue;
        }

        synced_commits += 1;
        work_items_created += 1;
    }

    (synced_commits, work_items_created)
}
