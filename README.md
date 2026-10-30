# cpu-enough

Harness + measurement code for the study **"When is a CPU enough?"** —
SLO-constrained cost/energy of serving small LLMs on commodity free-tier
CPUs for agentic workloads. Paper track: TMLR; software track: JOSS.

## Install

Requires Rust (pinned — see `rust-toolchain.toml`):

```sh
cargo build --locked
```

## Usage

```sh
cargo run -- --help
```

(Subcommands land with the 1.2 harness skeleton; this scaffold exposes the
entry point only.)

## Reproducing the paper

See `paper/` (from stage S4) and `reproduce.sh` (one-script figure
regeneration). Every released result pins engine + commit + quant + model +
prompt + threads (see `docs/qualified-configs.md` from stage S2).

## License

MIT — see `LICENSE-MIT`.
