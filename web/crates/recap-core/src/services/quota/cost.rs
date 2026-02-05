//! Cost calculation from Claude Code JSONL session files
//!
//! This module provides functionality to calculate token usage and costs
//! from local Claude Code session files, similar to CodexBar/ccusage.
//!
//! # Overview
//!
//! Claude Code stores session data in JSONL files under `~/.claude/projects/`.
//! Each line contains message data including token usage information.
//!
//! # Pricing
//!
//! Pricing is based on Anthropic's published rates (per million tokens):
//! - Claude Opus 4.5: $5/$25 (input/output), $6.25/$0.50 (cache write/read)
//! - Claude Sonnet 4: $3/$15 (input/output), $3.75/$0.30 (cache write/read)
//! - Claude 3.5 Sonnet: $3/$15 (input/output), $3.75/$0.30 (cache write/read)
//! - Claude 3 Opus: $15/$75 (input/output)

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Pricing Constants (per token, not per million)
// ============================================================================

/// Pricing for Claude Opus 4/4.5 models
mod opus_pricing {
    pub const INPUT: f64 = 5.0 / 1_000_000.0;
    pub const OUTPUT: f64 = 25.0 / 1_000_000.0;
    pub const CACHE_WRITE: f64 = 6.25 / 1_000_000.0;
    pub const CACHE_READ: f64 = 0.50 / 1_000_000.0;
}

/// Pricing for Claude Sonnet 4 / 3.5 Sonnet models
mod sonnet_pricing {
    pub const INPUT: f64 = 3.0 / 1_000_000.0;
    pub const OUTPUT: f64 = 15.0 / 1_000_000.0;
    pub const CACHE_WRITE: f64 = 3.75 / 1_000_000.0;
    pub const CACHE_READ: f64 = 0.30 / 1_000_000.0;
}

/// Pricing for Claude 3 Opus (legacy)
mod opus3_pricing {
    pub const INPUT: f64 = 15.0 / 1_000_000.0;
    pub const OUTPUT: f64 = 75.0 / 1_000_000.0;
    pub const CACHE_WRITE: f64 = 18.75 / 1_000_000.0;
    pub const CACHE_READ: f64 = 1.50 / 1_000_000.0;
}

/// Pricing for Claude Haiku models
mod haiku_pricing {
    pub const INPUT: f64 = 0.80 / 1_000_000.0;
    pub const OUTPUT: f64 = 4.0 / 1_000_000.0;
    pub const CACHE_WRITE: f64 = 1.0 / 1_000_000.0;
    pub const CACHE_READ: f64 = 0.08 / 1_000_000.0;
}

// ============================================================================
// Types
// ============================================================================

/// Token usage for a single API call
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
}

impl TokenUsage {
    pub fn total_tokens(&self) -> i64 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }
}

/// Aggregated usage for a specific model
#[derive(Debug, Clone, Default, Serialize)]
pub struct ModelUsage {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub total_cost: f64,
}

/// Daily usage summary
#[derive(Debug, Clone, Serialize)]
pub struct DailyUsage {
    pub date: String, // YYYY-MM-DD format
    pub total_tokens: i64,
    pub total_cost: f64,
    pub models: Vec<ModelUsage>,
}

/// Cost summary for a time period
#[derive(Debug, Clone, Serialize)]
pub struct CostSummary {
    /// Total cost for today
    pub today_cost: f64,
    /// Total tokens for today
    pub today_tokens: i64,
    /// Total cost for the last 30 days
    pub last_30_days_cost: f64,
    /// Total tokens for the last 30 days
    pub last_30_days_tokens: i64,
    /// Daily breakdown
    pub daily_usage: Vec<DailyUsage>,
    /// Per-model breakdown
    pub model_breakdown: Vec<ModelUsage>,
}

// ============================================================================
// JSONL Parsing Types
// ============================================================================

/// A line in the JSONL file
#[derive(Debug, Deserialize)]
struct JsonlLine {
    #[serde(rename = "type")]
    line_type: Option<String>,
    message: Option<MessageData>,
}

/// Message data containing usage info
#[derive(Debug, Deserialize)]
struct MessageData {
    model: Option<String>,
    usage: Option<UsageData>,
}

/// Token usage data from the API response
#[derive(Debug, Deserialize)]
struct UsageData {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
}

// ============================================================================
// Cost Calculator
// ============================================================================

/// Calculator for token costs from JSONL files
pub struct CostCalculator {
    /// Root directory for Claude projects (default: ~/.claude/projects)
    projects_root: PathBuf,
}

impl CostCalculator {
    /// Create a new cost calculator with default projects root
    pub fn new() -> Self {
        let projects_root = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude")
            .join("projects");
        Self { projects_root }
    }

    /// Create with custom projects root (for testing)
    pub fn with_root(projects_root: PathBuf) -> Self {
        Self { projects_root }
    }

