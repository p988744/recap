//! HTTP Export commands
//!
//! Tauri commands for generic HTTP export configuration and execution.

use serde::{Deserialize, Serialize};
use tauri::State;

use recap_core::auth::verify_token;
use recap_core::services::http_export::{
    self, HttpExportClient, HttpExportConfig,
};
use recap_core::services::llm::{create_llm_service, parse_error_usage};
use recap_core::services::llm_usage::save_usage_log;

use super::AppState;

// ── Types ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub id: String,
    pub name: String,
    pub url: String,
    pub method: String,
    pub auth_type: String,
    // NOTE: auth_token is intentionally omitted for security
    pub auth_header_name: Option<String>,
    pub custom_headers: Option<String>,
    pub payload_template: String,
    pub llm_prompt: Option<String>,
    pub batch_mode: bool,
    pub batch_wrapper_key: String,
    pub enabled: bool,
    pub timeout_seconds: i64,
}

#[derive(Debug, Deserialize)]
pub struct SaveConfigRequest {
    pub id: Option<String>,
    pub name: String,
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_auth_type")]
    pub auth_type: String,
    pub auth_token: Option<String>,
    pub auth_header_name: Option<String>,
    pub custom_headers: Option<String>,
    pub payload_template: String,
    pub llm_prompt: Option<String>,
    #[serde(default)]
    pub batch_mode: bool,
    #[serde(default = "default_batch_wrapper_key")]
    pub batch_wrapper_key: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: Option<i64>,
}

fn default_method() -> String {
    "POST".to_string()
}
fn default_auth_type() -> String {
    "none".to_string()
}
fn default_batch_wrapper_key() -> Option<String> {
    Some("items".to_string())
}
fn default_timeout() -> Option<i64> {
    Some(30)
}

