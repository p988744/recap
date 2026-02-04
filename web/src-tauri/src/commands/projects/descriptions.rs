//! Project description commands
//!
//! CRUD operations for project descriptions (goal, tech stack, features, notes).

use recap_core::auth::verify_token;
use tauri::State;
use uuid::Uuid;

use super::types::{ProjectDescription, UpdateProjectDescriptionRequest};
use crate::commands::AppState;

/// Get project description
#[tauri::command(rename_all = "camelCase")]
pub async fn get_project_description(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
) -> Result<Option<ProjectDescription>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let row = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>, Option<String>)>(
        "SELECT goal, tech_stack, key_features, notes FROM project_descriptions WHERE user_id = ? AND project_name = ?",
    )
    .bind(&claims.sub)
    .bind(&project_name)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    match row {
        Some((goal, tech_stack, key_features_json, notes)) => {
            let key_features: Option<Vec<String>> = key_features_json
                .and_then(|s| serde_json::from_str(&s).ok());

            Ok(Some(ProjectDescription {
                project_name,
                goal,
                tech_stack,
                key_features,
                notes,
            }))
        }
        None => Ok(None),
    }
}

/// Update or create project description
#[tauri::command(rename_all = "camelCase")]
pub async fn update_project_description(
    state: State<'_, AppState>,
    token: String,
    request: UpdateProjectDescriptionRequest,
) -> Result<String, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let key_features_json = request
        .key_features
        .map(|f| serde_json::to_string(&f).unwrap_or_default());

    let id = Uuid::new_v4().to_string();

    sqlx::query(
        r#"
        INSERT INTO project_descriptions (id, user_id, project_name, goal, tech_stack, key_features, notes, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(user_id, project_name) DO UPDATE SET
            goal = excluded.goal,
            tech_stack = excluded.tech_stack,
            key_features = excluded.key_features,
            notes = excluded.notes,
            orphaned = 0,
            orphaned_at = NULL,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&request.project_name)
    .bind(&request.goal)
    .bind(&request.tech_stack)
    .bind(&key_features_json)
    .bind(&request.notes)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok("Description updated".to_string())
}

/// Delete project description
#[tauri::command(rename_all = "camelCase")]
pub async fn delete_project_description(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
) -> Result<String, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    sqlx::query("DELETE FROM project_descriptions WHERE user_id = ? AND project_name = ?")
        .bind(&claims.sub)
        .bind(&project_name)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok("Description deleted".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_description_serialization() {
        let desc = ProjectDescription {
            project_name: "test".to_string(),
            goal: Some("Test goal".to_string()),
            tech_stack: Some("Rust, React".to_string()),
            key_features: Some(vec!["Feature 1".to_string(), "Feature 2".to_string()]),
            notes: None,
        };

        let json = serde_json::to_string(&desc).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Test goal"));
    }
}
