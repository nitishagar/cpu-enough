# Gate Log (S4 GPU arm: READY or BLOCKED)

Verdict schema — one entry, decided in 1.4, consumed by S4:

- Verdict: READY | BLOCKED
- Evidence: T4 asserted (`nvidia-smi -L` transcript in runlog) + quota
  values transcribed; or FAILED with reason.
- If BLOCKED: reason + `docs/d4-trigger-log.md` entry (no silent drop).

## Verdict

TBD (1.4 undispatched).
