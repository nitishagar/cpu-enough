// cpu-enough: harness CLI. Subcommands grow per plan phase; 1.2 owns
// `fingerprint` (UNION capture for probe workflows) and `probe-record`
// (append-only ledger writer with a strict completeness gate).

use clap::{Parser, Subcommand, ValueEnum};
use cpu_enough::{fingerprint, ledger};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "cpu-enough", version, about = "Free-tier CPU LLM-inference benchmark harness")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Print the UNION machine fingerprint as JSON (all sources attempted).
    Fingerprint,
    /// Pre-dispatch day-cap check: increments today's counter, refuses
    /// over cap. Counter path overridable via PROBE_DISPATCH_COUNTER (drill
    /// hook; env wins over --counter).
    DispatchCheck {
        #[arg(long)]
        matrix: String,
        #[arg(long)]
        cell: String,
        #[arg(long, default_value_t = 12)]
        cap: u32,
        #[arg(long, default_value = "ledger/dispatch-counter.json")]
        counter: PathBuf,
    },
    /// Append one probe cell to the ledger (refuses overwrite; --strict
    /// refuses gapped fingerprints).
    ProbeRecord {
        #[arg(long)]
        matrix: String,
        #[arg(long)]
        cell: String,
        #[arg(long, default_value_t = 1)]
        attempt: u32,
        #[arg(long)]
        planned: u32,
        #[arg(long)]
        completed: u32,
        #[arg(long, value_enum)]
        status: CellStatusArg,
        #[arg(long)]
        fingerprint: PathBuf,
        #[arg(long, default_value = "ledger/probe-ledger.jsonl")]
        ledger: PathBuf,
        #[arg(long)]
        strict: bool,
        #[arg(long, default_value = "")]
        note: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CellStatusArg {
    Completed,
    #[value(name = "under-filled")]
    UnderFilled,
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Fingerprint => {
            let fp = fingerprint::collect().map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&fp).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        Commands::DispatchCheck {
            matrix,
            cell,
            cap,
            counter,
        } => {
            if matrix.trim().is_empty() || cell.trim().is_empty() {
                return Err("dispatch-check: --matrix and --cell must be non-empty".to_string());
            }
            let counter_path = std::env::var("PROBE_DISPATCH_COUNTER")
                .map(PathBuf::from)
                .unwrap_or(counter);
            let day_key = ledger::today_day_key();
            let count = ledger::DispatchCounter::open(&counter_path)
                .check_and_increment(day_key, cap)
                .map_err(|e| e.to_string())?;
            println!("dispatch-check: {matrix}/{cell} day {day_key} count {count}/{cap}");
            Ok(())
        }
        Commands::ProbeRecord {
            matrix,
            cell,
            attempt,
            planned,
            completed,
            status,
            fingerprint: fp_path,
            ledger: ledger_path,
            strict,
            note,
        } => {
            if completed > planned {
                return Err(format!(
                    "completed ({completed}) exceeds planned ({planned}): refusing"
                ));
            }
            let text =
                std::fs::read_to_string(&fp_path).map_err(|e| format!("fingerprint read: {e}"))?;
            let fp: fingerprint::Fingerprint =
                serde_json::from_str(&text).map_err(|e| format!("fingerprint parse: {e}"))?;
            if strict {
                let missing = fp.missing_sources();
                if !missing.is_empty() {
                    return Err(format!(
                        "strict: refusing gapped fingerprint, missing: {}",
                        missing.join(",")
                    ));
                }
            }
            let entry = ledger::LedgerEntry {
                cell: ledger::CellId {
                    stage: "s1".to_string(),
                    matrix,
                    cell,
                    attempt,
                },
                planned_runs: planned,
                completed_runs: completed,
                status: match status {
                    CellStatusArg::Completed => ledger::CellStatus::Completed,
                    CellStatusArg::UnderFilled => ledger::CellStatus::UnderFilled,
                },
                fingerprint: fp,
                sample_refs: Vec::new(),
                recorded_unix_secs: ledger::now_unix_secs(),
                note,
            };
            ledger::Ledger::open(&ledger_path)
                .append(&entry)
                .map_err(|e| e.to_string())?;
            println!("appended {}", entry.cell);
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cpu-enough: error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
