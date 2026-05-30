#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_PATH="${1:-$ROOT/jscpd/fixtures}"
RUNS="${RUNS:-5}"
MIN_TOKENS="${MIN_TOKENS:-20}"
MIN_LINES="${MIN_LINES:-3}"
MAX_SIZE="${MAX_SIZE:-10mb}"
FORMAT="${FORMAT:-}"

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

rust_cmd=("$ROOT/target/release/jscpd" "$TARGET_PATH" --silent --min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size "$MAX_SIZE")
node_cmd=(node "$ROOT/jscpd/apps/jscpd/bin/jscpd" "$TARGET_PATH" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size "$MAX_SIZE")

if [[ -n "$FORMAT" ]]; then
  rust_cmd+=(--format "$FORMAT")
  node_cmd+=(--format "$FORMAT")
fi

measure() {
  local label="$1"
  shift
  local total="0"
  printf '%s\n' "$label"
  for run in $(seq 1 "$RUNS"); do
    local start_ns
    local end_ns
    local seconds
    start_ns="$(date +%s%N)"
    "$@" >/tmp/jscpd-rs-bench.out 2>/tmp/jscpd-rs-bench.err || {
      cat /tmp/jscpd-rs-bench.out >&2 || true
      cat /tmp/jscpd-rs-bench.err >&2 || true
      return 1
    }
    end_ns="$(date +%s%N)"
    seconds="$(awk -v start="$start_ns" -v end="$end_ns" 'BEGIN { printf "%.6f", (end - start) / 1000000000 }')"
    printf '  run %s: %ss\n' "$run" "$seconds"
    total="$(awk -v a="$total" -v b="$seconds" 'BEGIN { printf "%.6f", a + b }')"
  done
  awk -v total="$total" -v runs="$RUNS" 'BEGIN { printf "  avg: %.6fs\n", total / runs }'
}

printf 'target: %s\n' "$TARGET_PATH"
printf 'runs: %s\n' "$RUNS"
printf 'min tokens: %s, min lines: %s, max size: %s\n\n' "$MIN_TOKENS" "$MIN_LINES" "$MAX_SIZE"
if [[ -n "$FORMAT" ]]; then
  printf 'format: %s\n\n' "$FORMAT"
fi

measure "rust mvp" "${rust_cmd[@]}"
printf '\n'
measure "upstream jscpd" "${node_cmd[@]}"
