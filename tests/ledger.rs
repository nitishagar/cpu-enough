//! Ledger contract tests: append/readback, no-overwrite, UNDER-FILLED
//! audit, corrupt-line loudness. Each test uses a unique temp file.

use cpu_enough::fingerprint::Fingerprint;
use cpu_enough::ledger::{AuditReport, CellId, CellStatus, Ledger, LedgerEntry};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_ledger_path(test: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut p = std::env::temp_dir();
    p.push(format!(
        "cpu-enough-ledger-test-{}-{}-{n}.jsonl",
        std::process::id(),
        test
    ));
    let _ = std::fs::remove_file(&p);
    p
}

fn test_fingerprint() -> Fingerprint {
    Fingerprint {
        arch: "x86_64".to_string(),
        cpu_model: "test cpu".to_string(),
        cpu_model_source: "test".to_string(),
        cpu_flags: vec!["fpu".to_string()],
        cpu_mhz: Some("2200.000".to_string()),
        kernel: "test".to_string(),
        image_version: Some("test-image".to_string()),
        vm_size: Some("test-vm".to_string()),
        vcpus: 4,
    }
}

fn entry(cell: &str, attempt: u32, planned: u32, completed: u32) -> LedgerEntry {
    LedgerEntry {
        cell: CellId {
            stage: "s1".to_string(),
            matrix: "probe".to_string(),
            cell: cell.to_string(),
            attempt,
        },
        planned_runs: planned,
        completed_runs: completed,
        status: if completed >= planned {
            CellStatus::Completed
        } else {
            CellStatus::UnderFilled
        },
        fingerprint: test_fingerprint(),
        sample_refs: vec![],
        recorded_unix_secs: 1_700_000_000,
        note: String::new(),
    }
}

#[test]
fn append_then_readback_roundtrip() {
    let path = temp_ledger_path("roundtrip");
    let ledger = Ledger::open(&path);
    ledger.append(&entry("c1", 1, 5, 5)).expect("appends");
    let entries = ledger.read_all().expect("reads");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].cell.attempt, 1);
    assert_eq!(entries[0].completed_runs, 5);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn same_attempt_twice_is_refused() {
    let path = temp_ledger_path("overwrite");
    let ledger = Ledger::open(&path);
    ledger.append(&entry("c1", 1, 5, 5)).expect("first appends");
    let err = ledger.append(&entry("c1", 1, 5, 5)).expect_err("refused");
    assert!(matches!(err, cpu_enough::ledger::LedgerError::OverwriteAttempt { .. }));
    // A HIGHER attempt on the same cell is the legal retry path.
    ledger.append(&entry("c1", 2, 5, 5)).expect("retry appends");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn audit_reports_underfilled_and_counts_max_attempt() {
    let path = temp_ledger_path("audit");
    let ledger = Ledger::open(&path);
    ledger.append(&entry("full", 1, 5, 5)).expect("appends");
    ledger.append(&entry("short", 1, 5, 2)).expect("appends");
    ledger.append(&entry("short", 2, 5, 5)).expect("retry appends");
    let AuditReport {
        cells,
        under_filled,
        gaps,
    } = ledger.audit().expect("audits");
    assert_eq!(cells, 2);
    assert!(under_filled.is_empty(), "retry completed: {under_filled:?}");
    assert!(gaps.is_empty(), "attempts contiguous: {gaps:?}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn retry_keeps_original_planned_on_old_row() {
    // Floors shrink the future: a retry with reduced planned must not
    // rewrite the original row's planned count.
    let path = temp_ledger_path("original-planned");
    let ledger = Ledger::open(&path);
    ledger.append(&entry("c1", 1, 5, 2)).expect("appends");
    ledger.append(&entry("c1", 2, 3, 3)).expect("retry appends");
    let entries = ledger.read_all().expect("reads");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].planned_runs, 5);
    assert_eq!(entries[1].planned_runs, 3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn attempt_gap_is_reported_not_hidden() {
    let path = temp_ledger_path("gap");
    let ledger = Ledger::open(&path);
    ledger.append(&entry("c1", 1, 5, 5)).expect("appends");
    ledger.append(&entry("c1", 3, 5, 5)).expect("appends");
    let report = ledger.audit().expect("audits");
    assert_eq!(report.gaps.len(), 1);
    assert!(report.gaps[0].contains("missing attempt 2"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn dispatch_counter_caps_and_resets_by_day() {
    use cpu_enough::ledger::{today_day_key, DispatchCounter};
    let mut path = std::env::temp_dir();
    path.push(format!("cpu-enough-counter-test-{}-cap.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let counter = DispatchCounter::open(&path);
    let today = today_day_key();
    assert_eq!(counter.check_and_increment(today, 2).expect("1st"), 1);
    assert_eq!(counter.check_and_increment(today, 2).expect("2nd"), 2);
    let err = counter.check_and_increment(today, 2).expect_err("capped");
    assert!(matches!(
        err,
        cpu_enough::ledger::DispatchError::DayCapReached { count: 2, cap: 2 }
    ));
    // A new UTC day restarts the count.
    assert_eq!(counter.check_and_increment(today + 1, 2).expect("new day"), 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn dispatch_counter_corrupt_blocks_loud() {
    use cpu_enough::ledger::DispatchCounter;
    let mut path = std::env::temp_dir();
    path.push(format!(
        "cpu-enough-counter-test-{}-corrupt.json",
        std::process::id()
    ));
    std::fs::write(&path, "{ broken").expect("writes");
    let counter = DispatchCounter::open(&path);
    let err = counter
        .check_and_increment(1, 12)
        .expect_err("corrupt blocks");
    assert!(matches!(err, cpu_enough::ledger::DispatchError::Corrupt { .. }));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn corrupt_line_is_loud() {
    let path = temp_ledger_path("corrupt");
    std::fs::write(&path, "{ not json }\n").expect("writes");
    let ledger = Ledger::open(&path);
    let err = ledger.read_all().expect_err("loud");
    assert!(matches!(err, cpu_enough::ledger::LedgerError::Corrupt { .. }));
    let _ = std::fs::remove_file(&path);
}
