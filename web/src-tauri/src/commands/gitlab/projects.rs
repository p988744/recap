//! GitLab project management commands
//!
//! Commands for managing tracked GitLab projects.

use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use recap_core::auth::verify_token;
use recap_core::models::GitLabProject;

use crate::commands::AppState;
use super::types::{AddProjectRequest, GitLabProjectInfo, SearchProjectsRequest};

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
