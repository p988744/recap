//! Project queries
//!
//! Commands for listing projects, getting project details,
//! and managing project visibility.

use std::collections::HashMap;
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::models::WorkItem;

use crate::commands::AppState;
use super::types::{
    AddManualProjectRequest, AntigravitySessionPathResponse, ClaudeCodeDirEntry,
    ClaudeSessionPathResponse, ProjectDetail, ProjectDirectories, ProjectInfo,
    ProjectSourceInfo, ProjectStats, SetProjectVisibilityRequest, WorkItemSummary,
};

/// Extract project name from work item title "[ProjectName] ..." pattern
fn extract_project_name(title: &str) -> Option<String> {
    if title.starts_with('[') {
        title.split(']').next().map(|s| s.trim_start_matches('[').to_string())
    } else {
        None
    }
}

/// Derive project name from either title pattern or project_path
fn derive_project_name(item: &WorkItem) -> String {
    if let Some(name) = extract_project_name(&item.title) {
        if !name.is_empty() {
            return name;
        }
    }
    if let Some(path) = &item.project_path {
        if let Some(last) = std::path::Path::new(path).file_name().and_then(|n| n.to_str()) {
            return last.to_string();
        }
    }
    "unknown".to_string()
}

/// List all projects auto-discovered from work_items, with visibility preferences
#[tauri::command]
pub async fn list_projects(
    state: State<'_, AppState>,
    token: String,
) -> Result<Vec<ProjectInfo>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Fetch all work items for this user
    let items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? ORDER BY date DESC",
    )
    .bind(&claims.sub)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Fetch project preferences (including manual projects)
    let prefs: Vec<(String, bool, Option<String>, Option<String>, Option<bool>)> = sqlx::query_as(
        "SELECT project_name, hidden, display_name, project_path, manual_added FROM project_preferences WHERE user_id = ?",
    )
    .bind(&claims.sub)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let pref_map: HashMap<String, (bool, Option<String>, Option<String>, bool)> = prefs
        .into_iter()
        .map(|(name, hidden, display_name, path, manual)| {
            (name, (hidden, display_name, path, manual.unwrap_or(false)))
        })
        .collect();

    // Group work items by project name
    struct ProjectAgg {
        project_path: Option<String>,
        sources: HashMap<String, i64>,
        total_hours: f64,
        total_count: i64,
        latest_date: Option<String>,
    }

    let mut project_map: HashMap<String, ProjectAgg> = HashMap::new();

    for item in &items {
        let project_name = derive_project_name(item);
        let entry = project_map.entry(project_name).or_insert_with(|| ProjectAgg {
            project_path: None,
            sources: HashMap::new(),
            total_hours: 0.0,
            total_count: 0,
            latest_date: None,
        });

        if entry.project_path.is_none() {
            entry.project_path = item.project_path.clone();
        }

        *entry.sources.entry(item.source.clone()).or_insert(0) += 1;
        entry.total_hours += item.hours;
        entry.total_count += 1;

        let date_str = item.date.to_string();
        if entry.latest_date.is_none() || entry.latest_date.as_deref() < Some(date_str.as_str()) {
            entry.latest_date = Some(date_str);
        }
    }

    // Build response from work-item-discovered projects
    let mut projects: Vec<ProjectInfo> = project_map
        .into_iter()
        .map(|(name, agg)| {
            let (hidden, display_name, pref_path, _manual) = pref_map
                .get(&name)
                .cloned()
                .unwrap_or((false, None, None, false));

            // Determine primary source (the one with most items)
            let primary_source = agg
                .sources
                .iter()
                .max_by_key(|(_, &count)| count)
                .map(|(src, _)| src.clone())
                .unwrap_or_else(|| "unknown".to_string());

            // Collect all sources (sorted by count descending)
            let mut all_sources: Vec<(String, i64)> = agg.sources.into_iter().collect();
            all_sources.sort_by(|a, b| b.1.cmp(&a.1));
            let sources: Vec<String> = all_sources.into_iter().map(|(src, _)| src).collect();

            let project_path = agg.project_path.or(pref_path);

            ProjectInfo {
                project_name: name,
                project_path,
                source: primary_source,
                sources,
                work_item_count: agg.total_count,
                total_hours: agg.total_hours,
                latest_date: agg.latest_date,
                hidden,
                display_name,
            }
        })
        .collect();

    // Add manually-added projects that don't appear in work items
    let discovered_names: std::collections::HashSet<String> =
        projects.iter().map(|p| p.project_name.clone()).collect();

    for (name, (hidden, display_name, pref_path, manual)) in &pref_map {
        if *manual && !discovered_names.contains(name) {
            projects.push(ProjectInfo {
                project_name: name.clone(),
                project_path: pref_path.clone(),
                source: "manual".to_string(),
                sources: vec!["manual".to_string()],
                work_item_count: 0,
                total_hours: 0.0,
                latest_date: None,
                hidden: *hidden,
                display_name: display_name.clone(),
            });
        }
    }

    // Sort: visible first, then by total_hours descending
    projects.sort_by(|a, b| {
        a.hidden.cmp(&b.hidden).then(
            b.total_hours
                .partial_cmp(&a.total_hours)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });

    Ok(projects)
}

