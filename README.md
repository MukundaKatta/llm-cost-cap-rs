# llm-cost-cap

[![Crates.io](https://img.shields.io/crates/v/llm-cost-cap.svg)](https://crates.io/crates/llm-cost-cap)
[![CI](https://github.com/MukundaKatta/llm-cost-cap-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/MukundaKatta/llm-cost-cap-rs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

Pre-flight USD cost gate for a single LLM call.

Before sending a request, estimate the worst-case cost (input tokens at
the input rate plus the requested `max_output_tokens` at the output rate)
and reject if it would exceed a configured per-call cap. Catches the
failure mode where an agent burns $20 on one call before the call ever
leaves the process. Zero non-std deps. Built-in price table covers
Anthropic, OpenAI, Gemini, and Bedrock variants.

The gate **fails closed**: an unknown model, or a custom price that
produces a non-finite (NaN/∞) estimate, is rejected rather than waved
through. See [Errors](#errors).

## Install

```toml
[dependencies]
llm-cost-cap = "0.1"
```

## Example

```rust
use llm_cost_cap::{CostCap, CheckError};

let cap = CostCap::new(0.50);

match cap.check("claude-opus-4-7", 10_000, 4_000) {
    Ok(est) => println!("ok, projected ${:.4}", est.total_usd),
    Err(CheckError::CapExceeded(e)) => {
        println!("rejected {}: ${:.4} > ${:.4}", e.model, e.projected_usd, e.cap_usd);
    }
    Err(CheckError::UnknownModel(e)) => panic!("missing model: {}", e.model),
    // `CheckError` is `#[non_exhaustive]`, so always include a catch-all.
    Err(e) => println!("rejected: {e}"),
}
```

`check()` returns the `EstimatedCost` breakdown on success so you can
log or budget against the exact number that gated the call.

## Wrap a call

`run()` does the check first and only invokes your closure if the
estimate passes.

```rust
use llm_cost_cap::CostCap;

let cap = CostCap::new(0.50);

let result = cap.run("claude-opus-4-7", 10_000, 4_000, || {
    // send the actual request here
    "response"
})?;
# Ok::<(), llm_cost_cap::CheckError>(())
```

## Custom prices

```rust
use llm_cost_cap::{CostCap, ModelPrice};
use std::collections::HashMap;

let mut prices = HashMap::new();
prices.insert("my-private-model".to_string(), ModelPrice::new(2.0, 8.0));
let cap = CostCap::with_prices(prices, 1.0);
```

Or register one model into an existing cap:

```rust
# use llm_cost_cap::{CostCap, ModelPrice};
let mut cap = CostCap::new(1.0);
cap.add_model("homemade", ModelPrice::new(0.5, 1.5));
```

## Errors

`check()` (and `run()`) return `Result<_, CheckError>`. `CheckError` is
`#[non_exhaustive]`, so always include a catch-all `_` arm when matching:

| Variant                    | When                                                              |
| -------------------------- | ---------------------------------------------------------------- |
| `CheckError::CapExceeded`  | The projected cost is above the configured cap.                  |
| `CheckError::UnknownModel` | The model id is not in the price table.                          |
| `CheckError::NonFiniteCost`| The estimate is NaN or ∞ (e.g. a custom price with a bad rate).  |

The `NonFiniteCost` case exists because `NaN > cap` is always `false` in
IEEE-754: a naive comparison would silently admit a call whose cost is
undefined. The gate rejects it instead, so a malformed custom price
table can never disable the cap. `estimate()` performs no gating and
returns `EstimateError` only for an unknown model. Every error type
implements `std::error::Error`.

## Features

- `serde` — derive `Serialize` / `Deserialize` on `ModelPrice` and
  `EstimatedCost`.

## Siblings

- [`claude-cost`](https://crates.io/crates/claude-cost) — cache-aware
  cost calculator for Anthropic.
- [`bedrock-cost`](https://crates.io/crates/bedrock-cost) — cross-vendor
  Bedrock pricing.
- [`token-budget-pool`](https://crates.io/crates/token-budget-pool) —
  concurrent token / USD budget.

## License

MIT
