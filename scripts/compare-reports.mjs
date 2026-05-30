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
const sourceLineCounts = new Map();
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
const rustFragments = rustDuplicates.flatMap(fragmentRecords);
const upstreamFragments = upstreamDuplicates.flatMap(fragmentRecords);
const rustStartKeys = new Set(rustDuplicates.map(startKey));
const upstreamStartKeys = new Set(upstreamDuplicates.map(startKey));
const rustFragmentKeys = new Set(rustFragments.map(fragmentKey));
const upstreamFragmentKeys = new Set(upstreamFragments.map(fragmentKey));
const commonStartKeys = [...rustStartKeys].filter((key) => upstreamStartKeys.has(key));
const commonFragmentKeys = [...rustFragmentKeys].filter((key) => upstreamFragmentKeys.has(key));
const coveredUpstreamFragments = upstreamFragments.filter((fragment) =>
  fragmentLineCoveredBy(fragment, rustFragments),
);
const coveredRustFragments = rustFragments.filter((fragment) =>
  fragmentLineCoveredBy(fragment, upstreamFragments),
);
const allowedMissingCoverageRanges = parseAllowedRanges(process.env.ALLOW_MISSING_COVERAGE);
const allMissingStartKeys = upstreamDuplicates
  .filter((duplicate) => !rustStartKeys.has(startKey(duplicate)));
const allExtraStartKeys = rustDuplicates
  .filter((duplicate) => !upstreamStartKeys.has(startKey(duplicate)));
const allMissingFragmentKeys = [...upstreamFragmentKeys]
  .filter((key) => !rustFragmentKeys.has(key));
const missingCoverageFragmentsRaw = upstreamFragments
  .filter((fragment) => !fragmentLineCoveredBy(fragment, rustFragments));
const ignoredMissingCoverageFragments = missingCoverageFragmentsRaw
  .filter((fragment) => allowedMissingCoverageRanges.has(fragmentRangeKey(fragment)));
const allMissingCoverageFragments = missingCoverageFragmentsRaw
  .filter((fragment) => !allowedMissingCoverageRanges.has(fragmentRangeKey(fragment)));
const missingStartKeys = allMissingStartKeys.slice(0, 10);
const extraStartKeys = allExtraStartKeys.slice(0, 10);
const missingFragmentKeys = allMissingFragmentKeys.slice(0, 10);
const missingCoverageFragments = allMissingCoverageFragments.slice(0, 10);

console.log('');
console.log(
  `clone start overlap: ${commonStartKeys.length}/${upstreamStartKeys.size} upstream, ${commonStartKeys.length}/${rustStartKeys.size} rust`,
);
console.log(
  `clone exact fragment overlap: ${commonFragmentKeys.length}/${upstreamFragmentKeys.size} upstream, ${commonFragmentKeys.length}/${rustFragmentKeys.size} rust`,
);
console.log(
  `clone line coverage: ${coveredUpstreamFragments.length}/${upstreamFragments.length} upstream fragments, ${coveredRustFragments.length}/${rustFragments.length} rust fragments`,
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
  console.log('missing exact upstream fragments:');
  for (const key of missingFragmentKeys) console.log(`  ${key}`);
}

if (missingCoverageFragments.length > 0) {
  console.log('');
  console.log('missing upstream line coverage:');
  for (const fragment of missingCoverageFragments) console.log(`  ${fragmentRangeKey(fragment)}`);
}

