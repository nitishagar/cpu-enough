//! Preflight + evidence diagnostics: 4 loud failures. Anything this module
//! cannot verify FAILS — it never passes silently (missing tools error,
//! corrupt inputs error). Scaffolds (trace fixtures, pairing format) are
//! placeholders documented in `docs/scaffold-contracts.md`, owned for
//! completion by S3.

use std::fmt;
use std::path::Path;
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
pub enum DiagnosticError {
    EmptyTrace,
    MalformedTrace { line: usize, detail: String },
    CorruptGguf { detail: String },
    DiskFull {
        path: String,
        available_bytes: u64,
        required_bytes: u64,
    },
    ClockSkew { detail: String },
    ToolMissing { tool: &'static str, detail: String },
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTrace => write!(f, "trace is empty"),
            Self::MalformedTrace { line, detail } => {
                write!(f, "trace line {line} is not valid JSON: {detail}")
            }
            Self::CorruptGguf { detail } => write!(f, "corrupt GGUF: {detail}"),
            Self::DiskFull {
                path,
                available_bytes,
                required_bytes,
            } => write!(
                f,
                "disk full: {path} has {available_bytes} bytes, need {required_bytes}"
            ),
            Self::ClockSkew { detail } => write!(f, "clock skew: {detail}"),
            Self::ToolMissing { tool, detail } => {
                write!(f, "required tool missing: {tool}: {detail}")
            }
        }
    }
}

impl std::error::Error for DiagnosticError {}

/// Trace structural check: non-empty + every non-blank line valid JSON.
/// Schema-agnostic on purpose — S2 records the real schema
/// (`llamabench-schema.json`); this only rejects empty/garbage.
pub fn check_trace_bytes(bytes: &[u8]) -> Result<usize, DiagnosticError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| DiagnosticError::MalformedTrace { line: 1, detail: e.to_string() })?;
    let mut lines = 0;
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        serde_json::from_str::<serde_json::Value>(line).map_err(|e| {
            DiagnosticError::MalformedTrace {
                line: i + 1,
                detail: e.to_string(),
            }
        })?;
        lines += 1;
    }
    if lines == 0 {
        return Err(DiagnosticError::EmptyTrace);
    }
    Ok(lines)
}

/// GGUF integrity check: magic bytes `GGUF` + non-trivial size.
pub fn check_gguf_bytes(bytes: &[u8]) -> Result<u64, DiagnosticError> {
    if bytes.len() < 4 {
        return Err(DiagnosticError::CorruptGguf {
            detail: format!("file too small: {} bytes", bytes.len()),
        });
    }
    if &bytes[0..4] != b"GGUF" {
        return Err(DiagnosticError::CorruptGguf {
            detail: "bad magic (expected GGUF)".to_string(),
        });
    }
    Ok(bytes.len() as u64)
}

/// Parse `df -B1 --output=avail <path>` output (header + one byte count).
pub fn parse_df_avail(text: &str) -> Result<u64, DiagnosticError> {
    let mut lines = text.lines();
    lines.next(); // header
    let value = lines.next().ok_or_else(|| DiagnosticError::ToolMissing {
        tool: "df",
        detail: "no output line".to_string(),
    })?;
    value.trim().parse::<u64>().map_err(|e| DiagnosticError::ToolMissing {
        tool: "df",
        detail: format!("unparsable avail: {e}"),
    })
}

/// Disk preflight: refuses when available < required. A missing/unparseable
/// `df` is a loud error (never a silent pass). GNU df (linux runners + dev).
pub fn preflight_disk_space(path: &Path, required_bytes: u64) -> Result<u64, DiagnosticError> {
    let out = Command::new("df")
        .args(["-B1", "--output=avail"])
        .arg(path)
        .output()
        .map_err(|e| DiagnosticError::ToolMissing {
            tool: "df",
            detail: e.to_string(),
        })?;
    if !out.status.success() {
        return Err(DiagnosticError::ToolMissing {
            tool: "df",
            detail: String::from_utf8_lossy(&out.stderr).into_owned(),
        });
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let available = parse_df_avail(&text)?;
    if available < required_bytes {
        return Err(DiagnosticError::DiskFull {
            path: path.display().to_string(),
            available_bytes: available,
            required_bytes,
        });
    }
    Ok(available)
}

/// Clock-skew check: wall-clock delta vs monotonic elapsed must agree
/// within tolerance, and wall time must not run backward.
pub fn check_clock_skew(
    wall_start_secs: u64,
    wall_end_secs: u64,
    monotonic_elapsed_secs: u64,
    tolerance_secs: u64,
) -> Result<(), DiagnosticError> {
    if wall_end_secs < wall_start_secs {
        return Err(DiagnosticError::ClockSkew {
            detail: "wall clock ran backward".to_string(),
        });
    }
    let wall_delta = wall_end_secs - wall_start_secs;
    let drift = wall_delta.abs_diff(monotonic_elapsed_secs);
    if drift > tolerance_secs {
        return Err(DiagnosticError::ClockSkew {
            detail: format!("wall delta {wall_delta}s vs monotonic {monotonic_elapsed_secs}s"),
        });
    }
    Ok(())
}

/// SCAFFOLD (contract: `docs/scaffold-contracts.md` § trace fixtures).
/// Shape placeholder only — S2 replaces it with schema-validated fixtures.
pub fn trace_schema_fixture() -> serde_json::Value {
    serde_json::json!({
        "scaffold": true,
        "contract": "docs/scaffold-contracts.md",
        "trace": { "prompt_tokens": 0, "gen_tokens": 0, "samples": [] }
    })
}

/// SCAFFOLD (contract: `docs/scaffold-contracts.md` § pairing format).
/// `left|right` placeholder — S3 pins the real format.
pub fn format_pairing_stub(left: &str, right: &str) -> String {
    format!("{left}|{right}")
}
