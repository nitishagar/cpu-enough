//! Append-only JSONL evidence ledger (spec: [Retry & partial failure],
//! [Evidence & compliance discipline]).
//!
//! Identity: (cell_id, attempt). Audit counts max attempt per cell_id, so
//! kept duplicates can never double-count. Original-planned is retained on
//! every entry; floors shrink the future, never rewrite the past.

use crate::fingerprint::Fingerprint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CellId {
    pub stage: String,
    pub matrix: String,
    pub cell: String,
    pub attempt: u32,
}

impl fmt::Display for CellId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}/{}", self.stage, self.matrix, self.cell, self.attempt)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CellStatus {
    Completed,
    UnderFilled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub cell: CellId,
    pub planned_runs: u32,
    pub completed_runs: u32,
    pub status: CellStatus,
    pub fingerprint: Fingerprint,
    pub sample_refs: Vec<String>,
    pub recorded_unix_secs: u64,
    pub note: String,
}

#[derive(Debug)]
pub enum LedgerError {
    Io(std::io::Error),
    OverwriteAttempt { cell: CellId },
    Corrupt { line: usize, detail: String },
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "ledger I/O: {e}"),
            Self::OverwriteAttempt { cell } => {
                write!(f, "refusing to overwrite {cell}: append a new attempt instead")
            }
            Self::Corrupt { line, detail } => {
                write!(f, "corrupt ledger line {line}: {detail}")
            }
        }
    }
}

impl std::error::Error for LedgerError {}

impl From<std::io::Error> for LedgerError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub struct Ledger {
    path: PathBuf,
}

impl Ledger {
    pub fn open(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    /// Read all entries. A corrupt line is a loud error, never skipped:
    /// silent drops would fake the evidence.
    pub fn read_all(&self) -> Result<Vec<LedgerEntry>, LedgerError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let text = std::fs::read_to_string(&self.path)?;
        let mut out = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: LedgerEntry =
                serde_json::from_str(line).map_err(|e| LedgerError::Corrupt {
                    line: i + 1,
                    detail: e.to_string(),
                })?;
            out.push(entry);
        }
        Ok(out)
    }

    /// Append one entry. Refuses any attempt number that is not greater
    /// than the recorded max for that cell (no overwrite, no backfill).
    pub fn append(&self, entry: &LedgerEntry) -> Result<(), LedgerError> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        for existing in self.read_all()? {
            if existing.cell.stage == entry.cell.stage
                && existing.cell.matrix == entry.cell.matrix
                && existing.cell.cell == entry.cell.cell
                && existing.cell.attempt >= entry.cell.attempt
            {
                return Err(LedgerError::OverwriteAttempt {
                    cell: entry.cell.clone(),
                });
            }
        }
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        writeln!(file, "{}", serde_json::to_string(entry).expect("entry serializes"))?;
        file.sync_all()?;
        Ok(())
    }

    /// Audit: group by cell (stage/matrix/cell), keep max attempt each.
    pub fn audit(&self) -> Result<AuditReport, LedgerError> {
        let mut latest: HashMap<(String, String, String), &LedgerEntry> = HashMap::new();
        let entries = self.read_all()?;
        for entry in &entries {
            let key = (
                entry.cell.stage.clone(),
                entry.cell.matrix.clone(),
                entry.cell.cell.clone(),
            );
            let is_newer = latest
                .get(&key)
                .map(|e| entry.cell.attempt > e.cell.attempt)
                .unwrap_or(true);
            if is_newer {
                latest.insert(key, entry);
            }
        }
        let mut under_filled = Vec::new();
        let mut gaps = Vec::new();
        for ((stage, matrix, cell), entry) in &latest {
            if entry.status == CellStatus::UnderFilled
                || entry.completed_runs < entry.planned_runs
            {
                under_filled.push(format!("{stage}/{matrix}/{cell}"));
            }
        }
        // Attempt-sequence gaps (e.g. attempts 1,3 with no 2) are recorded.
        let mut by_cell: HashMap<(String, String, String), Vec<u32>> = HashMap::new();
        for entry in &entries {
            by_cell
                .entry((
                    entry.cell.stage.clone(),
                    entry.cell.matrix.clone(),
                    entry.cell.cell.clone(),
                ))
                .or_default()
                .push(entry.cell.attempt);
        }
        for ((stage, matrix, cell), mut attempts) in by_cell {
            attempts.sort_unstable();
            attempts.dedup();
            let mut expected = 1u32;
            for got in attempts {
                while expected < got {
                    gaps.push(format!("{stage}/{matrix}/{cell}: missing attempt {expected}"));
                    expected += 1;
                }
                expected = expected.max(got + 1);
            }
        }
        Ok(AuditReport {
            cells: latest.len(),
            under_filled,
            gaps,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AuditReport {
    pub cells: usize,
    pub under_filled: Vec<String>,
    pub gaps: Vec<String>,
}

pub fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// UTC day key for the dispatch day-cap (whole days since epoch; no date
/// formatting needed, rollover at UTC midnight).
pub fn today_day_key() -> u64 {
    now_unix_secs() / 86400
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CounterState {
    day_key: u64,
    count: u32,
}

#[derive(Debug)]
pub enum DispatchError {
    Io(std::io::Error),
    Corrupt { detail: String },
    DayCapReached { count: u32, cap: u32 },
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "dispatch counter I/O: {e}"),
            Self::Corrupt { detail } => write!(
                f,
                "dispatch counter corrupt ({detail}): dispatch BLOCKED — recover per runbook (archive + restart + log), never auto-reset"
            ),
            Self::DayCapReached { count, cap } => write!(
                f,
                "day-cap reached: {count}/{cap} dispatched today — refusing (runbook counter rules)"
            ),
        }
    }
}

impl std::error::Error for DispatchError {}

impl From<std::io::Error> for DispatchError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Pre-dispatch day-cap counter (runbook counter rules). One JSON state
/// file; a new UTC day resets. Corrupt state BLOCKS (loud) rather than
/// resetting — a silent reset would zero the books.
pub struct DispatchCounter {
    path: PathBuf,
}

impl DispatchCounter {
    pub fn open(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    /// Check the cap and increment. Returns the new count (1-based).
    pub fn check_and_increment(&self, day_key: u64, cap: u32) -> Result<u32, DispatchError> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let state: Option<CounterState> = if self.path.exists() {
            let text = std::fs::read_to_string(&self.path)?;
            Some(serde_json::from_str(&text).map_err(|e| DispatchError::Corrupt {
                detail: e.to_string(),
            })?)
        } else {
            None
        };
        let count = match state {
            Some(s) if s.day_key == day_key => {
                if s.count >= cap {
                    return Err(DispatchError::DayCapReached { count: s.count, cap });
                }
                s.count + 1
            }
            _ => 1, // new day (or first dispatch) starts the count
        };
        let next = CounterState { day_key, count };
        std::fs::write(&self.path, serde_json::to_string(&next).expect("state serializes"))?;
        Ok(count)
    }
}
