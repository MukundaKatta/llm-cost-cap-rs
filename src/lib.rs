/*!
llm-cost-cap: pre-flight USD cost gate for LLM calls.

Estimate the cost of a planned LLM call before making it. Raises
`BudgetExceeded` if the estimated cost exceeds the cap.

Built-in price tables for Anthropic, OpenAI, Gemini, and Bedrock
(prices as of 2026-05-24 — update via `custom_prices()`).

```rust
use llm_cost_cap::{CostCap, CostEstimate, ModelPrices};

let cap = CostCap::new(0.10); // $0.10 cap
let prices = ModelPrices::for_model("claude-opus-4-7").unwrap();
let estimate = cap.estimate(&prices, 1000, 500, None).unwrap();
assert!(estimate.total_usd > 0.0);
```
*/

use std::collections::HashMap;

/// Per-token prices for a model (USD per token).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelPrices {
    pub model: String,
    /// Cost per input token (USD).
    pub input_usd_per_token: f64,
    /// Cost per output token (USD).
    pub output_usd_per_token: f64,
    /// Cost per cached input token (USD). Defaults to input price if None.
    pub cached_usd_per_token: Option<f64>,
}

impl ModelPrices {
    pub fn new(model: &str, input: f64, output: f64) -> Self {
        Self {
            model: model.to_owned(),
            input_usd_per_token: input,
            output_usd_per_token: output,
            cached_usd_per_token: None,
        }
    }

    pub fn with_cache(mut self, cached: f64) -> Self {
        self.cached_usd_per_token = Some(cached);
        self
    }

    /// Look up built-in prices for a known model. Returns None if not found.
    pub fn for_model(model: &str) -> Option<Self> {
        built_in_prices().get(model).cloned()
    }

    /// Effective cached input price (falls back to full input price).
    pub fn effective_cached_price(&self) -> f64 {
        self.cached_usd_per_token.unwrap_or(self.input_usd_per_token)
    }
}

/// Result of a cost estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct CostEstimate {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub input_usd: f64,
    pub cached_usd: f64,
    pub output_usd: f64,
    pub total_usd: f64,
}

/// Raised when an estimated cost would exceed the cap.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetExceeded {
    pub estimated_usd: f64,
    pub cap_usd: f64,
    pub model: String,
}

impl std::fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BudgetExceeded: ${:.6} estimated > ${:.6} cap (model={})",
            self.estimated_usd, self.cap_usd, self.model
        )
    }
}

impl std::error::Error for BudgetExceeded {}

/// Pre-flight cost gate.
pub struct CostCap {
    pub cap_usd: f64,
}

impl CostCap {
    /// Create a cap at `cap_usd` dollars.
    pub fn new(cap_usd: f64) -> Self {
        Self { cap_usd }
    }

    /// Estimate the cost. Returns Err(BudgetExceeded) if estimate > cap.
    pub fn estimate(
        &self,
        prices: &ModelPrices,
        input_tokens: u64,
        output_tokens: u64,
        cached_tokens: Option<u64>,
    ) -> Result<CostEstimate, BudgetExceeded> {
        let cached = cached_tokens.unwrap_or(0);
        let non_cached_input = input_tokens.saturating_sub(cached);

        let input_usd = non_cached_input as f64 * prices.input_usd_per_token;
        let cached_usd = cached as f64 * prices.effective_cached_price();
        let output_usd = output_tokens as f64 * prices.output_usd_per_token;
        let total_usd = input_usd + cached_usd + output_usd;

        if total_usd > self.cap_usd {
            return Err(BudgetExceeded {
                estimated_usd: total_usd,
                cap_usd: self.cap_usd,
                model: prices.model.clone(),
            });
        }

        Ok(CostEstimate {
            model: prices.model.clone(),
            input_tokens,
            output_tokens,
            cached_tokens: cached,
            input_usd,
            cached_usd,
            output_usd,
            total_usd,
        })
    }