    /// Calculate cost summary for the last N days
    pub fn calculate_summary(&self, days: u32) -> CostSummary {
        let today = Utc::now().date_naive();
        let start_date = today - chrono::Duration::days(days as i64 - 1);

        // Collect all JSONL files
        let jsonl_files = self.find_jsonl_files();
        log::debug!(
            "[cost] Found {} JSONL files in {:?}",
            jsonl_files.len(),
            self.projects_root
        );

        // Parse and aggregate usage by date and model
        let mut daily_map: HashMap<NaiveDate, HashMap<String, ModelUsage>> = HashMap::new();

        for file_path in jsonl_files {
            self.process_jsonl_file(&file_path, start_date, &mut daily_map);
        }

        // Convert to sorted daily usage list
        let mut daily_usage: Vec<DailyUsage> = daily_map
            .into_iter()
            .map(|(date, models)| {
                let model_list: Vec<ModelUsage> = models.into_values().collect();
                let total_tokens: i64 = model_list
                    .iter()
                    .map(|m| {
                        m.input_tokens
                            + m.output_tokens
                            + m.cache_creation_tokens
                            + m.cache_read_tokens
                    })
                    .sum();
                let total_cost: f64 = model_list.iter().map(|m| m.total_cost).sum();

                DailyUsage {
                    date: date.format("%Y-%m-%d").to_string(),
                    total_tokens,
                    total_cost,
                    models: model_list,
                }
            })
            .collect();

        daily_usage.sort_by(|a, b| a.date.cmp(&b.date));

        // Calculate totals
        let today_str = today.format("%Y-%m-%d").to_string();
        let today_usage = daily_usage.iter().find(|d| d.date == today_str);
        let today_cost = today_usage.map(|d| d.total_cost).unwrap_or(0.0);
        let today_tokens = today_usage.map(|d| d.total_tokens).unwrap_or(0);

        let last_30_days_cost: f64 = daily_usage.iter().map(|d| d.total_cost).sum();
        let last_30_days_tokens: i64 = daily_usage.iter().map(|d| d.total_tokens).sum();

        // Aggregate model breakdown
        let mut model_totals: HashMap<String, ModelUsage> = HashMap::new();
        for day in &daily_usage {
            for model in &day.models {
                let entry = model_totals
                    .entry(model.model.clone())
                    .or_insert_with(|| ModelUsage {
                        model: model.model.clone(),
                        ..Default::default()
                    });
                entry.input_tokens += model.input_tokens;
                entry.output_tokens += model.output_tokens;
                entry.cache_creation_tokens += model.cache_creation_tokens;
                entry.cache_read_tokens += model.cache_read_tokens;
                entry.total_cost += model.total_cost;
            }
        }

        let mut model_breakdown: Vec<ModelUsage> = model_totals.into_values().collect();
        model_breakdown.sort_by(|a, b| b.total_cost.partial_cmp(&a.total_cost).unwrap());

        CostSummary {
            today_cost,
            today_tokens,
            last_30_days_cost,
            last_30_days_tokens,
            daily_usage,
            model_breakdown,
        }
    }

    /// Find all JSONL files in the projects directory
    fn find_jsonl_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        if !self.projects_root.exists() {
            log::warn!("[cost] Projects root does not exist: {:?}", self.projects_root);
            return files;
        }

