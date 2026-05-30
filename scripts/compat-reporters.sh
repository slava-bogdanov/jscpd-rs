#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_PATH="${1:-$ROOT/jscpd/fixtures/clike/file2.c}"
if (($# > 0)); then
  shift
fi
EXTRA_ARGS=("$@")
MIN_TOKENS="${MIN_TOKENS:-20}"
MIN_LINES="${MIN_LINES:-3}"
MAX_SIZE="${MAX_SIZE:-1mb}"
REPORTERS="${REPORTERS:-json,csv,markdown,xml,sarif,badge,html}"
FORMAT="${FORMAT:-}"
DETECTION_MODE="${DETECTION_MODE:-}"
TMP_ROOT="${TMP_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/jscpd-rs-reporters.XXXXXX")}"
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
  --reporters "$REPORTERS"
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
  --reporters "$REPORTERS"
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
if ((${#EXTRA_ARGS[@]} > 0)); then
  rust_cmd+=("${EXTRA_ARGS[@]}")
  node_cmd+=("${EXTRA_ARGS[@]}")
fi

printf 'target: %s\n' "$TARGET_PATH"
printf 'reporters: %s\n' "$REPORTERS"
printf 'min tokens: %s, min lines: %s, max size: %s\n' "$MIN_TOKENS" "$MIN_LINES" "$MAX_SIZE"
if [[ -n "$FORMAT" ]]; then
  printf 'format: %s\n' "$FORMAT"
fi
if [[ -n "$DETECTION_MODE" ]]; then
  printf 'mode: %s\n' "$DETECTION_MODE"
fi
if ((${#EXTRA_ARGS[@]} > 0)); then
  printf 'extra args:'
  printf ' %q' "${EXTRA_ARGS[@]}"
  printf '\n'
fi
printf 'tmp: %s\n\n' "$TMP_ROOT"

"${rust_cmd[@]}"
"${node_cmd[@]}"

artifacts=(
  "jscpd-report.json"
  "jscpd-report.csv"
  "jscpd-report.md"
  "jscpd-report.xml"
  "jscpd-sarif.json"
  "jscpd-badge.svg"
  "html/index.html"
  "html/jscpd-report.json"
)

check_artifacts() {
  local label="$1"
  local dir="$2"
  local failed=0

  for artifact in "${artifacts[@]}"; do
    if [[ ! -s "$dir/$artifact" ]]; then
      printf '%s artifact missing or empty: %s\n' "$label" "$dir/$artifact" >&2
      failed=1
    fi
  done

  return "$failed"
}

check_artifacts rust "$RUST_OUT"
check_artifacts upstream "$UPSTREAM_OUT"

node --input-type=module - "$RUST_OUT" "$UPSTREAM_OUT" <<'NODE'
import fs from 'node:fs';
import path from 'node:path';

const [rustDir, upstreamDir] = process.argv.slice(2);
for (const [label, dir] of [['rust', rustDir], ['upstream', upstreamDir]]) {
  parseJson(label, dir, 'jscpd-report.json');
  parseJson(label, dir, 'jscpd-sarif.json');
  parseJson(label, dir, path.join('html', 'jscpd-report.json'));
  requireContains(label, dir, 'jscpd-report.csv', 'Format,Files analyzed');
  requireContains(label, dir, 'jscpd-report.md', '# Copy/paste detection report');
  requireContains(label, dir, 'jscpd-report.xml', '<pmd-cpd>');
  requireContains(label, dir, 'jscpd-badge.svg', '<svg');
  requireContains(label, dir, path.join('html', 'index.html'), 'Copy/Paste Detector Report');
}

function parseJson(label, dir, file) {
  const fullPath = path.join(dir, file);
  try {
    JSON.parse(fs.readFileSync(fullPath, 'utf8'));
  } catch (error) {
    console.error(`${label} invalid JSON: ${fullPath}`);
    console.error(error.message);
    process.exit(1);
  }
}

function requireContains(label, dir, file, needle) {
  const fullPath = path.join(dir, file);
  const content = fs.readFileSync(fullPath, 'utf8');
  if (!content.includes(needle)) {
    console.error(`${label} artifact does not contain ${JSON.stringify(needle)}: ${fullPath}`);
    process.exit(1);
  }
}
NODE

STRICT="${STRICT:-coverage}" node "$ROOT/scripts/compare-reports.mjs" \
  "$RUST_OUT/jscpd-report.json" \
  "$UPSTREAM_OUT/jscpd-report.json"

if [[ "${KEEP:-0}" == "1" ]]; then
  printf '\nrust report dir: %s\n' "$RUST_OUT"
  printf 'upstream report dir: %s\n' "$UPSTREAM_OUT"
fi
