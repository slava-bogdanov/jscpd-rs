#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_REL="${TARGET_REL:-jscpd/fixtures/clike/file2.c}"
TARGET_FILE="${TARGET_FILE:-$ROOT/$TARGET_REL}"
TARGET_FILE_ABS="$(cd "$(dirname "$TARGET_FILE")" && pwd)/$(basename "$TARGET_FILE")"
MIN_TOKENS="${MIN_TOKENS:-20}"
MIN_LINES="${MIN_LINES:-3}"
MAX_SIZE="${MAX_SIZE:-1mb}"
TMP_ROOT="${TMP_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/jscpd-rs-cli.XXXXXX")}"
LAST_CASE_DIR=""

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

run_command() {
  local code_file="$1"
  local stdout_file="$2"
  local stderr_file="$3"
  shift 3

  set +e
  "$@" >"$stdout_file" 2>"$stderr_file"
  local code=$?
  set -e
  printf '%s' "$code" >"$code_file"
}

strip_ansi() {
  local input="$1"
  local output="$2"

  node --input-type=module - "$input" "$output" <<'NODE'
import fs from 'node:fs';

const [input, output] = process.argv.slice(2);
const ansi = /\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])/g;
fs.writeFileSync(output, fs.readFileSync(input, 'utf8').replace(ansi, ''));
NODE
}

case_slug() {
  printf '%s' "$1" | tr -cs '[:alnum:]' '-'
}

run_case() {
  local name="$1"
  local expected_code="$2"
  shift 2
  local args=("$@")
  local slug
  slug="$(case_slug "$name")"
  local case_dir="$TMP_ROOT/$slug"
  mkdir -p "$case_dir"

  run_command \
    "$case_dir/rust.code" \
    "$case_dir/rust.stdout" \
    "$case_dir/rust.stderr" \
    "$ROOT/target/release/jscpd-rs" \
    "${args[@]}"
  run_command \
    "$case_dir/upstream.code" \
    "$case_dir/upstream.stdout" \
    "$case_dir/upstream.stderr" \
    node "$ROOT/jscpd/apps/jscpd/bin/jscpd" \
    "${args[@]}"

  strip_ansi "$case_dir/rust.stdout" "$case_dir/rust.stdout.clean"
  strip_ansi "$case_dir/rust.stderr" "$case_dir/rust.stderr.clean"
  strip_ansi "$case_dir/upstream.stdout" "$case_dir/upstream.stdout.clean"
  strip_ansi "$case_dir/upstream.stderr" "$case_dir/upstream.stderr.clean"

  local rust_code upstream_code
  rust_code="$(<"$case_dir/rust.code")"
  upstream_code="$(<"$case_dir/upstream.code")"
  if [[ "$rust_code" != "$expected_code" || "$upstream_code" != "$expected_code" ]]; then
    printf 'exit code mismatch for %s: rust=%s upstream=%s expected=%s\n' \
      "$name" "$rust_code" "$upstream_code" "$expected_code" >&2
    print_case "$case_dir"
    return 1
  fi

  printf 'ok %-26s code=%s\n' "$name" "$expected_code"
  LAST_CASE_DIR="$case_dir"
}

require_both_contain() {
  local stream="$1"
  local needle="$2"
  local rust_file="$LAST_CASE_DIR/rust.$stream.clean"
  local upstream_file="$LAST_CASE_DIR/upstream.$stream.clean"

  require_contains "$rust_file" "$needle" "rust $stream"
  require_contains "$upstream_file" "$needle" "upstream $stream"
}

require_contains() {
  local file="$1"
  local needle="$2"
  local label="$3"

  if ! grep -Fq -- "$needle" "$file"; then
    printf '%s missing expected text: %s\n' "$label" "$needle" >&2
    print_case "$LAST_CASE_DIR"
    return 1
  fi
}

require_both_not_contain() {
  local stream="$1"
  local needle="$2"
  local rust_file="$LAST_CASE_DIR/rust.$stream.clean"
  local upstream_file="$LAST_CASE_DIR/upstream.$stream.clean"

  require_not_contains "$rust_file" "$needle" "rust $stream"
  require_not_contains "$upstream_file" "$needle" "upstream $stream"
}

require_not_contains() {
  local file="$1"
  local needle="$2"
  local label="$3"

  if grep -Fq -- "$needle" "$file"; then
    printf '%s had unexpected text: %s\n' "$label" "$needle" >&2
    print_case "$LAST_CASE_DIR"
    return 1
  fi
}

