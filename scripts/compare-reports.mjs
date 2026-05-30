#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const [rustReportPath, upstreamReportPath] = process.argv.slice(2);

if (!rustReportPath || !upstreamReportPath) {
  console.error('usage: compare-reports.mjs <rust-report.json> <upstream-report.json>');
  process.exit(2);
}

const rust = readReport(rustReportPath);
const upstream = readReport(upstreamReportPath);
const rustSummary = summarize(rust);
const upstreamSummary = summarize(upstream);

printMetric('sources', rustSummary.sources, upstreamSummary.sources);
printMetric('lines', rustSummary.lines, upstreamSummary.lines);
printMetric('tokens', rustSummary.tokens, upstreamSummary.tokens);
printMetric('clones', rustSummary.clones, upstreamSummary.clones);
printMetric('duplicatedLines', rustSummary.duplicatedLines, upstreamSummary.duplicatedLines);
printMetric('duplicatedTokens', rustSummary.duplicatedTokens, upstreamSummary.duplicatedTokens);
printMetric('percentage', rustSummary.percentage, upstreamSummary.percentage, 2);

const rustDuplicates = getDuplicates(rust);
const upstreamDuplicates = getDuplicates(upstream);
const rustStartKeys = new Set(rustDuplicates.map(startKey));
const upstreamStartKeys = new Set(upstreamDuplicates.map(startKey));
const rustFragmentKeys = new Set(rustDuplicates.flatMap(fragmentKeys));
const upstreamFragmentKeys = new Set(upstreamDuplicates.flatMap(fragmentKeys));
const commonStartKeys = [...rustStartKeys].filter((key) => upstreamStartKeys.has(key));
const commonFragmentKeys = [...rustFragmentKeys].filter((key) => upstreamFragmentKeys.has(key));
const allMissingStartKeys = upstreamDuplicates
  .filter((duplicate) => !rustStartKeys.has(startKey(duplicate)));
const allExtraStartKeys = rustDuplicates
  .filter((duplicate) => !upstreamStartKeys.has(startKey(duplicate)));
const allMissingFragmentKeys = [...upstreamFragmentKeys]
  .filter((key) => !rustFragmentKeys.has(key));
const missingStartKeys = allMissingStartKeys.slice(0, 10);
const extraStartKeys = allExtraStartKeys.slice(0, 10);
const missingFragmentKeys = allMissingFragmentKeys.slice(0, 10);

console.log('');
console.log(
  `clone start overlap: ${commonStartKeys.length}/${upstreamStartKeys.size} upstream, ${commonStartKeys.length}/${rustStartKeys.size} rust`,
);
console.log(
  `clone fragment coverage: ${commonFragmentKeys.length}/${upstreamFragmentKeys.size} upstream, ${commonFragmentKeys.length}/${rustFragmentKeys.size} rust`,
);

console.log('');
console.log('first duplicates:');
for (let index = 0; index < Math.max(rustDuplicates.length, upstreamDuplicates.length, 5); index += 1) {
  if (index >= 5) break;
  const left = formatDuplicate(rustDuplicates[index]);
  const right = formatDuplicate(upstreamDuplicates[index]);
  console.log(`${String(index + 1).padStart(2, ' ')} rust=${left}`);
  console.log(`   upstream=${right}`);
}

if (missingStartKeys.length > 0) {
  console.log('');
  console.log('missing upstream starts:');
  for (const duplicate of missingStartKeys) console.log(`  ${startKey(duplicate)}`);
}

if (extraStartKeys.length > 0) {
  console.log('');
  console.log('extra rust starts:');
  for (const duplicate of extraStartKeys) console.log(`  ${startKey(duplicate)}`);
}

if (missingFragmentKeys.length > 0) {
  console.log('');
  console.log('missing upstream fragments:');
  for (const key of missingFragmentKeys) console.log(`  ${key}`);
}

if (process.env.DETAILS === '1') {
  printDuplicateDetails('missing upstream details', missingStartKeys);
  printDuplicateDetails('extra rust details', extraStartKeys);
}

