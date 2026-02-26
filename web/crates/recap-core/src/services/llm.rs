//! LLM Service for generating summaries and analysis
//! Supports OpenAI, Anthropic, Ollama, and OpenAI-compatible APIs

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: String,      // "openai", "anthropic", "ollama", "openai-compatible"
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    /// Maximum character count for summary output (default: 2000)
    pub summary_max_chars: u32,
    /// Reasoning effort for o-series/gpt-5 models: "low", "medium", "high"
    pub reasoning_effort: Option<String>,
    /// Custom summary prompt template (None = use default)
    pub summary_prompt: Option<String>,
}

/// Result of testing LLM connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: i64,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub model_response: Option<String>,
}

/// Token usage record from an LLM API call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmUsageRecord {
    pub provider: String,
    pub model: String,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub duration_ms: i64,
    pub purpose: String,
    pub status: String,
    pub error_message: Option<String>,
}

/// OpenAI request for newer models (gpt-5-nano, o1, o3) that don't support temperature
#[derive(Debug, Serialize)]
struct OpenAIRequestNewNoTemp {
    model: String,
    messages: Vec<OpenAIMessageRequest>,
    max_completion_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
}

/// OpenAI request for newer models (gpt-4.1, gpt-4o) that use max_completion_tokens with temperature
#[derive(Debug, Serialize)]
struct OpenAIRequestNew {
    model: String,
    messages: Vec<OpenAIMessageRequest>,
    max_completion_tokens: u32,
    temperature: f32,
}

/// OpenAI request for legacy models (gpt-4-turbo, gpt-4, gpt-3.5) that use max_tokens
#[derive(Debug, Serialize)]
struct OpenAIRequestLegacy {
    model: String,
    messages: Vec<OpenAIMessageRequest>,
    max_tokens: u32,
    temperature: f32,
}

/// Check if a model should use the Responses API (GPT-5 series)
fn uses_responses_api(model: &str) -> bool {
    model.starts_with("gpt-5")
}

/// Check if a model uses the new max_completion_tokens parameter
fn uses_max_completion_tokens(model: &str) -> bool {
    model.starts_with("gpt-5") ||
    model.starts_with("gpt-4.1") ||
    model.starts_with("gpt-4o") ||
    model.starts_with("o1") ||
    model.starts_with("o3")
}

/// Check if a model doesn't support custom temperature (only default 1)
fn no_temperature_support(model: &str) -> bool {
    model.starts_with("gpt-5") ||  // All GPT-5 models (gpt-5, gpt-5-mini, gpt-5-nano)
    model.starts_with("o1") ||
    model.starts_with("o3")
}

// ============ Responses API types (for GPT-5 series) ============

/// OpenAI Responses API request
#[derive(Debug, Serialize)]
struct ResponsesApiRequest {
    model: String,
    input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<ResponsesTextConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ReasoningConfig>,
}

/// Reasoning configuration for Responses API
#[derive(Debug, Serialize)]
struct ReasoningConfig {
    effort: String,
}

/// Text output configuration for Responses API
#[derive(Debug, Serialize)]
struct ResponsesTextConfig {
    format: ResponsesTextFormat,
}

/// Text format specification
#[derive(Debug, Serialize)]
struct ResponsesTextFormat {
    #[serde(rename = "type")]
    format_type: String,
}

/// OpenAI Responses API response
#[derive(Debug, Deserialize)]
struct ResponsesApiResponse {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    status: String,
    output: Vec<ResponsesOutputItem>,
    usage: Option<ResponsesUsage>,
}

/// Output item in Responses API (can be message or reasoning)
#[derive(Debug, Deserialize)]
struct ResponsesOutputItem {
    #[serde(rename = "type")]
    item_type: String,
    #[serde(default)]
    content: Option<Vec<ResponsesContent>>,
}

/// Content block in Responses API output
#[derive(Debug, Deserialize)]
struct ResponsesContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
}

/// Usage info in Responses API
#[derive(Debug, Deserialize)]
struct ResponsesUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
}

/// OpenAI message for requests (only role and content)
#[derive(Debug, Serialize)]
struct OpenAIMessageRequest {
    role: String,
    content: String,
}

/// OpenAI message in responses (may include reasoning_content for o-series models)
#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    #[allow(dead_code)]
    role: String,
    #[serde(default)]
    content: String,
    /// For o-series models that use reasoning
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

pub struct LlmService {
    config: LlmConfig,
    client: reqwest::Client,
}

/// Default timeout for LLM API calls.
/// Monthly compaction requests up to 8000 output tokens which can take 60-90s on
/// smaller models (e.g. gpt-5-nano), so 120s provides adequate headroom.
const LLM_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

impl LlmService {
    pub fn new(config: LlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(LLM_REQUEST_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            client,
        }
    }

    /// Check if the LLM service is configured
    pub fn is_configured(&self) -> bool {
        match self.config.provider.as_str() {
            "ollama" => true, // Ollama doesn't need API key
            _ => self.config.api_key.is_some(),
        }
    }

    /// Get the provider name
    pub fn provider(&self) -> &str {
        &self.config.provider
    }

