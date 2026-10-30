# Scaffold Contracts (S1 placeholders, S3-owned completion)

Two scaffolds ship in S1 so downstream code has stable call sites. Each
lists its completion criteria; S3 (campaign) must satisfy them before any
headline number flows through the scaffolded path.

## 1. Trace-schema fixtures

- Call site: `src/diagnostics.rs` → `trace_schema_fixture()`.
- Current behavior: returns a shape placeholder (`{"scaffold": true, ...}`).
  It is NEVER valid evidence — `check_trace_bytes` validates structure
  only and knows nothing of this shape.
- Completion criteria (S3): S2 has recorded `docs/llamabench-schema.json`;
  fixtures validate against that schema; `trace_schema_fixture` is
  replaced by schema-conformant constructors (or deleted if unused); the
  S1 placeholder shape appears nowhere in S3 outputs.
- Non-goals: inventing trace content; passing schema validation by
  weakening the schema.

## 2. Pairing format

- Call site: `src/diagnostics.rs` → `format_pairing_stub(left, right)`.
- Current behavior: returns `left|right` with no escaping or typing.
- Completion criteria (S3): the campaign pins the real pairing record
  (fields, separator/escaping, parse + round-trip tests); the stub is
  replaced at all call sites; no `|`-joined string reaches committed
  evidence.
- Non-goals: changing already-recorded S1/S2 evidence formats.