if (process.env.SOURCE_DELTAS) {
  const limit = Number(process.env.SOURCE_DELTAS) || 20;
  printSourceDeltas(rust, upstream, limit);
}

if (process.env.STRICT === 'coverage' || process.env.STRICT === 'coverage-first') {
  const failures = coverageFailures();
  if (failures.length > 0) {
    console.error(`coverage comparison failed: ${failures.join(', ')}`);
    process.exit(1);
  }
}

if (process.env.STRICT === '1') {
  const mismatches = Object.entries(rustSummary)
    .filter(([key, value]) => Number(value) !== Number(upstreamSummary[key]))
    .map(([key]) => key);
  if (mismatches.length > 0) {
    console.error(`strict comparison failed: ${mismatches.join(', ')}`);
    process.exit(1);
  }
}

function coverageFailures() {
  const failures = [];
  if (allMissingFragmentKeys.length > 0) {
    failures.push(`missing upstream clone fragments: ${allMissingFragmentKeys.length}`);
  }
  if (rustSummary.clones < upstreamSummary.clones) {
    failures.push(`rust clones ${rustSummary.clones} < upstream clones ${upstreamSummary.clones}`);
  }
  if (rustSummary.sources < upstreamSummary.sources) {
    failures.push(`rust sources ${rustSummary.sources} < upstream sources ${upstreamSummary.sources}`);
  }
  if (rustSummary.lines < upstreamSummary.lines) {
    failures.push(`rust lines ${rustSummary.lines} < upstream lines ${upstreamSummary.lines}`);
  }
  return failures;
}

function readReport(path) {
  return JSON.parse(fs.readFileSync(path, 'utf8'));
}

function summarize(report) {
  const total = report.statistics?.total ?? {};
  const duplicates = getDuplicates(report);
  return {
    sources: number(total.sources ?? total.files ?? report.sources?.length),
    lines: number(total.lines),
    tokens: number(total.tokens),
    clones: number(total.clones ?? duplicates.length),
    duplicatedLines: number(total.duplicatedLines ?? total.duplicated_lines),
    duplicatedTokens: number(total.duplicatedTokens ?? total.duplicated_tokens),
    percentage: number(total.percentage),
  };
}

function getDuplicates(report) {
  return report.duplicates ?? report.clones ?? [];
}

function number(value) {
  return Number.isFinite(Number(value)) ? Number(value) : 0;
}

function printMetric(name, rustValue, upstreamValue, digits = 0) {
  const delta = rustValue - upstreamValue;
  const ratio = upstreamValue === 0 ? 'n/a' : `${(rustValue / upstreamValue).toFixed(3)}x`;
  const left = formatNumber(rustValue, digits).padStart(12, ' ');
  const right = formatNumber(upstreamValue, digits).padStart(12, ' ');
  const diff = formatSigned(delta, digits).padStart(12, ' ');
  console.log(`${name.padEnd(16, ' ')} rust=${left} upstream=${right} delta=${diff} ratio=${ratio}`);
}

function formatNumber(value, digits) {
  return digits === 0 ? String(Math.round(value)) : value.toFixed(digits);
}

function formatSigned(value, digits) {
  const prefix = value > 0 ? '+' : '';
  return `${prefix}${formatNumber(value, digits)}`;
}

function formatDuplicate(duplicate) {
  if (!duplicate) return '<none>';
  const first = duplicate.firstFile ?? duplicate.duplicationA;
  const second = duplicate.secondFile ?? duplicate.duplicationB;
  const format = duplicate.format ?? 'unknown';
  const tokens = number(duplicate.tokens);
  const lines = number(duplicate.lines);
  return `${format}:${tokens}t/${lines}l ${formatFile(first)} -> ${formatFile(second)}`;
}

function startKey(duplicate) {
  if (!duplicate) return '<none>';
  const first = duplicate.firstFile ?? duplicate.duplicationA;
  const second = duplicate.secondFile ?? duplicate.duplicationB;
  const format = duplicate.format ?? 'unknown';
  return `${format}:` + [formatStart(first), formatStart(second)].sort().join(' <> ');
}