#[derive(Debug, Deserialize)]
pub struct InlineWorkItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub hours: f64,
    pub date: String,
    pub source: String,
    pub jira_issue_key: Option<String>,
    pub category: Option<String>,
    pub project_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteExportRequest {
    pub config_id: String,
    pub work_item_ids: Vec<String>,
    /// Optional inline work items — used when items don't exist in DB (e.g. Worklog page)
    pub inline_items: Option<Vec<InlineWorkItem>>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub struct ExportItemResult {
    pub work_item_id: String,
    pub work_item_title: String,
    pub status: String,
    pub http_status: Option<u16>,
    pub error_message: Option<String>,
    pub payload_preview: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<ExportItemResult>,
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub http_status: Option<u16>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateTemplateResponse {
    pub valid: bool,
    pub fields_used: Vec<String>,
    pub sample_output: Option<String>,
    pub error: Option<String>,
}

// ── Commands ─────────────────────────────────────────────────

/// List all HTTP export configs for the current user
#[tauri::command]
pub async fn list_http_export_configs(
    state: State<'_, AppState>,
    token: String,
) -> Result<Vec<ConfigResponse>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    log::info!("[http_export] list_http_export_configs called for user: {}", claims.sub);

    let rows = sqlx::query_as::<_, (
        String,  // id
        String,  // name
        String,  // url
        String,  // method
        String,  // auth_type
        Option<String>, // auth_header_name
        Option<String>, // custom_headers
        String,  // payload_template
        Option<String>, // llm_prompt
        bool,    // batch_mode
        Option<String>, // batch_wrapper_key
        bool,    // enabled
        i64,     // timeout_seconds
    )>(
        r#"SELECT id, name, url, method, auth_type, auth_header_name,
                  custom_headers, payload_template, llm_prompt, batch_mode,
                  batch_wrapper_key, enabled, timeout_seconds
           FROM http_export_configs
           WHERE user_id = ?
           ORDER BY created_at ASC"#,
    )
    .bind(&claims.sub)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    log::info!("[http_export] found {} configs", rows.len());

    Ok(rows
        .into_iter()
        .map(|r| ConfigResponse {
            id: r.0,
            name: r.1,
            url: r.2,
            method: r.3,
            auth_type: r.4,
            auth_header_name: r.5,
            custom_headers: r.6,
            payload_template: r.7,
            llm_prompt: r.8,
            batch_mode: r.9,
            batch_wrapper_key: r.10.unwrap_or_else(|| "items".to_string()),
            enabled: r.11,
            timeout_seconds: r.12,
        })
        .collect())
}

/// Save (create or update) an HTTP export config
#[tauri::command]
pub async fn save_http_export_config(
    state: State<'_, AppState>,
    token: String,
    request: SaveConfigRequest,
) -> Result<MessageResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let config_id = request.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let batch_wrapper_key = request.batch_wrapper_key.unwrap_or_else(|| "items".to_string());
    let timeout = request.timeout_seconds.unwrap_or(30);

    sqlx::query(
        r#"INSERT INTO http_export_configs
           (id, user_id, name, url, method, auth_type, auth_token,
            auth_header_name, custom_headers, payload_template, llm_prompt,
            batch_mode, batch_wrapper_key, timeout_seconds, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
           ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             url = excluded.url,
             method = excluded.method,
             auth_type = excluded.auth_type,
             auth_token = COALESCE(excluded.auth_token, http_export_configs.auth_token),
             auth_header_name = excluded.auth_header_name,
             custom_headers = excluded.custom_headers,
             payload_template = excluded.payload_template,
             llm_prompt = excluded.llm_prompt,
             batch_mode = excluded.batch_mode,
             batch_wrapper_key = excluded.batch_wrapper_key,
             timeout_seconds = excluded.timeout_seconds,
             updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(&config_id)
    .bind(&claims.sub)
    .bind(&request.name)
    .bind(&request.url)
    .bind(&request.method)
    .bind(&request.auth_type)
    .bind(&request.auth_token)
    .bind(&request.auth_header_name)
    .bind(&request.custom_headers)
    .bind(&request.payload_template)
    .bind(&request.llm_prompt)
    .bind(request.batch_mode)
    .bind(&batch_wrapper_key)
    .bind(timeout)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(MessageResponse {
        success: true,
        message: "Config saved".to_string(),
    })
}

/// Delete an HTTP export config
#[tauri::command]
pub async fn delete_http_export_config(
    state: State<'_, AppState>,
    token: String,
    config_id: String,
) -> Result<MessageResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    sqlx::query("DELETE FROM http_export_configs WHERE id = ? AND user_id = ?")
        .bind(&config_id)
        .bind(&claims.sub)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(MessageResponse {
        success: true,
        message: "Config deleted".to_string(),
    })
}

/// Execute HTTP export for a set of work items
#[tauri::command]
pub async fn execute_http_export(
    state: State<'_, AppState>,
    token: String,
    request: ExecuteExportRequest,
) -> Result<ExportResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    // Load config (including auth_token from DB)
    let row = sqlx::query_as::<_, (
        String, String, String, String, String, Option<String>, Option<String>,
        Option<String>, String, Option<String>, bool, Option<String>, i64,
    )>(
        r#"SELECT id, name, url, method, auth_type, auth_token,
                  auth_header_name, custom_headers, payload_template, llm_prompt,
                  batch_mode, batch_wrapper_key, timeout_seconds
           FROM http_export_configs
           WHERE id = ? AND user_id = ?"#,
    )
    .bind(&request.config_id)
    .bind(&claims.sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Config not found".to_string())?;

    let config = HttpExportConfig {
        id: row.0.clone(),
        name: row.1.clone(),
        url: row.2,
        method: row.3,
        auth_type: row.4,
        auth_token: row.5,
        auth_header_name: row.6,
        custom_headers: row.7,
        payload_template: row.8.clone(),
        llm_prompt: row.9.clone(),
        batch_mode: row.10,
        batch_wrapper_key: row.11.unwrap_or_else(|| "items".to_string()),
        timeout_seconds: row.12 as u32,
    };

    // Load work items — use inline items if provided, otherwise query DB
    let work_items: Vec<(
        String, String, Option<String>, f64, String, String,
        Option<String>, Option<String>, Option<String>,
    )> = if let Some(inline) = request.inline_items {
        inline.into_iter().map(|item| (
            item.id,
            item.title,
            item.description,
            item.hours,
            item.date,
            item.source,
            item.jira_issue_key,
            item.category,
            item.project_name,
        )).collect()
    } else {
        let placeholders = request
            .work_item_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query_str = format!(
            r#"SELECT id, title, description, hours, date, source,
                      jira_issue_key, category, project_path
               FROM work_items
               WHERE id IN ({}) AND user_id = ?"#,
            placeholders
        );

        let mut query = sqlx::query_as::<_, (
            String, String, Option<String>, f64, String, String,
            Option<String>, Option<String>, Option<String>,
        )>(&query_str);
        for id in &request.work_item_ids {
            query = query.bind(id);
        }
        query = query.bind(&claims.sub);

        query
            .fetch_all(&db.pool)
            .await
            .map_err(|e| e.to_string())?
    };

    // Optionally generate LLM summaries
    let mut llm_summaries: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if config.llm_prompt.is_some() {
        let llm = create_llm_service(&db.pool, &claims.sub).await.ok();
        if let Some(ref llm) = llm {
            if llm.is_configured() {
                for item in &work_items {
                    let prompt_data = serde_json::json!({
                        "title": item.1,
                        "description": item.2.as_deref().unwrap_or(""),
                        "hours": item.3,
                        "date": item.4,
                        "source": item.5,
                        "jira_issue_key": item.6.as_deref().unwrap_or(""),
                        "category": item.7.as_deref().unwrap_or(""),
                        "project_name": extract_project_name(item.8.as_deref()),
                    });

                    let prompt_template = config.llm_prompt.as_deref().unwrap_or("");
                    if let Ok(rendered_prompt) = http_export::render_template(
                        &format!("\"{}\"", prompt_template.replace('"', "\\\"")),
                        &prompt_data,
                    ) {
                        // Strip the surrounding quotes from the rendered result
                        let clean_prompt = rendered_prompt
                            .trim_start_matches('"')
                            .trim_end_matches('"')
                            .replace("\\\"", "\"")
                            .replace("\\n", "\n");

                        match llm.complete_with_usage(&clean_prompt, "http_export_summary", 1000).await {
                            Ok((summary, usage)) => {
                                let _ = save_usage_log(&db.pool, &claims.sub, &usage).await;
                                llm_summaries.insert(item.0.clone(), summary);
                            }
                            Err(err) => {
                                if let Some(usage) = parse_error_usage(&err) {
                                    let _ = save_usage_log(&db.pool, &claims.sub, &usage).await;
                                }
                                log::warn!("LLM summary failed for {}: {}", item.0, err);
                            }
                        }
                    }
                }
            }
        }
    }

    // Render payloads
    let mut rendered_items: Vec<(String, String, serde_json::Value)> = Vec::new();
    let mut render_errors: Vec<ExportItemResult> = Vec::new();

    for item in &work_items {
        let data = serde_json::json!({
            "title": item.1,
            "description": item.2.as_deref().unwrap_or(""),
            "hours": item.3,
            "date": item.4,
            "source": item.5,
            "jira_issue_key": item.6.as_deref().unwrap_or(""),
            "category": item.7.as_deref().unwrap_or(""),
            "project_name": extract_project_name(item.8.as_deref()),
            "llm_summary": llm_summaries.get(&item.0).cloned().unwrap_or_default(),
        });

        match http_export::render_template(&config.payload_template, &data) {
            Ok(rendered) => {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&rendered) {
                    rendered_items.push((item.0.clone(), item.1.clone(), payload));
                } else {
                    render_errors.push(ExportItemResult {
                        work_item_id: item.0.clone(),
                        work_item_title: item.1.clone(),
                        status: "error".to_string(),
                        http_status: None,
                        error_message: Some("Failed to parse rendered payload as JSON".to_string()),
                        payload_preview: Some(rendered),
                    });
                }
            }
            Err(e) => {
                render_errors.push(ExportItemResult {
                    work_item_id: item.0.clone(),
                    work_item_title: item.1.clone(),
                    status: "error".to_string(),
                    http_status: None,
                    error_message: Some(format!("Template render error: {}", e)),
                    payload_preview: None,
                });
            }
        }
    }

    // Execute HTTP export
    let client = HttpExportClient::new(config.clone()).map_err(|e| e.to_string())?;
    let mut batch_result = client.export_items(&rendered_items, request.dry_run).await;

    // Merge render errors into results
    batch_result.failed += render_errors.len();
    batch_result.total += render_errors.len();
    batch_result.results.extend(render_errors.into_iter().map(|e| {
        http_export::ExportItemResult {
            work_item_id: e.work_item_id,
            work_item_title: e.work_item_title,
            status: e.status,
            http_status: e.http_status,
            error_message: e.error_message,
            payload_preview: e.payload_preview,
        }
    }));

    // Save export logs
    for r in &batch_result.results {
        let _ = sqlx::query(
            r#"INSERT INTO http_export_logs
               (id, user_id, config_id, config_name, work_item_id, status,
                http_status, response_body, error_message, payload_sent)
               VALUES (?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)"#,
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&claims.sub)
        .bind(&request.config_id)
        .bind(&row.1)
        .bind(&r.work_item_id)
        .bind(&r.status)
        .bind(r.http_status.map(|s| s as i64))
        .bind(&r.error_message)
        .bind(&r.payload_preview)
        .execute(&db.pool)
        .await;
    }

    Ok(ExportResponse {
        total: batch_result.total,
        successful: batch_result.successful,
        failed: batch_result.failed,
        results: batch_result
            .results
            .into_iter()
            .map(|r| ExportItemResult {
                work_item_id: r.work_item_id,
                work_item_title: r.work_item_title,
                status: r.status,
                http_status: r.http_status,
                error_message: r.error_message,
                payload_preview: r.payload_preview,
            })
            .collect(),
        dry_run: batch_result.dry_run,
    })
}

