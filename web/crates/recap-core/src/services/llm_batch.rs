//! OpenAI Batch API Service
//!
//! Handles batch processing for LLM requests, providing 50% cost savings
//! with 24-hour turnaround for non-time-sensitive workloads.
//!
//! Workflow:
//! 1. Collect pending hourly summaries
//! 2. Create JSONL file with batch requests
//! 3. Upload file and create batch job
//! 4. Poll for completion
//! 5. Download results and save summaries

use chrono::{DateTime, Utc};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::llm::LlmConfig;

// ============================================================================
// Types
// ============================================================================

/// Status of a batch job
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchJobStatus {
    Pending,      // Not yet submitted to OpenAI
    Submitted,    // Submitted, waiting for processing
    InProgress,   // Being processed by OpenAI
    Completed,    // Successfully completed
    Failed,       // Failed with error
    Cancelled,    // Cancelled by user
    Expired,      // Expired (24h limit)
}

impl std::fmt::Display for BatchJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchJobStatus::Pending => write!(f, "pending"),
            BatchJobStatus::Submitted => write!(f, "submitted"),
            BatchJobStatus::InProgress => write!(f, "in_progress"),
            BatchJobStatus::Completed => write!(f, "completed"),
            BatchJobStatus::Failed => write!(f, "failed"),
            BatchJobStatus::Cancelled => write!(f, "cancelled"),
            BatchJobStatus::Expired => write!(f, "expired"),
        }
    }
}

impl From<&str> for BatchJobStatus {
    fn from(s: &str) -> Self {
        match s {
            "pending" => BatchJobStatus::Pending,
            "submitted" => BatchJobStatus::Submitted,
            "in_progress" | "validating" | "finalizing" => BatchJobStatus::InProgress,
            "completed" => BatchJobStatus::Completed,
            "failed" => BatchJobStatus::Failed,
            "cancelled" => BatchJobStatus::Cancelled,
            "expired" => BatchJobStatus::Expired,
            _ => BatchJobStatus::Pending,
        }
    }
}

/// A batch job record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BatchJob {
    pub id: String,
    pub user_id: String,
    pub openai_batch_id: Option<String>,
    pub status: String,
    pub purpose: String,
    pub total_requests: i64,
    pub completed_requests: i64,
    pub failed_requests: i64,
    pub input_file_id: Option<String>,
    pub output_file_id: Option<String>,
    pub error_file_id: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// A batch request record (maps to a single hourly compaction)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BatchRequest {
    pub id: String,
    pub batch_job_id: String,
    pub custom_id: String,
    pub project_path: String,
    pub hour_bucket: String,
    pub prompt: String,
    pub status: String,
    pub response: Option<String>,
    pub error_message: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// OpenAI Batch API request line (JSONL format)
#[derive(Debug, Serialize)]
struct BatchRequestLine {
    custom_id: String,
    method: String,
    url: String,
    body: serde_json::Value,
}

/// Batch request body for models that don't support temperature (gpt-5 series, o1, o3)
#[derive(Debug, Serialize)]
struct BatchRequestBodyNewNoTemp {
    model: String,
    messages: Vec<ChatMessage>,
    max_completion_tokens: u32,
}

/// Batch request body for newer models (gpt-4.1, gpt-4o) with temperature
#[derive(Debug, Serialize)]
struct BatchRequestBodyNew {
    model: String,
    messages: Vec<ChatMessage>,
    max_completion_tokens: u32,
    temperature: f32,
}