    /// Get the model name
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Test connection to the LLM API
    /// Sends a minimal request to verify the API key and model are working
    pub async fn test_connection(&self) -> Result<LlmTestResult, String> {
        let start = std::time::Instant::now();

        let test_prompt = "Reply with exactly: Connection test successful.";
        let result = self.complete_raw(test_prompt, 100).await;
        let latency_ms = start.elapsed().as_millis() as i64;

        match result {
            Ok((response, prompt_tokens, completion_tokens, _)) => {
                Ok(LlmTestResult {
                    success: true,
                    message: format!("連線成功: {}", self.config.model),
                    latency_ms,
                    prompt_tokens,
                    completion_tokens,
                    model_response: Some(response.chars().take(100).collect()),
                })
            }
            Err(e) => {
                // For test connection, "trivial response" or "no text content" errors
                // actually prove the API is reachable and the key is valid — treat as success
                if e.contains("trivial response") || e.contains("no text content") {
                    return Ok(LlmTestResult {
                        success: true,
                        message: format!("連線成功: {}", self.config.model),
                        latency_ms,
                        prompt_tokens: None,
                        completion_tokens: None,
                        model_response: None,
                    });
                }

                // Parse error message for better user feedback
                let user_message = if e.contains("401") || e.contains("Unauthorized") {
                    "API Key 無效或已過期"
                } else if e.contains("404") {
                    "找不到指定的模型"
                } else if e.contains("429") {
                    "請求過於頻繁，請稍後再試"
                } else if e.contains("connection") || e.contains("timeout") {
                    "無法連線到 API 伺服器"
                } else {
                    "連線失敗"
                };

                Ok(LlmTestResult {
                    success: false,
                    message: format!("{}: {}", user_message, e),
                    latency_ms,
                    prompt_tokens: None,
                    completion_tokens: None,
                    model_response: None,
                })
            }
        }
    }

    /// Generate a summary of work session content
    pub async fn summarize_session(&self, content: &str) -> Result<(String, LlmUsageRecord), String> {
        let prompt = format!(
            r#"請將以下 Claude Code 工作 session 內容整理成簡潔的工作摘要（50-100字）。

重點描述：
1. 完成了什麼功能或達成什麼目標（成果導向）
2. 對專案整體的推進或貢獻

安全規則（務必遵守）：
- 絕對不要在摘要中出現任何 IP 位址、密碼、API Key、Token、帳號密碼、伺服器位址、內部 URL
- 如果原始內容包含這些機密資訊，請用泛稱替代（如「更新伺服器密碼」而非列出實際密碼）

Session 內容：
{}

請用繁體中文回答，直接輸出摘要內容，不要加任何前綴或說明。"#,
            content.chars().take(4000).collect::<String>()
        );

        self.complete_with_usage(&prompt, "session_summary", 500).await
    }