/// Get detailed information about a specific project
#[tauri::command]
pub async fn get_project_detail(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
) -> Result<ProjectDetail, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Fetch all work items for this user
    let items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? ORDER BY date DESC",
    )
    .bind(&claims.sub)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // Filter items belonging to this project
    let project_items: Vec<&WorkItem> = items
        .iter()
        .filter(|item| derive_project_name(item) == project_name)
        .collect();

    // Fetch preference
    let pref: Option<(bool, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT hidden, display_name, project_path FROM project_preferences WHERE user_id = ? AND project_name = ?",
    )
    .bind(&claims.sub)
    .bind(&project_name)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let (hidden, display_name, pref_path) = pref.unwrap_or((false, None, None));

    // Build source breakdown
    // For "aggregated" items, resolve to their children's real sources
    struct SourceAgg {
        count: i64,
        latest_date: Option<String>,
        project_path: Option<String>,
    }
    let mut source_map: HashMap<String, SourceAgg> = HashMap::new();

    for item in &project_items {
        if item.source == "aggregated" {
            // Find children of this aggregated item
            let children: Vec<WorkItem> = sqlx::query_as(
                "SELECT * FROM work_items WHERE parent_id = ? AND user_id = ?",
            )
            .bind(&item.id)
            .bind(&claims.sub)
            .fetch_all(&db.pool)
            .await
            .map_err(|e| e.to_string())?;

            if children.is_empty() {
                // No children found, skip this aggregated item
                continue;
            }

            for child in &children {
                let entry = source_map
                    .entry(child.source.clone())
                    .or_insert_with(|| SourceAgg {
                        count: 0,
                        latest_date: None,
                        project_path: None,
                    });
                entry.count += 1;
                let date_str = child.date.to_string();
                if entry.latest_date.is_none() || entry.latest_date.as_deref() < Some(date_str.as_str()) {
                    entry.latest_date = Some(date_str);
                }
                if entry.project_path.is_none() {
                    entry.project_path = child.project_path.clone();
                }
            }
        } else {
            let entry = source_map
                .entry(item.source.clone())
                .or_insert_with(|| SourceAgg {
                    count: 0,
                    latest_date: None,
                    project_path: None,
                });
            entry.count += 1;
            let date_str = item.date.to_string();
            if entry.latest_date.is_none() || entry.latest_date.as_deref() < Some(date_str.as_str()) {
                entry.latest_date = Some(date_str);
            }
            if entry.project_path.is_none() {
                entry.project_path = item.project_path.clone();
            }
        }
    }

    let sources: Vec<ProjectSourceInfo> = source_map
        .into_iter()
        .map(|(source, agg)| ProjectSourceInfo {
            source,
            item_count: agg.count,
            latest_date: agg.latest_date,
            project_path: agg.project_path,
        })
        .collect();

    // Recent items (first 10, already sorted by date DESC)
    let recent_items: Vec<WorkItemSummary> = project_items
        .iter()
        .take(10)
        .map(|item| WorkItemSummary {
            id: item.id.clone(),
            title: item.title.clone(),
            date: item.date.to_string(),
            hours: item.hours,
            source: item.source.clone(),
        })
        .collect();

    // Stats
    let total_items = project_items.len() as i64;
    let total_hours: f64 = project_items.iter().map(|i| i.hours).sum();
    let dates: Vec<String> = project_items.iter().map(|i| i.date.to_string()).collect();
    let date_range = if !dates.is_empty() {
        let min = dates.iter().min().cloned().unwrap();
        let max = dates.iter().max().cloned().unwrap();
        Some((min, max))
    } else {
        None
    };

    // Get project_path from first item or pref
    let project_path = project_items
        .iter()
        .find_map(|i| i.project_path.clone())
        .or(pref_path);

    Ok(ProjectDetail {
        project_name,
        project_path,
        hidden,
        display_name,
        sources,
        recent_items,
        stats: ProjectStats {
            total_items,
            total_hours,
            date_range,
        },
    })
}

