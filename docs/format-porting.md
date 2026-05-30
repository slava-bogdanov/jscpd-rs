# Format Porting Guide

The first-release policy is coverage-first for hot JS/TS formats and smoke-only
for long-tail generic formats. A format can find more than upstream while
compatibility converges, but release-compatible formats must not miss upstream
duplicate fragments on their fixtures.

## Status Levels

- `generic`: format is recognized through the upstream-synchronized registry and
  uses coarse whitespace tokenization.
- `native-smoke`: Rust has format-specific logic and local smoke tests, but no
  upstream coverage claim.
- `coverage`: `MODE=compat scripts/check-format.sh <format> <target>` passes
  with `STRICT=coverage`.
- `release`: docs and tests make the support level explicit, and the format is
  included in the release matrix.

## Files To Know

- `src/formats.rs`: generated format and extension registry. Do not edit by
  hand; run `node scripts/sync-formats.mjs` after upstream tokenizer changes.
- `src/tokenizer.rs`: native and generic tokenization.
- `src/files.rs`: discovery and format filtering.
- `src/detector.rs`: clone detection; do not change for ordinary format work.
- `scripts/check-format.sh`: one-format smoke/compat checks.
- `scripts/compat.sh`: Rust vs upstream report comparison.
- `docs/compat-baseline.md`: current compatibility claims and known deltas.

## Minimal Format Task

1. Confirm the format is present:

   ```bash
   cargo run --quiet -- --list | rg '^<format>$'
   ```

2. Add or reuse a tiny target directory for the format.

3. Run smoke mode:

   ```bash
   scripts/check-format.sh <format> <target>
   ```

4. If the task claims upstream coverage, run compat mode:

   ```bash
   MODE=compat scripts/check-format.sh <format> <target>
   ```

5. Add focused tests near the code being changed.

6. Update docs only when the support level changes.

## Native Tokenizer Task

Native tokenizers should be added only when generic tokenization is too noisy or
misses practical clones. Prefer maintained Rust crates where available. If a
custom scanner is needed, keep it small and format-specific.

Expected shape:

- One small scanner/helper for the format.
- Unit tests for token slices, comments, weak mode, and at least one duplicate
  detection path.
- No detector changes unless there is a proven cross-format contract issue.
- No JavaScript runtime fallback.

## Junior-Safe Format Tasks

- Add a smoke test for a format already handled by generic tokenization.
- Add one comment-style test and no production code.
- Add one small production helper by copying an existing tokenizer pattern.
- Run `scripts/check-format.sh` and report exact `sources`/`clones` output.

## Main-Agent-Only Decisions

- Promoting a format to `coverage` or `release`.
- Adding dependencies.
- Changing detector contracts.
- Changing compatibility gate semantics.
- Editing generated registry logic.
