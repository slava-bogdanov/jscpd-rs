#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_FIXTURE="${TARGET_FIXTURE:-$ROOT/jscpd/fixtures/one-file/one-file.js}"
TMP_ROOT="${TMP_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/jscpd-rs-config.XXXXXX")}"
RUST_PROJECT="$TMP_ROOT/rust"
UPSTREAM_PROJECT="$TMP_ROOT/upstream"
PACKAGE_RUST_PROJECT="$TMP_ROOT/rust-package"
PACKAGE_UPSTREAM_PROJECT="$TMP_ROOT/upstream-package"
INVALID_PACKAGE_RUST_PROJECT="$TMP_ROOT/rust-invalid-package"
INVALID_PACKAGE_UPSTREAM_PROJECT="$TMP_ROOT/upstream-invalid-package"
NAMES_RUST_PROJECT="$TMP_ROOT/rust-formats-names"
NAMES_UPSTREAM_PROJECT="$TMP_ROOT/upstream-formats-names"

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

make_project() {
  local project="$1"
  mkdir -p "$project/src"
  cp "$TARGET_FIXTURE" "$project/src/one.dup"
  cat >"$project/.jscpd.json" <<'JSON'
{
  "path": ["src"],
  "minTokens": 50,
  "minLines": 5,
  "maxSize": "1mb",
  "reporters": ["json"],
  "silent": true,
  "noTips": true,
  "output": "report",
  "formatsExts": {
    "typescript": ["dup"],
    "javascript": ["dup"]
  }
}
JSON
}

make_package_project() {
  local project="$1"
  mkdir -p "$project/src"
  cp "$TARGET_FIXTURE" "$project/src/one.dup"
  cat >"$project/package.json" <<'JSON'
{
  "name": "jscpd-config-fixture",
  "version": "1.0.0",
  "jscpd": {
    "path": ["src"],
    "minTokens": 50,
    "minLines": 5,
    "maxSize": "1mb",
    "reporters": ["json"],
    "silent": true,
    "noTips": true,
    "output": "report",
    "exitCode": 0,
    "formatsExts": {
      "typescript": ["dup"],
      "javascript": ["dup"]
    }
  }
}
JSON
}