    /// Return just the estimated USD without checking the cap.
    pub fn estimate_usd(
        prices: &ModelPrices,
        input_tokens: u64,
        output_tokens: u64,
        cached_tokens: Option<u64>,
    ) -> f64 {
        let cached = cached_tokens.unwrap_or(0);
        let non_cached_input = input_tokens.saturating_sub(cached);
        let input_usd = non_cached_input as f64 * prices.input_usd_per_token;
        let cached_usd = cached as f64 * prices.effective_cached_price();
        let output_usd = output_tokens as f64 * prices.output_usd_per_token;
        input_usd + cached_usd + output_usd
    }
}

// Prices per token (USD). Source: public pricing pages, 2026-05-24.
fn built_in_prices() -> HashMap<String, ModelPrices> {
    let mut m = HashMap::new();

    // Anthropic Claude 4.x
    m.insert(
        "claude-opus-4-7".to_owned(),
        ModelPrices::new("claude-opus-4-7", 15.0 / 1_000_000.0, 75.0 / 1_000_000.0)
            .with_cache(1.5 / 1_000_000.0),
    );
    m.insert(
        "claude-sonnet-4-6".to_owned(),
        ModelPrices::new("claude-sonnet-4-6", 3.0 / 1_000_000.0, 15.0 / 1_000_000.0)
            .with_cache(0.3 / 1_000_000.0),
    );
    m.insert(
        "claude-haiku-4-5-20251001".to_owned(),
        ModelPrices::new("claude-haiku-4-5-20251001", 0.8 / 1_000_000.0, 4.0 / 1_000_000.0)
            .with_cache(0.08 / 1_000_000.0),
    );

    // OpenAI
    m.insert(
        "gpt-4o".to_owned(),
        ModelPrices::new("gpt-4o", 2.5 / 1_000_000.0, 10.0 / 1_000_000.0)
            .with_cache(1.25 / 1_000_000.0),
    );
    m.insert(
        "gpt-4o-mini".to_owned(),
        ModelPrices::new("gpt-4o-mini", 0.15 / 1_000_000.0, 0.6 / 1_000_000.0)
            .with_cache(0.075 / 1_000_000.0),
    );
    m.insert(
        "gpt-5.4".to_owned(),
        ModelPrices::new("gpt-5.4", 2.0 / 1_000_000.0, 8.0 / 1_000_000.0),
    );

    // Google Gemini
    m.insert(
        "gemini-2.5-pro".to_owned(),
        ModelPrices::new("gemini-2.5-pro", 1.25 / 1_000_000.0, 5.0 / 1_000_000.0),
    );
    m.insert(
        "gemini-2.5-flash".to_owned(),
        ModelPrices::new("gemini-2.5-flash", 0.15 / 1_000_000.0, 0.6 / 1_000_000.0),
    );

    // AWS Bedrock (pass-through Anthropic)
    m.insert(
        "us.anthropic.claude-opus-4-7-v1:0".to_owned(),
        ModelPrices::new(
            "us.anthropic.claude-opus-4-7-v1:0",
            15.0 / 1_000_000.0,
            75.0 / 1_000_000.0,
        ),
    );

    m
}

/// Return a list of all built-in model names.
pub fn known_models() -> Vec<String> {
    let mut names: Vec<String> = built_in_prices().into_keys().collect();
    names.sort();
    names
}

