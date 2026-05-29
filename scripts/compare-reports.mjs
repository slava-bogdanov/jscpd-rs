#!/usr/bin/env node
import fs from 'node:fs';

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

console.log('');
console.log('first duplicates:');
for (let index = 0; index < Math.max(rustDuplicates.length, upstreamDuplicates.length, 5); index += 1) {
  if (index >= 5) break;
  const left = formatDuplicate(rustDuplicates[index]);
  const right = formatDuplicate(upstreamDuplicates[index]);
  console.log(`${String(index + 1).padStart(2, ' ')} rust=${left}`);
  console.log(`   upstream=${right}`);
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

function formatFile(file) {
  if (!file) return '<unknown>';
  const name = file.name ?? file.sourceId ?? file.source_id ?? '<unknown>';
  const start = file.start ?? file.startLoc?.line ?? file.start?.line ?? '?';
  const end = file.end ?? file.endLoc?.line ?? file.end?.line ?? '?';
  return `${name}:${start}-${end}`;
}