function fragmentKeys(duplicate) {
  if (!duplicate) return [];
  const first = duplicate.firstFile ?? duplicate.duplicationA;
  const second = duplicate.secondFile ?? duplicate.duplicationB;
  const format = duplicate.format ?? 'unknown';
  return [formatFragment(format, first), formatFragment(format, second)];
}

function formatFragment(format, file) {
  return `${format}:${formatStart(file)}`;
}

function formatStart(file) {
  if (!file) return '<unknown>';
  const name = file.name ?? file.sourceId ?? file.source_id ?? '<unknown>';
  const start = file.start ?? file.startLoc?.line ?? file.start?.line ?? '?';
  return `${name}:${start}`;
}

function formatFile(file) {
  if (!file) return '<unknown>';
  const name = file.name ?? file.sourceId ?? file.source_id ?? '<unknown>';
  const start = file.start ?? file.startLoc?.line ?? file.start?.line ?? '?';
  const end = file.end ?? file.endLoc?.line ?? file.end?.line ?? '?';
  return `${name}:${start}-${end}`;
}

function printDuplicateDetails(title, duplicates) {
  if (duplicates.length === 0) return;
  console.log('');
  console.log(`${title}:`);
  for (const duplicate of duplicates) console.log(`  ${detailKey(duplicate)}`);
}

function detailKey(duplicate) {
  const first = duplicate.firstFile ?? duplicate.duplicationA;
  const second = duplicate.secondFile ?? duplicate.duplicationB;
  return [formatDetailedStart(first), formatDetailedStart(second)].sort().join(' <> ');
}

function formatDetailedStart(file) {
  if (!file) return '<unknown>';
  const name = normalizePath(file.name ?? file.sourceId ?? file.source_id ?? '<unknown>');
  const loc = file.startLoc?.line
    ? file.startLoc
    : typeof file.start === 'object'
      ? file.start
      : { line: file.start ?? '?', column: file.startLoc?.column ?? '?' };
  return `${name}:${loc.line}:${loc.column ?? '?'}`;
}

function printSourceDeltas(rustReport, upstreamReport, limit) {
  const rustSources = collectSources(rustReport);
  const upstreamSources = collectSources(upstreamReport);
  const keys = new Set([...rustSources.keys(), ...upstreamSources.keys()]);
  const rows = [...keys]
    .map((key) => {
      const left = rustSources.get(key) ?? {};
      const right = upstreamSources.get(key) ?? {};
      return {
        key,
        rustTokens: number(left.tokens),
        upstreamTokens: number(right.tokens),
        rustLines: number(left.lines),
        upstreamLines: number(right.lines),
      };
    })
    .map((row) => ({
      ...row,
      tokenDelta: row.rustTokens - row.upstreamTokens,
      lineDelta: row.rustLines - row.upstreamLines,
    }))
    .sort((a, b) => Math.abs(b.tokenDelta) - Math.abs(a.tokenDelta))
    .slice(0, limit);

  console.log('');
  console.log(`top source token deltas (${rows.length}):`);
  for (const row of rows) {
    console.log(
      `  ${formatSigned(row.tokenDelta, 0).padStart(9, ' ')} tokens ` +
        `rust=${String(row.rustTokens).padStart(9, ' ')} ` +
        `upstream=${String(row.upstreamTokens).padStart(9, ' ')} ` +
        `lines_delta=${formatSigned(row.lineDelta, 0).padStart(5, ' ')} ` +
        row.key,
    );
  }
}

function collectSources(report) {
  const rows = new Map();
  for (const [formatName, format] of Object.entries(report.statistics?.formats ?? {})) {
    for (const [source, stats] of Object.entries(format.sources ?? {})) {
      rows.set(`${formatName}:${normalizePath(source)}`, stats);
    }
  }
  return rows;
}

function normalizePath(value) {
  return path.relative(process.cwd(), path.resolve(value));
}
