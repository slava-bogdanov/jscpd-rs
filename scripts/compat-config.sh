#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_FIXTURE="${TARGET_FIXTURE:-$ROOT/jscpd/fixtures/one-file/one-file.js}"
TMP_ROOT="${TMP_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/jscpd-rs-config.XXXXXX")}"
RUST_PROJECT="$TMP_ROOT/rust"
UPSTREAM_PROJECT="$TMP_ROOT/upstream"

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

make_project "$RUST_PROJECT"
make_project "$UPSTREAM_PROJECT"

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

node --input-type=module - "$RUST_PROJECT/report/jscpd-report.json" "$UPSTREAM_PROJECT/report/jscpd-report.json" <<'NODE'
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

STRICT="${STRICT:-coverage}" node "$ROOT/scripts/compare-reports.mjs" \
  "$RUST_PROJECT/report/jscpd-report.json" \
  "$UPSTREAM_PROJECT/report/jscpd-report.json"

if [[ "${KEEP:-0}" == "1" ]]; then
  printf '\nrust project: %s\n' "$RUST_PROJECT"
  printf 'upstream project: %s\n' "$UPSTREAM_PROJECT"
fi