/// Batch request body for legacy models (gpt-4-turbo, gpt-4, gpt-3.5)
#[derive(Debug, Serialize)]
struct BatchRequestBodyLegacy {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

/// Check if a model uses the new max_completion_tokens parameter
fn uses_max_completion_tokens(model: &str) -> bool {
    model.starts_with("gpt-5") ||
    model.starts_with("gpt-4.1") ||
    model.starts_with("gpt-4o") ||
    model.starts_with("o1") ||
    model.starts_with("o3")
}

/// Check if a model doesn't support custom temperature
fn no_temperature_support(model: &str) -> bool {
    model.starts_with("gpt-5") ||  // All GPT-5 models (gpt-5, gpt-5-mini, gpt-5-nano)
    model.starts_with("o1") ||
    model.starts_with("o3")
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI file upload response
#[derive(Debug, Deserialize)]
struct FileUploadResponse {
    id: String,
}

/// OpenAI batch create response
#[derive(Debug, Deserialize)]
struct BatchCreateResponse {
    id: String,
    status: String,
}

/// OpenAI batch status response
#[derive(Debug, Deserialize)]
struct BatchStatusResponse {
    id: String,
    status: String,
    request_counts: Option<BatchRequestCounts>,
    output_file_id: Option<String>,
    error_file_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BatchRequestCounts {
    total: i64,
    completed: i64,
    failed: i64,
}

/// OpenAI batch result line (from output file)
#[derive(Debug, Deserialize)]
struct BatchResultLine {
    custom_id: String,
    response: Option<BatchResultResponse>,
    error: Option<BatchResultError>,
}

#[derive(Debug, Deserialize)]
struct BatchResultResponse {
    status_code: i32,
    body: BatchResultBody,
}

#[derive(Debug, Deserialize)]
struct BatchResultBody {
    choices: Vec<BatchResultChoice>,
    usage: Option<BatchResultUsage>,
}

#[derive(Debug, Deserialize)]
struct BatchResultChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct BatchResultUsage {
    prompt_tokens: i64,
    completion_tokens: i64,
}

#[derive(Debug, Deserialize)]
struct BatchResultError {
    code: String,
    message: String,
}

/// Result of submitting a batch job
#[derive(Debug, Clone, Serialize)]
pub struct BatchSubmitResult {
    pub job_id: String,
    pub openai_batch_id: String,
    pub total_requests: usize,
}

/// Result of processing completed batch
#[derive(Debug, Clone, Serialize)]
pub struct BatchProcessResult {
    pub job_id: String,
    pub completed: usize,
    pub failed: usize,
    pub summaries_saved: usize,
}

// ============================================================================
// Service
// ============================================================================

pub struct LlmBatchService {
    config: LlmConfig,
    client: reqwest::Client,
}

impl LlmBatchService {
    pub fn new(config: LlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            client,
        }
    }

    /// Check if batch API is available (only for OpenAI)
    pub fn is_batch_available(&self) -> bool {
        self.config.provider == "openai" && self.config.api_key.is_some()
    }

    /// Create a new batch job for hourly compaction
    pub async fn create_batch_job(
        &self,
        pool: &SqlitePool,
        user_id: &str,
        requests: Vec<HourlyCompactionRequest>,
    ) -> Result<String, String> {
        if requests.is_empty() {
            return Err("No requests to batch".to_string());
        }

        let job_id = Uuid::new_v4().to_string();

        // Insert batch job
        sqlx::query(
            r#"
            INSERT INTO llm_batch_jobs (id, user_id, status, purpose, total_requests)
            VALUES (?, ?, 'pending', 'hourly_compaction', ?)
            "#,
        )
        .bind(&job_id)
        .bind(user_id)
        .bind(requests.len() as i64)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to create batch job: {}", e))?;

        // Insert batch requests
        for (idx, req) in requests.iter().enumerate() {
            let request_id = Uuid::new_v4().to_string();
            let custom_id = format!("hourly-{}-{}", idx, Uuid::new_v4().to_string()[..8].to_string());

            sqlx::query(
                r#"
                INSERT INTO llm_batch_requests
                (id, batch_job_id, custom_id, project_path, hour_bucket, prompt, status)
                VALUES (?, ?, ?, ?, ?, ?, 'pending')
                "#,
            )
            .bind(&request_id)
            .bind(&job_id)
            .bind(&custom_id)
            .bind(&req.project_path)
            .bind(&req.hour_bucket)
            .bind(&req.prompt)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to insert batch request: {}", e))?;
        }

        Ok(job_id)
    }

    /// Submit a batch job to OpenAI
    pub async fn submit_batch_job(
        &self,
        pool: &SqlitePool,
        job_id: &str,
    ) -> Result<BatchSubmitResult, String> {
        let api_key = self.config.api_key.as_ref()
            .ok_or("OpenAI API key not configured")?;

        // Fetch batch requests
        let requests: Vec<BatchRequest> = sqlx::query_as(
            "SELECT * FROM llm_batch_requests WHERE batch_job_id = ? ORDER BY created_at",
        )
        .bind(job_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch batch requests: {}", e))?;

        if requests.is_empty() {
            return Err("No requests found for batch job".to_string());
        }

        // Build JSONL content
        let mut jsonl_lines = Vec::new();
        let no_temp = no_temperature_support(&self.config.model);
        let use_new_param = uses_max_completion_tokens(&self.config.model);

        for req in &requests {
            let messages = vec![ChatMessage {
                role: "user".to_string(),
                content: req.prompt.clone(),
            }];

            let body = if no_temp {
                // Models like gpt-5-mini, o1, o3 don't support custom temperature
                serde_json::to_value(BatchRequestBodyNewNoTemp {
                    model: self.config.model.clone(),
                    messages,
                    max_completion_tokens: 500,
                }).map_err(|e| e.to_string())?
            } else if use_new_param {
                // Models like gpt-4.1, gpt-4o use max_completion_tokens with temperature
                serde_json::to_value(BatchRequestBodyNew {
                    model: self.config.model.clone(),
                    messages,
                    max_completion_tokens: 500,
                    temperature: 0.3,
                }).map_err(|e| e.to_string())?
            } else {
                // Legacy models use max_tokens with temperature
                serde_json::to_value(BatchRequestBodyLegacy {
                    model: self.config.model.clone(),
                    messages,
                    max_tokens: 500,
                    temperature: 0.3,
                }).map_err(|e| e.to_string())?
            };

            let line = BatchRequestLine {
                custom_id: req.custom_id.clone(),
                method: "POST".to_string(),
                url: "/v1/chat/completions".to_string(),
                body,
            };
            jsonl_lines.push(serde_json::to_string(&line).map_err(|e| e.to_string())?);
        }
        let jsonl_content = jsonl_lines.join("\n");

        // Upload file to OpenAI
        let file_part = multipart::Part::bytes(jsonl_content.into_bytes())
            .file_name("batch_requests.jsonl")
            .mime_str("application/jsonl")
            .map_err(|e: reqwest::Error| e.to_string())?;

        let form = multipart::Form::new()
            .part("file", file_part)
            .text("purpose", "batch");

        let upload_response = self.client
            .post("https://api.openai.com/v1/files")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Failed to upload batch file: {}", e))?;

        if !upload_response.status().is_success() {
            let text = upload_response.text().await.unwrap_or_default();
            return Err(format!("File upload failed: {}", text));
        }

        let file_response: FileUploadResponse = upload_response.json().await
            .map_err(|e| format!("Failed to parse file upload response: {}", e))?;

        // Create batch job
        let batch_request = serde_json::json!({
            "input_file_id": file_response.id,
            "endpoint": "/v1/chat/completions",
            "completion_window": "24h"
        });

        let batch_response = self.client
            .post("https://api.openai.com/v1/batches")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&batch_request)
            .send()
            .await
            .map_err(|e| format!("Failed to create batch: {}", e))?;

        if !batch_response.status().is_success() {
            let text = batch_response.text().await.unwrap_or_default();
            return Err(format!("Batch creation failed: {}", text));
        }

        let batch_create: BatchCreateResponse = batch_response.json().await
            .map_err(|e| format!("Failed to parse batch response: {}", e))?;

        // Update job record
        sqlx::query(
            r#"
            UPDATE llm_batch_jobs
            SET openai_batch_id = ?, input_file_id = ?, status = 'submitted', submitted_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(&batch_create.id)
        .bind(&file_response.id)
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update batch job: {}", e))?;

        Ok(BatchSubmitResult {
            job_id: job_id.to_string(),
            openai_batch_id: batch_create.id,
            total_requests: requests.len(),
        })
    }

    /// Check batch job status from OpenAI
    pub async fn check_batch_status(
        &self,
        pool: &SqlitePool,
        job_id: &str,
    ) -> Result<BatchJobStatus, String> {
        let api_key = self.config.api_key.as_ref()
            .ok_or("OpenAI API key not configured")?;

        // Fetch job
        let job: BatchJob = sqlx::query_as(
            "SELECT * FROM llm_batch_jobs WHERE id = ?",
        )
        .bind(job_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Failed to fetch batch job: {}", e))?
        .ok_or("Batch job not found")?;

        let openai_batch_id = job.openai_batch_id
            .ok_or("Batch job not yet submitted")?;

        // Check status from OpenAI
        let response = self.client
            .get(format!("https://api.openai.com/v1/batches/{}", openai_batch_id))
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| format!("Failed to check batch status: {}", e))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Status check failed: {}", text));
        }

        let status_response: BatchStatusResponse = response.json().await
            .map_err(|e| format!("Failed to parse status response: {}", e))?;

        let status = BatchJobStatus::from(status_response.status.as_str());

        // Update job record
        let (completed, failed) = status_response.request_counts
            .map(|c| (c.completed, c.failed))
            .unwrap_or((0, 0));

        sqlx::query(
            r#"
            UPDATE llm_batch_jobs
            SET status = ?, completed_requests = ?, failed_requests = ?,
                output_file_id = ?, error_file_id = ?,
                completed_at = CASE WHEN ? IN ('completed', 'failed', 'expired') THEN CURRENT_TIMESTAMP ELSE completed_at END
            WHERE id = ?
            "#,
        )
        .bind(status.to_string())
        .bind(completed)
        .bind(failed)
        .bind(&status_response.output_file_id)
        .bind(&status_response.error_file_id)
        .bind(status.to_string())
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update batch job status: {}", e))?;

        Ok(status)
    }

    /// Process completed batch results
    pub async fn process_batch_results(
        &self,
        pool: &SqlitePool,
        job_id: &str,
    ) -> Result<BatchProcessResult, String> {
        let api_key = self.config.api_key.as_ref()
            .ok_or("OpenAI API key not configured")?;

        // Fetch job
        let job: BatchJob = sqlx::query_as(
            "SELECT * FROM llm_batch_jobs WHERE id = ?",
        )
        .bind(job_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Failed to fetch batch job: {}", e))?
        .ok_or("Batch job not found")?;

        let output_file_id = job.output_file_id
            .ok_or("No output file available")?;

        // Download output file
        let response = self.client
            .get(format!("https://api.openai.com/v1/files/{}/content", output_file_id))
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| format!("Failed to download output file: {}", e))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Output download failed: {}", text));
        }

        let content = response.text().await
            .map_err(|e| format!("Failed to read output content: {}", e))?;

        // Parse JSONL results
        let mut completed = 0;
        let mut failed = 0;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let result: BatchResultLine = serde_json::from_str(line)
                .map_err(|e| format!("Failed to parse result line: {}", e))?;

            if let Some(resp) = result.response {
                if resp.status_code == 200 {
                    let text = resp.body.choices.first()
                        .map(|c| c.message.content.clone())
                        .unwrap_or_default();

                    let (prompt_tokens, completion_tokens) = resp.body.usage
                        .map(|u| (Some(u.prompt_tokens), Some(u.completion_tokens)))
                        .unwrap_or((None, None));

                    // Update request record
                    sqlx::query(
                        r#"
                        UPDATE llm_batch_requests
                        SET status = 'completed', response = ?, prompt_tokens = ?, completion_tokens = ?, completed_at = CURRENT_TIMESTAMP
                        WHERE batch_job_id = ? AND custom_id = ?
                        "#,
                    )
                    .bind(&text)
                    .bind(prompt_tokens)
                    .bind(completion_tokens)
                    .bind(job_id)
                    .bind(&result.custom_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Failed to update request: {}", e))?;

                    completed += 1;
                } else {
                    failed += 1;
                }
            } else if let Some(err) = result.error {
                sqlx::query(
                    r#"
                    UPDATE llm_batch_requests
                    SET status = 'failed', error_message = ?, completed_at = CURRENT_TIMESTAMP
                    WHERE batch_job_id = ? AND custom_id = ?
                    "#,
                )
                .bind(format!("{}: {}", err.code, err.message))
                .bind(job_id)
                .bind(&result.custom_id)
                .execute(pool)
                .await
                .map_err(|e| format!("Failed to update failed request: {}", e))?;

                failed += 1;
            }
        }

        Ok(BatchProcessResult {
            job_id: job_id.to_string(),
            completed,
            failed,
            summaries_saved: completed, // Will be updated by compaction
        })
    }

    /// Get pending batch job for user
    pub async fn get_pending_job(
        pool: &SqlitePool,
        user_id: &str,
    ) -> Result<Option<BatchJob>, String> {
        sqlx::query_as(
            "SELECT * FROM llm_batch_jobs WHERE user_id = ? AND status IN ('pending', 'submitted', 'in_progress') ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Failed to fetch pending job: {}", e))
    }

