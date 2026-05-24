use llm_cost_cap::{builtin_prices, known_models, ModelPrice, MODEL_PRICES};

#[test]
fn builtin_prices_contains_required_models() {
    let table = builtin_prices();
    for model in [
        "claude-opus-4-7",
        "claude-sonnet-4-6",
        "claude-haiku-4-5",
        "gpt-5.4",
        "gpt-5",
        "gemini-2.5-pro",
        "gemini-2.5-flash",
    ] {
        assert!(table.contains_key(model), "missing required model {model:?}");
    }
}

#[test]
fn anthropic_models_have_cached_input_rate() {
    let table = builtin_prices();
    for model in ["claude-opus-4-7", "claude-sonnet-4-6", "claude-haiku-4-5"] {
        let price = table.get(model).unwrap();
        let cached = price
            .cached_input_per_million_usd
            .expect("{model} should publish a cached read rate");
        assert!(
            cached < price.input_per_million_usd,
            "cached should be cheaper than full input for {model}"
        );
    }
}

#[test]
fn short_aliases_resolve_to_canonical_models() {
    let table = builtin_prices();
    assert_eq!(table.get("opus"), table.get("claude-opus-4-7"));
    assert_eq!(table.get("sonnet"), table.get("claude-sonnet-4-6"));
    assert_eq!(table.get("haiku"), table.get("claude-haiku-4-5"));
}

#[test]
fn builtin_prices_returns_a_fresh_map_each_call() {
    let mut a = builtin_prices();
    let b = builtin_prices();
    a.insert("mutated".to_string(), ModelPrice::new(99.0, 99.0));
    assert!(!b.contains_key("mutated"));
}

#[test]
fn known_models_includes_aliases_and_canonical() {
    let models = known_models();
    assert!(models.iter().any(|m| m == "claude-opus-4-7"));
    assert!(models.iter().any(|m| m == "opus"));
}

#[test]
fn known_models_is_sorted() {
    let models = known_models();
    let mut sorted = models.clone();
    sorted.sort();
    assert_eq!(models, sorted);
}

#[test]
fn model_prices_const_matches_builtin_input_rates() {
    let table = builtin_prices();
    for (id, input, output) in MODEL_PRICES {
        let price = table.get(*id).unwrap_or_else(|| panic!("missing {id}"));
        assert!((price.input_per_million_usd - *input).abs() < 1e-9);
        assert!((price.output_per_million_usd - *output).abs() < 1e-9);
    }
}

#[test]
fn model_price_constructors() {
    let p = ModelPrice::new(1.0, 2.0);
    assert_eq!(p.input_per_million_usd, 1.0);
    assert_eq!(p.output_per_million_usd, 2.0);
    assert_eq!(p.cached_input_per_million_usd, None);

    let p2 = ModelPrice::with_cached(1.0, 2.0, 0.5);
    assert_eq!(p2.cached_input_per_million_usd, Some(0.5));
}

#[cfg(feature = "serde")]
#[test]
fn model_price_serde_roundtrip() {
    let p = ModelPrice::with_cached(1.0, 2.0, 0.5);
    let json = serde_json::to_string(&p).unwrap();
    let back: ModelPrice = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}