/// Register custom prices (overrides built-ins).
pub fn custom_prices(entries: impl IntoIterator<Item = ModelPrices>) -> HashMap<String, ModelPrices> {
    let mut base = built_in_prices();
    for p in entries {
        base.insert(p.model.clone(), p);
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_within_cap_ok() {
        let cap = CostCap::new(1.0);
        let prices = ModelPrices::for_model("claude-sonnet-4-6").unwrap();
        let est = cap.estimate(&prices, 1000, 500, None).unwrap();
        assert!(est.total_usd > 0.0);
        assert_eq!(est.model, "claude-sonnet-4-6");
    }

    #[test]
    fn estimate_exceeds_cap_err() {
        let cap = CostCap::new(0.000001);
        let prices = ModelPrices::for_model("claude-opus-4-7").unwrap();
        let err = cap.estimate(&prices, 100_000, 50_000, None).unwrap_err();
        assert!(err.estimated_usd > 0.0);
        assert_eq!(err.cap_usd, 0.000001);
    }

    #[test]
    fn zero_tokens_zero_cost() {
        let cap = CostCap::new(0.0);
        let prices = ModelPrices::new("test", 0.001, 0.002);
        let est = cap.estimate(&prices, 0, 0, None).unwrap();
        assert_eq!(est.total_usd, 0.0);
    }

    #[test]
    fn cached_tokens_use_lower_price() {
        let prices = ModelPrices::new("test", 1.0 / 1_000_000.0, 2.0 / 1_000_000.0)
            .with_cache(0.1 / 1_000_000.0);
        let cap = CostCap::new(10.0);
        // 100 input tokens, 50 cached of those
        let est = cap.estimate(&prices, 100, 10, Some(50)).unwrap();
        // non-cached input = 50 tokens * 1e-6 = 5e-5
        // cached = 50 tokens * 0.1e-6 = 5e-6
        // output = 10 tokens * 2e-6 = 2e-5
        assert!(est.cached_usd < est.input_usd);
    }

    #[test]
    fn no_cache_price_falls_back_to_input() {
        let prices = ModelPrices::new("test", 1.0 / 1_000_000.0, 2.0 / 1_000_000.0);
        assert_eq!(
            prices.effective_cached_price(),
            prices.input_usd_per_token
        );
    }

    #[test]
    fn for_model_known_returns_some() {
        assert!(ModelPrices::for_model("claude-opus-4-7").is_some());
        assert!(ModelPrices::for_model("gpt-4o").is_some());
        assert!(ModelPrices::for_model("gemini-2.5-pro").is_some());
    }

    #[test]
    fn for_model_unknown_returns_none() {
        assert!(ModelPrices::for_model("does-not-exist").is_none());
    }

    #[test]
    fn known_models_non_empty() {
        assert!(!known_models().is_empty());
    }

    #[test]
    fn known_models_sorted() {
        let m = known_models();
        let mut sorted = m.clone();
        sorted.sort();
        assert_eq!(m, sorted);
    }

    #[test]
    fn estimate_usd_static_fn() {
        let prices = ModelPrices::for_model("gpt-4o-mini").unwrap();
        let usd = CostCap::estimate_usd(&prices, 1000, 500, None);
        assert!(usd > 0.0);
    }

    #[test]
    fn budget_exceeded_display() {
        let err = BudgetExceeded {
            estimated_usd: 0.15,
            cap_usd: 0.10,
            model: "gpt-4o".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("gpt-4o"));
        assert!(s.contains("BudgetExceeded"));
    }

    #[test]
    fn cached_tokens_clamped_to_input() {
        // cached_tokens > input_tokens: non_cached_input saturates to 0
        let prices = ModelPrices::new("test", 1.0 / 1_000_000.0, 1.0 / 1_000_000.0)
            .with_cache(0.1 / 1_000_000.0);
        let cap = CostCap::new(100.0);
        let est = cap.estimate(&prices, 10, 5, Some(100)).unwrap();
        assert_eq!(est.input_usd, 0.0); // all tokens treated as cached
    }

    #[test]
    fn custom_prices_override() {
        let custom = ModelPrices::new("custom-model", 0.0, 0.0);
        let table = custom_prices([custom]);
        assert!(table.contains_key("custom-model"));
        assert!(table.contains_key("claude-opus-4-7")); // base still present
    }

    #[test]
    fn haiku_model_lookup() {
        assert!(ModelPrices::for_model("claude-haiku-4-5-20251001").is_some());
    }

    #[test]
    fn bedrock_model_lookup() {
        assert!(ModelPrices::for_model("us.anthropic.claude-opus-4-7-v1:0").is_some());
    }

    #[test]
    fn cost_estimate_fields_sum_correctly() {
        let prices = ModelPrices::new("m", 1.0 / 1_000_000.0, 2.0 / 1_000_000.0)
            .with_cache(0.5 / 1_000_000.0);
        let cap = CostCap::new(10.0);
        let est = cap.estimate(&prices, 200, 100, Some(50)).unwrap();
        let expected = est.input_usd + est.cached_usd + est.output_usd;
        assert!((est.total_usd - expected).abs() < 1e-12);
    }

    #[test]
    fn estimate_zero_cap_only_passes_zero_cost() {
        let cap = CostCap::new(0.0);
        let prices = ModelPrices::new("m", 0.0, 0.0);
        assert!(cap.estimate(&prices, 100, 100, None).is_ok());
    }
}
