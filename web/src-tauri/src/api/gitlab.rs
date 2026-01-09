//! GitLab API routes

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::AuthUser, db::Database, models::GitLabProject};

/// GitLab routes
pub fn routes() -> Router<Database> {
    Router::new()
        .route("/projects", get(list_projects))
        .route("/projects", post(add_project))
        .route("/projects/:id", delete(remove_project))
        .route("/sync", post(sync_gitlab))
        .route("/search-projects", get(search_gitlab_projects))
}

/// List user's tracked GitLab projects
async fn list_projects(
    State(db): State<Database>,
    auth: AuthUser,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let projects: Vec<GitLabProject> =
        sqlx::query_as("SELECT * FROM gitlab_projects WHERE user_id = ? ORDER BY name")
            .bind(&auth.0.sub)
            .fetch_all(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(projects))
}

#[derive(Debug, Deserialize)]
pub struct AddProjectRequest {
    pub gitlab_project_id: i64,
    pub name: String,
    pub path_with_namespace: String,
    pub gitlab_url: String,
    pub default_branch: Option<String>,
}

/// Add a GitLab project to track
async fn add_project(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<AddProjectRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let default_branch = req.default_branch.unwrap_or_else(|| "main".to_string());

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
    .bind(&auth.0.sub)
    .bind(req.gitlab_project_id)
    .bind(&req.name)
    .bind(&req.path_with_namespace)
    .bind(&req.gitlab_url)
    .bind(&default_branch)
    .bind(now)
    .execute(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let project: GitLabProject = sqlx::query_as(
        "SELECT * FROM gitlab_projects WHERE user_id = ? AND gitlab_project_id = ?",
    )
    .bind(&auth.0.sub)
    .bind(req.gitlab_project_id)
    .fetch_one(&db.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(project)))
}

/// Remove a GitLab project from tracking
async fn remove_project(
    State(db): State<Database>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM gitlab_projects WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&auth.0.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Project not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub project_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub synced_commits: i64,
    pub synced_merge_requests: i64,
    pub work_items_created: i64,
}

/// Sync GitLab data to work items
async fn sync_gitlab(
    State(db): State<Database>,
    auth: AuthUser,
    Json(req): Json<SyncRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get user's GitLab config
    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let gitlab_url = user
        .gitlab_url
        .ok_or((StatusCode::BAD_REQUEST, "GitLab URL not configured".to_string()))?;

    let gitlab_pat = user
        .gitlab_pat
        .ok_or((StatusCode::BAD_REQUEST, "GitLab PAT not configured".to_string()))?;

    // Get projects to sync
    let projects: Vec<GitLabProject> = if let Some(project_id) = &req.project_id {
        sqlx::query_as("SELECT * FROM gitlab_projects WHERE id = ? AND user_id = ? AND enabled = 1")
            .bind(project_id)
            .bind(&auth.0.sub)
            .fetch_all(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        sqlx::query_as("SELECT * FROM gitlab_projects WHERE user_id = ? AND enabled = 1")
            .bind(&auth.0.sub)
            .fetch_all(&db.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
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
            .query(&[("per_page", "100")])
            .send()
            .await;

        if let Ok(response) = commits_result {
            if response.status().is_success() {
                if let Ok(commits) = response.json::<Vec<GitLabCommit>>().await {
                    for commit in commits {
                        // Check if already exists
                        let existing: Option<(i64,)> = sqlx::query_as(
                            "SELECT COUNT(*) FROM work_items WHERE source = 'gitlab' AND source_id = ?",
                        )
                        .bind(&commit.id)
                        .fetch_optional(&db.pool)
                        .await
                        .ok()
                        .flatten();

                        if existing.map(|r| r.0).unwrap_or(0) > 0 {
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

                        sqlx::query(
                            r#"
                            INSERT INTO work_items (id, user_id, source, source_id, source_url, title,
                                description, hours, date, created_at, updated_at)
                            VALUES (?, ?, 'gitlab', ?, ?, ?, ?, 0, ?, ?, ?)
                            "#,
                        )
                        .bind(&work_item_id)
                        .bind(&auth.0.sub)
                        .bind(&commit.id)
                        .bind(&source_url)
                        .bind(&commit.title)
                        .bind(&commit.message)
                        .bind(commit_date)
                        .bind(now)
                        .bind(now)
                        .execute(&db.pool)
                        .await
                        .ok();

                        synced_commits += 1;
                        work_items_created += 1;
                    }
                }
            }
        }

        // Update last_synced
        let now = Utc::now();
        sqlx::query("UPDATE gitlab_projects SET last_synced = ? WHERE id = ?")
            .bind(now)
            .bind(&project.id)
            .execute(&db.pool)
            .await
            .ok();
    }

    Ok(Json(SyncResponse {
        synced_commits,
        synced_merge_requests,
        work_items_created,
    }))
}

#[derive(Debug, Deserialize)]
struct GitLabCommit {
    id: String,
    title: String,
    message: Option<String>,
    committed_date: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchProjectsQuery {
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

/// Search GitLab projects
async fn search_gitlab_projects(
    State(db): State<Database>,
    auth: AuthUser,
    Query(query): Query<SearchProjectsQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get user's GitLab config
    let user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&auth.0.sub)
        .fetch_one(&db.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let gitlab_url = user
        .gitlab_url
        .ok_or((StatusCode::BAD_REQUEST, "GitLab URL not configured".to_string()))?;

    let gitlab_pat = user
        .gitlab_pat
        .ok_or((StatusCode::BAD_REQUEST, "GitLab PAT not configured".to_string()))?;

    let client = reqwest::Client::new();

    let url = format!("{}/api/v4/projects", gitlab_url);
    let mut params = vec![("membership", "true"), ("per_page", "50")];

    if let Some(search) = &query.search {
        params.push(("search", search));
    }

    let response = client
        .get(&url)
        .header("PRIVATE-TOKEN", &gitlab_pat)
        .query(&params)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("GitLab API error: {}", e)))?;

    if !response.status().is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("GitLab API returned: {}", response.status()),
        ));
    }

    let projects: Vec<GitLabProjectInfo> = response
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Failed to parse response: {}", e)))?;

    Ok(Json(projects))
}
