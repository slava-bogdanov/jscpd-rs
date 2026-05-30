# Changelog

## 0.1.0 - 2026-05-31

First release candidate for `jscpd-rs`, a native Rust clone of upstream
`jscpd`.

### Added

- Native `jscpd` CLI binary with upstream-compatible command name and help
  shape.
- Native `jscpd-server` binary exposing `/`, `/api/health`, `/api/stats`,
  `/api/check`, `/api/recheck`, and `/mcp`.
- Coverage-first compatibility gates against the upstream `jscpd` submodule.
- CLI/config support for the main upstream option surface, including Commander
  edge cases covered by compatibility scripts.
- Native file discovery with `.gitignore`, global Git excludes, symlink policy,
  shebang detection, max size, max line, custom extension, and custom filename
  handling.
- Upstream-synchronized format registry with 223 formats and 206 extension
  mappings.
- Native Oxc-backed JavaScript, TypeScript, JSX, and TSX token processing.
- Native generic tokenization for long-tail formats, plus block handling for
  Markdown, markup, Vue, Svelte, Astro, Apex, and TAP where needed for current
  coverage gates.
- Built-in native reporters: `ai`, `console`, `consoleFull`, `csv`, `html`,
  `json`, `markdown`, `silent`, `sarif`, `threshold`, `xcode`, `xml`, and
  `badge`.
- Native `git blame -w` support in reports.
- Native Rust API for path-based detection and in-memory `SourceFile`
  detection.
- Public benchmark suite on pinned React, Next.js, and Prometheus revisions.

### Compatibility And Performance

The first release is intentionally coverage-first: Rust must not miss duplicated
upstream lines on the same inputs/options. Additional Rust findings are allowed
while compatibility converges and remain visible in comparison output.

Latest release-candidate public benchmark measurements from
`scripts/release-candidate.sh`:

| Case | Commit | Format | Rust avg | Upstream avg | Speedup | Compat |
| --- | --- | --- | ---: | ---: | ---: | --- |
| React | `f0dfee3` | JavaScript | 0.193660s | 9.879824s | 51.02x | pass |
| Next.js | `2bbb67b9` | TypeScript | 0.249000s | 14.349172s | 57.63x | pass |
| Prometheus | `a0524ee` | Go | 0.080205s | 4.576102s | 57.06x | pass |

### Known First-Release Deviations

- Dynamic npm reporters, stores, listeners, and plugins are not loaded.
- External reporter and store names keep upstream-style warning/fallback
  behavior where upstream continues.
- Exact clone pair ordering, token totals, and fragment boundaries remain
  diagnostic as long as upstream duplicated lines are covered.
- HTML output is self-contained and practically compatible, not pixel-perfect.
- The Rust crate exposes a native Rust API, not the upstream JavaScript package
  API.
- Full Prism grammar parity for every long-tail format is not attempted in this
  release. Formats should be promoted from generic tokenization when concrete
  coverage gates show missed upstream lines.
