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

/// Parse LLM response into summary lines (exported for testing)
pub(crate) fn parse_summary_response(response: &str) -> Vec<String> {
    response
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && s.len() > 3)
        .map(|s| s.trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == '-' || c == '•' || c == '*').trim().to_string())
        .filter(|s| !s.is_empty())
        .take(5)
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // LlmConfig Tests
    // ========================================================================

    #[test]
    fn test_llm_config_creation() {
        let config = LlmConfig {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: Some("sk-test".to_string()),
            base_url: None,
        };

        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.api_key, Some("sk-test".to_string()));
        assert!(config.base_url.is_none());
    }

    #[test]
    fn test_llm_config_with_custom_base_url() {
        let config = LlmConfig {
            provider: "openai-compatible".to_string(),
            model: "custom-model".to_string(),
            api_key: Some("key".to_string()),
            base_url: Some("https://custom-api.example.com".to_string()),
        };

        assert_eq!(config.provider, "openai-compatible");
        assert_eq!(config.base_url, Some("https://custom-api.example.com".to_string()));
    }

    // ========================================================================
    // LlmService Tests
    // ========================================================================

    #[test]
    fn test_llm_service_new() {
        let config = LlmConfig {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: Some("sk-test".to_string()),
            base_url: None,
        };

        let service = LlmService::new(config);
        assert!(service.is_configured());
    }

    #[test]
    fn test_is_configured_openai_with_key() {
        let config = LlmConfig {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: Some("sk-test".to_string()),
            base_url: None,
        };

        let service = LlmService::new(config);
        assert!(service.is_configured());
    }

    #[test]
    fn test_is_configured_openai_without_key() {
        let config = LlmConfig {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: None,
            base_url: None,
        };

        let service = LlmService::new(config);
        assert!(!service.is_configured());
    }

    #[test]
    fn test_is_configured_anthropic_with_key() {
        let config = LlmConfig {
            provider: "anthropic".to_string(),
            model: "claude-3".to_string(),
            api_key: Some("sk-ant-test".to_string()),
            base_url: None,
        };

        let service = LlmService::new(config);
        assert!(service.is_configured());
    }

    #[test]
    fn test_is_configured_anthropic_without_key() {
        let config = LlmConfig {
            provider: "anthropic".to_string(),
            model: "claude-3".to_string(),
            api_key: None,
            base_url: None,
        };

        let service = LlmService::new(config);
        assert!(!service.is_configured());
    }

    #[test]
    fn test_is_configured_ollama_no_key_required() {
        let config = LlmConfig {
            provider: "ollama".to_string(),
            model: "llama2".to_string(),
            api_key: None,
            base_url: Some("http://localhost:11434".to_string()),
        };

        let service = LlmService::new(config);
        // Ollama doesn't require API key
        assert!(service.is_configured());
    }

    #[test]
    fn test_is_configured_openai_compatible_with_key() {
        let config = LlmConfig {
            provider: "openai-compatible".to_string(),
            model: "custom".to_string(),
            api_key: Some("key".to_string()),
            base_url: Some("https://api.example.com".to_string()),
        };

        let service = LlmService::new(config);
        assert!(service.is_configured());
    }

    #[test]
    fn test_is_configured_openai_compatible_without_key() {
        let config = LlmConfig {
            provider: "openai-compatible".to_string(),
            model: "custom".to_string(),
            api_key: None,
            base_url: Some("https://api.example.com".to_string()),
        };

        let service = LlmService::new(config);
        assert!(!service.is_configured());
    }

    // ========================================================================
    // Response Parsing Tests
    // ========================================================================

    #[test]
    fn test_parse_summary_response_basic() {
        let response = "實作用戶認證功能\n優化資料庫查詢效能\n修復表單驗證錯誤";
        let summaries = parse_summary_response(response);

        assert_eq!(summaries.len(), 3);
        assert_eq!(summaries[0], "實作用戶認證功能");
        assert_eq!(summaries[1], "優化資料庫查詢效能");
        assert_eq!(summaries[2], "修復表單驗證錯誤");
    }

    #[test]
    fn test_parse_summary_response_with_numbers() {
        let response = "1. 實作用戶認證功能\n2. 優化資料庫查詢\n3. 修復錯誤";
        let summaries = parse_summary_response(response);

        assert_eq!(summaries.len(), 3);
        assert_eq!(summaries[0], "實作用戶認證功能");
        assert_eq!(summaries[1], "優化資料庫查詢");
        assert_eq!(summaries[2], "修復錯誤");
    }

    #[test]
    fn test_parse_summary_response_with_bullets() {
        let response = "• 實作功能\n- 優化效能\n* 修復錯誤";
        let summaries = parse_summary_response(response);

        assert_eq!(summaries.len(), 3);
        assert_eq!(summaries[0], "實作功能");
        assert_eq!(summaries[1], "優化效能");
        assert_eq!(summaries[2], "修復錯誤");
    }

    #[test]
    fn test_parse_summary_response_filters_short() {
        let response = "實作用戶認證功能\nab\nc\n優化資料庫";
        let summaries = parse_summary_response(response);

        // Should filter out "ab" and "c" (too short)
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0], "實作用戶認證功能");
        assert_eq!(summaries[1], "優化資料庫");
    }

    #[test]
    fn test_parse_summary_response_filters_empty_lines() {
        let response = "實作功能\n\n\n優化效能\n   \n修復錯誤";
        let summaries = parse_summary_response(response);

        assert_eq!(summaries.len(), 3);
    }

    #[test]
    fn test_parse_summary_response_max_five() {
        let response = "項目1\n項目2\n項目3\n項目4\n項目5\n項目6\n項目7";
        let summaries = parse_summary_response(response);

        // Should only return first 5
        assert_eq!(summaries.len(), 5);
    }

    #[test]
    fn test_parse_summary_response_trims_whitespace() {
        let response = "  實作功能  \n\t優化效能\t\n  修復錯誤  ";
        let summaries = parse_summary_response(response);

        assert_eq!(summaries.len(), 3);
        assert_eq!(summaries[0], "實作功能");
        assert_eq!(summaries[1], "優化效能");
        assert_eq!(summaries[2], "修復錯誤");
    }

    #[test]
    fn test_parse_summary_response_empty() {
        let response = "";
        let summaries = parse_summary_response(response);
        assert!(summaries.is_empty());
    }

    #[test]
    fn test_parse_summary_response_only_short_lines() {
        let response = "a\nb\nc\nd";
        let summaries = parse_summary_response(response);
        assert!(summaries.is_empty());
    }

    #[test]
    fn test_parse_summary_response_mixed_prefixes() {
        let response = "1. 項目一\n• 項目二\n- 項目三\n* 項目四\n項目五";
        let summaries = parse_summary_response(response);

        assert_eq!(summaries.len(), 5);
        // All should have prefixes removed
        for summary in &summaries {
            assert!(!summary.starts_with('1'));
            assert!(!summary.starts_with('.'));
            assert!(!summary.starts_with('•'));
            assert!(!summary.starts_with('-'));
            assert!(!summary.starts_with('*'));
        }
    }

    // ========================================================================
    // Request/Response Struct Tests
    // ========================================================================

    #[test]
    fn test_openai_request_serialization() {
        let request = OpenAIRequest {
            model: "gpt-4".to_string(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: 500,
            temperature: 0.3,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"gpt-4\""));
        assert!(json.contains("\"max_tokens\":500"));
        assert!(json.contains("\"role\":\"user\""));
    }

    #[test]
    fn test_openai_response_deserialization() {
        let json = r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello, how can I help?"
                }
            }]
        }"#;

        let response: OpenAIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.content, "Hello, how can I help?");
    }

    #[test]
    fn test_anthropic_request_serialization() {
        let request = AnthropicRequest {
            model: "claude-3".to_string(),
            max_tokens: 500,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"claude-3\""));
        assert!(json.contains("\"max_tokens\":500"));
    }

    #[test]
    fn test_anthropic_response_deserialization() {
        let json = r#"{
            "content": [{
                "text": "Hello, I'm Claude."
            }]
        }"#;

        let response: AnthropicResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.content[0].text, "Hello, I'm Claude.");
    }
}