/// Set project visibility (show/hide)
#[tauri::command]
pub async fn set_project_visibility(
    state: State<'_, AppState>,
    token: String,
    request: SetProjectVisibilityRequest,
) -> Result<String, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Upsert into project_preferences
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO project_preferences (id, user_id, project_name, hidden, updated_at)
        VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(user_id, project_name) DO UPDATE SET
            hidden = excluded.hidden,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&request.project_name)
    .bind(request.hidden)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok("ok".to_string())
}

/// Get list of hidden project names for global filtering
#[tauri::command]
pub async fn get_hidden_projects(
    state: State<'_, AppState>,
    token: String,
) -> Result<Vec<String>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let hidden: Vec<(String,)> = sqlx::query_as(
        "SELECT project_name FROM project_preferences WHERE user_id = ? AND hidden = 1",
    )
    .bind(&claims.sub)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(hidden.into_iter().map(|(name,)| name).collect())
}

/// Get project directories (Claude Code session dir + Git repo path)
///
/// Scans the user's Claude session path (default ~/.claude) /projects/ to find
/// directories matching the project, reads sessions-index.json for git repo path.
#[tauri::command]
pub async fn get_project_directories(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
) -> Result<ProjectDirectories, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // 0. Get user's Claude session path (or default)
    let claude_base: Option<String> = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT claude_session_path FROM users WHERE id = ?",
    )
    .bind(&claims.sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?
    .and_then(|(path,)| path);

    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let claude_base_path = claude_base
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".claude"));

    // 0b. Check if project has a manual git_repo_path in preferences
    let manual_git_repo: Option<String> = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT git_repo_path FROM project_preferences WHERE user_id = ? AND project_name = ?",
    )
    .bind(&claims.sub)
    .bind(&project_name)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?
    .and_then(|(path,)| path);

    // 1. Get project_path from work items
    let items: Vec<WorkItem> = sqlx::query_as(
        "SELECT * FROM work_items WHERE user_id = ? ORDER BY date DESC",
    )
    .bind(&claims.sub)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let project_path: Option<String> = items
        .iter()
        .filter(|item| derive_project_name(item) == project_name)
        .find_map(|item| item.project_path.clone());

    // 2. Scan <claude_base>/projects/ for ALL matching directories
    let claude_projects_dir = claude_base_path.join("projects");

    let mut claude_code_dirs: Vec<ClaudeCodeDirEntry> = Vec::new();
    let mut git_repo_path: Option<String> = None;

    if claude_projects_dir.exists() {
        // Encode the project_path to match Claude Code's directory naming:
        // /Users/foo/bar → -Users-foo-bar
        let encoded_prefix: Option<String> = project_path.as_ref().map(|p| {
            p.replace(['/', '\\'], "-")
        });

        // Fallback: match dirs ending with -<project_name>
        let target_suffix = format!("-{}", project_name);

        let entries = std::fs::read_dir(&claude_projects_dir)
            .map_err(|e| e.to_string())?;

        for entry in entries.flatten() {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }

            // Match: exact encoded path OR starts with encoded path + "-" (subdirs)
            // Fallback: ends with -<project_name> or starts with -...-<project_name>-
            let matched = if let Some(ref enc) = encoded_prefix {
                dir_name == *enc || dir_name.starts_with(&format!("{}-", enc))
            } else {
                dir_name.ends_with(&target_suffix)
                    || dir_name.contains(&format!("{}-", target_suffix))
            };

            if !matched {
                continue;
            }

            let dir_path = entry.path();
            let mut dir_session_count: i64 = 0;

            // Read sessions-index.json
            let index_path = dir_path.join("sessions-index.json");
            if index_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&index_path) {
                    if let Ok(index) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(sess_entries) = index.get("entries").and_then(|e| e.as_array()) {
                            dir_session_count = sess_entries.len() as i64;

                            // Try to get git repo path from the first session with projectPath
                            if git_repo_path.is_none() {
                                for sess in sess_entries {
                                    if let Some(pp) = sess.get("projectPath").and_then(|v| v.as_str()) {
                                        git_repo_path = Some(pp.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Count .jsonl files as sessions if no index
                if let Ok(files) = std::fs::read_dir(&dir_path) {
                    dir_session_count = files
                        .flatten()
                        .filter(|f| {
                            f.path().extension().map(|e| e == "jsonl").unwrap_or(false)
                        })
                        .count() as i64;
                }
            }

            claude_code_dirs.push(ClaudeCodeDirEntry {
                path: dir_path.to_string_lossy().to_string(),
                session_count: dir_session_count,
            });
        }
    }

    // Sort by path (main dir first, then subdirs)
    claude_code_dirs.sort_by(|a, b| a.path.len().cmp(&b.path.len()));

    // Use manual git_repo_path first, then session-discovered, then work item fallback
    if git_repo_path.is_none() {
        git_repo_path = manual_git_repo.or(project_path);
    }

    // Resolve git_repo_path to the .git directory
    let git_repo_path = git_repo_path.and_then(|p| {
        let mut path = std::path::PathBuf::from(&p);
        loop {
            let git_dir = path.join(".git");
            if git_dir.exists() {
                return Some(git_dir.to_string_lossy().to_string());
            }
            if !path.pop() {
                break;
            }
        }
        None
    });

    Ok(ProjectDirectories {
        claude_code_dirs,
        git_repo_path,
    })
}

/// Get the user's Claude session path setting
#[tauri::command]
pub async fn get_claude_session_path(
    state: State<'_, AppState>,
    token: String,
) -> Result<ClaudeSessionPathResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT claude_session_path FROM users WHERE id = ?",
    )
    .bind(&claims.sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let custom_path = row.and_then(|(p,)| p);
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let default_path = home.join(".claude").to_string_lossy().to_string();

    let is_default = custom_path.is_none() || custom_path.as_deref() == Some(&default_path);
    let path = custom_path.unwrap_or_else(|| default_path);

    Ok(ClaudeSessionPathResponse {
        path,
        is_default,
    })
}

/// Update the user's Claude session path
#[tauri::command]
pub async fn update_claude_session_path(
    state: State<'_, AppState>,
    token: String,
    path: Option<String>,
) -> Result<String, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Validate path exists and is a directory
    if let Some(ref p) = path {
        let path_buf = std::path::PathBuf::from(p);
        if !path_buf.is_dir() {
            return Err(format!("Path is not a valid directory: {}", p));
        }
    }

    sqlx::query("UPDATE users SET claude_session_path = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&path)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok("ok".to_string())
}

/// Get the user's Antigravity (Gemini Code) session path setting
#[tauri::command]
pub async fn get_antigravity_session_path(
    state: State<'_, AppState>,
    token: String,
) -> Result<AntigravitySessionPathResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT antigravity_session_path FROM users WHERE id = ?",
    )
    .bind(&claims.sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let custom_path = row.and_then(|(p,)| p);
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let default_path = home.join(".gemini").join("antigravity").to_string_lossy().to_string();

    let is_default = custom_path.is_none() || custom_path.as_deref() == Some(&default_path);
    let path = custom_path.unwrap_or_else(|| default_path);

    Ok(AntigravitySessionPathResponse {
        path,
        is_default,
    })
}

