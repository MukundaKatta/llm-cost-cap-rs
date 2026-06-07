/*!
llm-cost-cap: pre-flight USD cost gate for a single LLM call.

Estimate the worst-case cost of a planned LLM call before making it
(input tokens at the input rate plus the requested `max_output_tokens`
at the output rate) and reject it if it would exceed a configured
per-call cap.

Built-in price tables cover Anthropic, OpenAI, Gemini, and Bedrock
variants (prices as of 2026-05-24 — override via [`CostCap::with_prices`]
or [`CostCap::add_model`]).

```rust
use llm_cost_cap::{CostCap, CheckError};

let cap = CostCap::new(0.50); // $0.50 per-call cap

match cap.check("claude-opus-4-7", 10_000, 4_000) {
    Ok(est) => assert!(est.total_usd > 0.0),
    Err(CheckError::CapExceeded(e)) => {
        eprintln!("rejected {}: ${:.4} > ${:.4}", e.model, e.projected_usd, e.cap_usd);
    }
    Err(CheckError::UnknownModel(e)) => panic!("missing model: {}", e.model),
}
```
*/

mod cap;
mod prices;

pub use cap::{CapExceeded, CheckError, CostCap, EstimateError, EstimatedCost, UnknownModel};
pub use prices::{builtin_prices, known_models, ModelPrice, MODEL_PRICES};