make_invalid_package_project() {
  local project="$1"
  mkdir -p "$project"
  cp "$ROOT/jscpd/fixtures/clike/file2.c" "$project/file.c"
  cat >"$project/package.json" <<'JSON'
{ invalid json
JSON
}

make_formats_names_project() {
  local project="$1"
  mkdir -p "$project/src/a" "$project/src/b"
  cat >"$project/src/a/CustomScript" <<'EOF_JS'
function alpha() {
  const one = 1;
  const two = 2;
  return one + two;
}
EOF_JS
  cp "$project/src/a/CustomScript" "$project/src/b/CustomScript"
  cat >"$project/.jscpd.json" <<'JSON'
{
  "path": ["src"],
  "minTokens": 5,
  "minLines": 2,
  "maxSize": "1mb",
  "reporters": ["json"],
  "silent": true,
  "noTips": true,
  "output": "report",
  "formatsNames": {
    "javascript": ["CustomScript"]
  }
}
JSON
}

check_typescript_mapping() {
  local rust_report="$1"
  local upstream_report="$2"

  node --input-type=module - "$rust_report" "$upstream_report" <<'NODE'
import fs from 'node:fs';

const [rustPath, upstreamPath] = process.argv.slice(2);
for (const [label, reportPath] of [['rust', rustPath], ['upstream', upstreamPath]]) {
  const report = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
  const formats = Object.keys(report.statistics?.formats ?? {});
  if (!formats.includes('typescript')) {
    console.error(`${label} config formatsExts did not preserve first object mapping: ${formats.join(', ')}`);
    process.exit(1);
  }
  if (report.duplicates?.some((duplicate) => duplicate.format !== 'typescript')) {
    console.error(`${label} duplicate used a non-typescript format`);
    process.exit(1);
  }
}
NODE
}

check_javascript_only() {
  local rust_report="$1"
  local upstream_report="$2"

  node --input-type=module - "$rust_report" "$upstream_report" <<'NODE'
import fs from 'node:fs';

const [rustPath, upstreamPath] = process.argv.slice(2);
for (const [label, reportPath] of [['rust', rustPath], ['upstream', upstreamPath]]) {
  const report = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
  const formats = Object.keys(report.statistics?.formats ?? {});
  if (formats.length !== 1 || formats[0] !== 'javascript') {
    console.error(`${label} config formatsNames did not map extensionless files to javascript: ${formats.join(', ')}`);
    process.exit(1);
  }
  if (report.duplicates?.some((duplicate) => duplicate.format !== 'javascript')) {
    console.error(`${label} duplicate used a non-javascript format`);
    process.exit(1);
  }
}
NODE
}

compare_reports() {
  local rust_report="$1"
  local upstream_report="$2"

  STRICT="${STRICT:-coverage}" node "$ROOT/scripts/compare-reports.mjs" \
    "$rust_report" \
    "$upstream_report"
}

run_invalid_package_case() {
  local project="$1"
  local tool="$2"
  local stdout_file="$project/stdout.txt"
  local stderr_file="$project/stderr.txt"
  local code
  local cmd=()

  if [[ "$tool" == "rust" ]]; then
    cmd=("$ROOT/target/release/jscpd-rs")
  else
    cmd=(node "$ROOT/jscpd/apps/jscpd/bin/jscpd")
  fi

  set +e
  (
    cd "$project"
    "${cmd[@]}" file.c \
      --reporters json \
      --silent \
      --noTips \
      --min-tokens 20 \
      --min-lines 3 \
      --max-size 1mb \
      --exitCode 0
  ) >"$stdout_file" 2>"$stderr_file"
  code=$?
  set -e

  if [[ "$code" != "0" ]]; then
    printf '%s invalid package.json case exited with %s\n' "$tool" "$code" >&2
    sed -n '1,80p' "$stdout_file" >&2 || true
    sed -n '1,80p' "$stderr_file" >&2 || true
    return 1
  fi
  if ! grep -Fq "Warning: Could not parse" "$stderr_file"; then
    printf '%s invalid package.json case did not warn\n' "$tool" >&2
    sed -n '1,80p' "$stderr_file" >&2 || true
    return 1
  fi
  if ! grep -Fq "package.json" "$stderr_file"; then
    printf '%s invalid package.json warning did not name package.json\n' "$tool" >&2
    sed -n '1,80p' "$stderr_file" >&2 || true
    return 1
  fi
}

make_project "$RUST_PROJECT"
make_project "$UPSTREAM_PROJECT"
make_package_project "$PACKAGE_RUST_PROJECT"
make_package_project "$PACKAGE_UPSTREAM_PROJECT"
make_invalid_package_project "$INVALID_PACKAGE_RUST_PROJECT"
make_invalid_package_project "$INVALID_PACKAGE_UPSTREAM_PROJECT"
make_formats_names_project "$NAMES_RUST_PROJECT"
make_formats_names_project "$NAMES_UPSTREAM_PROJECT"

printf 'fixture: %s\n' "$TARGET_FIXTURE"
printf 'tmp: %s\n\n' "$TMP_ROOT"

(
  cd "$RUST_PROJECT"
  "$ROOT/target/release/jscpd-rs"
)
(
  cd "$UPSTREAM_PROJECT"
  node "$ROOT/jscpd/apps/jscpd/bin/jscpd"
)

check_typescript_mapping \
  "$RUST_PROJECT/report/jscpd-report.json" \
  "$UPSTREAM_PROJECT/report/jscpd-report.json"

compare_reports \
  "$RUST_PROJECT/report/jscpd-report.json" \
  "$UPSTREAM_PROJECT/report/jscpd-report.json"

printf '\npackage.json config fixture\n\n'

(
  cd "$PACKAGE_RUST_PROJECT"
  "$ROOT/target/release/jscpd-rs"
)
(
  cd "$PACKAGE_UPSTREAM_PROJECT"
  node "$ROOT/jscpd/apps/jscpd/bin/jscpd"
)

check_typescript_mapping \
  "$PACKAGE_RUST_PROJECT/report/jscpd-report.json" \
  "$PACKAGE_UPSTREAM_PROJECT/report/jscpd-report.json"

compare_reports \
  "$PACKAGE_RUST_PROJECT/report/jscpd-report.json" \
  "$PACKAGE_UPSTREAM_PROJECT/report/jscpd-report.json"

printf '\ninvalid package.json fixture\n\n'

run_invalid_package_case "$INVALID_PACKAGE_RUST_PROJECT" rust
run_invalid_package_case "$INVALID_PACKAGE_UPSTREAM_PROJECT" upstream

compare_reports \
  "$INVALID_PACKAGE_RUST_PROJECT/report/jscpd-report.json" \
  "$INVALID_PACKAGE_UPSTREAM_PROJECT/report/jscpd-report.json"

printf '\nformatsNames config fixture\n\n'

(
  cd "$NAMES_RUST_PROJECT"
  "$ROOT/target/release/jscpd-rs"
)
(
  cd "$NAMES_UPSTREAM_PROJECT"
  node "$ROOT/jscpd/apps/jscpd/bin/jscpd"
)

check_javascript_only \
  "$NAMES_RUST_PROJECT/report/jscpd-report.json" \
  "$NAMES_UPSTREAM_PROJECT/report/jscpd-report.json"

compare_reports \
  "$NAMES_RUST_PROJECT/report/jscpd-report.json" \
  "$NAMES_UPSTREAM_PROJECT/report/jscpd-report.json"

if [[ "${KEEP:-0}" == "1" ]]; then
  printf '\nrust project: %s\n' "$RUST_PROJECT"
  printf 'upstream project: %s\n' "$UPSTREAM_PROJECT"
  printf 'rust package project: %s\n' "$PACKAGE_RUST_PROJECT"
  printf 'upstream package project: %s\n' "$PACKAGE_UPSTREAM_PROJECT"
  printf 'rust invalid package project: %s\n' "$INVALID_PACKAGE_RUST_PROJECT"
  printf 'upstream invalid package project: %s\n' "$INVALID_PACKAGE_UPSTREAM_PROJECT"
  printf 'rust formatsNames project: %s\n' "$NAMES_RUST_PROJECT"
  printf 'upstream formatsNames project: %s\n' "$NAMES_UPSTREAM_PROJECT"
fi