/// Test HTTP export connection
#[tauri::command]
pub async fn test_http_export_connection(
    state: State<'_, AppState>,
    token: String,
    config_id: String,
) -> Result<TestConnectionResponse, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let row = sqlx::query_as::<_, (
        String, String, String, String, String, Option<String>, Option<String>,
        Option<String>, String, Option<String>, bool, Option<String>, i64,
    )>(
        r#"SELECT id, name, url, method, auth_type, auth_token,
                  auth_header_name, custom_headers, payload_template, llm_prompt,
                  batch_mode, batch_wrapper_key, timeout_seconds
           FROM http_export_configs
           WHERE id = ? AND user_id = ?"#,
    )
    .bind(&config_id)
    .bind(&claims.sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Config not found".to_string())?;

    let config = HttpExportConfig {
        id: row.0,
        name: row.1,
        url: row.2,
        method: row.3,
        auth_type: row.4,
        auth_token: row.5,
        auth_header_name: row.6,
        custom_headers: row.7,
        payload_template: row.8,
        llm_prompt: row.9,
        batch_mode: row.10,
        batch_wrapper_key: row.11.unwrap_or_else(|| "items".to_string()),
        timeout_seconds: row.12 as u32,
    };

    let client = HttpExportClient::new(config).map_err(|e| e.to_string())?;
    let result = client.test_connection().await;

    Ok(TestConnectionResponse {
        success: result.success,
        http_status: result.http_status,
        message: result.message,
    })
}

