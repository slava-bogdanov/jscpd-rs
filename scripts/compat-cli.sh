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
  local env_prefix=()
  if [[ "${RUN_WITHOUT_CI:-0}" == "1" ]]; then
    env_prefix=(env -u CI)
  fi

  run_command \
    "$case_dir/rust.code" \
    "$case_dir/rust.stdout" \
    "$case_dir/rust.stderr" \
    "${env_prefix[@]}" \
    "$ROOT/target/release/jscpd" \
    "${args[@]}"
  run_command \
    "$case_dir/upstream.code" \
    "$case_dir/upstream.stdout" \
    "$case_dir/upstream.stderr" \
    "${env_prefix[@]}" \
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

run_case_without_ci() {
  RUN_WITHOUT_CI=1 run_case "$@"
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

require_both_count() {
  local stream="$1"
  local needle="$2"
  local expected="$3"
  local rust_file="$LAST_CASE_DIR/rust.$stream.clean"
  local upstream_file="$LAST_CASE_DIR/upstream.$stream.clean"

  require_count "$rust_file" "$needle" "$expected" "rust $stream"
  require_count "$upstream_file" "$needle" "$expected" "upstream $stream"
}

require_count() {
  local file="$1"
  local needle="$2"
  local expected="$3"
  local label="$4"
  local actual
  actual="$(grep -F -- "$needle" "$file" | wc -l | tr -d ' ')"

  if [[ "$actual" != "$expected" ]]; then
    printf '%s had %s occurrences of %s, expected %s\n' "$label" "$actual" "$needle" "$expected" >&2
    print_case "$LAST_CASE_DIR"
    return 1
  fi
}

require_both_before() {
  local stream="$1"
  local first="$2"
  local second="$3"
  local rust_file="$LAST_CASE_DIR/rust.$stream.clean"
  local upstream_file="$LAST_CASE_DIR/upstream.$stream.clean"

  require_before "$rust_file" "$first" "$second" "rust $stream"
  require_before "$upstream_file" "$first" "$second" "upstream $stream"
}

require_before() {
  local file="$1"
  local first="$2"
  local second="$3"
  local label="$4"

  node --input-type=module - "$file" "$first" "$second" "$label" <<'NODE'
import fs from 'node:fs';

const [file, first, second, label] = process.argv.slice(2);
const content = fs.readFileSync(file, 'utf8');
const firstIndex = content.indexOf(first);
const secondIndex = content.indexOf(second);

if (firstIndex === -1 || secondIndex === -1 || firstIndex > secondIndex) {
  console.error(`${label} expected "${first}" before "${second}"`);
  process.exit(1);
}
NODE
  local code=$?
  if [[ "$code" != "0" ]]; then
    print_case "$LAST_CASE_DIR"
    return "$code"
  fi
}

require_both_match() {
  local stream="$1"
  local pattern="$2"
  local rust_file="$LAST_CASE_DIR/rust.$stream.clean"
  local upstream_file="$LAST_CASE_DIR/upstream.$stream.clean"

  require_match "$rust_file" "$pattern" "rust $stream"
  require_match "$upstream_file" "$pattern" "upstream $stream"
}

require_match() {
  local file="$1"
  local pattern="$2"
  local label="$3"

  if ! grep -Eq -- "$pattern" "$file"; then
    printf '%s missing expected pattern: %s\n' "$label" "$pattern" >&2
    print_case "$LAST_CASE_DIR"
    return 1
  fi
}

require_help_option_sets_equal() {
  node --input-type=module - "$LAST_CASE_DIR/rust.stdout.clean" "$LAST_CASE_DIR/upstream.stdout.clean" <<'NODE'
import fs from 'node:fs';

const [rustPath, upstreamPath] = process.argv.slice(2);
const optionPattern = /--[A-Za-z][A-Za-z0-9-]*/g;

const extract = (path) => new Set(fs.readFileSync(path, 'utf8').match(optionPattern) ?? []);
const rust = extract(rustPath);
const upstream = extract(upstreamPath);
const missing = [...upstream].filter((option) => !rust.has(option)).sort();
const extra = [...rust].filter((option) => !upstream.has(option)).sort();

if (missing.length || extra.length) {
  console.error(`help option set mismatch`);
  if (missing.length) console.error(`missing in rust: ${missing.join(', ')}`);
  if (extra.length) console.error(`extra in rust: ${extra.join(', ')}`);
  process.exit(1);
}
NODE
}

require_help_option_aliases_equal() {
  node --input-type=module - "$LAST_CASE_DIR/rust.stdout.clean" "$LAST_CASE_DIR/upstream.stdout.clean" <<'NODE'
import fs from 'node:fs';

const [rustPath, upstreamPath] = process.argv.slice(2);
const aliasPattern = /(?<![A-Za-z0-9])-([A-Za-z])\b/g;

const extract = (path) => new Set(fs.readFileSync(path, 'utf8').match(aliasPattern) ?? []);
const rust = extract(rustPath);
const upstream = extract(upstreamPath);
const missing = [...upstream].filter((alias) => !rust.has(alias)).sort();
const extra = [...rust].filter((alias) => !upstream.has(alias)).sort();

if (missing.length || extra.length) {
  console.error(`help short-alias mismatch`);
  if (missing.length) console.error(`missing in rust: ${missing.join(', ')}`);
  if (extra.length) console.error(`extra in rust: ${extra.join(', ')}`);
  process.exit(1);
}
NODE
}

require_list_formats_equal() {
  node --input-type=module - "$LAST_CASE_DIR/rust.stdout.clean" "$LAST_CASE_DIR/upstream.stdout.clean" <<'NODE'
import fs from 'node:fs';

const [rustPath, upstreamPath] = process.argv.slice(2);
const extract = (path) => fs.readFileSync(path, 'utf8')
  .split(/\r?\n/)
  .flatMap((line) => line.includes(',') ? line.split(',') : [])
  .map((format) => format.trim())
  .filter(Boolean);

const rust = extract(rustPath);
const upstream = extract(upstreamPath);
const missing = upstream.filter((format) => !rust.includes(format));
const extra = rust.filter((format) => !upstream.includes(format));
const firstOrderMismatch = upstream.findIndex((format, index) => rust[index] !== format);

if (missing.length || extra.length || firstOrderMismatch !== -1) {
  console.error('supported format list mismatch');
  if (missing.length) console.error(`missing in rust: ${missing.join(', ')}`);
  if (extra.length) console.error(`extra in rust: ${extra.join(', ')}`);
  if (firstOrderMismatch !== -1) {
    console.error(`order mismatch at ${firstOrderMismatch}: rust=${rust[firstOrderMismatch]} upstream=${upstream[firstOrderMismatch]}`);
  }
  process.exit(1);
}
NODE
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
FORMATS_NAMES_SUMMARY="Duplications detection: Found 1 exact clones with 4(50%) duplicated lines in 2 (1 formats) files."
FORMATS_EXTS_SUMMARY="Duplications detection: Found 2 exact clones with 38(67.86%) duplicated lines in 2 (1 formats) files."
IGNORE_PATTERN_SUMMARY="Duplications detection: Found 1 exact clones with 7(14.58%) duplicated lines in 2 (1 formats) files."
IGNORE_CASE_OFF_SUMMARY="Duplications detection: Found 0 exact clones with 0(0%) duplicated lines in 2 (1 formats) files."
IGNORE_CASE_ON_SUMMARY="Duplications detection: Found 1 exact clones with 11(15.49%) duplicated lines in 2 (1 formats) files."
THRESHOLD_ERROR="ERROR: jscpd found too many duplicates (35.71%) over threshold (10%)"
EXIT_CODE_TYPE_ERROR_STRING="TypeError [ERR_INVALID_ARG_TYPE]: The \"code\" argument must be of type number. Received type string ('nope')"
EXIT_CODE_RANGE_ERROR="RangeError [ERR_OUT_OF_RANGE]: The value of \"code\" is out of range. It must be an integer. Received 7.5"
EXIT_CODE_TYPE_ERROR_BOOLEAN="TypeError [ERR_INVALID_ARG_TYPE]: The \"code\" argument must be of type number. Received type boolean (true)"
BARE_CONFIG_TYPE_ERROR="TypeError [ERR_INVALID_ARG_TYPE]: The \"paths[0]\" argument must be of type string. Received type boolean (true)"
UNKNOWN_REPORTER_WARNING="warning: badgezz not installed (install packages named @jscpd/badgezz-reporter or jscpd-badgezz-reporter)"
UNKNOWN_REPORTER_MODULE_ERROR="Cannot find module 'jscpd-badgezz-reporter'"
TIME_REPORTER_WARNING="warning: time not installed (install packages named @jscpd/time-reporter or jscpd-time-reporter)"
TIME_REPORTER_MODULE_ERROR="Cannot find module 'jscpd-time-reporter'"
STORE_WARNING="store name leveldb not installed."
BARE_STORE_WARNING="store name true not installed."
TIP_AI="Auto-refactor with AI"
TIP_GANGSTA="Gangsta Agents"
TIP_SUPPORT="Support jscpd project"
XCODE_ABSOLUTE_WARNING="$TARGET_FILE_ABS:18:3: warning: Found 10 lines (18-28) duplicated on file $TARGET_FILE_ABS (8-18)"
XCODE_RELATIVE_WARNING="$TARGET_FILE_ABS:18:3: warning: Found 10 lines (18-28) duplicated on file $TARGET_REL (8-18)"
IGNORE_ABS_DIR="$TMP_ROOT/relative-ignore-absolute"
mkdir -p "$IGNORE_ABS_DIR/patches" "$IGNORE_ABS_DIR/src"
cat >"$IGNORE_ABS_DIR/patches/patch.js" <<'EOF_JS'
const alpha = 1;
const beta = 2;
const gamma = 3;
EOF_JS
cp "$IGNORE_ABS_DIR/patches/patch.js" "$IGNORE_ABS_DIR/src/main.js"
SYMLINK_ROOT_DIR="$TMP_ROOT/no-symlink-root"
mkdir -p "$SYMLINK_ROOT_DIR/real"
cat >"$SYMLINK_ROOT_DIR/real/file.js" <<'EOF_JS'
const alpha = 1;
const beta = 2;
const gamma = 3;
EOF_JS
ln -s "$SYMLINK_ROOT_DIR/real" "$SYMLINK_ROOT_DIR/linkdir"
CWD_GITIGNORE_DIR="$TMP_ROOT/cwd-gitignore"
mkdir -p "$CWD_GITIGNORE_DIR/src" "$CWD_GITIGNORE_DIR/target" "$CWD_GITIGNORE_DIR/report" "$CWD_GITIGNORE_DIR/.bench" "$CWD_GITIGNORE_DIR/.idea"
for dir in src target report .bench .idea; do
  cat >"$CWD_GITIGNORE_DIR/$dir/file.js" <<'EOF_JS'
const alpha = 1;
const beta = 2;
const gamma = 3;
EOF_JS
done
FORMATS_NAMES_DIR="$TMP_ROOT/formats-names"
mkdir -p "$FORMATS_NAMES_DIR/a" "$FORMATS_NAMES_DIR/b"
cat >"$FORMATS_NAMES_DIR/a/CustomScript" <<'EOF_JS'
function alpha() {
  const one = 1;
  const two = 2;
  return one + two;
}
EOF_JS
cp "$FORMATS_NAMES_DIR/a/CustomScript" "$FORMATS_NAMES_DIR/b/CustomScript"
SKIP_COMMENTS_DIR="$TMP_ROOT/skip-comments"
mkdir -p "$SKIP_COMMENTS_DIR/a" "$SKIP_COMMENTS_DIR/b"
cat >"$SKIP_COMMENTS_DIR/a/file1.js" <<'EOF_JS'
// shared heading one
// shared heading two
// shared heading three
// shared heading four
// shared heading five
const alpha = 1;
const beta = 2;
const gamma = alpha + beta;
console.log(gamma);
EOF_JS
cp "$SKIP_COMMENTS_DIR/a/file1.js" "$SKIP_COMMENTS_DIR/b/file2.js"

printf 'target: %s\n' "$TARGET_REL"
printf 'tmp: %s\n\n' "$TMP_ROOT"

run_case "help output" 0 --help
require_both_contain stdout "detector of copy/paste in files"
require_both_contain stdout "Usage: jscpd [options] <path ...>"
require_both_contain stdout "min size of duplication in code lines"
require_both_contain stdout "ignore comments during detection"
require_both_contain stdout "alias for --mode"
require_both_contain stdout "output the version number"
require_help_option_sets_equal
require_help_option_aliases_equal

run_case "bare config help" 0 --config --help
require_both_contain stdout "Usage: jscpd [options] <path ...>"
require_both_contain stdout "path to config file"

run_case "version output" 0 --version
require_both_match stdout '^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9_.-]+)?$'
require_both_not_contain stdout "jscpd "

run_case "short version output" 0 -V
require_both_match stdout '^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9_.-]+)?$'
require_both_not_contain stdout "jscpd "

run_case "list formats" 0 --list --silent --format abcdefghijklmnopqrstuvwxyz
require_both_contain stdout "Supported formats:"
require_both_contain stdout "typescript"
require_list_formats_equal

run_case "bare config list" 1 --config --list
require_both_contain stdout "$BARE_CONFIG_TYPE_ERROR"

run_case "debug listing" 0 "$TARGET_REL" --debug --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "Options:"
require_both_contain stdout "path: ["
require_both_contain stdout "mode: [Function: mild]"
require_both_contain stdout "maxSize: '1mb'"
require_both_contain stdout "Found 1 files to detect."

run_case "relative ignore absolute path" 0 "$IGNORE_ABS_DIR" --debug --noTips --format javascript --ignore "patches/**" --min-tokens 1 --min-lines 1 --max-size 1mb
require_both_contain stdout "Found 1 files to detect."
require_both_not_contain stdout "patches/patch.js"

run_case "bare pattern directory" 0 jscpd/fixtures/clike --debug --noTips --format c --pattern --min-tokens 1 --min-lines 1 --max-size 1mb
require_both_contain stdout "Found 0 files to detect."

run_case "no symlinks root" 0 "$SYMLINK_ROOT_DIR/linkdir" --debug --noTips --format javascript --noSymlinks --min-tokens 1 --min-lines 1 --max-size 1mb
require_both_contain stdout "Found 0 files to detect."
require_both_not_contain stdout "file.js"

run_case "cwd gitignore absolute path" 0 "$CWD_GITIGNORE_DIR" --debug --noTips --format javascript --min-tokens 1 --min-lines 1 --max-size 1mb
require_both_contain stdout "Found 1 files to detect."
require_both_not_contain stdout "target/file.js"
require_both_not_contain stdout "report/file.js"

run_case "no gitignore disables cwd ignore" 0 "$CWD_GITIGNORE_DIR" --debug --noTips --format javascript --no-gitignore --min-tokens 1 --min-lines 1 --max-size 1mb
require_both_contain stdout "Found 5 files to detect."
require_both_contain stdout "target/file.js"
require_both_contain stdout "report/file.js"

run_case "formats names discovery" 0 "$FORMATS_NAMES_DIR" --format javascript --formats-names javascript:CustomScript --reporters silent --noTips --min-tokens 5 --min-lines 2 --max-size 1mb
require_both_contain stdout "$FORMATS_NAMES_SUMMARY"

run_case "formats exts discovery" 0 jscpd/fixtures/custom --formats-exts c:ccc,cc1 --reporters silent --noTips --min-tokens 50 --min-lines 5 --max-size 1mb
require_both_contain stdout "$FORMATS_EXTS_SUMMARY"

run_case "ignore pattern" 0 jscpd/fixtures/ignore-pattern --ignore-pattern "import.*from\\s*'.*'" --reporters silent --noTips --min-tokens 20 --min-lines 5 --max-size 1mb
require_both_contain stdout "$IGNORE_PATTERN_SUMMARY"

run_case "ignore case off" 0 jscpd/fixtures/ignore-case --reporters silent --noTips --min-tokens 50 --min-lines 5 --max-size 1mb
require_both_contain stdout "$IGNORE_CASE_OFF_SUMMARY"

run_case "ignore case on" 0 jscpd/fixtures/ignore-case --ignoreCase --reporters silent --noTips --min-tokens 50 --min-lines 5 --max-size 1mb
require_both_contain stdout "$IGNORE_CASE_ON_SUMMARY"

run_case "skip comments alias" 0 "$SKIP_COMMENTS_DIR" --skipComments --format javascript --reporters xcode --noTips --min-tokens 5 --min-lines 1 --max-size 1mb
require_both_contain stdout "file1.js:6:1: warning: Found 3 lines (6-9)"
require_both_not_contain stdout "file1.js:1:1: warning"

run_case "skip comments explicit mode" 0 "$SKIP_COMMENTS_DIR" --skipComments --mode strict --format javascript --reporters xcode --noTips --min-tokens 5 --min-lines 1 --max-size 1mb
require_both_contain stdout "file1.js:1:1: warning: Found 9 lines (1-10)"

run_case "exit code on clones" 7 "$TARGET_REL" --exitCode 7 --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$SUMMARY"

run_case "hex exit code on clones" 16 "$TARGET_REL" --exitCode 0x10 --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$SUMMARY"

run_case "invalid exit code string" 1 "$TARGET_REL" --exitCode nope --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$SUMMARY"
require_both_contain stdout "$EXIT_CODE_TYPE_ERROR_STRING"

run_case "fractional exit code" 1 "$TARGET_REL" --exitCode 7.5 --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$SUMMARY"
require_both_contain stdout "$EXIT_CODE_RANGE_ERROR"

run_case "bare exit code" 1 "$TARGET_REL" --exitCode --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$SUMMARY"
require_both_contain stdout "$EXIT_CODE_TYPE_ERROR_BOOLEAN"

run_case "decimal max size" 0 "$TARGET_REL" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size 1.5kb
require_both_contain stdout "$SUMMARY"

run_case "decimal numeric limits" 0 "$TARGET_REL" --silent --noTips --min-tokens 20.9 --min-lines 3.9 --max-lines 1000.9 --max-size 1mb
require_both_contain stdout "$SUMMARY"

run_case "missing numeric limit values" 0 "$TARGET_REL" --silent --noTips --min-lines --min-tokens --max-lines --max-size 1mb
require_both_contain stdout "$SUMMARY"

run_case "bare max size" 0 "$TARGET_REL" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size
require_both_not_contain stdout "Duplications detection:"

run_case "terabyte max size" 0 "$TARGET_REL" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size 1tb
require_both_contain stdout "$SUMMARY"

run_case "short suffix max size" 0 "$TARGET_REL" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size 1k
require_both_not_contain stdout "Duplications detection:"

run_case "invalid max size" 0 "$TARGET_REL" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines "$MIN_LINES" --max-size nope
require_both_not_contain stdout "Duplications detection:"

run_case "unknown mode" 1 "$TARGET_REL" --mode zzz --noTips

run_case "bare config" 1 --config
require_both_contain stdout "$BARE_CONFIG_TYPE_ERROR"

run_case "line filter no files" 0 "$TARGET_REL" --silent --noTips --min-tokens "$MIN_TOKENS" --min-lines 999999 --max-size "$MAX_SIZE"
require_both_not_contain stdout "Duplications detection:"

run_case "store fallback warning" 0 "$TARGET_REL" --store leveldb --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$SUMMARY"
require_both_contain stderr "$STORE_WARNING"

run_case "bare store warning" 0 "$TARGET_REL" --store --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$SUMMARY"
require_both_contain stderr "$BARE_STORE_WARNING"

run_case "store path debug" 0 "$TARGET_REL" --debug --store leveldb --store-path .jscpd-cache --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "store: 'leveldb'"
require_both_contain stdout "storePath: '.jscpd-cache'"

run_case "duplicate silent reporter" 0 "$TARGET_REL" --reporters silent --silent --noTips "${COMMON_ARGS[@]}"
require_both_count stdout "$SUMMARY" 2

run_case "unknown reporter warning" 0 "$TARGET_REL" --reporters badgezz --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$UNKNOWN_REPORTER_WARNING"
require_both_contain stdout "$UNKNOWN_REPORTER_MODULE_ERROR"
require_both_contain stdout "$SUMMARY"

run_case "time reporter warning" 0 "$TARGET_REL" --reporters time --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$TIME_REPORTER_WARNING"
require_both_contain stdout "$TIME_REPORTER_MODULE_ERROR"
require_both_contain stdout "Clone found (c):"
require_both_contain stdout "time:"
require_both_before stdout "$TIME_REPORTER_WARNING" "Clone found (c):"

run_case_without_ci "terminal footer tips" 0 "$TARGET_REL" --reporters silent "${COMMON_ARGS[@]}"
require_both_contain stdout "time:"
require_both_contain stdout "$TIP_AI"
require_both_contain stdout "$TIP_GANGSTA"
require_both_contain stdout "$TIP_SUPPORT"

run_case "terminal footer no tips" 0 "$TARGET_REL" --reporters silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "time:"
require_both_not_contain stdout "$TIP_AI"
require_both_not_contain stdout "$TIP_GANGSTA"
require_both_not_contain stdout "$TIP_SUPPORT"

run_case "ai reporter" 0 "$TARGET_REL" --reporters ai --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "Clones:"
require_both_contain stdout "$TARGET_REL 18-28 ~ 8-18"
require_both_contain stdout "35.7% duplication"
require_both_not_contain stdout "Clone found (c):"

run_case "verbose events" 0 "$TARGET_REL" --verbose --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "START_DETECTION"
require_both_contain stdout "Start detection for source id=$TARGET_REL format=c"
require_both_contain stdout "CLONE_FOUND"
require_both_contain stdout '"format": "c"'
require_both_contain stdout "Clone found (c):"
require_both_contain stdout "Found 1 clones."

run_case "bare threshold" 1 "$TARGET_REL" --threshold --noTips "${COMMON_ARGS[@]}"
require_both_contain stderr "ERROR: jscpd found too many duplicates (35.71%) over threshold (1%)"

run_case "hex threshold" 1 "$TARGET_REL" --threshold 0x10 --noTips "${COMMON_ARGS[@]}"
require_both_contain stderr "ERROR: jscpd found too many duplicates (35.71%) over threshold (16%)"

run_case "nan threshold" 0 "$TARGET_REL" --threshold nope --silent --noTips "${COMMON_ARGS[@]}"
require_both_contain stdout "$SUMMARY"
require_both_not_contain stderr "ERROR: jscpd found too many duplicates"

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
