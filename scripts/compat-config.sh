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
EXPLICIT_RUST_PROJECT="$TMP_ROOT/rust-explicit-config"
EXPLICIT_UPSTREAM_PROJECT="$TMP_ROOT/upstream-explicit-config"
BADGE_RUST_PROJECT="$TMP_ROOT/rust-badge-options"
BADGE_UPSTREAM_PROJECT="$TMP_ROOT/upstream-badge-options"
OPTIONS_RUST_PROJECT="$TMP_ROOT/rust-option-surface"
OPTIONS_UPSTREAM_PROJECT="$TMP_ROOT/upstream-option-surface"

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

make_explicit_config_project() {
  local project="$1"
  mkdir -p "$project/configs/src"
  cp "$TARGET_FIXTURE" "$project/configs/src/one.dup"
  cat >"$project/configs/jscpd.custom.json" <<'JSON'
{
  "path": ["src"],
  "minTokens": 50,
  "minLines": 5,
  "maxSize": "1mb",
  "reporters": ["json"],
  "silent": true,
  "noTips": true,
  "output": "explicit-report",
  "formatsExts": {
    "typescript": ["dup"],
    "javascript": ["dup"]
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

make_badge_options_project() {
  local project="$1"
  mkdir -p "$project/src"
  cp "$TARGET_FIXTURE" "$project/src/one.dup"
  cat >"$project/.jscpd.json" <<'JSON'
{
  "path": ["src"],
  "minTokens": 50,
  "minLines": 5,
  "maxSize": "1mb",
  "reporters": ["json", "badge"],
  "silent": true,
  "noTips": true,
  "output": "report",
  "formatsExts": {
    "typescript": ["dup"],
    "javascript": ["dup"]
  },
  "reportersOptions": {
    "badge": {
      "subject": "Duplicates",
      "status": "blocked",
      "color": "purple",
      "path": "custom-badge.svg"
    }
  }
}
JSON
}

make_option_surface_project() {
  local project="$1"
  mkdir -p "$project/src"
  cp "$TARGET_FIXTURE" "$project/src/one.dup"
  cat >"$project/.jscpd.json" <<'JSON'
{
  "path": ["src"],
  "minTokens": 50,
  "minLines": 5,
  "maxSize": "1mb",
  "reporters": ["console"],
  "debug": true,
  "cache": false,
  "listeners": ["console"],
  "tokensToSkip": ["comment", "block-comment"],
  "noTips": true,
  "formatsExts": {
    "typescript": ["dup"],
    "javascript": ["dup"]
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

check_badge_options() {
  local rust_project="$1"
  local upstream_project="$2"

  node --input-type=module - "$rust_project" "$upstream_project" <<'NODE'
import fs from 'node:fs';
import path from 'node:path';

const [rustProject, upstreamProject] = process.argv.slice(2);
for (const [label, project] of [['rust', rustProject], ['upstream', upstreamProject]]) {
  const customBadge = path.join(project, 'custom-badge.svg');
  const defaultBadge = path.join(project, 'report', 'jscpd-badge.svg');
  if (!fs.existsSync(customBadge)) {
    console.error(`${label} did not write reportersOptions.badge.path`);
    process.exit(1);
  }
  if (fs.existsSync(defaultBadge)) {
    console.error(`${label} wrote default badge path despite reportersOptions.badge.path`);
    process.exit(1);
  }
  const svg = fs.readFileSync(customBadge, 'utf8');
  if (!svg.includes('<title>Duplicates: blocked</title>')) {
    console.error(`${label} badge title did not use subject/status options`);
    process.exit(1);
  }
  if (!svg.includes('aria-label="Duplicates: blocked"')) {
    console.error(`${label} badge aria label did not use subject/status options`);
    process.exit(1);
  }
  if (!svg.includes('fill="#94E"')) {
    console.error(`${label} badge did not use color option`);
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
    cmd=("$ROOT/target/release/jscpd")
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

run_option_surface_debug_case() {
  local project="$1"
  local tool="$2"
  local stdout_file="$project/stdout.txt"
  local stderr_file="$project/stderr.txt"
  local code
  local cmd=()

  if [[ "$tool" == "rust" ]]; then
    cmd=("$ROOT/target/release/jscpd")
  else
    cmd=(node "$ROOT/jscpd/apps/jscpd/bin/jscpd")
  fi

  set +e
  (
    cd "$project"
    "${cmd[@]}"
  ) >"$stdout_file" 2>"$stderr_file"
  code=$?
  set -e

  if [[ "$code" != "0" ]]; then
    printf '%s option-surface debug case exited with %s\n' "$tool" "$code" >&2
    sed -n '1,120p' "$stdout_file" >&2 || true
    sed -n '1,80p' "$stderr_file" >&2 || true
    return 1
  fi
  for expected in \
    "Options:" \
    "cache: false" \
    "listeners: [ 'console' ]" \
    "tokensToSkip: [ 'comment', 'block-comment' ]" \
    "config: '$project/.jscpd.json'" \
    "Found 1 files to detect."
  do
    if ! grep -Fq "$expected" "$stdout_file"; then
      printf '%s option-surface debug output missing: %s\n' "$tool" "$expected" >&2
      sed -n '1,160p' "$stdout_file" >&2 || true
      return 1
    fi
  done
}

make_project "$RUST_PROJECT"
make_project "$UPSTREAM_PROJECT"
make_package_project "$PACKAGE_RUST_PROJECT"
make_package_project "$PACKAGE_UPSTREAM_PROJECT"
make_explicit_config_project "$EXPLICIT_RUST_PROJECT"
make_explicit_config_project "$EXPLICIT_UPSTREAM_PROJECT"
make_invalid_package_project "$INVALID_PACKAGE_RUST_PROJECT"
make_invalid_package_project "$INVALID_PACKAGE_UPSTREAM_PROJECT"
make_formats_names_project "$NAMES_RUST_PROJECT"
make_formats_names_project "$NAMES_UPSTREAM_PROJECT"
make_badge_options_project "$BADGE_RUST_PROJECT"
make_badge_options_project "$BADGE_UPSTREAM_PROJECT"
make_option_surface_project "$OPTIONS_RUST_PROJECT"
make_option_surface_project "$OPTIONS_UPSTREAM_PROJECT"

printf 'fixture: %s\n' "$TARGET_FIXTURE"
printf 'tmp: %s\n\n' "$TMP_ROOT"

(
  cd "$RUST_PROJECT"
  "$ROOT/target/release/jscpd"
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
  "$ROOT/target/release/jscpd"
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

printf '\nexplicit --config fixture\n\n'

(
  cd "$EXPLICIT_RUST_PROJECT"
  "$ROOT/target/release/jscpd" --config configs/jscpd.custom.json
)
(
  cd "$EXPLICIT_UPSTREAM_PROJECT"
  node "$ROOT/jscpd/apps/jscpd/bin/jscpd" --config configs/jscpd.custom.json
)

check_typescript_mapping \
  "$EXPLICIT_RUST_PROJECT/explicit-report/jscpd-report.json" \
  "$EXPLICIT_UPSTREAM_PROJECT/explicit-report/jscpd-report.json"

compare_reports \
  "$EXPLICIT_RUST_PROJECT/explicit-report/jscpd-report.json" \
  "$EXPLICIT_UPSTREAM_PROJECT/explicit-report/jscpd-report.json"

printf '\ninvalid package.json fixture\n\n'

run_invalid_package_case "$INVALID_PACKAGE_RUST_PROJECT" rust
run_invalid_package_case "$INVALID_PACKAGE_UPSTREAM_PROJECT" upstream

compare_reports \
  "$INVALID_PACKAGE_RUST_PROJECT/report/jscpd-report.json" \
  "$INVALID_PACKAGE_UPSTREAM_PROJECT/report/jscpd-report.json"

printf '\nformatsNames config fixture\n\n'

(
  cd "$NAMES_RUST_PROJECT"
  "$ROOT/target/release/jscpd"
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

printf '\nbadge reportersOptions config fixture\n\n'

(
  cd "$BADGE_RUST_PROJECT"
  "$ROOT/target/release/jscpd"
)
(
  cd "$BADGE_UPSTREAM_PROJECT"
  node "$ROOT/jscpd/apps/jscpd/bin/jscpd"
)

check_badge_options \
  "$BADGE_RUST_PROJECT" \
  "$BADGE_UPSTREAM_PROJECT"

check_typescript_mapping \
  "$BADGE_RUST_PROJECT/report/jscpd-report.json" \
  "$BADGE_UPSTREAM_PROJECT/report/jscpd-report.json"

compare_reports \
  "$BADGE_RUST_PROJECT/report/jscpd-report.json" \
  "$BADGE_UPSTREAM_PROJECT/report/jscpd-report.json"

printf '\noption-surface debug config fixture\n\n'

run_option_surface_debug_case "$OPTIONS_RUST_PROJECT" rust
run_option_surface_debug_case "$OPTIONS_UPSTREAM_PROJECT" upstream

if [[ "${KEEP:-0}" == "1" ]]; then
  printf '\nrust project: %s\n' "$RUST_PROJECT"
  printf 'upstream project: %s\n' "$UPSTREAM_PROJECT"
  printf 'rust package project: %s\n' "$PACKAGE_RUST_PROJECT"
  printf 'upstream package project: %s\n' "$PACKAGE_UPSTREAM_PROJECT"
  printf 'rust explicit config project: %s\n' "$EXPLICIT_RUST_PROJECT"
  printf 'upstream explicit config project: %s\n' "$EXPLICIT_UPSTREAM_PROJECT"
  printf 'rust invalid package project: %s\n' "$INVALID_PACKAGE_RUST_PROJECT"
  printf 'upstream invalid package project: %s\n' "$INVALID_PACKAGE_UPSTREAM_PROJECT"
  printf 'rust formatsNames project: %s\n' "$NAMES_RUST_PROJECT"
  printf 'upstream formatsNames project: %s\n' "$NAMES_UPSTREAM_PROJECT"
  printf 'rust badge options project: %s\n' "$BADGE_RUST_PROJECT"
  printf 'upstream badge options project: %s\n' "$BADGE_UPSTREAM_PROJECT"
  printf 'rust option-surface project: %s\n' "$OPTIONS_RUST_PROJECT"
  printf 'upstream option-surface project: %s\n' "$OPTIONS_UPSTREAM_PROJECT"
fi
