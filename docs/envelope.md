# Operating Envelope (S1 probes → S2/S3 inputs)

Method + merge procedure are FINAL in 1.3; result sections fill as probe
matrices complete. Nothing here is estimated — TBD means undispatched.

## Method (adaptive SKU survey)

- Population: GitHub-hosted `ubuntu-24.04` (x86) + `ubuntu-24.04-arm`.
- Rule: min 10 draws, continue to 30 or until no-new-SKU-over-10 draws,
  whichever comes first. Each draw = one wrapper dispatch (`survey`
  matrix) = one fingerprinted artifact + one ledger entry.
- SKU identity = full UNION fingerprint pooling key (see No-pooling rule);
  the observed-SKU table below is the census. Stopping before 30 draws
  without satisfying no-new-SKU-over-10 requires a coverage caveat.
- Capability matrices (one dispatch each unless fault-injection demands
  more): `arm-identity` (ARM label), `sudo`, `perf`, `powercap` — named
  commands in `probe.yml`; negatives are recorded evidence, never
  failures (except the fingerprint step, which fails loud).

## Observed SKUs

TBD (undispatched). Table: SKU (pooling key) × draws × first-seen run.

## Coverage caveat

TBD. Required text if the survey stops early: draws completed, stop
reason, and the explicit statement of which SKUs the campaign numbers do
NOT cover.

## No-pooling rule

Samples pool ONLY within identical pooling keys:
arch + cpu_model + kernel + image_version + vm_size + vcpus
(cpu_mhz + cpu_flags are informational, never key parts). Cross-key
input to the gate is a hard ERROR (`MultiSku`), never averaged. Figures
carry per-SKU series or the modal SKU with the spread stated.

## Reps/spread

TBD (per-SKU rep counts + observed spread from the survey).

## Healthy-output baselines

TBD (one known-good transcript per probe matrix after the first green
set; later runs diff against these).

## Sudo matrix

TBD (named command × exit code × output digest).

## Verdicts

TBD: `perf` usable/blocked; powercap/RAPL readable/blocked; ARM identity
resolved/unresolved — each with the exact evidence line.

## Timing lower-bounds

TBD (per-cell minimum durations; S2/S3 sizing inputs, measured not
guessed).

## Ledger merge procedure (manual, exact)

The workflow NEVER writes the ledger (permissions: contents: read).
After each dispatched run completes:

1. `gh run download <run-id> -n probe-<run_tag> -D results/<run-id>/`
   (`results/` is gitignored raws; committed evidence never lives there).
2. Inspect `results/<run-id>/out/fingerprint.json`: complete? If
   `run_tag.txt` reads UNTAGGED, the run is still recorded + flagged in
   `docs/dispatch-log.md` (daily reconciliation duty).
3. `cargo run --locked --quiet -- probe-record --matrix <m> --cell <c>
   --attempt <n> --planned <p> --completed <k> --status <completed|
   under-filled> --fingerprint results/<run-id>/out/fingerprint.json
   --strict --note <run-id/run_tag>`
4. Killed/canceled jobs: record `--completed 0 --status under-filled`
   with the kill reason in `--note`; original-planned retained; nothing
   backfilled (fault-injection path).
5. Commit ledger + dispatch-log entry (runs, SHAs, caps consumed).