        // Walk the directory tree
        if let Ok(entries) = std::fs::read_dir(&self.projects_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Look for JSONL files in subdirectories
                    if let Ok(sub_entries) = std::fs::read_dir(&path) {
                        for sub_entry in sub_entries.flatten() {
                            let sub_path = sub_entry.path();
                            if sub_path.extension().map_or(false, |ext| ext == "jsonl") {
                                files.push(sub_path);
                            }
                        }
                    }
                }
            }
        }

        files
    }

    /// Process a single JSONL file and aggregate usage
    fn process_jsonl_file(
        &self,
        file_path: &PathBuf,
        start_date: NaiveDate,
        daily_map: &mut HashMap<NaiveDate, HashMap<String, ModelUsage>>,
    ) {
        // Get file modification date as a fallback
        let file_date = std::fs::metadata(file_path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| {
                let datetime = chrono::DateTime::<Utc>::from(t);
                Some(datetime.date_naive())
            });

        // Try to extract date from filename (format: session_YYYYMMDD_*.jsonl or similar)
        let filename_date = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|name| {
                // Try various date patterns in filename
                if name.len() >= 8 {
                    // Look for YYYYMMDD pattern
                    for i in 0..name.len().saturating_sub(7) {
                        if let Ok(date) = NaiveDate::parse_from_str(&name[i..i + 8], "%Y%m%d") {
                            return Some(date);
                        }
                    }
                }
                None
            });

        let use_date = filename_date.or(file_date).unwrap_or_else(|| Utc::now().date_naive());

        // Skip files older than start_date
        if use_date < start_date {
            return;
        }

        // Read and parse the file
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[cost] Failed to read {:?}: {}", file_path, e);
                return;
            }
        };

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Parse the JSON line
            let parsed: Result<JsonlLine, _> = serde_json::from_str(line);
            let json_line = match parsed {
                Ok(l) => l,
                Err(_) => continue, // Skip malformed lines
            };

            // Extract usage data
            if let Some(message) = json_line.message {
                if let (Some(model), Some(usage)) = (message.model, message.usage) {
                    let token_usage = TokenUsage {
                        input_tokens: usage.input_tokens.unwrap_or(0),
                        output_tokens: usage.output_tokens.unwrap_or(0),
                        cache_creation_tokens: usage.cache_creation_input_tokens.unwrap_or(0),
                        cache_read_tokens: usage.cache_read_input_tokens.unwrap_or(0),
                    };

                    // Calculate cost for this usage
                    let cost = self.calculate_cost(&model, &token_usage);

                    // Add to daily map
                    let day_models = daily_map.entry(use_date).or_insert_with(HashMap::new);
                    let model_usage = day_models.entry(model.clone()).or_insert_with(|| ModelUsage {
                        model: model.clone(),
                        ..Default::default()
                    });

                    model_usage.input_tokens += token_usage.input_tokens;
                    model_usage.output_tokens += token_usage.output_tokens;
                    model_usage.cache_creation_tokens += token_usage.cache_creation_tokens;
                    model_usage.cache_read_tokens += token_usage.cache_read_tokens;
                    model_usage.total_cost += cost;
                }
            }
        }
    }

    /// Calculate cost for a given model and token usage
    fn calculate_cost(&self, model: &str, usage: &TokenUsage) -> f64 {
        let model_lower = model.to_lowercase();

        // Determine pricing based on model name
        let (input_price, output_price, cache_write_price, cache_read_price) =
            if model_lower.contains("opus-4") || model_lower.contains("opus-4-5") {
                (
                    opus_pricing::INPUT,
                    opus_pricing::OUTPUT,
                    opus_pricing::CACHE_WRITE,
                    opus_pricing::CACHE_READ,
                )
            } else if model_lower.contains("opus-3") || model_lower.contains("opus-20240229") {
                (
                    opus3_pricing::INPUT,
                    opus3_pricing::OUTPUT,
                    opus3_pricing::CACHE_WRITE,
                    opus3_pricing::CACHE_READ,
                )
            } else if model_lower.contains("haiku") {
                (
                    haiku_pricing::INPUT,
                    haiku_pricing::OUTPUT,
                    haiku_pricing::CACHE_WRITE,
                    haiku_pricing::CACHE_READ,
                )
            } else {
                // Default to Sonnet pricing (most common)
                (
                    sonnet_pricing::INPUT,
                    sonnet_pricing::OUTPUT,
                    sonnet_pricing::CACHE_WRITE,
                    sonnet_pricing::CACHE_READ,
                )
            };

        let cost = (usage.input_tokens as f64 * input_price)
            + (usage.output_tokens as f64 * output_price)
            + (usage.cache_creation_tokens as f64 * cache_write_price)
            + (usage.cache_read_tokens as f64 * cache_read_price);

        cost
    }
}

impl Default for CostCalculator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage_total() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_tokens: 200,
            cache_read_tokens: 300,
        };
        assert_eq!(usage.total_tokens(), 650);
    }

    #[test]
    fn test_calculate_cost_opus() {
        let calc = CostCalculator::new();
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 100_000,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        };

        let cost = calc.calculate_cost("claude-opus-4-5-20251101", &usage);
        // $5/M input + $25/M * 0.1M output = $5 + $2.5 = $7.5
        assert!((cost - 7.5).abs() < 0.01);
    }

    #[test]
    fn test_calculate_cost_sonnet() {
        let calc = CostCalculator::new();
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 100_000,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        };

        let cost = calc.calculate_cost("claude-sonnet-4-20250514", &usage);
        // $3/M input + $15/M * 0.1M output = $3 + $1.5 = $4.5
        assert!((cost - 4.5).abs() < 0.01);
    }

    #[test]
    fn test_calculate_cost_with_cache() {
        let calc = CostCalculator::new();
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_tokens: 10000,
            cache_read_tokens: 50000,
        };

        let cost = calc.calculate_cost("claude-sonnet-4", &usage);
        // input: 100 * $3/M = $0.0003
        // output: 50 * $15/M = $0.00075
        // cache_write: 10000 * $3.75/M = $0.0375
        // cache_read: 50000 * $0.30/M = $0.015
        // total = $0.05355
        assert!((cost - 0.05355).abs() < 0.001);
    }

    #[test]
    fn test_model_detection() {
        let calc = CostCalculator::new();
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        };

        // Opus 4.5
        let cost1 = calc.calculate_cost("claude-opus-4-5-20251101", &usage);
        assert!((cost1 - 5.0).abs() < 0.01);

        // Opus 3
        let cost2 = calc.calculate_cost("claude-3-opus-20240229", &usage);
        assert!((cost2 - 15.0).abs() < 0.01);

        // Sonnet
        let cost3 = calc.calculate_cost("claude-3-5-sonnet-20241022", &usage);
        assert!((cost3 - 3.0).abs() < 0.01);

        // Haiku
        let cost4 = calc.calculate_cost("claude-3-5-haiku-20241022", &usage);
        assert!((cost4 - 0.80).abs() < 0.01);
    }
}
