//! Core [`CostCap`] implementation.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::prices::{builtin_prices, ModelPrice};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A breakdown of the per-call estimate for one model. All four fields
/// are USD; `total_usd == input_usd + output_usd + cached_input_usd`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EstimatedCost {
    pub total_usd: f64,
    pub input_usd: f64,
    pub output_usd: f64,
    pub cached_input_usd: f64,
}

/// Returned when the estimated cost of a single call exceeds the cap.
#[derive(Debug, Clone, PartialEq)]
pub struct CapExceeded {
    pub projected_usd: f64,
    pub cap_usd: f64,
    pub model: String,
}

impl fmt::Display for CapExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "estimated cost ${:.6} for model {:?} exceeds cap ${:.6}",
            self.projected_usd, self.model, self.cap_usd
        )
    }
}

impl Error for CapExceeded {}

/// Returned when a model id is not in the price table.
#[derive(Debug, Clone, PartialEq)]
pub struct UnknownModel {
    pub model: String,
}

impl fmt::Display for UnknownModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown model {:?}: pass a custom price table or call add_model",
            self.model
        )
    }
}

impl Error for UnknownModel {}

/// Errors returned by [`CostCap::estimate`] when the model is unknown
/// or token counts are negative. (Negative counts are not representable
/// with `u64`, but the model lookup can still fail.)
#[derive(Debug, Clone, PartialEq)]
pub enum EstimateError {
    UnknownModel(UnknownModel),
}

impl fmt::Display for EstimateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EstimateError::UnknownModel(e) => e.fmt(f),
        }
    }
}

impl Error for EstimateError {}

impl From<UnknownModel> for EstimateError {
    fn from(e: UnknownModel) -> Self {
        EstimateError::UnknownModel(e)
    }
}

/// Errors returned by [`CostCap::check`].
#[derive(Debug, Clone, PartialEq)]
pub enum CheckError {
    UnknownModel(UnknownModel),
    CapExceeded(CapExceeded),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::UnknownModel(e) => e.fmt(f),
            CheckError::CapExceeded(e) => e.fmt(f),
        }
    }
}

impl Error for CheckError {}

impl From<UnknownModel> for CheckError {
    fn from(e: UnknownModel) -> Self {
        CheckError::UnknownModel(e)
    }
}

impl From<CapExceeded> for CheckError {
    fn from(e: CapExceeded) -> Self {
        CheckError::CapExceeded(e)
    }
}

impl From<EstimateError> for CheckError {
    fn from(e: EstimateError) -> Self {
        match e {
            EstimateError::UnknownModel(u) => CheckError::UnknownModel(u),
        }
    }
}

/// Pre-flight cost gate for a single LLM call.
///
/// Build once with the per-call USD cap, then call [`CostCap::check`]
/// before sending each request. If the estimated cost for the requested
/// model and token counts exceeds the cap, an error is returned and the
/// caller can short circuit before paying for the call.
///
/// Cost model:
///   * `input_tokens` are billed at the model's input rate.
///   * `max_output_tokens` are billed at the model's output rate (worst
///     case ceiling, because the cap protects against the worst case).
///   * `cached_input_tokens` are billed at the model's cached rate if
///     one is published, otherwise treated as zero.
#[derive(Debug, Clone)]
pub struct CostCap {
    cap_usd: f64,
    prices: HashMap<String, ModelPrice>,
}

impl CostCap {
    /// Build a cap with the built-in price table.
    pub fn new(max_usd: f64) -> Self {
        assert!(max_usd >= 0.0, "max_usd must be >= 0");
        Self {
            cap_usd: max_usd,
            prices: builtin_prices(),
        }
    }

    /// Build a cap with a caller-supplied price table. The built-in
    /// table is not used; only entries in `prices` are recognized.
    pub fn with_prices(prices: HashMap<String, ModelPrice>, max_usd: f64) -> Self {
        assert!(max_usd >= 0.0, "max_usd must be >= 0");
        Self {
            cap_usd: max_usd,
            prices,
        }
    }

    pub fn cap_usd(&self) -> f64 {
        self.cap_usd
    }

    /// Register or replace one model in the price table.
    pub fn add_model<S: Into<String>>(&mut self, model: S, price: ModelPrice) {
        self.prices.insert(model.into(), price);
    }

    /// Sorted list of model ids registered with this cap.
    pub fn known_models(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.prices.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Return the per-call cost breakdown. Does not raise on overage.
    pub fn estimate(
        &self,
        model: &str,
        input_tokens: u64,
        max_output_tokens: u64,
    ) -> Result<EstimatedCost, EstimateError> {
        self.estimate_with_cached(model, input_tokens, max_output_tokens, 0)
    }

    /// Like [`CostCap::estimate`] but includes a separate cached input
    /// token count. Cached tokens are billed at the model's cached rate
    /// if one is published; otherwise they are zero in the estimate.
    pub fn estimate_with_cached(
        &self,
        model: &str,
        input_tokens: u64,
        max_output_tokens: u64,
        cached_input_tokens: u64,
    ) -> Result<EstimatedCost, EstimateError> {
        let price = self.prices.get(model).ok_or_else(|| UnknownModel {
            model: model.to_string(),
        })?;
        let input_usd = (input_tokens as f64 / 1_000_000.0) * price.input_per_million_usd;
        let output_usd = (max_output_tokens as f64 / 1_000_000.0) * price.output_per_million_usd;
        let cached_input_usd = match price.cached_input_per_million_usd {
            Some(rate) if cached_input_tokens > 0 => {
                (cached_input_tokens as f64 / 1_000_000.0) * rate
            }
            _ => 0.0,
        };
        Ok(EstimatedCost {
            total_usd: input_usd + output_usd + cached_input_usd,
            input_usd,
            output_usd,
            cached_input_usd,
        })
    }

    /// Estimate the call. Returns `Ok(cost)` if under cap, `Err` with
    /// either an [`UnknownModel`] or a [`CapExceeded`] otherwise.
    pub fn check(
        &self,
        model: &str,
        input_tokens: u64,
        max_output_tokens: u64,
    ) -> Result<EstimatedCost, CheckError> {
        self.check_with_cached(model, input_tokens, max_output_tokens, 0)
    }

    /// Like [`CostCap::check`] but with a separate cached input token
    /// count.
    pub fn check_with_cached(
        &self,
        model: &str,
        input_tokens: u64,
        max_output_tokens: u64,
        cached_input_tokens: u64,
    ) -> Result<EstimatedCost, CheckError> {
        let est = self.estimate_with_cached(
            model,
            input_tokens,
            max_output_tokens,
            cached_input_tokens,
        )?;
        if est.total_usd > self.cap_usd {
            return Err(CheckError::CapExceeded(CapExceeded {
                projected_usd: est.total_usd,
                cap_usd: self.cap_usd,
                model: model.to_string(),
            }));
        }
        Ok(est)
    }

    /// Gate then invoke. Returns whatever `f` returns. `f` is only
    /// called if [`CostCap::check`] passes.
    pub fn run<T>(
        &self,
        model: &str,
        input_tokens: u64,
        max_output_tokens: u64,
        f: impl FnOnce() -> T,
    ) -> Result<T, CheckError> {
        self.check(model, input_tokens, max_output_tokens)?;
        Ok(f())
    }
}
