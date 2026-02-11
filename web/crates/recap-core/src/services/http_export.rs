//! Generic HTTP Export service
//!
//! Template engine + HTTP client for exporting work items
//! to arbitrary external APIs.

use anyhow::{anyhow, Result};
use reqwest::{header, Client, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ── Template Engine ──────────────────────────────────────────

/// Available template fields and their descriptions
pub fn available_fields() -> Vec<(&'static str, &'static str)> {
    vec![
        ("title", "Work item title"),
        ("description", "Work item description"),
        ("hours", "Hours worked (number)"),
        ("date", "Date (YYYY-MM-DD)"),
        ("source", "Data source (e.g. claude_code, git)"),
        ("jira_issue_key", "Jira issue key"),
        ("project_name", "Project name"),
        ("category", "Work item category"),
        ("llm_summary", "LLM-generated summary"),
    ]
}

/// Result of template validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResult {
    pub valid: bool,
    pub fields_used: Vec<String>,
    pub sample_output: Option<String>,
    pub error: Option<String>,
}

/// Render a JSON template by replacing `{{field}}` placeholders with values.
///
/// - String values are JSON-escaped and quoted in the output.
/// - Number values are inserted raw.
/// - Missing or null values become `""` for strings or `null` for numbers.
pub fn render_template(template: &str, item: &serde_json::Value) -> Result<String> {
    let mut result = template.to_string();

    // Find all {{...}} placeholders
    let fields = extract_placeholders(template);

    for field in &fields {
        let placeholder = format!("{{{{{}}}}}", field);
        let in_string = is_inside_json_string(template, &placeholder);
        let replacement = match item.get(field) {
            Some(serde_json::Value::String(s)) => {
                if in_string {
                    json_escape_string(s)
                } else {
                    format!("\"{}\"", json_escape_string(s))
                }
            }
            Some(serde_json::Value::Number(n)) => {
                if in_string {
                    // Inside a string: render as text (e.g., "3.5")
                    n.to_string()
                } else {
                    // Outside a string: render as raw JSON number
                    n.to_string()
                }
            }
            Some(serde_json::Value::Bool(b)) => b.to_string(),
            Some(serde_json::Value::Null) | None => {
                if in_string {
                    String::new()
                } else {
                    "null".to_string()
                }
            }
            _ => String::new(),
        };

        result = result.replace(&placeholder, &replacement);
    }

    // Validate the result is valid JSON
    serde_json::from_str::<serde_json::Value>(&result)
        .map_err(|e| anyhow!("Rendered template is not valid JSON: {}", e))?;

    Ok(result)
}

/// Validate a template and return sample output
pub fn validate_template(template: &str) -> ValidateResult {
    let fields = extract_placeholders(template);

    // Build sample data
    let sample = serde_json::json!({
        "title": "修改登入頁面 UI",
        "description": "調整登入表單樣式，新增忘記密碼連結",
        "hours": 2.5,
        "date": "2026-02-11",
        "source": "claude_code",
        "jira_issue_key": "PROJ-42",
        "project_name": "recap",
        "category": "development",
        "llm_summary": "UI adjustment for login page"
    });

    match render_template(template, &sample) {
        Ok(output) => ValidateResult {
            valid: true,
            fields_used: fields,
            sample_output: Some(output),
            error: None,
        },
        Err(e) => ValidateResult {
            valid: false,
            fields_used: fields,
            sample_output: None,
            error: Some(e.to_string()),
        },
    }
}

/// Extract all `{{field}}` placeholder names from a template
fn extract_placeholders(template: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        if let Some(end) = rest[start + 2..].find("}}") {
            let field = rest[start + 2..start + 2 + end].trim().to_string();
            if !field.is_empty() && !fields.contains(&field) {
                fields.push(field);
            }
            rest = &rest[start + 2 + end + 2..];
        } else {
            break;
        }
    }

    fields
}

/// Check if a placeholder appears inside a JSON string value
/// by counting unescaped quotes before the placeholder position.
/// Odd count = inside string, even count = outside string.
fn is_inside_json_string(template: &str, placeholder: &str) -> bool {
    if let Some(pos) = template.find(placeholder) {
        let before = &template[..pos];
        let mut quote_count = 0;
        let bytes = before.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'"' {
                // Check it's not escaped
                if i == 0 || bytes[i - 1] != b'\\' {
                    quote_count += 1;
                }
            }
            i += 1;
        }
        // Odd number of quotes means we're inside a string
        quote_count % 2 == 1
    } else {
        false
    }
}

