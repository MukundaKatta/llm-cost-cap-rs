# llm-cost-cap

[![Crates.io](https://img.shields.io/crates/v/llm-cost-cap.svg)](https://crates.io/crates/llm-cost-cap)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

Pre-flight USD cost gate for a single LLM call.

Before sending a request, estimate the worst-case cost (input tokens at
the input rate plus the requested `max_output_tokens` at the output rate)
and reject if it would exceed a configured per-call cap. Catches the
failure mode where an agent burns $20 on one call before the call ever
leaves the process. Zero non-std deps. Built-in price table covers
Anthropic, OpenAI, Gemini, and Bedrock variants.

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
