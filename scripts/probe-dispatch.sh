#!/bin/sh
# probe-dispatch: pre-dispatch gate (runbook v0 counter rules). Linux-only
# (flock; dev + runners are linux). sh-compatible, set -eu.
#
# Usage:
#   scripts/probe-dispatch.sh --matrix M --cell C -- <gh workflow run args...>
#
# Gates (ALL must pass; any failure REFUSES without dispatching):
#   1. UTC-day cap (default 12) via `dispatch-check` (owns the counter).
#   2. Concurrent in-progress runs (repo-wide, incl. CI) < 6 via gh api.
#   3. gh query failures refuse (safe default — verify manually, re-run).
# The whole gate+dispatch runs under one flock so parallel shells cannot
# double-count the day-cap. Weekly pacing (<=5 campaign days/week) is
# operator discipline — check docs/dispatch-log.md before a dispatch day.
#
# Drill hook: PROBE_DISPATCH_COUNTER overrides the counter path (see the
# over-cap refusal drill in docs/tos-runbook.md).

set -eu

refuse() {
    echo "probe-dispatch: REFUSED: $1" >&2
    exit 1
}

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

MATRIX=""
CELL=""
CAP="12"
while [ $# -gt 0 ]; do
    case "$1" in
        --matrix) MATRIX="${2:-}"; shift 2 ;;
        --cell) CELL="${2:-}"; shift 2 ;;
        --cap) CAP="${2:-}"; shift 2 ;;
        --) shift; break ;;
        *) refuse "unknown argument: $1 (expected --matrix/--cell/--cap/--)" ;;
    esac
done
[ -n "$MATRIX" ] || refuse "--matrix is required"
[ -n "$CELL" ] || refuse "--cell is required"
[ $# -gt 0 ] || refuse "no gh workflow run args after --"
case "$*" in
    *run_tag*) refuse "run_tag is wrapper-issued; do not pass it manually" ;;
esac
case "$CAP" in
    *[!0-9]*|"") refuse "--cap must be a positive integer" ;;
esac
[ "$CAP" -gt 0 ] || refuse "--cap must be a positive integer"

COUNTER="${PROBE_DISPATCH_COUNTER:-$ROOT/ledger/dispatch-counter.json}"

# Serialize gate+dispatch under one lock (re-exec once).
if [ -z "${PROBE_DISPATCH_LOCKED:-}" ]; then
    export PROBE_DISPATCH_LOCKED=1
    exec flock "$COUNTER.lock" "$0" --matrix "$MATRIX" --cell "$CELL" --cap "$CAP" -- "$@"
fi

IN_PROGRESS=$(gh run list --status in_progress --limit 100 --json databaseId --jq 'length') \
    || refuse "gh in-progress query failed (safe default; verify manually and re-run)"
case "$IN_PROGRESS" in
    *[!0-9]*|"") refuse "gh in-progress query returned non-numeric: $IN_PROGRESS" ;;
esac
[ "$IN_PROGRESS" -lt 6 ] \
    || refuse "concurrent cap reached: $IN_PROGRESS in-progress (cap 6)"

CHECK_OUT=$(cargo run --quiet -- dispatch-check --matrix "$MATRIX" --cell "$CELL" --cap "$CAP" --counter "$COUNTER") \
    || refuse "day-cap check failed (see above)"
echo "$CHECK_OUT"
RUN_TAG=$(printf '%s' "$CHECK_OUT" | sed -n 's/.*day \([0-9][0-9]*\) count \([0-9][0-9]*\)\/.*/\1-\2/p')
[ -n "$RUN_TAG" ] || refuse "could not parse run_tag from dispatch-check output"

echo "probe-dispatch: gates passed; dispatching: gh workflow run $* -f run_tag=$RUN_TAG"
gh workflow run "$@" -f run_tag="$RUN_TAG"

echo "probe-dispatch: dispatched. Log it in docs/dispatch-log.md (runs, SHAs, caps consumed, signals)."
echo "  run id: gh run list --workflow=$1 --limit 1 --json databaseId --jq '.[0].databaseId'"