/// Escape a string for inclusion in a JSON string value
fn json_escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// ── HTTP Export Client ───────────────────────────────────────

/// Configuration for an HTTP export endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpExportConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    pub method: String,
    pub auth_type: String,
    pub auth_token: Option<String>,
    pub auth_header_name: Option<String>,
    pub custom_headers: Option<String>,
    pub payload_template: String,
    pub llm_prompt: Option<String>,
    pub batch_mode: bool,
    pub batch_wrapper_key: String,
    pub timeout_seconds: u32,
}

/// Result of exporting a single item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportItemResult {
    pub work_item_id: String,
    pub work_item_title: String,
    pub status: String, // "success" | "error" | "dry_run"
    pub http_status: Option<u16>,
    pub error_message: Option<String>,
    pub payload_preview: Option<String>,
}

/// Result of an export batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBatchResult {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<ExportItemResult>,
    pub dry_run: bool,
}

/// Test connection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub http_status: Option<u16>,
    pub message: String,
}

/// HTTP export client
pub struct HttpExportClient {
    config: HttpExportConfig,
    client: Client,
}

impl HttpExportClient {
    /// Create a new HTTP export client from config
    pub fn new(config: HttpExportConfig) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );

        // Set auth headers
        match config.auth_type.as_str() {
            "bearer" => {
                if let Some(ref token) = config.auth_token {
                    headers.insert(
                        header::AUTHORIZATION,
                        header::HeaderValue::from_str(&format!("Bearer {}", token))
                            .map_err(|e| anyhow!("Invalid bearer token: {}", e))?,
                    );
                }
            }
            "basic" => {
                if let Some(ref token) = config.auth_token {
                    let encoded = base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        token.as_bytes(),
                    );
                    headers.insert(
                        header::AUTHORIZATION,
                        header::HeaderValue::from_str(&format!("Basic {}", encoded))
                            .map_err(|e| anyhow!("Invalid basic auth: {}", e))?,
                    );
                }
            }
            "header" => {
                if let (Some(ref name), Some(ref value)) =
                    (&config.auth_header_name, &config.auth_token)
                {
                    headers.insert(
                        header::HeaderName::from_bytes(name.as_bytes())
                            .map_err(|e| anyhow!("Invalid header name: {}", e))?,
                        header::HeaderValue::from_str(value)
                            .map_err(|e| anyhow!("Invalid header value: {}", e))?,
                    );
                }
            }
            _ => {} // "none"
        }

        // Parse custom headers
        if let Some(ref custom) = config.custom_headers {
            if !custom.trim().is_empty() {
                if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(custom) {
                    for (k, v) in &map {
                        if let (Ok(name), Ok(value)) = (
                            header::HeaderName::from_bytes(k.as_bytes()),
                            header::HeaderValue::from_str(v),
                        ) {
                            headers.insert(name, value);
                        }
                    }
                }
            }
        }

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(config.timeout_seconds as u64))
            .build()?;

        Ok(Self { config, client })
    }

    /// Export a list of items (rendered as JSON values)
    pub async fn export_items(
        &self,
        items: &[(String, String, serde_json::Value)], // (id, title, rendered_payload)
        dry_run: bool,
    ) -> ExportBatchResult {
        if dry_run {
            let results: Vec<ExportItemResult> = items
                .iter()
                .map(|(id, title, payload)| ExportItemResult {
                    work_item_id: id.clone(),
                    work_item_title: title.clone(),
                    status: "dry_run".to_string(),
                    http_status: None,
                    error_message: None,
                    payload_preview: Some(payload.to_string()),
                })
                .collect();
            return ExportBatchResult {
                total: results.len(),
                successful: results.len(),
                failed: 0,
                results,
                dry_run: true,
            };
        }

        let method = match self.config.method.to_uppercase().as_str() {
            "PUT" => Method::PUT,
            "PATCH" => Method::PATCH,
            _ => Method::POST,
        };

        if self.config.batch_mode {
            self.export_batch(&method, items).await
        } else {
            self.export_individually(&method, items).await
        }
    }

    /// Export items one by one
    async fn export_individually(
        &self,
        method: &Method,
        items: &[(String, String, serde_json::Value)],
    ) -> ExportBatchResult {
        let mut results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;

        for (id, title, payload) in items {
            match self
                .client
                .request(method.clone(), &self.config.url)
                .json(payload)
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status().as_u16();
                    if response.status().is_success() {
                        successful += 1;
                        results.push(ExportItemResult {
                            work_item_id: id.clone(),
                            work_item_title: title.clone(),
                            status: "success".to_string(),
                            http_status: Some(status),
                            error_message: None,
                            payload_preview: Some(payload.to_string()),
                        });
                    } else {
                        let body = response
                            .text()
                            .await
                            .unwrap_or_default()
                            .chars()
                            .take(2000)
                            .collect::<String>();
                        failed += 1;
                        results.push(ExportItemResult {
                            work_item_id: id.clone(),
                            work_item_title: title.clone(),
                            status: "error".to_string(),
                            http_status: Some(status),
                            error_message: Some(format!("HTTP {}: {}", status, body)),
                            payload_preview: Some(payload.to_string()),
                        });
                    }
                }
                Err(e) => {
                    failed += 1;
                    results.push(ExportItemResult {
                        work_item_id: id.clone(),
                        work_item_title: title.clone(),
                        status: "error".to_string(),
                        http_status: None,
                        error_message: Some(e.to_string()),
                        payload_preview: Some(payload.to_string()),
                    });
                }
            }
        }

        ExportBatchResult {
            total: items.len(),
            successful,
            failed,
            results,
            dry_run: false,
        }
    }

    /// Export items as a single batch array
    async fn export_batch(
        &self,
        method: &Method,
        items: &[(String, String, serde_json::Value)],
    ) -> ExportBatchResult {
        let payloads: Vec<&serde_json::Value> = items.iter().map(|(_, _, p)| p).collect();
        let batch_payload =
            serde_json::json!({ &self.config.batch_wrapper_key: payloads });

        match self
            .client
            .request(method.clone(), &self.config.url)
            .json(&batch_payload)
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                if response.status().is_success() {
                    let results: Vec<ExportItemResult> = items
                        .iter()
                        .map(|(id, title, payload)| ExportItemResult {
                            work_item_id: id.clone(),
                            work_item_title: title.clone(),
                            status: "success".to_string(),
                            http_status: Some(status),
                            error_message: None,
                            payload_preview: Some(payload.to_string()),
                        })
                        .collect();
                    ExportBatchResult {
                        total: items.len(),
                        successful: items.len(),
                        failed: 0,
                        results,
                        dry_run: false,
                    }
                } else {
                    let body = response
                        .text()
                        .await
                        .unwrap_or_default()
                        .chars()
                        .take(2000)
                        .collect::<String>();
                    let err_msg = format!("HTTP {}: {}", status, body);
                    let results: Vec<ExportItemResult> = items
                        .iter()
                        .map(|(id, title, payload)| ExportItemResult {
                            work_item_id: id.clone(),
                            work_item_title: title.clone(),
                            status: "error".to_string(),
                            http_status: Some(status),
                            error_message: Some(err_msg.clone()),
                            payload_preview: Some(payload.to_string()),
                        })
                        .collect();
                    ExportBatchResult {
                        total: items.len(),
                        successful: 0,
                        failed: items.len(),
                        results,
                        dry_run: false,
                    }
                }
            }
            Err(e) => {
                let err_msg = e.to_string();
                let results: Vec<ExportItemResult> = items
                    .iter()
                    .map(|(id, title, payload)| ExportItemResult {
                        work_item_id: id.clone(),
                        work_item_title: title.clone(),
                        status: "error".to_string(),
                        http_status: None,
                        error_message: Some(err_msg.clone()),
                        payload_preview: Some(payload.to_string()),
                    })
                    .collect();
                ExportBatchResult {
                    total: items.len(),
                    successful: 0,
                    failed: items.len(),
                    results,
                    dry_run: false,
                }
            }
        }
    }

    /// Test connection by sending a small test payload
    pub async fn test_connection(&self) -> TestConnectionResult {
        let sample = serde_json::json!({"test": true, "source": "recap"});

        let method = match self.config.method.to_uppercase().as_str() {
            "PUT" => Method::PUT,
            "PATCH" => Method::PATCH,
            _ => Method::POST,
        };

        match self
            .client
            .request(method, &self.config.url)
            .json(&sample)
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                if response.status().is_success() {
                    TestConnectionResult {
                        success: true,
                        http_status: Some(status),
                        message: format!("Connected successfully (HTTP {})", status),
                    }
                } else {
                    let body = response
                        .text()
                        .await
                        .unwrap_or_default()
                        .chars()
                        .take(500)
                        .collect::<String>();
                    TestConnectionResult {
                        success: false,
                        http_status: Some(status),
                        message: format!("HTTP {}: {}", status, body),
                    }
                }
            }
            Err(e) => TestConnectionResult {
                success: false,
                http_status: None,
                message: format!("Connection failed: {}", e),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_placeholders() {
        let tpl = r#"{"title": "{{title}}", "hours": {{hours}}, "note": "{{llm_summary}}"}"#;
        let fields = extract_placeholders(tpl);
        assert_eq!(fields, vec!["title", "hours", "llm_summary"]);
    }

    #[test]
    fn test_extract_placeholders_duplicates() {
        let tpl = r#"{"a": "{{title}}", "b": "{{title}}"}"#;
        let fields = extract_placeholders(tpl);
        assert_eq!(fields, vec!["title"]);
    }

    #[test]
    fn test_render_template_basic() {
        let tpl = r#"{"summary": "{{title}}", "hours": {{hours}}}"#;
        let item = serde_json::json!({
            "title": "Fix login",
            "hours": 2.5
        });
        let result = render_template(tpl, &item).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["summary"], "Fix login");
        assert_eq!(parsed["hours"], 2.5);
    }

    #[test]
    fn test_render_template_missing_field() {
        let tpl = r#"{"summary": "{{title}}", "key": "{{jira_issue_key}}"}"#;
        let item = serde_json::json!({
            "title": "Test"
        });
        let result = render_template(tpl, &item).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["summary"], "Test");
        assert_eq!(parsed["key"], "");
    }

    #[test]
    fn test_render_template_special_chars() {
        let tpl = r#"{"summary": "{{title}}"}"#;
        let item = serde_json::json!({
            "title": "Fix \"quotes\" and\nnewline"
        });
        let result = render_template(tpl, &item).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["summary"], "Fix \"quotes\" and\nnewline");
    }

    #[test]
    fn test_validate_template_valid() {
        let tpl = r#"{"summary": "{{title}}", "hours": {{hours}}}"#;
        let result = validate_template(tpl);
        assert!(result.valid);
        assert_eq!(result.fields_used, vec!["title", "hours"]);
        assert!(result.sample_output.is_some());
    }

    #[test]
    fn test_validate_template_invalid_json() {
        let tpl = r#"{"summary": {{title}}}"#; // title is string, not quoted
        let result = validate_template(tpl);
        // This might be valid or invalid depending on sample data (title is string)
        // Since title renders as a quoted string, it's actually valid
        assert!(result.valid);
    }

    #[test]
    fn test_render_template_mixed_placeholders_in_string() {
        // This is the real-world template: multiple placeholders inside one string value
        let tpl = r#"{
  "date": "{{date}}",
  "content": "{{title}} ({{hours}}h) - {{description}}"
}"#;
        let item = serde_json::json!({
            "title": "trip-agent",
            "hours": 3.0,
            "date": "2026-02-11",
            "description": "Build travel planning agent"
        });
        let result = render_template(tpl, &item).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["date"], "2026-02-11");
        assert_eq!(parsed["content"], "trip-agent (3.0h) - Build travel planning agent");
    }

    #[test]
    fn test_available_fields() {
        let fields = available_fields();
        assert!(fields.len() >= 9);
        assert!(fields.iter().any(|(name, _)| *name == "title"));
        assert!(fields.iter().any(|(name, _)| *name == "llm_summary"));
    }

    #[test]
    fn test_json_escape_string() {
        assert_eq!(json_escape_string("hello"), "hello");
        assert_eq!(json_escape_string("a\"b"), "a\\\"b");
        assert_eq!(json_escape_string("a\nb"), "a\\nb");
    }
}
