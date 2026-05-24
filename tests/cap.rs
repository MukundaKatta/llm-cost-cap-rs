use llm_cost_cap::{
    CapExceeded, CheckError, CostCap, EstimateError, EstimatedCost, ModelPrice, UnknownModel,
};
use std::collections::HashMap;

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// ---------- estimate ----------

#[test]
fn estimate_matches_hand_calculation_for_opus() {
    let cap = CostCap::new(10.0);
    let est = cap.estimate("claude-opus-4-7", 10_000, 4_000).unwrap();
    // opus is $15/M input, $75/M output
    assert!(approx(est.input_usd, 0.15));
    assert!(approx(est.output_usd, 0.30));
    assert_eq!(est.cached_input_usd, 0.0);
    assert!(approx(est.total_usd, 0.45));
}

#[test]
fn estimate_for_haiku_is_cheaper_than_opus() {
    let cap = CostCap::new(10.0);
    let opus = cap.estimate("claude-opus-4-7", 1000, 1000).unwrap();
    let haiku = cap.estimate("claude-haiku-4-5", 1000, 1000).unwrap();
    assert!(haiku.total_usd < opus.total_usd);
}

#[test]
fn estimate_returns_breakdown_fields() {
    let cap = CostCap::new(10.0);
    let est = cap.estimate("gpt-5", 1_000_000, 100_000).unwrap();
    assert!(approx(est.input_usd, 1.25));
    assert!(approx(est.output_usd, 1.0));
    assert!(approx(est.total_usd, 2.25));
}

#[test]
fn estimate_does_not_raise_even_when_over_cap() {
    let cap = CostCap::new(0.0001);
    let est = cap
        .estimate("claude-opus-4-7", 1_000_000, 1_000_000)
        .unwrap();
    assert!(est.total_usd > cap.cap_usd());
}

#[test]
fn estimate_for_estimated_cost_is_copy() {
    let cap = CostCap::new(10.0);
    let est: EstimatedCost = cap.estimate("gpt-5", 1, 1).unwrap();
    let copy = est;
    assert_eq!(est.total_usd, copy.total_usd);
}

// ---------- cached input ----------

#[test]
fn cached_input_reduces_cost_when_model_publishes_cached_rate() {
    let cap = CostCap::new(10.0);
    // sonnet publishes $0.30/M cached
    let est = cap
        .estimate_with_cached("claude-sonnet-4-6", 0, 0, 1_000_000)
        .unwrap();
    assert!(approx(est.cached_input_usd, 0.30));
    assert!(approx(est.total_usd, 0.30));
}

#[test]
fn cached_input_is_zero_when_model_has_no_cached_rate() {
    let cap = CostCap::new(10.0);
    let est = cap
        .estimate_with_cached("gpt-5", 0, 0, 1_000_000)
        .unwrap();
    assert_eq!(est.cached_input_usd, 0.0);
}

#[test]
fn cached_input_is_zero_when_count_is_zero_even_with_published_rate() {
    let cap = CostCap::new(10.0);
    let est = cap
        .estimate_with_cached("claude-opus-4-7", 100, 100, 0)
        .unwrap();
    assert_eq!(est.cached_input_usd, 0.0);
}

// ---------- check ----------

#[test]
fn check_passes_when_under_cap() {
    let cap = CostCap::new(0.50);
    let est = cap.check("claude-opus-4-7", 1000, 1000).unwrap();
    assert!(approx(est.total_usd, 0.090));
    assert!(est.total_usd < 0.50);
}

#[test]
fn check_returns_cap_exceeded_with_correct_fields() {
    let cap = CostCap::new(0.10);
    let err = cap.check("claude-opus-4-7", 10_000, 4_000).unwrap_err();
    match err {
        CheckError::CapExceeded(CapExceeded {
            projected_usd,
            cap_usd,
            model,
        }) => {
            assert_eq!(model, "claude-opus-4-7");
            assert!(approx(cap_usd, 0.10));
            assert!(approx(projected_usd, 0.45));
            assert!(projected_usd > cap_usd);
        }
        other => panic!("expected CapExceeded, got {other:?}"),
    }
}

#[test]
fn cap_exceeded_display_mentions_both_numbers() {
    let cap = CostCap::new(0.10);
    let err = cap.check("claude-opus-4-7", 10_000, 4_000).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("0.450000"));
    assert!(msg.contains("0.100000"));
    assert!(msg.contains("claude-opus-4-7"));
}

#[test]
fn check_equal_to_cap_passes_not_raises() {
    let mut cap = CostCap::new(1.0);
    // Build a model whose 1 input token costs exactly $1.
    cap.add_model("tester", ModelPrice::new(1_000_000.0, 0.0));
    let est = cap.check("tester", 1, 0).unwrap();
    assert!(approx(est.total_usd, 1.0));
}

