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

node --input-type=module - "$RUST_OUT" "$UPSTREAM_OUT" "$ROOT" <<'NODE'
import fs from 'node:fs';
import path from 'node:path';

const [rustDir, upstreamDir, rootDir] = process.argv.slice(2);
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

compareJsonReportMirrors(rustDir);
compareJsonReportMirrors(upstreamDir);
compareCsvReports(rustDir, upstreamDir);
compareMarkdownReports(rustDir, upstreamDir);
compareXmlReports(rustDir, upstreamDir);
compareSarifReports(rustDir, upstreamDir, rootDir);
compareBadgeReports(rustDir, upstreamDir);
compareHtmlReports(rustDir, upstreamDir);

function parseJson(label, dir, file) {
  const fullPath = path.join(dir, file);
  try {
    return JSON.parse(fs.readFileSync(fullPath, 'utf8'));
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

function compareJsonReportMirrors(dir) {
  const root = parseJson(path.basename(dir), dir, 'jscpd-report.json');
  const html = parseJson(path.basename(dir), dir, path.join('html', 'jscpd-report.json'));
  assertDeepEqual(
    html,
    root,
    `${path.basename(dir)} html/jscpd-report.json mirrors root JSON report`,
  );
}

function compareCsvReports(rustDir, upstreamDir) {
  const rust = parseCsv(path.join(rustDir, 'jscpd-report.csv'));
  const upstream = parseCsv(path.join(upstreamDir, 'jscpd-report.csv'));
  assertDeepEqual(
    stableTabularRows(rust),
    stableTabularRows(upstream),
    'CSV stable summary columns match upstream',
  );
}

function parseCsv(file) {
  const rows = fs.readFileSync(file, 'utf8').trimEnd().split(/\r?\n/).map((line) => line.split(','));
  const expectedHeader = [
    'Format',
    'Files analyzed',
    'Total lines',
    'Total tokens',
    'Clones found',
    'Duplicated lines',
    'Duplicated tokens',
  ];
  assertDeepEqual(rows[0], expectedHeader, `${file} CSV header`);
  return rows.slice(1);
}

function compareMarkdownReports(rustDir, upstreamDir) {
  const rust = parseMarkdownTable(path.join(rustDir, 'jscpd-report.md'));
  const upstream = parseMarkdownTable(path.join(upstreamDir, 'jscpd-report.md'));
  assertDeepEqual(
    stableTabularRows(rust),
    stableTabularRows(upstream),
    'Markdown stable summary columns match upstream',
  );
}

function parseMarkdownTable(file) {
  const rows = fs
    .readFileSync(file, 'utf8')
    .split(/\r?\n/)
    .filter((line) => line.startsWith('| '))
    .map((line) => line.split('|').slice(1, -1).map((cell) => cell.trim().replaceAll('**', '')));
  const header = rows[0];
  assertDeepEqual(
    header,
    [
      'Format',
      'Files analyzed',
      'Total lines',
      'Total tokens',
      'Clones found',
      'Duplicated lines',
      'Duplicated tokens',
    ],
    `${file} Markdown header`,
  );
  return rows.slice(2);
}

function stableTabularRows(rows) {
  return rows.map((row) => ({
    format: row[0],
    files: row[1],
    lines: row[2],
    clones: row[4],
    duplicatedLines: row[5],
  }));
}

function compareXmlReports(rustDir, upstreamDir) {
  const rust = normalizeNewlines(fs.readFileSync(path.join(rustDir, 'jscpd-report.xml'), 'utf8'));
  const upstream = normalizeNewlines(fs.readFileSync(path.join(upstreamDir, 'jscpd-report.xml'), 'utf8'));
  assertDeepEqual(rust, upstream, 'XML PMD CPD report matches upstream exactly');
}

function compareSarifReports(rustDir, upstreamDir, rootDir) {
  const rust = stableSarif(parseJson('rust', rustDir, 'jscpd-sarif.json'), rootDir);
  const upstream = stableSarif(parseJson('upstream', upstreamDir, 'jscpd-sarif.json'), rootDir);
  assertDeepEqual(rust, upstream, 'SARIF stable structure matches upstream');
}

function stableSarif(report, rootDir) {
  const run = report.runs?.[0] ?? {};
  const driver = run.tool?.driver ?? {};
  return {
    schema: report.$schema,
    version: report.version,
    tool: {
      name: driver.name,
      version: driver.version,
      informationUri: driver.informationUri,
      rules: (driver.rules ?? []).map((rule) => ({
        id: rule.id,
        text: rule.shortDescription?.text,
        helpUri: rule.helpUri,
      })),
    },
    artifacts: (run.artifacts ?? []).map((artifact) => ({
      sourceLanguage: artifact.sourceLanguage,
      uri: normalizeReportPath(artifact.location?.uri, rootDir),
    })),
    results: (run.results ?? []).map((result) => ({
      ruleId: result.ruleId,
      ruleIndex: result.ruleIndex,
      level: result.level,
      message: normalizeReportPath(result.message?.text, rootDir),
      locations: (result.locations ?? []).map((location) => ({
        uri: normalizeReportPath(location.physicalLocation?.artifactLocation?.uri, rootDir),
        region: location.physicalLocation?.region,
      })),
    })),
  };
}

function compareBadgeReports(rustDir, upstreamDir) {
  const rust = badgeContract(path.join(rustDir, 'jscpd-badge.svg'));
  const upstream = badgeContract(path.join(upstreamDir, 'jscpd-badge.svg'));
  assertDeepEqual(rust, upstream, 'badge title and aria contract match upstream');
}

function badgeContract(file) {
  const svg = fs.readFileSync(file, 'utf8');
  return {
    aria: extract(svg, /aria-label="([^"]+)"/, file),
    title: extract(svg, /<title>([^<]+)<\/title>/, file),
  };
}

function compareHtmlReports(rustDir, upstreamDir) {
  const rust = htmlContract(path.join(rustDir, 'html', 'index.html'));
  const upstream = htmlContract(path.join(upstreamDir, 'html', 'index.html'));
  assertDeepEqual(rust, upstream, 'HTML stable text contract matches upstream');
}

function htmlContract(file) {
  const html = fs.readFileSync(file, 'utf8');
  return {
    title: extract(html, /<title>([^<]+)<\/title>/, file),
    h1: extract(html, /<h1[^>]*>([^<]+)<\/h1>/, file),
    cloneText: extract(html, /(jscpd\/fixtures\/clike\/file2\.c \(Line 18:3 - Line 28:9\), jscpd\/fixtures\/clike\/file2\.c \(Line 8:3 - Line 18:8\))/, file),
    showCode: html.includes('Show code'),
    hideCode: html.includes('Hide code'),
  };
}

function extract(content, pattern, file) {
  const match = content.match(pattern);
  if (!match) {
    console.error(`${file} did not match ${pattern}`);
    process.exit(1);
  }
  return match[1];
}

function normalizeNewlines(value) {
  return value.replace(/\r\n/g, '\n');
}

function normalizeReportPath(value, rootDir) {
  return typeof value === 'string' ? value.replaceAll(`${rootDir}/`, '') : value;
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
  "$RUST_OUT/jscpd-report.json" \
  "$UPSTREAM_OUT/jscpd-report.json"

if [[ "${KEEP:-0}" == "1" ]]; then
  printf '\nrust report dir: %s\n' "$RUST_OUT"
  printf 'upstream report dir: %s\n' "$UPSTREAM_OUT"
fi
