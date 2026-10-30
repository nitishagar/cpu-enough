# Contributing

Solo project (`nitishagar`). No co-authored commits or PRs.

## Cadence rule (JOSS clock)

Small, frequent commits; no end-of-month dumps. Ledger appends (`ledger/`)
merge daily. Every commit message names the stage/phase it belongs to
(e.g. `s1-1.1: scaffold CI`).

## Verification before push

```sh
cargo test --locked
cargo clippy --all-targets -- -D warnings
```

CI runs the same on `ubuntu-24.04` and `ubuntu-24.04-arm`, plus the fixture
cap (100 MB) and the action-SHA lint (no floating tags).