#[test]
fn check_just_over_cap_fails() {
    let mut cap = CostCap::new(1.0);
    cap.add_model("tester", ModelPrice::new(1_000_001.0, 0.0));
    let err = cap.check("tester", 1, 0).unwrap_err();
    assert!(matches!(err, CheckError::CapExceeded(_)));
}

// ---------- run ----------

#[test]
fn run_invokes_fn_and_returns_result_when_under_cap() {
    let cap = CostCap::new(10.0);
    let result = cap
        .run("claude-sonnet-4-6", 500, 500, || String::from("echo: hello"))
        .unwrap();
    assert_eq!(result, "echo: hello");
}

#[test]
fn run_does_not_invoke_fn_when_over_cap() {
    let cap = CostCap::new(0.0001);
    let calls = std::cell::Cell::new(0u32);
    let err = cap
        .run("claude-opus-4-7", 10_000, 4_000, || {
            calls.set(calls.get() + 1);
            "should not happen"
        })
        .unwrap_err();
    assert!(matches!(err, CheckError::CapExceeded(_)));
    assert_eq!(calls.get(), 0);
}

#[test]
fn run_passes_captured_data_to_closure() {
    let cap = CostCap::new(10.0);
    let prefix = String::from("hi");
    let suffix = String::from("world");
    let out = cap
        .run("gpt-5", 10, 10, || format!("{prefix}:{suffix}"))
        .unwrap();
    assert_eq!(out, "hi:world");
}

// ---------- price-table behaviour ----------

#[test]
fn unknown_model_returns_unknown_model_error_from_check() {
    let cap = CostCap::new(1.0);
    let err = cap.check("some-random-model", 10, 10).unwrap_err();
    match err {
        CheckError::UnknownModel(UnknownModel { model }) => {
            assert_eq!(model, "some-random-model");
        }
        other => panic!("expected UnknownModel, got {other:?}"),
    }
}

#[test]
fn unknown_model_returns_unknown_model_error_from_estimate() {
    let cap = CostCap::new(1.0);
    let err = cap.estimate("some-random-model", 10, 10).unwrap_err();
    match err {
        EstimateError::UnknownModel(UnknownModel { model }) => {
            assert_eq!(model, "some-random-model");
        }
    }
}

#[test]
fn custom_prices_override_builtin_table_entirely() {
    let mut prices: HashMap<String, ModelPrice> = HashMap::new();
    prices.insert("my-private-model".to_string(), ModelPrice::new(2.0, 8.0));
    let cap = CostCap::with_prices(prices, 10.0);

    // built-in model is no longer known
    let err = cap.check("claude-opus-4-7", 10, 10).unwrap_err();
    assert!(matches!(err, CheckError::UnknownModel(_)));

    // custom model is gated against the new prices
    let est = cap.check("my-private-model", 1_000_000, 100_000).unwrap();
    assert!(approx(est.input_usd, 2.0));
    assert!(approx(est.output_usd, 0.8));
    assert!(approx(est.total_usd, 2.8));
}

#[test]
fn add_model_registers_one_model_into_an_existing_cap() {
    let mut cap = CostCap::new(10.0);
    cap.add_model("homemade", ModelPrice::new(0.5, 1.5));
    let est = cap.check("homemade", 1_000_000, 1_000_000).unwrap();
    assert!(approx(est.input_usd, 0.5));
    assert!(approx(est.output_usd, 1.5));
}

#[test]
fn caller_cannot_mutate_internal_price_table_via_input_map() {
    let mut custom: HashMap<String, ModelPrice> = HashMap::new();
    custom.insert("x".to_string(), ModelPrice::new(1.0, 1.0));
    let cap = CostCap::with_prices(custom.clone(), 10.0);
    // Caller adds another entry to their map after construction.
    custom.insert("y".to_string(), ModelPrice::new(2.0, 2.0));
    let err = cap.check("y", 10, 10).unwrap_err();
    assert!(matches!(err, CheckError::UnknownModel(_)));
}

#[test]
fn known_models_lists_registered_entries() {
    let mut prices: HashMap<String, ModelPrice> = HashMap::new();
    prices.insert("a".to_string(), ModelPrice::new(1.0, 2.0));
    prices.insert("b".to_string(), ModelPrice::new(3.0, 4.0));
    let cap = CostCap::with_prices(prices, 1.0);
    assert_eq!(cap.known_models(), vec!["a".to_string(), "b".to_string()]);
}

// ---------- constructor ----------

#[test]
#[should_panic(expected = "max_usd must be >= 0")]
fn negative_cap_panics_at_construction() {
    let _ = CostCap::new(-0.01);
}

#[test]
fn cap_usd_returns_configured_value() {
    let cap = CostCap::new(2.5);
    assert_eq!(cap.cap_usd(), 2.5);
}

// ---------- error std::error::Error contract ----------

#[test]
fn errors_implement_std_error() {
    fn assert_err<E: std::error::Error>() {}
    assert_err::<CapExceeded>();
    assert_err::<UnknownModel>();
    assert_err::<EstimateError>();
    assert_err::<CheckError>();
}