/// Validate a payload template
#[tauri::command]
pub async fn validate_http_export_template(
    _state: State<'_, AppState>,
    token: String,
    template: String,
) -> Result<ValidateTemplateResponse, String> {
    let _claims = verify_token(&token).map_err(|e| e.to_string())?;

    let result = http_export::validate_template(&template);

    Ok(ValidateTemplateResponse {
        valid: result.valid,
        fields_used: result.fields_used,
        sample_output: result.sample_output,
        error: result.error,
    })
}

/// Response for export history
#[derive(Debug, Serialize)]
pub struct ExportHistoryRecord {
    pub work_item_id: String,
    pub exported_at: String,
}

/// Get export history for a config — which items have been successfully exported
#[tauri::command]
pub async fn get_http_export_history(
    state: State<'_, AppState>,
    token: String,
    config_id: String,
    work_item_ids: Vec<String>,
) -> Result<Vec<ExportHistoryRecord>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    if work_item_ids.is_empty() {
        return Ok(vec![]);
    }

    let placeholders = work_item_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query_str = format!(
        r#"SELECT work_item_id, MAX(created_at) as last_exported
           FROM http_export_logs
           WHERE config_id = ? AND user_id = ? AND status = 'success'
             AND work_item_id IN ({})
           GROUP BY work_item_id"#,
        placeholders
    );

    let mut query = sqlx::query_as::<_, (String, String)>(&query_str);
    query = query.bind(&config_id).bind(&claims.sub);
    for id in &work_item_ids {
        query = query.bind(id);
    }

    let rows = query.fetch_all(&db.pool).await.map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| ExportHistoryRecord {
            work_item_id: r.0,
            exported_at: r.1,
        })
        .collect())
}

/// Extract project name from project_path
fn extract_project_name(path: Option<&str>) -> String {
    path.and_then(|p| {
        std::path::Path::new(p)
            .file_name()
            .and_then(|n| n.to_str())
            .map(String::from)
    })
    .unwrap_or_default()
}