    /// Get completed batch requests for a job
    pub async fn get_completed_requests(
        pool: &SqlitePool,
        job_id: &str,
    ) -> Result<Vec<BatchRequest>, String> {
        sqlx::query_as(
            "SELECT * FROM llm_batch_requests WHERE batch_job_id = ? AND status = 'completed'",
        )
        .bind(job_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch completed requests: {}", e))
    }
}

/// Request for hourly compaction batch
#[derive(Debug, Clone)]
pub struct HourlyCompactionRequest {
    pub project_path: String,
    pub hour_bucket: String,
    pub prompt: String,
    pub snapshot_ids: Vec<String>,
    pub key_activities: String,
    pub git_summary: String,
    pub previous_context: Option<String>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_job_status_from_str() {
        assert_eq!(BatchJobStatus::from("pending"), BatchJobStatus::Pending);
        assert_eq!(BatchJobStatus::from("completed"), BatchJobStatus::Completed);
        assert_eq!(BatchJobStatus::from("in_progress"), BatchJobStatus::InProgress);
        assert_eq!(BatchJobStatus::from("validating"), BatchJobStatus::InProgress);
        assert_eq!(BatchJobStatus::from("failed"), BatchJobStatus::Failed);
    }

    #[test]
    fn test_batch_job_status_display() {
        assert_eq!(BatchJobStatus::Pending.to_string(), "pending");
        assert_eq!(BatchJobStatus::Completed.to_string(), "completed");
        assert_eq!(BatchJobStatus::InProgress.to_string(), "in_progress");
    }
}
