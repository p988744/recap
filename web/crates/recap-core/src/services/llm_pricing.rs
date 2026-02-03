//! LLM Pricing Module
//!
//! Estimates cost for LLM API calls based on provider, model, and token counts.

/// Estimate the cost (in USD) given provider, model, and token counts.
pub fn estimate_cost(
    provider: &str,
    model: &str,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
) -> f64 {
    let (input_price, output_price) = get_pricing(provider, model);
    let input_cost = prompt_tokens.unwrap_or(0) as f64 * input_price / 1_000_000.0;
    let output_cost = completion_tokens.unwrap_or(0) as f64 * output_price / 1_000_000.0;
    input_cost + output_cost
}

/// Get pricing per 1M tokens (input, output) for a given provider and model.
fn get_pricing(provider: &str, model: &str) -> (f64, f64) {
    match provider {
        "openai" | "openai-compatible" => match model {
            m if m.starts_with("gpt-5-nano") => (0.10, 0.40),
            m if m.starts_with("gpt-5-mini") => (0.15, 0.60),
            m if m.starts_with("gpt-5") => (2.00, 8.00),
            m if m.starts_with("gpt-4.1-nano") => (0.10, 0.40),
            m if m.starts_with("gpt-4.1-mini") => (0.15, 0.60),
            m if m.starts_with("gpt-4.1") => (2.00, 8.00),
            m if m.starts_with("gpt-4o-mini") => (0.15, 0.60),
            m if m.starts_with("gpt-4o") => (2.50, 10.00),
            m if m.starts_with("gpt-4-turbo") => (10.00, 30.00),
            m if m.starts_with("gpt-4") => (30.00, 60.00),
            m if m.starts_with("gpt-3.5") => (0.50, 1.50),
            m if m.starts_with("o1-mini") => (3.00, 12.00),
            m if m.starts_with("o1") => (15.00, 60.00),
            _ => (1.00, 3.00), // Conservative default for unknown models
        },
        "anthropic" => match model {
            m if m.contains("claude-3-5-sonnet") || m.contains("claude-3.5-sonnet") => (3.00, 15.00),
            m if m.contains("claude-3-5-haiku") || m.contains("claude-3.5-haiku") => (0.80, 4.00),
            m if m.contains("claude-3-opus") => (15.00, 75.00),
            m if m.contains("claude-3-sonnet") => (3.00, 15.00),
            m if m.contains("claude-3-haiku") => (0.25, 1.25),
            _ => (3.00, 15.00), // Default to sonnet pricing
        },
        "ollama" => (0.0, 0.0), // Local, no cost
        _ => (0.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_cost_openai_gpt4o_mini() {
        let cost = estimate_cost("openai", "gpt-4o-mini", Some(1000), Some(500));
        // 1000 * 0.15 / 1M + 500 * 0.60 / 1M = 0.00015 + 0.0003 = 0.00045
        assert!((cost - 0.00045).abs() < 1e-10);
    }

    #[test]
    fn test_estimate_cost_anthropic_sonnet() {
        let cost = estimate_cost("anthropic", "claude-3-5-sonnet-20241022", Some(1000), Some(500));
        // 1000 * 3.0 / 1M + 500 * 15.0 / 1M = 0.003 + 0.0075 = 0.0105
        assert!((cost - 0.0105).abs() < 1e-10);
    }

    #[test]
    fn test_estimate_cost_ollama_free() {
        let cost = estimate_cost("ollama", "llama3", Some(10000), Some(5000));
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_estimate_cost_none_tokens() {
        let cost = estimate_cost("openai", "gpt-4o-mini", None, None);
        assert_eq!(cost, 0.0);
    }
}
