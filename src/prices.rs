//! Built-in price table for common LLM models.
//!
//! All rates are USD per million tokens, reflecting published provider
//! pricing as of 2026-05-24. Where a vendor publishes a cached read rate
//! distinct from the standard input rate (currently Anthropic), it is
//! exposed via [`ModelPrice::cached_input_per_million_usd`].

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Per-million-token pricing for one model.
///
/// `cached_input_per_million_usd` covers vendor prompt-cache reads. Pass
/// `None` when the vendor does not publish a distinct cached read rate.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModelPrice {
    pub input_per_million_usd: f64,
    pub output_per_million_usd: f64,
    pub cached_input_per_million_usd: Option<f64>,
}

impl ModelPrice {
    pub const fn new(input_per_million_usd: f64, output_per_million_usd: f64) -> Self {
        Self {
            input_per_million_usd,
            output_per_million_usd,
            cached_input_per_million_usd: None,
        }
    }

    pub const fn with_cached(
        input_per_million_usd: f64,
        output_per_million_usd: f64,
        cached_input_per_million_usd: f64,
    ) -> Self {
        Self {
            input_per_million_usd,
            output_per_million_usd,
            cached_input_per_million_usd: Some(cached_input_per_million_usd),
        }
    }
}

// Canonical models. USD per 1M tokens as of 2026-05-24.
const BUILTIN: &[(&str, ModelPrice)] = &[
    // Anthropic
    ("claude-opus-4-7", ModelPrice::with_cached(15.00, 75.00, 1.50)),
    ("claude-opus-4-6", ModelPrice::with_cached(15.00, 75.00, 1.50)),
    ("claude-opus-4-5", ModelPrice::with_cached(15.00, 75.00, 1.50)),
    ("claude-sonnet-4-6", ModelPrice::with_cached(3.00, 15.00, 0.30)),
    ("claude-sonnet-4-5", ModelPrice::with_cached(3.00, 15.00, 0.30)),
    ("claude-haiku-4-5", ModelPrice::with_cached(0.80, 4.00, 0.08)),
    // OpenAI
    ("gpt-5.4", ModelPrice::new(1.25, 10.00)),
    ("gpt-5", ModelPrice::new(1.25, 10.00)),
    ("gpt-5-mini", ModelPrice::new(0.25, 2.00)),
    ("gpt-5-nano", ModelPrice::new(0.05, 0.40)),
    // Google Gemini
    ("gemini-2.5-pro", ModelPrice::new(1.25, 10.00)),
    ("gemini-2.5-flash", ModelPrice::new(0.30, 2.50)),
    // AWS Bedrock - Anthropic
    ("anthropic.claude-opus-4-7-v1:0", ModelPrice::with_cached(15.00, 75.00, 1.50)),
    ("anthropic.claude-sonnet-4-6-v1:0", ModelPrice::with_cached(3.00, 15.00, 0.30)),
    ("anthropic.claude-haiku-4-5-v1:0", ModelPrice::with_cached(0.80, 4.00, 0.08)),
    // AWS Bedrock - Meta
    ("meta.llama3-1-70b-instruct-v1:0", ModelPrice::new(2.65, 3.50)),
];

const ALIASES: &[(&str, &str)] = &[
    ("opus", "claude-opus-4-7"),
    ("sonnet", "claude-sonnet-4-6"),
    ("haiku", "claude-haiku-4-5"),
    ("gpt5", "gpt-5"),
    ("gemini-pro", "gemini-2.5-pro"),
    ("gemini-flash", "gemini-2.5-flash"),
];

/// Flat slice of (model, input_per_million_usd, output_per_million_usd)
/// for callers that want a quick read of the canonical table. Cached
/// rates are dropped; use [`builtin_prices`] for the full picture.
pub const MODEL_PRICES: &[(&str, f64, f64)] = &[
    ("claude-opus-4-7", 15.00, 75.00),
    ("claude-opus-4-6", 15.00, 75.00),
    ("claude-opus-4-5", 15.00, 75.00),
    ("claude-sonnet-4-6", 3.00, 15.00),
    ("claude-sonnet-4-5", 3.00, 15.00),
    ("claude-haiku-4-5", 0.80, 4.00),
    ("gpt-5.4", 1.25, 10.00),
    ("gpt-5", 1.25, 10.00),
    ("gpt-5-mini", 0.25, 2.00),
    ("gpt-5-nano", 0.05, 0.40),
    ("gemini-2.5-pro", 1.25, 10.00),
    ("gemini-2.5-flash", 0.30, 2.50),
    ("anthropic.claude-opus-4-7-v1:0", 15.00, 75.00),
    ("anthropic.claude-sonnet-4-6-v1:0", 3.00, 15.00),
    ("anthropic.claude-haiku-4-5-v1:0", 0.80, 4.00),
    ("meta.llama3-1-70b-instruct-v1:0", 2.65, 3.50),
];

/// Return a fresh map of the built-in price table with aliases resolved.
///
/// Callers receive an owned copy and can mutate it freely.
pub fn builtin_prices() -> HashMap<String, ModelPrice> {
    let mut table: HashMap<String, ModelPrice> = BUILTIN
        .iter()
        .map(|(id, p)| ((*id).to_string(), *p))
        .collect();
    for (alias, canonical) in ALIASES {
        if let Some(price) = table.get(*canonical).copied() {
            table.insert((*alias).to_string(), price);
        }
    }
    table
}

/// Sorted list of model ids (including aliases) the built-in table knows.
pub fn known_models() -> Vec<String> {
    let mut ids: Vec<String> = BUILTIN.iter().map(|(id, _)| (*id).to_string()).collect();
    for (alias, _) in ALIASES {
        ids.push((*alias).to_string());
    }
    ids.sort();
    ids.dedup();
    ids
}
