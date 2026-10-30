//! Diagnostics contract tests: 4 loud failures (fail + pass each) + live
//! disk preflight + scaffold shape checks.

use cpu_enough::diagnostics::{
    check_clock_skew, check_gguf_bytes, check_trace_bytes, format_pairing_stub, parse_df_avail,
    preflight_disk_space, trace_schema_fixture, DiagnosticError,
};

#[test]
fn valid_trace_returns_line_count() {
    let trace = "{\"a\":1}\n{\"b\":[1,2]}\n";
    assert_eq!(check_trace_bytes(trace.as_bytes()), Ok(2));
}

#[test]
fn empty_trace_is_loud() {
    assert_eq!(check_trace_bytes(b""), Err(DiagnosticError::EmptyTrace));
    assert_eq!(check_trace_bytes(b"\n  \n"), Err(DiagnosticError::EmptyTrace));
}

#[test]
fn malformed_trace_line_is_loud_with_line_number() {
    let trace = "{\"a\":1}\nnot json\n";
    assert!(matches!(
        check_trace_bytes(trace.as_bytes()),
        Err(DiagnosticError::MalformedTrace { line: 2, .. })
    ));
}

#[test]
fn gguf_magic_and_size_ok() {
    let mut file = b"GGUF".to_vec();
    file.extend_from_slice(&[0u8; 100]);
    assert_eq!(check_gguf_bytes(&file), Ok(104));
}

#[test]
fn gguf_bad_magic_is_loud() {
    assert!(matches!(
        check_gguf_bytes(b"NOPE rest of file"),
        Err(DiagnosticError::CorruptGguf { .. })
    ));
    assert!(matches!(
        check_gguf_bytes(b"GG"),
        Err(DiagnosticError::CorruptGguf { .. })
    ));
}

#[test]
fn df_output_parses() {
    assert_eq!(parse_df_avail("Avail\n123456789\n"), Ok(123456789));
    assert!(parse_df_avail("Avail\n").is_err());
    assert!(parse_df_avail("Avail\nabc\n").is_err());
}

#[test]
fn live_disk_preflight_passes_small_and_refuses_absurd() {
    let tmp = std::env::temp_dir();
    let avail = preflight_disk_space(&tmp, 1).expect("1 byte fits anywhere");
    assert!(avail > 0);
    assert!(matches!(
        preflight_disk_space(&tmp, u64::MAX),
        Err(DiagnosticError::DiskFull { .. })
    ));
}

#[test]
fn clock_agreement_passes_and_skew_is_loud() {
    assert!(check_clock_skew(1000, 1010, 10, 2).is_ok());
    assert!(matches!(
        check_clock_skew(1000, 1010, 60, 2),
        Err(DiagnosticError::ClockSkew { .. })
    ));
    assert!(matches!(
        check_clock_skew(1010, 1000, 0, 2),
        Err(DiagnosticError::ClockSkew { .. })
    ));
}

#[test]
fn scaffolds_have_documented_shapes() {
    let fx = trace_schema_fixture();
    assert_eq!(fx.get("scaffold"), Some(&serde_json::Value::Bool(true)));
    assert_eq!(format_pairing_stub("a", "b"), "a|b");
}
