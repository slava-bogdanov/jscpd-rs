#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'USAGE'
usage: scripts/check-format.sh <format> [target-path]

Run the standard local checks for one format task. By default this is a Rust
smoke check. Set MODE=compat to compare against upstream jscpd on the same
target with the coverage gate.

Environment:
  MODE        smoke | compat (default: smoke)
  MIN_TOKENS  minimum token window (default: 20)
  MIN_LINES   minimum clone lines (default: 3)
  MAX_SIZE    max file size (default: 10mb)
  STRICT      compare-reports strictness when MODE=compat (default: coverage)
  DETECTION_MODE strict | mild | weak (default: upstream/Rust default)

Examples:
  scripts/check-format.sh css jscpd/fixtures/css
  MODE=compat scripts/check-format.sh typescript jscpd/fixtures
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage >&2
  exit 2
fi

format="$1"
target="${2:-$ROOT/jscpd/fixtures}"
mode="${MODE:-smoke}"
min_tokens="${MIN_TOKENS:-20}"
min_lines="${MIN_LINES:-3}"
max_size="${MAX_SIZE:-10mb}"
detect_mode="${DETECTION_MODE:-}"
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/jscpd-rs-format.XXXXXX")"

cleanup() {
  if [[ "${KEEP:-0}" != "1" ]]; then
    rm -rf "$tmp_root"
  fi
}
trap cleanup EXIT

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
fi

cd "$ROOT"
cargo test

mkdir -p "$tmp_root/report"
rust_cmd=(
  cargo run --quiet --
  "$target"
  --format "$format"
  --reporters json
  --output "$tmp_root/report"
  --silent
  --min-tokens "$min_tokens"
  --min-lines "$min_lines"
  --max-size "$max_size"
  --exitCode 0
)
if [[ -n "$detect_mode" ]]; then
  rust_cmd+=(--mode "$detect_mode")
fi

"${rust_cmd[@]}"

node - <<'NODE' "$tmp_root/report/jscpd-report.json"
const fs = require('fs');
const path = process.argv[2];
const report = JSON.parse(fs.readFileSync(path, 'utf8'));
console.log(`rust sources=${report.statistics.total.sources} clones=${report.duplicates.length}`);
NODE

case "$mode" in
  smoke)
    ;;
  compat)
    STRICT="${STRICT:-coverage}" \
      DETECTION_MODE="$detect_mode" \
      FORMAT="$format" \
      MIN_TOKENS="$min_tokens" \
      MIN_LINES="$min_lines" \
      MAX_SIZE="$max_size" \
      "$ROOT/scripts/compat.sh" "$target"
    ;;
  *)
    printf 'unknown MODE: %s\n' "$mode" >&2
    usage >&2
    exit 2
    ;;
esac

if [[ "${KEEP:-0}" == "1" ]]; then
  printf 'report: %s\n' "$tmp_root/report/jscpd-report.json"
fi
