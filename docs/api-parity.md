# API Parity

Last updated: 2026-05-31.

This document tracks the upstream JavaScript API surface against the native Rust
API. The first release remains native-only: the crate does not embed Node.js,
does not call JavaScript at runtime, and does not ship a JavaScript package
wrapper unless that is chosen as a separate release target.

## App-Level API

| Upstream API | Rust status | Notes |
| --- | --- | --- |
| `detectClones(opts, store?, statisticProvider?)` | covered | Use `detect_clones(&Options)` for path-based detection. Custom store/statistic providers are not exposed for the first release. |
| `detectClonesAndStatistic(opts, store?)` | covered | Use `detect_clones_and_statistic(&Options)`. `detect_clones_and_statistics(&Options)` remains the idiomatic Rust spelling. |
| `jscpd(argv, exitCallback?)` | partial | Use the `jscpd` binary for argv-compatible behavior. Native integrations can use `get_options_from_args(args)` plus `detect_clones*`; an embeddable runner with exit-callback semantics is not exposed yet. |

## Core And Tokenizer Helpers

| Upstream API | Rust status | Notes |
| --- | --- | --- |
| `getDefaultOptions()` | covered | Use `get_default_options()`. Defaults are also available through `Options::default()`. |
| `getSupportedFormats()` | covered | Use `get_supported_formats()`. The registry is generated from upstream and currently has 223 formats. |
| `getFormatByFile(path, formatsExts?, formatsNames?)` | covered | Use `get_format_by_file(path)` for default mappings or `get_format_by_file_with_mappings(path, formats_exts, formats_names)` for explicit mappings. |
| `Tokenizer`, `Detector`, `Statistic`, `MemoryStore` classes | internal/native | Equivalent functionality exists in native modules, but class-shaped API parity is not a first-release target. |
| Validators, subscribers, custom stores, custom reporters | option-surface only | CLI/config options are preserved where practical; dynamic npm loading is intentionally out of scope for the first release. |

## Server API

| Upstream API | Rust status | Notes |
| --- | --- | --- |
| `jscpd-server` binary | partial | Native binary covers help, startup, core HTTP routes, and MCP smoke contracts through `scripts/compat-server.sh`. Exact Streamable HTTP SDK edge cases remain follow-up. |
| `startServer`, `JscpdServer`, `JscpdServerService` exports | partial | Native server modules exist, but JavaScript export shape parity is not implemented. |

## Remaining API Gaps

- Decide later whether a JavaScript package wrapper is worth shipping. The
  current recommendation is to keep the first release native-only.
- Expose an embeddable argv runner if users need programmatic CLI execution
  with upstream-style `exitCallback` semantics instead of argv parsing plus the
  detector API.
- Keep custom store/reporter/provider APIs out of the release path until a real
  integration requires native hooks.
