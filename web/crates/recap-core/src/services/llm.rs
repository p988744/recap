//! LLM Service for generating summaries and analysis
//! Supports OpenAI, Anthropic, Ollama, and OpenAI-compatible APIs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: String,      // "openai", "anthropic", "ollama", "openai-compatible"
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
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
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

pub struct LlmService {
    config: LlmConfig,
    client: reqwest::Client,
}

impl LlmService {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Check if the LLM service is configured
    pub fn is_configured(&self) -> bool {
        match self.config.provider.as_str() {
            "ollama" => true, // Ollama doesn't need API key
            _ => self.config.api_key.is_some(),
        }
    }

    /// Generate a summary of work session content
    pub async fn summarize_session(&self, content: &str) -> Result<String, String> {
        let prompt = format!(
            r#"請將以下 Claude Code 工作 session 內容整理成簡潔的工作摘要（50-100字）。
重點描述：
1. 主要完成了什麼任務
2. 使用了哪些技術或工具
3. 解決了什麼問題

Session 內容：
{}

請用繁體中文回答，直接輸出摘要內容，不要加任何前綴或說明。"#,
            content.chars().take(4000).collect::<String>()
        );

        self.complete(&prompt).await
    }

    /// Generate a project work summary for Tempo reporting
    pub async fn summarize_project_work(&self, project: &str, work_items: &str) -> Result<Vec<String>, String> {
        let prompt = format!(
            r#"你是一個工作報告助手。請將以下「{project}」專案的工作項目整理成 3-5 條簡潔的工作摘要。

工作項目：
{work_items}

要求：
1. 每條摘要 10-30 字
2. 使用動詞開頭（如：實作、研究、修復、設計、優化）
3. 合併相似的工作項目
4. 突出技術細節和成果
5. 使用繁體中文

請直接輸出摘要清單，每行一條，不要編號，不要其他說明。"#,
            project = project,
            work_items = work_items.chars().take(3000).collect::<String>()
        );

        let response = self.complete(&prompt).await?;

        let summaries: Vec<String> = response
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && s.len() > 3)
            .map(|s| s.trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == '-' || c == '•' || c == '*').trim().to_string())
            .filter(|s| !s.is_empty())
            .take(5)
            .collect();

        Ok(summaries)
    }

    /// Generate a daily work summary
    pub async fn summarize_daily_work(&self, sessions_info: &str, commits_info: &str) -> Result<String, String> {
        let prompt = format!(
            r#"請根據以下工作記錄整理成每日工作摘要（100-200字）。

Claude Code Sessions:
{}

Git Commits:
{}

請用繁體中文撰寫摘要，包含：
1. 今日主要工作內容
2. 完成的功能或修復
3. 使用的技術

直接輸出摘要內容，不要加任何前綴。"#,
            sessions_info.chars().take(2000).collect::<String>(),
            commits_info.chars().take(1000).collect::<String>()
        );

        self.complete(&prompt).await
    }

    /// Summarize a work period at a given time scale.
    /// `context` is the previous period's summary (for continuity).
    /// `current_data` is the current period's data to summarize.
    /// `scale` controls output length: hourly=50-100, daily=100-200, weekly=200-400, monthly=300-500 chars.
    pub async fn summarize_work_period(
        &self,
        context: &str,
        current_data: &str,
        scale: &str,
    ) -> Result<String, String> {
        let (length_hint, max_chars) = match scale {
            "hourly" => ("50-100字", 4000),
            "daily" => ("100-200字", 6000),
            "weekly" => ("200-400字", 8000),
            "monthly" => ("300-500字", 10000),
            _ => ("100-200字", 6000),
        };

        let context_section = if context.is_empty() {
            String::new()
        } else {
            format!(
                "\n前一時段摘要（作為前後文參考）：\n{}\n",
                context.chars().take(1000).collect::<String>()
            )
        };

        let prompt = format!(
            r#"你是工作記錄助手。請根據以下工作資料，產生簡潔的工作摘要（{length_hint}）。
{context_section}
本時段的工作資料：
{data}

請用繁體中文回答，重點描述完成了什麼、使用什麼技術、解決什麼問題。
若有 git commit，優先以 commit 訊息作為成果總結。
直接輸出摘要內容，不要加任何前綴或說明。"#,
            length_hint = length_hint,
            context_section = context_section,
            data = current_data.chars().take(max_chars).collect::<String>()
        );

        self.complete(&prompt).await
    }

    /// Send completion request to LLM
    async fn complete(&self, prompt: &str) -> Result<String, String> {
        match self.config.provider.as_str() {
            "openai" | "openai-compatible" => self.complete_openai(prompt).await,
            "anthropic" => self.complete_anthropic(prompt).await,
            "ollama" => self.complete_ollama(prompt).await,
            _ => Err(format!("Unsupported LLM provider: {}", self.config.provider)),
        }
    }

    async fn complete_openai(&self, prompt: &str) -> Result<String, String> {
        let api_key = self.config.api_key.as_ref()
            .ok_or("OpenAI API key not configured")?;

        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://api.openai.com/v1");

        let request = OpenAIRequest {
            model: self.config.model.clone(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: 500,
            temperature: 0.3,
        };

        let response = self.client
            .post(format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
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

        let result: OpenAIResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        result.choices.first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| "No response from LLM".to_string())
    }

    async fn complete_anthropic(&self, prompt: &str) -> Result<String, String> {
        let api_key = self.config.api_key.as_ref()
            .ok_or("Anthropic API key not configured")?;

        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: 500,
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

        result.content.first()
            .map(|c| c.text.clone())
            .ok_or_else(|| "No response from LLM".to_string())
    }

    async fn complete_ollama(&self, prompt: &str) -> Result<String, String> {
        let base_url = self.config.base_url.as_deref()
            .unwrap_or("http://localhost:11434");

        // Ollama uses OpenAI-compatible API
        let request = OpenAIRequest {
            model: self.config.model.clone(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: 500,
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

        result.choices.first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| "No response from Ollama".to_string())
    }
}

/// Create LLM service from database config
pub async fn create_llm_service(pool: &sqlx::SqlitePool, user_id: &str) -> Result<LlmService, String> {
    let row: (Option<String>, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT llm_provider, llm_model, llm_api_key, llm_base_url FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "User not found".to_string())?;

    let config = LlmConfig {
        provider: row.0.unwrap_or_else(|| "openai".to_string()),
        model: row.1.unwrap_or_else(|| "gpt-4o-mini".to_string()),
        api_key: row.2,
        base_url: row.3,
    };

    Ok(LlmService::new(config))
}
