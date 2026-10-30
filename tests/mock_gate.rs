//! Gate mock-contract tests: mock reference + mock samples exercise the
//! ±10% single-SKU rule and the multi-SKU hard error (no measurement).

use cpu_enough::gate::{evaluate, median, GateError, GateVerdict, TaggedSample};

fn samples(sku: &str, values: &[f64]) -> Vec<TaggedSample> {
    values
        .iter()
        .map(|v| TaggedSample {
            sku: sku.to_string(),
            value: *v,
        })
        .collect()
}

#[test]
fn within_tolerance_passes() {
    let s = samples("sku-a", &[104.0, 105.0, 106.0]);
    assert!(matches!(
        evaluate(100.0, &s),
        Ok(GateVerdict::Pass { .. })
    ));
}

#[test]
fn outside_tolerance_fails_with_deviation() {
    let s = samples("sku-a", &[119.0, 120.0, 121.0]);
    match evaluate(100.0, &s) {
        Ok(GateVerdict::Fail { deviation_pct }) => {
            assert!((deviation_pct - 20.0).abs() < 1e-9, "{deviation_pct}");
        }
        other => panic!("expected Fail, got {other:?}"),
    }
}

#[test]
fn boundary_is_inclusive() {
    let s = samples("sku-a", &[110.0]);
    assert!(matches!(evaluate(100.0, &s), Ok(GateVerdict::Pass { .. })));
    let s = samples("sku-a", &[90.0]);
    assert!(matches!(evaluate(100.0, &s), Ok(GateVerdict::Pass { .. })));
}

#[test]
fn multi_sku_is_hard_error() {
    let mut s = samples("sku-a", &[100.0]);
    s.extend(samples("sku-b", &[100.0]));
    match evaluate(100.0, &s) {
        Err(GateError::MultiSku { skus }) => {
            assert_eq!(skus, vec!["sku-a".to_string(), "sku-b".to_string()]);
        }
        other => panic!("expected MultiSku, got {other:?}"),
    }
}

#[test]
fn empty_samples_and_bad_reference_are_loud() {
    assert_eq!(evaluate(100.0, &[]), Err(GateError::EmptySamples));
    let s = samples("sku-a", &[100.0]);
    assert_eq!(evaluate(0.0, &s), Err(GateError::InvalidReference));
    assert_eq!(evaluate(f64::NAN, &s), Err(GateError::InvalidReference));
}

#[test]
fn non_finite_sample_is_loud() {
    let s = samples("sku-a", &[100.0, f64::INFINITY]);
    assert_eq!(
        evaluate(100.0, &s),
        Err(GateError::NonFiniteSample { index: 1 })
    );
}

#[test]
fn median_handles_odd_even_empty() {
    assert_eq!(median(vec![3.0, 1.0, 2.0]), Some(2.0));
    assert_eq!(median(vec![4.0, 1.0, 2.0, 3.0]), Some(2.5));
    assert_eq!(median(vec![]), None);
}
