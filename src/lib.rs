//! Pre-flight USD cost gate for a single LLM call.
//!
//! Before sending a request, estimate the worst-case cost (input tokens
//! at the input rate plus the requested `max_output_tokens` at the
//! output rate) and reject if it would exceed a configured per-call
//! cap. Catches the failure mode where an agent runs away and burns $20
//! on a single call before the call ever leaves the process.
//!
//! # Example
//!
//! ```
//! use llm_cost_cap::{CostCap, CheckError};
//!
//! let cap = CostCap::new(0.50);
//! match cap.check("claude-opus-4-7", 10_000, 4_000) {
//!     Ok(est) => println!("ok, projected ${:.4}", est.total_usd),
//!     Err(CheckError::CapExceeded(e)) => {
//!         println!("rejected {}: ${:.4} > ${:.4}", e.model, e.projected_usd, e.cap_usd);
//!     }
//!     Err(CheckError::UnknownModel(e)) => panic!("missing model: {}", e.model),
//! }
//! ```
//!
//! # Wrap a call
//!
//! ```
//! use llm_cost_cap::CostCap;
//!
//! let cap = CostCap::new(10.0);
//! let result = cap.run("claude-sonnet-4-6", 500, 500, || "fake-response").unwrap();
//! assert_eq!(result, "fake-response");
//! ```

mod cap;
mod prices;

pub use cap::{
    CapExceeded, CheckError, CostCap, EstimateError, EstimatedCost, UnknownModel,
};
pub use prices::{builtin_prices, known_models, ModelPrice, MODEL_PRICES};