/// Update the user's Antigravity (Gemini Code) session path
#[tauri::command]
pub async fn update_antigravity_session_path(
    state: State<'_, AppState>,
    token: String,
    path: Option<String>,
) -> Result<String, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Validate path exists and is a directory
    if let Some(ref p) = path {
        let path_buf = std::path::PathBuf::from(p);
        if !path_buf.is_dir() {
            return Err(format!("Path is not a valid directory: {}", p));
        }
    }

    sqlx::query("UPDATE users SET antigravity_session_path = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&path)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok("ok".to_string())
}

/// Add a manual project (non-Claude, requires git repo path)
#[tauri::command]
pub async fn add_manual_project(
    state: State<'_, AppState>,
    token: String,
    request: AddManualProjectRequest,
) -> Result<String, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Validate project_name: disallow characters that break LIKE pattern matching
    if request.project_name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    if request.project_name.contains('[') || request.project_name.contains(']') {
        return Err("Project name cannot contain [ or ] characters".to_string());
    }

    // Validate git repo path contains .git
    let git_path = std::path::PathBuf::from(&request.git_repo_path);
    if !git_path.is_dir() {
        return Err(format!("Path is not a directory: {}", request.git_repo_path));
    }
    let git_dir = git_path.join(".git");
    if !git_dir.exists() {
        return Err(format!(
            "Not a valid git repository: {} (no .git directory found)",
            request.git_repo_path
        ));
    }

    // For manual projects, project_path = git_repo_path (the project root)
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO project_preferences (id, user_id, project_name, project_path, git_repo_path, display_name, manual_added, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, 1, CURRENT_TIMESTAMP)
        ON CONFLICT(user_id, project_name) DO UPDATE SET
            project_path = excluded.project_path,
            git_repo_path = excluded.git_repo_path,
            display_name = COALESCE(excluded.display_name, project_preferences.display_name),
            manual_added = 1,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&request.project_name)
    .bind(&request.git_repo_path) // project_path: same as git repo root for manual projects
    .bind(&request.git_repo_path) // git_repo_path
    .bind(&request.display_name)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok("ok".to_string())
}

