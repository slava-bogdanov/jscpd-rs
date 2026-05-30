#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_PATH="${1:-$ROOT/jscpd/fixtures}"
MIN_TOKENS="${MIN_TOKENS:-20}"
MIN_LINES="${MIN_LINES:-3}"
MAX_SIZE="${MAX_SIZE:-10mb}"
FORMAT="${FORMAT:-}"
DETECTION_MODE="${DETECTION_MODE:-}"
TMP_ROOT="${TMP_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/jscpd-rs-compat.XXXXXX")}"
RUST_OUT="$TMP_ROOT/rust"
UPSTREAM_OUT="$TMP_ROOT/upstream"

cleanup() {
  if [[ "${KEEP:-0}" != "1" ]]; then
    rm -rf "$TMP_ROOT"
  fi
}
trap cleanup EXIT

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
fi

if command -v corepack >/dev/null 2>&1; then
  corepack prepare pnpm@10.28.0 --activate >/dev/null
fi

cd "$ROOT"
cargo build --release >/dev/null

if [[ ! -d "$ROOT/jscpd/node_modules" ]]; then
  pnpm --dir "$ROOT/jscpd" install --frozen-lockfile
fi

if [[ ! -f "$ROOT/jscpd/apps/jscpd/dist/bin/jscpd.js" ]]; then
  pnpm --dir "$ROOT/jscpd" build
fi

mkdir -p "$RUST_OUT" "$UPSTREAM_OUT"

rust_cmd=(
  "$ROOT/target/release/jscpd-rs"
  "$TARGET_PATH"
  --reporters json
  --output "$RUST_OUT"
  --silent
  --min-tokens "$MIN_TOKENS"
  --min-lines "$MIN_LINES"
  --max-size "$MAX_SIZE"
  --exitCode 0
)
node_cmd=(
  node "$ROOT/jscpd/apps/jscpd/bin/jscpd"
  "$TARGET_PATH"
  --reporters json
  --output "$UPSTREAM_OUT"
  --silent
  --noTips
  --min-tokens "$MIN_TOKENS"
  --min-lines "$MIN_LINES"
  --max-size "$MAX_SIZE"
  --exitCode 0
)

if [[ -n "$FORMAT" ]]; then
  rust_cmd+=(--format "$FORMAT")
  node_cmd+=(--format "$FORMAT")
fi
if [[ -n "$DETECTION_MODE" ]]; then
  rust_cmd+=(--mode "$DETECTION_MODE")
  node_cmd+=(--mode "$DETECTION_MODE")
fi

printf 'target: %s\n' "$TARGET_PATH"
printf 'min tokens: %s, min lines: %s, max size: %s\n' "$MIN_TOKENS" "$MIN_LINES" "$MAX_SIZE"
if [[ -n "$FORMAT" ]]; then
  printf 'format: %s\n' "$FORMAT"
fi
if [[ -n "$DETECTION_MODE" ]]; then
  printf 'mode: %s\n' "$DETECTION_MODE"
fi
printf 'tmp: %s\n\n' "$TMP_ROOT"

"${rust_cmd[@]}"
"${node_cmd[@]}"

node "$ROOT/scripts/compare-reports.mjs" \
  "$RUST_OUT/jscpd-report.json" \
  "$UPSTREAM_OUT/jscpd-report.json"

if [[ "${KEEP:-0}" == "1" ]]; then
  printf '\nrust report: %s\n' "$RUST_OUT/jscpd-report.json"
  printf 'upstream report: %s\n' "$UPSTREAM_OUT/jscpd-report.json"
fi
