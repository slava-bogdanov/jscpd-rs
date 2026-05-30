#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="${TMP_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/jscpd-rs-blame.XXXXXX")}"
PROJECT="$TMP_ROOT/project"

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

mkdir -p "$PROJECT/src" "$PROJECT/rust" "$PROJECT/upstream"
cat >"$PROJECT/src/a.js" <<'EOF_JS'
function alpha() {
  const one = 1;
  const two = 2;
  const three = 3;
  return one + two + three;
}
EOF_JS
cp "$PROJECT/src/a.js" "$PROJECT/src/b.js"

(
  cd "$PROJECT"
  git init -q
  git config user.email test@example.com
  git config user.name "Test User"
  git add src/a.js src/b.js
  GIT_AUTHOR_DATE='2024-01-02T03:04:05+0000' \
    GIT_COMMITTER_DATE='2024-01-02T03:04:05+0000' \
    git commit -q -m initial
)

printf 'tmp: %s\n\n' "$TMP_ROOT"

(
  cd "$PROJECT"
  "$ROOT/target/release/jscpd-rs" src \
    --format javascript \
    --reporters json \
    --output rust \
    --silent \
    --noTips \
    --blame \
    --min-tokens 10 \
    --min-lines 3 \
    --max-size 1mb \
    --exitCode 0
)
(
  cd "$PROJECT"
  node "$ROOT/jscpd/apps/jscpd/bin/jscpd" src \
    --format javascript \
    --reporters json \
    --output upstream \
    --silent \
    --noTips \
    --blame \
    --min-tokens 10 \
    --min-lines 3 \
    --max-size 1mb \
    --exitCode 0
)

node --input-type=module - \
  "$PROJECT/rust/jscpd-report.json" \
  "$PROJECT/upstream/jscpd-report.json" <<'NODE'
import fs from 'node:fs';

const [rustPath, upstreamPath] = process.argv.slice(2);
const rust = JSON.parse(fs.readFileSync(rustPath, 'utf8'));
const upstream = JSON.parse(fs.readFileSync(upstreamPath, 'utf8'));

assert(rust.duplicates?.length === 1, `rust duplicate count ${rust.duplicates?.length}`);
assert(upstream.duplicates?.length === 1, `upstream duplicate count ${upstream.duplicates?.length}`);

for (const [label, report] of [['rust', rust], ['upstream', upstream]]) {
  const duplicate = report.duplicates[0];
  assert(duplicate.firstFile?.blame, `${label} missing firstFile blame`);
  assert(duplicate.secondFile?.blame, `${label} missing secondFile blame`);
  assertDeepEqual(
    Object.keys(duplicate.firstFile.blame),
    ['1', '2', '3', '4', '5', '6'],
    `${label} firstFile blame keys`,
  );
  assertDeepEqual(
    Object.keys(duplicate.secondFile.blame),
    ['1', '2', '3', '4', '5', '6'],
    `${label} secondFile blame keys`,
  );
}

for (const side of ['firstFile', 'secondFile']) {
  for (const line of ['1', '6']) {
    const rustLine = stableBlame(rust.duplicates[0][side].blame[line]);
    const upstreamLine = stableBlame(upstream.duplicates[0][side].blame[line]);
    assertDeepEqual(rustLine, upstreamLine, `${side} blame line ${line}`);
  }
}

function stableBlame(line) {
  return {
    author: line.author,
    date: line.date,
    line: line.line,
    rev: line.rev,
  };
}

function assert(condition, message) {
  if (!condition) {
    console.error(message);
    process.exit(1);
  }
}

function assertDeepEqual(actual, expected, label) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    console.error(`${label} mismatch`);
    console.error('actual:');
    console.error(JSON.stringify(actual, null, 2));
    console.error('expected:');
    console.error(JSON.stringify(expected, null, 2));
    process.exit(1);
  }
}
NODE

STRICT="${STRICT:-coverage}" node "$ROOT/scripts/compare-reports.mjs" \
  "$PROJECT/rust/jscpd-report.json" \
  "$PROJECT/upstream/jscpd-report.json"

if [[ "${KEEP:-0}" == "1" ]]; then
  printf '\nkept project: %s\n' "$PROJECT"
fi
