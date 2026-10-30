# ToS Runbook v0 (PROVISIONAL)

Status: provisional. Budgets below are estimates with explicit resize gates;
they become binding only after the S2-opening and S3-sizing re-checks.
Written 2026-10-30; GitHub Terms + limits re-verified at write time (see
Sources). Any resize needs user sign-off.

## Sources (verified 2026-10-30)

- GitHub Terms of Service (effective 27 Apr 2026): GitHub may suspend or
  terminate access to all or any part of the service at any time, with or
  without cause or notice; API abuse/excessive use may suspend API access.
- GitHub Acceptable Use Policies (fetched 2026-10-30): bans using servers
  "to place undue burden on our servers through automated means";
  §9 Excessive Bandwidth Use (undue strain → suspend/throttle/delete);
  cryptocurrency mining ban; enforcement = suspension/termination/removal.
- GitHub Terms for Additional Products and Features (version effective
  27 Aug 2026), Actions section: Actions must not be used for cryptomining;
  "any activity that places a burden on our servers, where that burden is
  disproportionate to the benefits provided to users"; or, on
  GitHub-hosted runners, "any other activity unrelated to the production,
  testing, deployment, or publication of the software project associated
  with the repository". Enforcement ladder: job termination → restrictions
  → repository disable → account suspension/termination.

- Actions billing & usage (fetched 2026-10-30): usage free for standard
  GitHub-hosted runners in public repositories; numeric caps live in the
  Actions limits reference (500 MB artifact cap, 6 h/job per prior
  research — re-check at 1.5 freeze).

Assessment: benchmarking this repo's OWN harness is testing of the
associated software project (permitted on its face). The residual risk is
the disproportionate-burden clause on sustained max-CPU matrices — hence
the caps, pacing, and signal watch below. No 24/7 loops, ever.

## Binding rates (ONE ToS rule)

- ≤12 jobs/day (UTC day), ≤6 concurrent, ≤5 campaign days/week.
- "Sustained" means any 4-week window. S1's one-time setup counts toward
  the FIRST window (no exemption).
- NO percentage-of-unknown claims. Verdict per stage = absolute load +
  signal watch (zero throttle/queue-delay/warning signals) + weekly
  ceiling review.

## Budgets (PROVISIONAL — resize at gates)

- S1: ≈16 job-h / ~75 jobs over ≥2 weeks (probes ≈12.3 + CI ≈3.3);
  Kaggle 0.5–1.0 GPU-h (base + redraws).
- S2 (resize gate: S2 opening): ≈22 job-h (~43 cells × ≤30 min);
  ≈150 GB downloads from HF under serial discipline.
- S3 (resize gate: S3 sizing): worked example ~230 job-h (C=40, n=10,
  15 min/run ÷ modal 0.5 + allowance), paced per this runbook.
- Re-probe after runner-image change: ≈+11 job-h (explicit, pre-approved
  size — still subject to the rates above).

## Floors (below floor = UNDER-FILLED, never failed)

- Survey cells: the adaptive stop rule decides (sample until no new SKU
  over 10 draws, within the 30-draw budget); a stopped-early cell keeps
  its draws with a coverage caveat.
- CI: green on both labels (x86 + Arm) is the floor for any claim.
- Kaggle: T4 asserted by `nvidia-smi -L`, or the arm is BLOCKED (no CPU
  fallback numbers presented as GPU numbers).

## Counter rules

- Every `gh workflow run` probe dispatch goes through
  `scripts/probe-dispatch.sh` (day-cap 12 via `dispatch-check`, UTC day
  key; concurrent <6 via gh api; gh failures refuse by safe default).
- Counter file: `ledger/dispatch-counter.json` (gitignored local state;
  the LEDGER `ledger/probe-ledger.jsonl` is the committed truth).
- Corrupt counter BLOCKS dispatch. Recovery: archive the corrupt file,
  delete it, re-run (count restarts at 1), record the incident in
  `docs/dispatch-log.md`. Never auto-reset, never hand-edit the count.
- KNOWN BYPASS (accepted, reconciled): CI pushes bypass the wrapper.
  Reconciliation: flurry >5 CI jobs/day → pause pushes + review; every CI
  job is counted by hand in `docs/dispatch-log.md` toward the ≤12/day.
- UI/API dispatches outside the wrapper are forbidden (single operator +
  log review is the enforcement; the workflow stays technically
  dispatchable — validator-noted, accepted).

## Campaign pacing (operator discipline)

- ≤5 campaign days per week — check `docs/dispatch-log.md` for this
  week's campaign days BEFORE starting a dispatch day.
- Probes are seconds-long commands on ≤20-min-budget jobs; matrices stay
  reasonable; no unattended overnight loops.

## Stop procedure

Signals taxonomy: (1) queue-delay spike (jobs waiting far beyond normal
provisioning), (2) throttle/rate-limit responses, (3) any warning email
or banner from GitHub. ANY signal → STOP:

1. Cancel running probe jobs (`gh run cancel <id>` each; never cancel CI
   on someone else's PR — N/A solo, noted for completeness).
2. No new dispatches until reviewed.
3. Write a `## Stops` entry in `docs/dispatch-log.md`: timestamp,
   trigger, runs canceled, workflow disabled y/n.
4. Appeal checklist (if actioned): account status, trigger description,
   remediation taken, support-ticket draft.

## Record formats

- `docs/dispatch-log.md`: dated entries (runs, SHAs, caps consumed that
  day, signals). `## Stops` and `## Signals` sections as above (`## Signals`:
  date, signal type, source, action taken).
- Sign-off format: `Signed-off: <name> <date> <commit> — <scope>`.

## Drills

Over-cap refusal drill (run before first real dispatch; re-run after any
wrapper/counter change):

```sh
export PROBE_DISPATCH_COUNTER=/tmp/drill-counter.json
rm -f /tmp/drill-counter.json
i=1; while [ "$i" -le 12 ]; do
  cargo run --quiet -- dispatch-check --matrix drill --cell c --cap 12 \
    --counter /tmp/drill-counter.json
  i=$((i + 1))
done
# 13th must REFUSE (exit non-zero, "day-cap reached"):
cargo run --quiet -- dispatch-check --matrix drill --cell c --cap 12 \
  --counter /tmp/drill-counter.json
# Wrapper must REFUSE without dispatching (counter full; gh NOT called
# for a run — only the read-only in-progress query):
./scripts/probe-dispatch.sh --matrix drill --cell c -- probe.yml
unset PROBE_DISPATCH_COUNTER
```

## Honest books

- CI run 40991743701 (2026-10-30, x86 fail + arm cancelled) was dispatched
  BEFORE any caps row existed — a deviation from "nothing dispatches
  before a cap exists". It counts toward the first 4-week window. Remedy:
  this runbook + the wrapper/counter live before any further dispatch.

## Sign-off

Unsigned (provisional v0; sign at freeze with the format above).
