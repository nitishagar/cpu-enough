//! Fingerprint contract tests: x86 + ARM parser fixtures, gap listing,
//! and one live structural check (stable properties only — never
//! environment-dependent assertions).

use cpu_enough::fingerprint::{parse_lscpu_json, resolve_cpu_model, Fingerprint};
use std::collections::HashMap;

const X86_LSCPU: &str = r#"{"lscpu": [
  {"field":"Architecture:","data":"x86_64"},
  {"field":"CPU(s):","data":"4"},
  {"field":"Model name:","data":"AMD EPYC 7763 64-Core Processor"},
  {"field":"CPU MHz:","data":"2600.000"},
  {"field":"Flags:","data":"fpu vme de pse"}
]}"#;

const ARM_LSCPU_NO_MODEL_NAME: &str = r#"{"lscpu": [
  {"field":"Architecture:","data":"aarch64"},
  {"field":"CPU(s):","data":"4"},
  {"field":"Model:","data":"1"},
  {"field":"BIOS CPU family:","data":"8"},
  {"field":"Stepping:","data":"r0p1"}
]}"#;

#[test]
fn x86_model_name_wins_with_source_recorded() {
    let map = parse_lscpu_json(X86_LSCPU).expect("parses");
    assert_eq!(map.get("Architecture").map(String::as_str), Some("x86_64"));
    let (model, source) = resolve_cpu_model(&map).expect("resolves");
    assert!(model.contains("EPYC 7763"), "model: {model}");
    assert_eq!(source, "lscpu:Model name");
}

#[test]
fn arm_without_model_name_yields_recorded_composite() {
    let map = parse_lscpu_json(ARM_LSCPU_NO_MODEL_NAME).expect("parses");
    let (model, source) = resolve_cpu_model(&map).expect("composite resolves");
    assert!(model.contains("family:8"), "model: {model}");
    assert!(model.contains("model:1"), "model: {model}");
    assert_eq!(source, "lscpu:BIOS-composite");
}

#[test]
fn empty_map_yields_no_model() {
    let map = HashMap::new();
    assert!(resolve_cpu_model(&map).is_none());
}

#[test]
fn malformed_lscpu_is_loud_not_empty() {
    assert!(parse_lscpu_json("not json").is_err());
    assert!(parse_lscpu_json(r#"{"nope": []}"#).is_err());
}

fn gapped_fingerprint() -> Fingerprint {
    Fingerprint {
        arch: "x86_64".to_string(),
        cpu_model: "test cpu".to_string(),
        cpu_model_source: "test".to_string(),
        cpu_flags: vec![],
        cpu_mhz: None,
        kernel: "test".to_string(),
        image_version: None,
        vm_size: None,
        vcpus: 4,
    }
}

#[test]
fn missing_sources_lists_exactly_the_gaps() {
    let mut fp = gapped_fingerprint();
    assert_eq!(fp.missing_sources(), vec!["image_version", "vm_size"]);
    assert!(!fp.is_complete());
    fp.image_version = Some("test-image".to_string());
    fp.vm_size = Some("test-vm".to_string());
    assert!(fp.missing_sources().is_empty());
    assert!(fp.is_complete());
}

#[test]
fn live_collect_has_stable_structure() {
    // Stable on every machine (dev laptop, x86 runner, ARM runner):
    // collect() succeeds with non-empty arch/model/kernel and vcpus > 0.
    // Soft-source presence is environment-dependent and NOT asserted here.
    let fp = cpu_enough::fingerprint::collect().expect("collects on any machine");
    assert!(!fp.arch.is_empty());
    assert!(!fp.cpu_model.is_empty());
    assert!(!fp.kernel.is_empty());
    assert!(fp.vcpus > 0);
}