/// Remove a manually added project
#[tauri::command]
pub async fn remove_manual_project(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
) -> Result<String, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let result = sqlx::query(
        "DELETE FROM project_preferences WHERE user_id = ? AND project_name = ? AND manual_added = 1",
    )
    .bind(&claims.sub)
    .bind(&project_name)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err(format!("Manual project not found: {}", project_name));
    }

    Ok("ok".to_string())
}

/// Response for README content
#[derive(Debug, serde::Serialize)]
pub struct ProjectReadmeResponse {
    pub content: Option<String>,
    pub file_name: Option<String>,
}

/// Get the README content for a project
#[tauri::command]
pub async fn get_project_readme(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
) -> Result<ProjectReadmeResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Get git_repo_path from project_preferences first
    let pref: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT project_path, git_repo_path FROM project_preferences WHERE user_id = ? AND project_name = ?",
    )
    .bind(&claims.sub)
    .bind(&project_name)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut project_path: Option<String> = None;

    if let Some((pp, grp)) = pref {
        // Prefer git_repo_path, then project_path
        project_path = grp.or(pp);
    }

    // Fall back to work items if no preference
    if project_path.is_none() {
        let items: Vec<WorkItem> = sqlx::query_as(
            "SELECT * FROM work_items WHERE user_id = ? ORDER BY date DESC LIMIT 100",
        )
        .bind(&claims.sub)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

        project_path = items
            .iter()
            .filter(|item| derive_project_name(item) == project_name)
            .find_map(|item| item.project_path.clone());
    }

    let Some(path) = project_path else {
        return Ok(ProjectReadmeResponse {
            content: None,
            file_name: None,
        });
    };

    // Find the git root directory
    let mut dir = std::path::PathBuf::from(&path);
    loop {
        let git_dir = dir.join(".git");
        if git_dir.exists() {
            break;
        }
        if !dir.pop() {
            // No .git found, use original path
            dir = std::path::PathBuf::from(&path);
            break;
        }
    }

    // Try to find README file with various naming conventions
    let readme_names = [
        "README.md",
        "readme.md",
        "README.MD",
        "Readme.md",
        "README",
        "readme",
        "README.txt",
        "readme.txt",
        "README.rst",
        "readme.rst",
    ];

    for name in readme_names {
        let readme_path = dir.join(name);
        if readme_path.exists() && readme_path.is_file() {
            match std::fs::read_to_string(&readme_path) {
                Ok(content) => {
                    return Ok(ProjectReadmeResponse {
                        content: Some(content),
                        file_name: Some(name.to_string()),
                    });
                }
                Err(e) => {
                    log::warn!("Failed to read README file {:?}: {}", readme_path, e);
                    continue;
                }
            }
        }
    }

    // No README found
    Ok(ProjectReadmeResponse {
        content: None,
        file_name: None,
    })
}
