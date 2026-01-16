//! Tempo report helper functions
//!
//! Shared utilities for tempo report generation.

use anyhow::Result;
use std::collections::HashMap;

/// Extract project name from title with [project] format
pub fn extract_project_name(title: &str) -> String {
    if let Some(start) = title.find('[') {
        if let Some(end) = title.find(']') {
            if end > start {
                return title[start + 1..end].to_string();
            }
        }
    }
    "其他".to_string()
}

/// Clean title by removing [project] prefix and truncating
pub fn clean_title(title: &str) -> String {
    let cleaned = if let Some(end) = title.find(']') {
        title[end + 1..].trim().to_string()
    } else {
        title.to_string()
    };

    // Truncate long titles
    let chars: Vec<char> = cleaned.chars().collect();
    if chars.len() > 60 {
        format!("{}...", chars[..57].iter().collect::<String>())
    } else {
        cleaned
    }
}

/// Generate smart summary from work items without LLM
pub fn generate_smart_summary(items: &[&recap_core::WorkItem]) -> Vec<String> {
    let mut summaries: Vec<String> = Vec::new();
    let mut seen_keywords: HashMap<String, f64> = HashMap::new();

    for item in items {
        let title = clean_title(&item.title).to_lowercase();

        // Extract keywords and group
        let keywords = extract_keywords(&title);
        for keyword in keywords {
            *seen_keywords.entry(keyword).or_insert(0.0) += item.hours;
        }
    }

    // Sort by hours and generate summaries
    let mut keyword_list: Vec<_> = seen_keywords.into_iter().collect();
    keyword_list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (keyword, _hours) in keyword_list.iter().take(5) {
        if !keyword.is_empty() {
            summaries.push(format_keyword_summary(keyword, items));
        }
    }

    // If no good summaries, use titles directly
    if summaries.is_empty() {
        for item in items.iter().take(5) {
            let title = clean_title(&item.title);
            if !title.is_empty() && title.len() > 3 {
                summaries.push(title);
            }
        }
    }

    summaries.into_iter().take(5).collect()
}

/// Extract keywords from title for grouping
pub fn extract_keywords(title: &str) -> Vec<String> {
    let mut keywords = Vec::new();

    // Common work-related keywords
    let patterns = [
        ("實作", "實作"),
        ("implement", "實作"),
        ("設計", "設計"),
        ("design", "設計"),
        ("測試", "測試"),
        ("test", "測試"),
        ("修復", "修復"),
        ("fix", "修復"),
        ("bug", "修復"),
        ("研究", "研究"),
        ("調查", "研究"),
        ("文件", "文件撰寫"),
        ("doc", "文件撰寫"),
        ("review", "程式碼審查"),
        ("審查", "程式碼審查"),
        ("部署", "部署"),
        ("deploy", "部署"),
        ("優化", "效能優化"),
        ("refactor", "程式碼重構"),
        ("重構", "程式碼重構"),
        ("cli", "CLI 開發"),
        ("api", "API 開發"),
        ("sync", "資料同步"),
        ("同步", "資料同步"),
        ("gitlab", "GitLab 整合"),
        ("git", "版本控制"),
    ];

    for (pattern, keyword) in patterns {
        if title.contains(pattern) {
            keywords.push(keyword.to_string());
        }
    }

    keywords
}

/// Format a keyword into a meaningful summary
pub fn format_keyword_summary(keyword: &str, items: &[&recap_core::WorkItem]) -> String {
    let related: Vec<_> = items.iter()
        .filter(|i| clean_title(&i.title).to_lowercase().contains(&keyword.to_lowercase())
            || extract_keywords(&clean_title(&i.title).to_lowercase()).contains(&keyword.to_string()))
        .collect();

    if related.is_empty() {
        return keyword.to_string();
    }

    // Use the most descriptive title
    let best = related.iter()
        .max_by_key(|i| i.hours as i64)
        .map(|i| clean_title(&i.title))
        .unwrap_or_else(|| keyword.to_string());

    if best.len() > 5 {
        best
    } else {
        keyword.to_string()
    }
}

/// Get or find default user for CLI operations (prefers user with LLM configured)
pub async fn get_default_user_id(db: &recap_core::Database) -> Result<String> {
    // First try to find a user with LLM API key configured
    let user_with_llm: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM users WHERE llm_api_key IS NOT NULL AND llm_api_key != '' LIMIT 1"
    )
        .fetch_optional(&db.pool)
        .await?;

    if let Some((id,)) = user_with_llm {
        return Ok(id);
    }

    // Fall back to any user
    let user: Option<(String,)> = sqlx::query_as("SELECT id FROM users LIMIT 1")
        .fetch_optional(&db.pool)
        .await?;

    match user {
        Some((id,)) => Ok(id),
        None => Err(anyhow::anyhow!("No user found. Please run the app first to create a user.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_project_name_with_brackets() {
        assert_eq!(extract_project_name("[recap] some work"), "recap");
        assert_eq!(extract_project_name("[funtime-website] task"), "funtime-website");
        assert_eq!(extract_project_name("[my-project] description here"), "my-project");
    }

    #[test]
    fn test_extract_project_name_no_brackets() {
        assert_eq!(extract_project_name("no project tag"), "其他");
        assert_eq!(extract_project_name("plain text"), "其他");
    }

    #[test]
    fn test_extract_project_name_empty() {
        assert_eq!(extract_project_name(""), "其他");
    }

    #[test]
    fn test_extract_project_name_malformed() {
        assert_eq!(extract_project_name("[unclosed"), "其他");
        assert_eq!(extract_project_name("no start]"), "其他");
        assert_eq!(extract_project_name("][backwards"), "其他");
    }

    #[test]
    fn test_clean_title_with_tag() {
        assert_eq!(clean_title("[recap] some work"), "some work");
        assert_eq!(clean_title("[project] task description"), "task description");
    }

    #[test]
    fn test_clean_title_no_tag() {
        assert_eq!(clean_title("no tag"), "no tag");
        assert_eq!(clean_title("plain text"), "plain text");
    }

    #[test]
    fn test_clean_title_truncates_long() {
        let long_title = "[project] ".to_string() + &"a".repeat(100);
        let cleaned = clean_title(&long_title);
        assert!(cleaned.len() <= 63); // 57 chars + "..."
        assert!(cleaned.ends_with("..."));
    }
}