print_case() {
  local case_dir="$1"
  for tool in rust upstream; do
    printf '\n%s stdout:\n' "$tool" >&2
    sed -n '1,120p' "$case_dir/$tool.stdout.clean" >&2 || true
    printf '\n%s stderr:\n' "$tool" >&2
    sed -n '1,120p' "$case_dir/$tool.stderr.clean" >&2 || true
  done
  if [[ "${KEEP:-0}" != "1" ]]; then
    printf '\nrerun with KEEP=1 to preserve %s\n' "$TMP_ROOT" >&2
  fi
}

COMMON_ARGS=(--min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size "$MAX_SIZE")
SUMMARY="Duplications detection: Found 1 exact clones with 10(35.71%) duplicated lines in 1 (1 formats) files."
THRESHOLD_ERROR="ERROR: jscpd found too many duplicates (35.71%) over threshold (10%)"
UNKNOWN_REPORTER_WARNING="warning: badgezz not installed (install packages named @jscpd/badgezz-reporter or jscpd-badgezz-reporter)"
STORE_WARNING="store name leveldb not installed."
XCODE_ABSOLUTE_WARNING="$TARGET_FILE_ABS:18:3: warning: Found 10 lines (18-28) duplicated on file $TARGET_FILE_ABS (8-18)"
XCODE_RELATIVE_WARNING="$TARGET_FILE_ABS:18:3: warning: Found 10 lines (18-28) duplicated on file $TARGET_REL (8-18)"

printf 'target: %s\n' "$TARGET_REL"
printf 'tmp: %s\n\n' "$TMP_ROOT"

run_case "help output" 0 --help
require_both_contain stdout "detector of copy/paste in files"
require_both_contain stdout "Usage: jscpd [options] <path ...>"
require_both_contain stdout "min size of duplication in code lines"
require_both_contain stdout "ignore comments during detection"
require_both_contain stdout "alias for --mode"

run_case "list formats" 0 --list --silent --format abcdefghijklmnopqrstuvwxyz
require_both_contain stdout "Supported formats:"
require_both_contain stdout "typescript"

run_case "debug listing" 0 "$TARGET_REL" --debug --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "Options:"
require_both_contain stdout "path: ["
require_both_contain stdout "mode: [Function: mild]"
require_both_contain stdout "maxSize: '1mb'"
require_both_contain stdout "Found 1 files to detect."

run_case "exit code on clones" 7 "$TARGET_REL" --exitCode 7 --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$SUMMARY"

run_case "decimal max size" 0 "$TARGET_REL" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size 1.5kb
require_both_contain stdout "$SUMMARY"

run_case "terabyte max size" 0 "$TARGET_REL" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size 1tb
require_both_contain stdout "$SUMMARY"

run_case "short suffix max size" 0 "$TARGET_REL" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size 1k
require_both_not_contain stdout "Duplications detection:"

run_case "invalid max size" 0 "$TARGET_REL" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size nope
require_both_not_contain stdout "Duplications detection:"

run_case "line filter no files" 0 "$TARGET_REL" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines 999999 --max-size "$MAX_SIZE"
require_both_not_contain stdout "Duplications detection:"

run_case "store fallback warning" 0 "$TARGET_REL" --store leveldb --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$SUMMARY"
require_both_contain stderr "$STORE_WARNING"

run_case "unknown reporter warning" 0 "$TARGET_REL" --reporters badgezz --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$UNKNOWN_REPORTER_WARNING"
require_both_contain stdout "$SUMMARY"

run_case "threshold failure" 1 "$TARGET_REL" --threshold 10 --noTips "${COMMON_ARGS[@]}"
require_both_contain stderr "$THRESHOLD_ERROR"

run_case "xcode absolute" 0 "$TARGET_FILE_ABS" --reporters xcode --absolute --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$XCODE_ABSOLUTE_WARNING"
require_both_contain stdout "Found 1 clones."

run_case "xcode relative" 0 "$TARGET_FILE_ABS" --reporters xcode --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$XCODE_RELATIVE_WARNING"
require_both_contain stdout "Found 1 clones."

run_case "console full" 0 "$TARGET_REL" --reporters consoleFull --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "Clone found (c):"
require_both_contain stdout "Found 1 clones."

if [[ "${KEEP:-0}" == "1" ]]; then
  printf '\nkept output dir: %s\n' "$TMP_ROOT"
fi
