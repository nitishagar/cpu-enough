// Shared library surface: integration tests and (later) subcommands use the
// crate through these modules. (PLAN_1 1.2: enables the specified tests/*.rs
// layout — see ledger entry 2026-10-30 for the factual amendment note.)

pub mod diagnostics;
pub mod fingerprint;
pub mod gate;
pub mod ledger;