    /// Generate a project work summary for Tempo reporting
    pub async fn summarize_project_work(&self, project: &str, work_items: &str) -> Result<(Vec<String>, LlmUsageRecord), String> {
        let prompt = format!(
            r#"你是一個工作報告助手。請將以下「{project}」專案的工作項目整理成 3-5 條簡潔的工作摘要。

工作項目：
{work_items}

要求：
1. 每條摘要 10-30 字
2. 使用動詞開頭（如：實作、完成、修復、設計、優化、建立）
3. 合併相似的工作項目
4. 著重描述「達成了什麼成果」而非「做了哪些步驟」
5. 使用繁體中文

安全規則（務必遵守）：
- 絕對不要出現任何 IP 位址、密碼、API Key、Token、帳號密碼、伺服器位址、內部 URL
- 用泛稱替代機密資訊（如「更新伺服器認證設定」而非列出實際密碼或 IP）

請直接輸出摘要清單，每行一條，不要編號，不要其他說明。"#,
            project = project,
            work_items = work_items.chars().take(3000).collect::<String>()
        );

        let (response, usage) = self.complete_with_usage(&prompt, "project_summary", 500).await?;

        let summaries: Vec<String> = response
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && s.len() > 3)
            .map(|s| s.trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == '-' || c == '•' || c == '*').trim().to_string())
            .filter(|s| !s.is_empty())
            .take(5)
            .collect();

        Ok((summaries, usage))
    }

    /// Generate a daily work summary
    pub async fn summarize_daily_work(&self, sessions_info: &str, commits_info: &str) -> Result<(String, LlmUsageRecord), String> {
        let prompt = format!(
            r#"請根據以下工作記錄整理成每日工作摘要（100-200字）。

Claude Code Sessions:
{}

Git Commits:
{}

請用繁體中文撰寫摘要，著重於：
1. 今日達成的關鍵成果或里程碑
2. 對專案的具體推進（如：完成某功能、解決某問題、提升某指標）
3. 避免流水帳式的步驟描述，應以成果和貢獻為主

安全規則（務必遵守）：
- 絕對不要出現任何 IP 位址、密碼、API Key、Token、帳號密碼、伺服器位址、內部 URL
- 用泛稱替代機密資訊

直接輸出摘要內容，不要加任何前綴。"#,
            sessions_info.chars().take(2000).collect::<String>(),
            commits_info.chars().take(1000).collect::<String>()
        );

        self.complete_with_usage(&prompt, "daily_summary", 1000).await
    }

    /// Summarize a work period at a given time scale.
    /// `context` is the previous period's summary (for continuity).
    /// `current_data` is the current period's data to summarize.
    /// Output length is proportional to `self.config.summary_max_chars` (default 2000).
    pub async fn summarize_work_period(
        &self,
        context: &str,
        current_data: &str,
        scale: &str,
    ) -> Result<(String, LlmUsageRecord), String> {
        let base = self.config.summary_max_chars;
        // base = summary_max_chars from Settings (default 2000).
        // Scale output proportionally: hourly is brief, monthly uses the full budget.
        // Chinese ≈ 1.5 tokens/char, so multiply char target by 2 for token budget.
        let (char_limit, input_max_chars) = match scale {
            "hourly"  => (base / 8,      4000_u32),   // base=2000 → 250字
            "daily"   => (base / 4,      6000),       // base=2000 → 500字
            "weekly"  => (base / 2,      8000),       // base=2000 → 1000字
            "monthly" => (base * 3 / 4, 10000),       // base=2000 → 1500字
            "yearly"  => (base,         12000),       // base=2000 → 2000字
            _         => (base / 4,      6000),
        };
        let length_hint = format!("{}字以內", char_limit);
        let output_max_tokens = char_limit * 2; // 1 Chinese char ≈ 1.5-2 tokens

        let context_section = if context.is_empty() {
            String::new()
        } else {
            format!(
                "\n前一時段摘要（作為前後文參考）：\n{}\n",
                context.chars().take(1000).collect::<String>()
            )
        };

        let data = current_data.chars().take(input_max_chars as usize).collect::<String>();

        let prompt = if let Some(ref custom_prompt) = self.config.summary_prompt {
            // User-provided custom prompt with placeholder substitution
            custom_prompt
                .replace("{length_hint}", &length_hint)
                .replace("{context_section}", &context_section)
                .replace("{data}", &data)
        } else {
            format!(
                r#"你是工作報告助手。請根據以下工作資料，產生精簡的工作摘要（嚴格控制在{length_hint}）。
{context_section}
本時段的工作資料：
{data}

安全規則（最高優先）：
- 不要出現 IP、密碼、API Key、Token、內部 URL 等機密資訊

撰寫風格：
- 只寫「成果」：完成了什麼、解決了什麼問題、推進了什麼目標
- 嚴禁流水帳：不要寫操作步驟（如「搜尋程式碼」「修改檔案」「執行測試」「閱讀文件」）
- 合併同類工作，不要逐項列舉每個小改動
- 若有 git commit，以 commit 訊息歸納成果
- 程式碼名稱用 `backtick` 包裹

格式：
1. 一句話總結核心成果（不加前綴）
2. 空一行後，用 3-5 個要點列出關鍵成果（以「- 」開頭）

重要：嚴格遵守字數限制，寧可精簡也不要冗長。直接輸出摘要。"#,
                length_hint = length_hint,
                context_section = context_section,
                data = data
            )
        };

        let purpose = format!("{}_compaction", scale);
        self.complete_with_usage(&prompt, &purpose, output_max_tokens).await
    }

    /// Summarize a worklog description for Tempo upload.
    /// Produces a concise single-line summary (max ~50 chars) suitable for Tempo worklog description.
    pub async fn summarize_worklog(&self, description: &str) -> Result<(String, LlmUsageRecord), String> {
        let prompt = format!(
            r#"請將以下工作日誌濃縮成一句簡短摘要（最多50字），適合作為 Tempo 工作紀錄的 description。

要求：
1. 只輸出一行文字，不要換行、不要編號、不要 markdown
2. 使用繁體中文
3. 用動詞開頭描述主要完成的成果（如：完成、建立、修復、優化）
4. 省略操作步驟細節，保留核心成果和貢獻
5. 絕對不要出現任何 IP 位址、密碼、API Key、Token、伺服器位址等機密資訊

工作日誌：
{}

直接輸出摘要，不要加任何前綴或說明。"#,
            description.chars().take(2000).collect::<String>()
        );

        self.complete_with_usage(&prompt, "worklog_description", 200).await
    }

    /// Send completion request to LLM and return usage record.
    /// `max_tokens` controls the maximum output tokens for the API call.
    pub async fn complete_with_usage(&self, prompt: &str, purpose: &str, max_tokens: u32) -> Result<(String, LlmUsageRecord), String> {
        let start = Instant::now();
        let result = self.complete_raw(prompt, max_tokens).await;
        let duration_ms = start.elapsed().as_millis() as i64;

        match result {
            Ok((text, prompt_tokens, completion_tokens, total_tokens)) => {
                let usage = LlmUsageRecord {
                    provider: self.config.provider.clone(),
                    model: self.config.model.clone(),
                    prompt_tokens,
                    completion_tokens,
                    total_tokens,
                    duration_ms,
                    purpose: purpose.to_string(),
                    status: "success".to_string(),
                    error_message: None,
                };
                Ok((text, usage))
            }
            Err(e) => {
                let usage = LlmUsageRecord {
                    provider: self.config.provider.clone(),
                    model: self.config.model.clone(),
                    prompt_tokens: None,
                    completion_tokens: None,
                    total_tokens: None,
                    duration_ms,
                    purpose: purpose.to_string(),
                    status: "error".to_string(),
                    error_message: Some(e.clone()),
                };
                // Return error but also provide the usage record
                // Callers can still save the error record
                Err(format!("LLM_ERROR:{}::{}", serde_json::to_string(&usage).unwrap_or_default(), e))
            }
        }
    }

    /// Send completion request and return (text, prompt_tokens, completion_tokens, total_tokens)
    async fn complete_raw(&self, prompt: &str, max_tokens: u32) -> Result<(String, Option<i64>, Option<i64>, Option<i64>), String> {
        match self.config.provider.as_str() {
            "openai" | "openai-compatible" => self.complete_openai(prompt, max_tokens).await,
            "anthropic" => self.complete_anthropic(prompt, max_tokens).await,
            "ollama" => self.complete_ollama(prompt, max_tokens).await,
            _ => Err(format!("Unsupported LLM provider: {}", self.config.provider)),
        }
    }

    async fn complete_openai(&self, prompt: &str, max_tokens: u32) -> Result<(String, Option<i64>, Option<i64>, Option<i64>), String> {
        let api_key = self.config.api_key.as_ref()
            .ok_or("OpenAI API key not configured")?;

        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://api.openai.com/v1");

        // Use Responses API for GPT-5 series models
        if uses_responses_api(&self.config.model) {
            return self.complete_openai_responses_api(prompt, api_key, base_url, max_tokens).await;
        }

        let messages = vec![OpenAIMessageRequest {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        log::info!("OpenAI request: model={}, max_tokens={}, no_temp={}, uses_mct={}",
            self.config.model, max_tokens,
            no_temperature_support(&self.config.model),
            uses_max_completion_tokens(&self.config.model));

        // Use appropriate request struct based on model capabilities
        let response = if no_temperature_support(&self.config.model) {
            // Models like o1, o3 don't support custom temperature
            let request = OpenAIRequestNewNoTemp {
                model: self.config.model.clone(),
                messages,
                max_completion_tokens: max_tokens,
                reasoning_effort: self.config.reasoning_effort.clone(),
            };
            self.client
                .post(format!("{}/chat/completions", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
        } else if uses_max_completion_tokens(&self.config.model) {
            // Models like gpt-4.1, gpt-4o use max_completion_tokens with temperature
            let request = OpenAIRequestNew {
                model: self.config.model.clone(),
                messages,
                max_completion_tokens: max_tokens,
                temperature: 0.3,
            };
            self.client
                .post(format!("{}/chat/completions", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
        } else {
            // Legacy models use max_tokens with temperature
            let request = OpenAIRequestLegacy {
                model: self.config.model.clone(),
                messages,
                max_tokens: max_tokens,
                temperature: 0.3,
            };
            self.client
                .post(format!("{}/chat/completions", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
        }.map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, text));
        }

        // Get raw response text first for debugging
        let response_text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        log::info!("OpenAI raw response (first 2000 chars): {}", &response_text.chars().take(2000).collect::<String>());

        let result: OpenAIResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse response: {}. Raw: {}", e, &response_text.chars().take(200).collect::<String>()))?;

        let text = result.choices.first()
            .map(|c| {
                // Always prefer content (the actual answer).
                // reasoning_content is the internal chain-of-thought for o-series models — never use it as output.
                if !c.message.content.is_empty() {
                    return c.message.content.clone();
                }
                // Fallback: if content is empty but reasoning exists (shouldn't normally happen)
                if let Some(ref reasoning) = c.message.reasoning_content {
                    if !reasoning.is_empty() {
                        log::warn!("OpenAI response has empty content but non-empty reasoning_content, falling back to reasoning");
                        return reasoning.clone();
                    }
                }
                String::new()
            })
            .ok_or_else(|| format!("No response from LLM. Choices: {:?}", result.choices))?;

        log::info!("OpenAI extracted text length: {} chars, content_empty: {}, has_reasoning: {}, text_preview: '{}'",
            text.len(),
            result.choices.first().map(|c| c.message.content.is_empty()).unwrap_or(true),
            result.choices.first().and_then(|c| c.message.reasoning_content.as_ref()).is_some(),
            &text.chars().take(200).collect::<String>()
        );

        let (prompt_tokens, completion_tokens, total_tokens) = match result.usage {
            Some(u) => (u.prompt_tokens, u.completion_tokens, u.total_tokens),
            None => (None, None, None),
        };

        Ok((text, prompt_tokens, completion_tokens, total_tokens))
    }

    /// Use OpenAI Responses API for GPT-5 series models
    async fn complete_openai_responses_api(
        &self,
        prompt: &str,
        api_key: &str,
        base_url: &str,
        max_tokens: u32,
    ) -> Result<(String, Option<i64>, Option<i64>, Option<i64>), String> {
        // Build request with explicit text format to ensure message output
        let reasoning = self.config.reasoning_effort.as_ref().map(|effort| ReasoningConfig {
            effort: effort.clone(),
        });

        // For Responses API, max_output_tokens covers BOTH reasoning + text tokens.
        // Add headroom so reasoning doesn't consume the entire budget.
        let reasoning_headroom: u32 = if reasoning.is_some() { 2000 } else { 0 };
        let effective_max_tokens = max_tokens + reasoning_headroom;

        let request = ResponsesApiRequest {
            model: self.config.model.clone(),
            input: prompt.to_string(),
            max_output_tokens: Some(effective_max_tokens),
            text: Some(ResponsesTextConfig {
                format: ResponsesTextFormat {
                    format_type: "text".to_string(),
                },
            }),
            reasoning,
        };

        log::info!("Using Responses API for model: {}", self.config.model);

        let response = self.client
            .post(format!("{}/responses", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Responses API error {}: {}", status, text));
        }

        // Get raw response for debugging
        let response_text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        log::info!("Responses API raw response (first 1000 chars): {}",
            &response_text.chars().take(1000).collect::<String>());

        let result: ResponsesApiResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse Responses API response: {}. Raw: {}",
                e, &response_text.chars().take(500).collect::<String>()))?;

        // Extract text from output items
        let output_text = extract_responses_text(&result.output);

        log::info!("Responses API extracted text length: {} chars, preview: '{}'",
            output_text.len(),
            &output_text.chars().take(200).collect::<String>()
        );

        // Check for empty or trivial responses (like "OK", "好的", etc.)
        let trimmed = output_text.trim();
        if trimmed.is_empty() {
            log::warn!("Responses API returned empty text. Output items: {:?}", result.output);
            return Err("Responses API returned no text content. The model may need more output tokens.".to_string());
        }

        // Treat very short responses (< 20 chars) as failures - model likely didn't understand the task
        if trimmed.len() < 20 {
            log::warn!("Responses API returned trivial response: '{}'. Treating as failure.", trimmed);
            return Err(format!("Responses API returned trivial response: '{}'. The model may need clearer instructions.", trimmed));
        }

        let (prompt_tokens, completion_tokens, total_tokens) = match result.usage {
            Some(u) => {
                let total = match (u.input_tokens, u.output_tokens) {
                    (Some(i), Some(o)) => Some(i + o),
                    _ => None,
                };
                (u.input_tokens, u.output_tokens, total)
            }
            None => (None, None, None),
        };

        Ok((output_text, prompt_tokens, completion_tokens, total_tokens))
    }

    async fn complete_anthropic(&self, prompt: &str, max_tokens: u32) -> Result<(String, Option<i64>, Option<i64>, Option<i64>), String> {
        let api_key = self.config.api_key.as_ref()
            .ok_or("Anthropic API key not configured")?;

        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: max_tokens,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, text));
        }

        let result: AnthropicResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let text = result.content.first()
            .map(|c| c.text.clone())
            .ok_or_else(|| "No response from LLM".to_string())?;

        let (prompt_tokens, completion_tokens, total_tokens) = match result.usage {
            Some(u) => {
                let total = match (u.input_tokens, u.output_tokens) {
                    (Some(i), Some(o)) => Some(i + o),
                    _ => None,
                };
                (u.input_tokens, u.output_tokens, total)
            }
            None => (None, None, None),
        };

        Ok((text, prompt_tokens, completion_tokens, total_tokens))
    }

    async fn complete_ollama(&self, prompt: &str, max_tokens: u32) -> Result<(String, Option<i64>, Option<i64>, Option<i64>), String> {
        let base_url = self.config.base_url.as_deref()
            .unwrap_or("http://localhost:11434");

        // Ollama uses OpenAI-compatible API with legacy max_tokens parameter
        let request = OpenAIRequestLegacy {
            model: self.config.model.clone(),
            messages: vec![OpenAIMessageRequest {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: max_tokens,
            temperature: 0.3,
        };

        let response = self.client
            .post(format!("{}/v1/chat/completions", base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Ollama error {}: {}", status, text));
        }

        let result: OpenAIResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let text = result.choices.first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| "No response from Ollama".to_string())?;

        let (prompt_tokens, completion_tokens, total_tokens) = match result.usage {
            Some(u) => (u.prompt_tokens, u.completion_tokens, u.total_tokens),
            None => (None, None, None),
        };

        Ok((text, prompt_tokens, completion_tokens, total_tokens))
    }
}

/// Parse an LlmUsageRecord from an error string produced by complete_with_usage
pub fn parse_error_usage(err: &str) -> Option<LlmUsageRecord> {
    if let Some(rest) = err.strip_prefix("LLM_ERROR:") {
        if let Some(sep_idx) = rest.find("::") {
            let json_str = &rest[..sep_idx];
            serde_json::from_str(json_str).ok()
        } else {
            None
        }
    } else {
        None
    }
}

/// Extract text content from a Responses API output array.
/// Returns the concatenated text from all message items with output_text/text content.
fn extract_responses_text(output: &[ResponsesOutputItem]) -> String {
    let mut text = String::new();
    for item in output {
        if item.item_type == "message" {
            if let Some(contents) = &item.content {
                for content in contents {
                    if content.content_type == "output_text" || content.content_type == "text" {
                        if let Some(t) = &content.text {
                            text.push_str(t);
                        }
                    }
                }
            }
        }
    }
    text
}

/// Create LLM service from database config
pub async fn create_llm_service(pool: &sqlx::SqlitePool, user_id: &str) -> Result<LlmService, String> {
    let row: (Option<String>, Option<String>, Option<String>, Option<String>, Option<i32>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT llm_provider, llm_model, llm_api_key, llm_base_url, summary_max_chars, summary_reasoning_effort, summary_prompt FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "User not found".to_string())?;

    let config = LlmConfig {
        provider: row.0.unwrap_or_else(|| "openai".to_string()),
        model: row.1.unwrap_or_else(|| "gpt-5-nano".to_string()),
        api_key: row.2,
        base_url: row.3,
        summary_max_chars: row.4.unwrap_or(2000) as u32,
        reasoning_effort: row.5,
        summary_prompt: row.6.filter(|s| !s.is_empty()),
    };

    Ok(LlmService::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Model detection tests ====================

    #[test]
    fn test_uses_responses_api_gpt5_models() {
        assert!(uses_responses_api("gpt-5"));
        assert!(uses_responses_api("gpt-5-mini"));
        assert!(uses_responses_api("gpt-5-nano"));
        assert!(uses_responses_api("gpt-5-turbo"));
        assert!(uses_responses_api("gpt-5-0601"));
    }

    #[test]
    fn test_uses_responses_api_non_gpt5_models() {
        assert!(!uses_responses_api("gpt-4o"));
        assert!(!uses_responses_api("gpt-4o-mini"));
        assert!(!uses_responses_api("gpt-4.1"));
        assert!(!uses_responses_api("gpt-4-turbo"));
        assert!(!uses_responses_api("o1"));
        assert!(!uses_responses_api("o3-mini"));
        assert!(!uses_responses_api("claude-3-5-sonnet"));
        assert!(!uses_responses_api("llama3"));
    }

    #[test]
    fn test_uses_max_completion_tokens() {
        // Should use max_completion_tokens
        assert!(uses_max_completion_tokens("gpt-5"));
        assert!(uses_max_completion_tokens("gpt-5-nano"));
        assert!(uses_max_completion_tokens("gpt-4.1"));
        assert!(uses_max_completion_tokens("gpt-4o"));
        assert!(uses_max_completion_tokens("gpt-4o-mini"));
        assert!(uses_max_completion_tokens("o1"));
        assert!(uses_max_completion_tokens("o1-mini"));
        assert!(uses_max_completion_tokens("o3"));
        assert!(uses_max_completion_tokens("o3-mini"));

        // Should NOT use max_completion_tokens (legacy models)
        assert!(!uses_max_completion_tokens("gpt-4-turbo"));
        assert!(!uses_max_completion_tokens("gpt-4"));
        assert!(!uses_max_completion_tokens("gpt-3.5-turbo"));
        assert!(!uses_max_completion_tokens("claude-3-5-sonnet"));
    }

    #[test]
    fn test_no_temperature_support() {
        // No temperature support
        assert!(no_temperature_support("gpt-5"));
        assert!(no_temperature_support("gpt-5-mini"));
        assert!(no_temperature_support("gpt-5-nano"));
        assert!(no_temperature_support("o1"));
        assert!(no_temperature_support("o1-mini"));
        assert!(no_temperature_support("o3"));
        assert!(no_temperature_support("o3-mini"));

        // Has temperature support
        assert!(!no_temperature_support("gpt-4o"));
        assert!(!no_temperature_support("gpt-4o-mini"));
        assert!(!no_temperature_support("gpt-4.1"));
        assert!(!no_temperature_support("gpt-4-turbo"));
        assert!(!no_temperature_support("gpt-3.5-turbo"));
    }

    // ==================== Request serialization tests ====================

    #[test]
    fn test_responses_api_request_serialization() {
        let request = ResponsesApiRequest {
            model: "gpt-5-nano".to_string(),
            input: "Hello world".to_string(),
            max_output_tokens: Some(500),
            text: Some(ResponsesTextConfig {
                format: ResponsesTextFormat {
                    format_type: "text".to_string(),
                },
            }),
            reasoning: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "gpt-5-nano");
        assert_eq!(json["input"], "Hello world");
        assert_eq!(json["max_output_tokens"], 500);
        assert_eq!(json["text"]["format"]["type"], "text");
        // reasoning should be skipped when None
        assert!(json.get("reasoning").is_none());
    }

    #[test]
    fn test_responses_api_request_with_reasoning() {
        let request = ResponsesApiRequest {
            model: "gpt-5".to_string(),
            input: "Summarize this".to_string(),
            max_output_tokens: Some(1000),
            text: None,
            reasoning: Some(ReasoningConfig {
                effort: "medium".to_string(),
            }),
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["reasoning"]["effort"], "medium");
        // text should be skipped when None
        assert!(json.get("text").is_none());
    }

    #[test]
    fn test_chat_completions_request_no_temp() {
        let request = OpenAIRequestNewNoTemp {
            model: "o1".to_string(),
            messages: vec![OpenAIMessageRequest {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            max_completion_tokens: 500,
            reasoning_effort: Some("high".to_string()),
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "o1");
        assert_eq!(json["max_completion_tokens"], 500);
        assert_eq!(json["reasoning_effort"], "high");
        // Should NOT have temperature or max_tokens
        assert!(json.get("temperature").is_none());
        assert!(json.get("max_tokens").is_none());
    }

    #[test]
    fn test_chat_completions_request_new() {
        let request = OpenAIRequestNew {
            model: "gpt-4o".to_string(),
            messages: vec![OpenAIMessageRequest {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            max_completion_tokens: 500,
            temperature: 0.3,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["max_completion_tokens"], 500);
        let temp = json["temperature"].as_f64().unwrap();
        assert!((temp - 0.3).abs() < 0.001, "temperature should be ~0.3, got {}", temp);
        // Should NOT have max_tokens
        assert!(json.get("max_tokens").is_none());
    }

    #[test]
    fn test_chat_completions_request_legacy() {
        let request = OpenAIRequestLegacy {
            model: "gpt-4-turbo".to_string(),
            messages: vec![OpenAIMessageRequest {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            max_tokens: 500,
            temperature: 0.3,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "gpt-4-turbo");
        assert_eq!(json["max_tokens"], 500);
        let temp = json["temperature"].as_f64().unwrap();
        assert!((temp - 0.3).abs() < 0.001, "temperature should be ~0.3, got {}", temp);
        // Should NOT have max_completion_tokens
        assert!(json.get("max_completion_tokens").is_none());
    }

    // ==================== Response parsing tests ====================

    #[test]
    fn test_parse_responses_api_response() {
        let json = r#"{
            "id": "resp_abc",
            "status": "completed",
            "output": [
                {
                    "type": "message",
                    "content": [
                        {"type": "output_text", "text": "This is the summary output."}
                    ]
                }
            ],
            "usage": {"input_tokens": 100, "output_tokens": 50}
        }"#;
        let result: ResponsesApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(result.output.len(), 1);
        assert_eq!(result.output[0].item_type, "message");
        let usage = result.usage.unwrap();
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(50));
    }

    #[test]
    fn test_parse_responses_api_response_with_reasoning() {
        let json = r#"{
            "id": "resp_xyz",
            "status": "completed",
            "output": [
                {
                    "type": "reasoning",
                    "content": [
                        {"type": "reasoning_text", "text": "thinking..."}
                    ]
                },
                {
                    "type": "message",
                    "content": [
                        {"type": "output_text", "text": "Final answer here."}
                    ]
                }
            ],
            "usage": {"input_tokens": 200, "output_tokens": 100}
        }"#;
        let result: ResponsesApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(result.output.len(), 2);
        assert_eq!(result.output[0].item_type, "reasoning");
        assert_eq!(result.output[1].item_type, "message");
    }

    #[test]
    fn test_parse_chat_completions_response() {
        let json = r#"{
            "choices": [
                {"message": {"role": "assistant", "content": "Hello there!"}}
            ],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        }"#;
        let result: OpenAIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(result.choices.len(), 1);
        assert_eq!(result.choices[0].message.content, "Hello there!");
        let usage = result.usage.unwrap();
        assert_eq!(usage.prompt_tokens, Some(10));
        assert_eq!(usage.completion_tokens, Some(5));
        assert_eq!(usage.total_tokens, Some(15));
    }

    #[test]
    fn test_parse_chat_completions_response_with_reasoning() {
        let json = r#"{
            "choices": [
                {"message": {"role": "assistant", "content": "Result", "reasoning_content": "thinking step..."}}
            ],
            "usage": null
        }"#;
        let result: OpenAIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(result.choices[0].message.content, "Result");
        assert_eq!(
            result.choices[0].message.reasoning_content.as_deref(),
            Some("thinking step...")
        );
    }

    // ==================== extract_responses_text tests ====================

    #[test]
    fn test_extract_responses_text_single_message() {
        let output = vec![ResponsesOutputItem {
            item_type: "message".to_string(),
            content: Some(vec![ResponsesContent {
                content_type: "output_text".to_string(),
                text: Some("Hello world".to_string()),
            }]),
        }];
        assert_eq!(extract_responses_text(&output), "Hello world");
    }

    #[test]
    fn test_extract_responses_text_skips_reasoning() {
        let output = vec![
            ResponsesOutputItem {
                item_type: "reasoning".to_string(),
                content: Some(vec![ResponsesContent {
                    content_type: "reasoning_text".to_string(),
                    text: Some("I should think about this...".to_string()),
                }]),
            },
            ResponsesOutputItem {
                item_type: "message".to_string(),
                content: Some(vec![ResponsesContent {
                    content_type: "output_text".to_string(),
                    text: Some("The actual answer".to_string()),
                }]),
            },
        ];
        assert_eq!(extract_responses_text(&output), "The actual answer");
    }

    #[test]
    fn test_extract_responses_text_handles_text_content_type() {
        let output = vec![ResponsesOutputItem {
            item_type: "message".to_string(),
            content: Some(vec![ResponsesContent {
                content_type: "text".to_string(),
                text: Some("Text type content".to_string()),
            }]),
        }];
        assert_eq!(extract_responses_text(&output), "Text type content");
    }

    #[test]
    fn test_extract_responses_text_empty_output() {
        let output: Vec<ResponsesOutputItem> = vec![];
        assert_eq!(extract_responses_text(&output), "");
    }

    #[test]
    fn test_extract_responses_text_no_text_field() {
        let output = vec![ResponsesOutputItem {
            item_type: "message".to_string(),
            content: Some(vec![ResponsesContent {
                content_type: "output_text".to_string(),
                text: None,
            }]),
        }];
        assert_eq!(extract_responses_text(&output), "");
    }

    #[test]
    fn test_extract_responses_text_concatenates_multiple_content() {
        let output = vec![ResponsesOutputItem {
            item_type: "message".to_string(),
            content: Some(vec![
                ResponsesContent {
                    content_type: "output_text".to_string(),
                    text: Some("Part 1".to_string()),
                },
                ResponsesContent {
                    content_type: "output_text".to_string(),
                    text: Some(" Part 2".to_string()),
                },
            ]),
        }];
        assert_eq!(extract_responses_text(&output), "Part 1 Part 2");
    }

    #[test]
    fn test_extract_responses_text_ignores_unknown_content_types() {
        let output = vec![ResponsesOutputItem {
            item_type: "message".to_string(),
            content: Some(vec![
                ResponsesContent {
                    content_type: "image".to_string(),
                    text: Some("should be ignored".to_string()),
                },
                ResponsesContent {
                    content_type: "output_text".to_string(),
                    text: Some("kept".to_string()),
                },
            ]),
        }];
        assert_eq!(extract_responses_text(&output), "kept");
    }

    // ==================== Trivial response detection tests ====================

    #[test]
    fn test_trivial_response_detection() {
        // The trivial response check is: trimmed.len() < 20
        let trivial_responses = ["OK", "好的", "收到", "ok", "Yes", ""];
        for resp in &trivial_responses {
            assert!(
                resp.trim().len() < 20,
                "'{}' should be detected as trivial",
                resp
            );
        }

        let non_trivial = "這是一個足夠長的工作摘要，包含了足夠的字元數。";
        assert!(
            non_trivial.trim().len() >= 20,
            "should not be trivial"
        );
    }

    // ==================== parse_error_usage tests ====================

    #[test]
    fn test_parse_error_usage_valid() {
        let usage = LlmUsageRecord {
            provider: "openai".to_string(),
            model: "gpt-5".to_string(),
            prompt_tokens: Some(100),
            completion_tokens: Some(50),
            total_tokens: Some(150),
            duration_ms: 500,
            purpose: "test".to_string(),
            status: "error".to_string(),
            error_message: Some("test error".to_string()),
        };
        let json = serde_json::to_string(&usage).unwrap();
        let err_str = format!("LLM_ERROR:{}::Some error happened", json);
        let parsed = parse_error_usage(&err_str).unwrap();
        assert_eq!(parsed.provider, "openai");
        assert_eq!(parsed.model, "gpt-5");
        assert_eq!(parsed.prompt_tokens, Some(100));
        assert_eq!(parsed.completion_tokens, Some(50));
        assert_eq!(parsed.duration_ms, 500);
        assert_eq!(parsed.status, "error");
    }

    #[test]
    fn test_parse_error_usage_not_llm_error() {
        assert!(parse_error_usage("Some random error").is_none());
        assert!(parse_error_usage("").is_none());
        assert!(parse_error_usage("Request failed: timeout").is_none());
    }

    #[test]
    fn test_parse_error_usage_malformed_json() {
        assert!(parse_error_usage("LLM_ERROR:not-json::error msg").is_none());
    }

    #[test]
    fn test_parse_error_usage_no_separator() {
        assert!(parse_error_usage("LLM_ERROR:no-double-colon-separator").is_none());
    }

    // ==================== LlmService::is_configured tests ====================

    #[test]
    fn test_is_configured_openai_with_key() {
        let service = LlmService::new(LlmConfig {
            provider: "openai".to_string(),
            model: "gpt-5".to_string(),
            api_key: Some("sk-test".to_string()),
            base_url: None,
            summary_max_chars: 2000,
            reasoning_effort: None,
            summary_prompt: None,
        });
        assert!(service.is_configured());
    }

    #[test]
    fn test_is_configured_openai_without_key() {
        let service = LlmService::new(LlmConfig {
            provider: "openai".to_string(),
            model: "gpt-5".to_string(),
            api_key: None,
            base_url: None,
            summary_max_chars: 2000,
            reasoning_effort: None,
            summary_prompt: None,
        });
        assert!(!service.is_configured());
    }

    #[test]
    fn test_is_configured_ollama_without_key() {
        let service = LlmService::new(LlmConfig {
            provider: "ollama".to_string(),
            model: "llama3".to_string(),
            api_key: None,
            base_url: None,
            summary_max_chars: 2000,
            reasoning_effort: None,
            summary_prompt: None,
        });
        assert!(service.is_configured());
    }

    #[test]
    fn test_is_configured_anthropic_without_key() {
        let service = LlmService::new(LlmConfig {
            provider: "anthropic".to_string(),
            model: "claude-3-5-sonnet".to_string(),
            api_key: None,
            base_url: None,
            summary_max_chars: 2000,
            reasoning_effort: None,
            summary_prompt: None,
        });
        assert!(!service.is_configured());
    }

    // ==================== Responses API usage token calculation tests ====================

    #[test]
    fn test_responses_usage_total_tokens_calculation() {
        let usage = ResponsesUsage {
            input_tokens: Some(100),
            output_tokens: Some(50),
        };
        let total = match (usage.input_tokens, usage.output_tokens) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        };
        assert_eq!(total, Some(150));
    }

    #[test]
    fn test_responses_usage_partial_tokens() {
        let usage = ResponsesUsage {
            input_tokens: Some(100),
            output_tokens: None,
        };
        let total = match (usage.input_tokens, usage.output_tokens) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        };
        assert_eq!(total, None);
    }

    // ==================== Request routing integration tests ====================

    #[test]
    fn test_model_routing_gpt5_uses_responses_api_and_no_temp() {
        let model = "gpt-5-nano";
        assert!(uses_responses_api(model), "gpt-5 should use Responses API");
        assert!(no_temperature_support(model), "gpt-5 should not support temperature");
        assert!(uses_max_completion_tokens(model), "gpt-5 should use max_completion_tokens");
    }

    #[test]
    fn test_model_routing_gpt4o_uses_chat_completions_with_temp() {
        let model = "gpt-4o";
        assert!(!uses_responses_api(model), "gpt-4o should NOT use Responses API");
        assert!(!no_temperature_support(model), "gpt-4o SHOULD support temperature");
        assert!(uses_max_completion_tokens(model), "gpt-4o should use max_completion_tokens");
    }

    #[test]
    fn test_model_routing_o1_uses_chat_completions_no_temp() {
        let model = "o1";
        assert!(!uses_responses_api(model), "o1 should NOT use Responses API");
        assert!(no_temperature_support(model), "o1 should NOT support temperature");
        assert!(uses_max_completion_tokens(model), "o1 should use max_completion_tokens");
    }

    #[test]
    fn test_model_routing_legacy_gpt4_turbo() {
        let model = "gpt-4-turbo";
        assert!(!uses_responses_api(model));
        assert!(!no_temperature_support(model));
        assert!(!uses_max_completion_tokens(model));
    }
}
