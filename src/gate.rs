//! Reproduce-first gate interface: reference median + SKU-tagged samples.
//! Single-SKU input passes within ±10% (inclusive); multi-SKU input is a
//! hard ERROR — samples are never pooled across SKUs (spec: [Platform
//! honesty]). Pure functions; tests use mock references (no measurement).

use std::fmt;

#[derive(Debug, Clone)]
pub struct TaggedSample {
    pub sku: String,
    pub value: f64,
}

#[derive(Debug, PartialEq)]
pub enum GateVerdict {
    Pass { deviation_pct: f64 },
    Fail { deviation_pct: f64 },
}

/// ±10% tolerance, boundary inclusive.
pub const TOLERANCE_PCT: f64 = 10.0;

#[derive(Debug, PartialEq)]
pub enum GateError {
    EmptySamples,
    InvalidReference,
    NonFiniteSample { index: usize },
    MultiSku { skus: Vec<String> },
}

impl fmt::Display for GateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySamples => write!(f, "no samples"),
            Self::InvalidReference => write!(f, "reference must be finite and > 0"),
            Self::NonFiniteSample { index } => {
                write!(f, "sample {index} is non-finite")
            }
            Self::MultiSku { skus } => {
                write!(f, "multi-SKU input refused (never pooled): {}", skus.join(","))
            }
        }
    }
}

impl std::error::Error for GateError {}

/// Median of finite values. Returns None on empty input.
pub fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).expect("finite inputs"));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[mid])
    } else {
        Some((values[mid - 1] + values[mid]) / 2.0)
    }
}

/// Evaluate sample median against the reference median.
/// deviation_pct = (median − reference) / reference × 100.
pub fn evaluate(
    reference_median: f64,
    samples: &[TaggedSample],
) -> Result<GateVerdict, GateError> {
    if !reference_median.is_finite() || reference_median <= 0.0 {
        return Err(GateError::InvalidReference);
    }
    if samples.is_empty() {
        return Err(GateError::EmptySamples);
    }
    for (i, s) in samples.iter().enumerate() {
        if !s.value.is_finite() {
            return Err(GateError::NonFiniteSample { index: i });
        }
    }
    let mut skus: Vec<String> = samples.iter().map(|s| s.sku.clone()).collect();
    skus.sort();
    skus.dedup();
    if skus.len() > 1 {
        return Err(GateError::MultiSku { skus });
    }
    let values: Vec<f64> = samples.iter().map(|s| s.value).collect();
    let med = median(values).expect("non-empty");
    let deviation_pct = (med - reference_median) / reference_median * 100.0;
    if deviation_pct.abs() <= TOLERANCE_PCT {
        Ok(GateVerdict::Pass { deviation_pct })
    } else {
        Ok(GateVerdict::Fail { deviation_pct })
    }
}
