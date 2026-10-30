# Prerequisite accounts + toolchain (public-safe: versions/dates only, no secrets)

| Account | Status | Evidence |
|---|---|---|
| GitHub | VERIFIED 2026-10-29 | `gh auth status`: nitishagar, active, ssh |
| Rust toolchain | VERIFIED 2026-10-29 | cargo/rustc 1.97.1; `rust-toolchain.toml` pinned |
| HuggingFace | PENDING | needs `huggingface-cli whoami` name |
| Oracle Cloud | PENDING | needs tenancy OCID |
| Kaggle | PENDING | needs transcribed quota-page values |
| ORCID | PENDING | needs profile URL |
| OpenReview | PENDING | needs profile URL |
| Zenodo | PENDING | needs username |
| arXiv | PENDING | needs username |
| Laptop fingerprint | PENDING | needs committed `lscpu` record (see below) |
| Laptop RAPL | PENDING | needs root double-read over sleep 10 |

Local repo path: `/home/nitish/repos/learn/cpu-enough` (recorded 2026-10-29).
Secret storage locations (outside repo): PENDING user confirm (HF + Kaggle tokens).

## Laptop fingerprint (committed 2026-10-29)

- Arch: x86_64 · Model: Intel(R) Core(TM) i5-7260U @ 2.20GHz · 2C/4T (SMT)
- Kernel: 7.0.0-38-generic · nproc: 4
- RAPL: `/sys/class/powercap/intel-rapl:0/energy_uj` exists, root-only
  (Permission denied unprivileged 2026-10-29) — privileged double-read PENDING.
