# Release Readiness

Last updated: 2026-05-31.

This is the working component checklist for the first release. The authoritative
policy decisions are still in `docs/release-decisions.md`; this file tracks the
current implementation status.

## Ready For First Release

| Component | Status | Notes |
| --- | --- | --- |
| Binary/package surface | ready | `jscpd` binary name, Cargo package include list, install check, version/help smoke checks. |
| CLI option surface | ready | Main upstream flags are parsed, including visible Commander quirks gated by `scripts/compat-cli.sh`. |
| Config loading | ready | `.jscpd.json` and `package.json#jscpd`, config-relative paths/ignore, malformed JSON behavior, symlinked explicit config paths. |
| File discovery | ready | Format filters, custom extensions/names, `.gitignore`, global Git excludes, symlink policy, shebang detection, max size/line filtering. |
| Format registry | ready | Generated from upstream tokenizer build; current registry has 223 formats and 206 extension mappings. |
| Detector core | ready | Numeric hashes, compact token streams, per-format sharding, parallel preparation/detection, coverage-first comparator. |
| Hot JS/TS tokenization | ready | Native Oxc-backed paths for JavaScript, TypeScript, JSX, and TSX with coverage gates. |
| Embedded/block formats | ready | Markdown, markup, Vue, Svelte, Astro, Apex, and TAP have native block handling where needed for upstream coverage. |
| Built-in reporters | ready | `ai`, `console`, `consoleFull`, `csv`, `html`, `json`, `markdown`, `silent`, `sarif`, `threshold`, `xcode`, `xml`, and `badge`. |
| Blame | ready | Native `git blame -w` data is populated and gated by `scripts/compat-blame.sh`. |
| Performance harness | ready | Local benchmark script and public benchmark suite with pinned output recording and speedup gates. |
| Release gates | ready | Default CI gate, full compatibility matrix, package check, reporter/config/CLI/blame gates. |

## Partial Or Follow-Up

| Component | Status | Recommended action |
| --- | --- | --- |
| Long-tail tokenization | coverage-first | Keep generic tokenization by default. Promote formats only when fixtures or public repos show missed upstream coverage. |
| Exact pair parity | diagnostic | Do not block release while every upstream duplicated line is covered. Reduce noisy extras after user-facing reports become annoying. |
| Token totals | diagnostic | Native token streams may differ from Prism. Keep report-visible clone coverage as the gate. |
| HTML reporter polish | practical parity | Keep self-contained HTML stable. Do not chase pixel-perfect upstream parity for the first release. |
| Terminal cosmetics | practical parity | Important messages are gated; exact wrapping/order remains lower priority. |
| Latest full publication gate | pending | `FULL=1 PUBLIC=0 scripts/release-gate.sh` passed on `bd7fdc8`; rerun `scripts/release-candidate.sh` before publishing. |

## Post-MVP

| Component | Decision |
| --- | --- |
| Dynamic npm reporters | Do not implement for the first release; keep upstream-style missing-package warnings. |
| Dynamic npm stores | Do not implement for the first release; default in-memory store is the release path. |
| Listeners/plugins runtime | Option-surface compatibility only unless a real workflow requires native support. |
| Persistent cache/store backends | Add only if public benchmark data proves the in-memory path is insufficient. |
| Full Prism grammar port | Do not rewrite all grammars eagerly; use native crates or small scanners only for proven gaps. |