if (ignoredMissingCoverageFragments.length > 0) {
  console.log('');
  console.log('ignored upstream line coverage exceptions:');
  for (const fragment of ignoredMissingCoverageFragments) console.log(`  ${fragmentRangeKey(fragment)}`);
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

if (process.env.STRICT === 'clone-summary') {
  const failures = summaryFailures(['sources', 'lines', 'clones', 'duplicatedLines', 'percentage']);
  if (failures.length > 0) {
    console.error(`clone-summary comparison failed: ${failures.join(', ')}`);
    process.exit(1);
  }
}

if (process.env.STRICT === 'clone-count') {
  const failures = summaryFailures(['clones']);
  if (failures.length > 0) {
    console.error(`clone-count comparison failed: ${failures.join(', ')}`);
    process.exit(1);
  }
}

if (process.env.STRICT === '1') {
  const mismatches = summaryFailures(Object.keys(rustSummary));
  if (mismatches.length > 0) {
    console.error(`strict comparison failed: ${mismatches.join(', ')}`);
    process.exit(1);
  }
}

function coverageFailures() {
  const failures = [];
  if (allMissingCoverageFragments.length > 0) {
    failures.push(`missing upstream clone line coverage: ${allMissingCoverageFragments.length}`);
  }
  if (rustSummary.clones < upstreamSummary.clones) {
    failures.push(`rust clones ${rustSummary.clones} < upstream clones ${upstreamSummary.clones}`);
  }
  return failures;
}

function summaryFailures(keys) {
  return keys.filter((key) => Number(rustSummary[key]) !== Number(upstreamSummary[key]));
}

function parseAllowedRanges(value) {
  return new Set(
    (value ?? '')
      .split(/[,\n]/)
      .map((range) => range.trim())
      .filter(Boolean),
  );
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

function fragmentRecords(duplicate) {
  if (!duplicate) return [];
  const first = duplicate.firstFile ?? duplicate.duplicationA;
  const second = duplicate.secondFile ?? duplicate.duplicationB;
  const format = duplicate.format ?? 'unknown';
  return [fragmentRecord(format, first), fragmentRecord(format, second)].filter(Boolean);
}

function fragmentRecord(format, file) {
  if (!file) return null;
  const start = fileStartLine(file);
  let end = fileEndLine(file);
  if (!Number.isFinite(start) || !Number.isFinite(end)) return null;
  const lineCount = sourceLineCount(file.name ?? file.sourceId ?? file.source_id);
  if (Number.isFinite(lineCount) && lineCount > 0) {
    end = Math.min(end, lineCount);
  }
  return {
    format,
    name: file.name ?? file.sourceId ?? file.source_id ?? '<unknown>',
    start: Math.min(start, end),
    end: Math.max(start, end),
  };
}

function fragmentKey(fragment) {
  return `${fragment.format}:${fragment.name}:${fragment.start}`;
}

function fragmentRangeKey(fragment) {
  return `${fragment.format}:${fragment.name}:${fragment.start}-${fragment.end}`;
}

function fragmentLineCoveredBy(fragment, candidates) {
  let nextLine = fragment.start;
  const ranges = candidates
    .filter((candidate) => candidate.name === fragment.name)
    .sort((left, right) => left.start - right.start || left.end - right.end);

  for (const range of ranges) {
    if (range.end < nextLine) continue;
    if (range.start > nextLine) return false;
    nextLine = Math.max(nextLine, range.end + 1);
    if (nextLine > fragment.end) return true;
  }

  return nextLine > fragment.end;
}

function formatFragment(format, file) {
  return `${format}:${formatStart(file)}`;
}

function formatStart(file) {
  if (!file) return '<unknown>';
  const name = file.name ?? file.sourceId ?? file.source_id ?? '<unknown>';
  const start = fileStartLine(file);
  return `${name}:${start}`;
}

function formatFile(file) {
  if (!file) return '<unknown>';
  const name = file.name ?? file.sourceId ?? file.source_id ?? '<unknown>';
  const start = fileStartLine(file);
  const end = fileEndLine(file);
  return `${name}:${start}-${end}`;
}

function fileStartLine(file) {
  return lineNumber(file.start ?? file.startLoc?.line);
}

function fileEndLine(file) {
  return lineNumber(file.end ?? file.endLoc?.line);
}

function lineNumber(value) {
  if (typeof value === 'object' && value !== null) return number(value.line);
  return number(value);
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

function sourceLineCount(value) {
  if (!value) return Number.NaN;
  const resolved = path.resolve(value);
  if (sourceLineCounts.has(resolved)) return sourceLineCounts.get(resolved);
  let count = Number.NaN;
  try {
    const content = fs.readFileSync(resolved, 'utf8');
    if (content.length === 0) {
      count = 0;
    } else {
      const newlines = (content.match(/\n/g) ?? []).length;
      count = content.endsWith('\n') ? newlines : newlines + 1;
    }
  } catch {
    count = Number.NaN;
  }
  sourceLineCounts.set(resolved, count);
  return count;
}
