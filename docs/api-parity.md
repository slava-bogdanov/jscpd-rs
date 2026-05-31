# API Parity

Last updated: 2026-05-31.

This document tracks the upstream JavaScript API surface against the native Rust
API. The first release remains native-only: the crate does not embed Node.js,
does not call JavaScript at runtime, and does not ship a JavaScript package
wrapper unless that is chosen as a separate release target.

## App-Level API

| Upstream API | Rust status | Notes |
| --- | --- | --- |
| `detectClones(opts, store?, statisticProvider?)` | covered | Use `detect_clones(&Options)` for path-based detection. Native `MemoryStore` and `Statistic` helpers are exposed, but custom provider injection is not part of the first-release API. |
| `detectClonesAndStatistic(opts, store?)` | covered | Use `detect_clones_and_statistic(&Options)`. `detect_clones_and_statistics(&Options)` remains the idiomatic Rust spelling. |
| `jscpd(argv, exitCallback?)` | covered natively | Use the `jscpd` binary for process behavior, `jscpd(args)` for embeddable argv execution, or `jscpd_with_exit_callback(args, callback)` for upstream-style duplicate exit callback semantics. Exact JavaScript package export shape is not implemented. |

## Core And Tokenizer Helpers

| Upstream API | Rust status | Notes |
| --- | --- | --- |
| `getDefaultOptions()` | covered | Use `get_default_options()`. Defaults are also available through `Options::default()`. |
| `getSupportedFormats()` | covered | Use `get_supported_formats()`. The registry is generated from upstream and currently has 223 formats. |
| `getFormatByFile(path, formatsExts?, formatsNames?)` | covered | Use `get_format_by_file(path)` for default mappings or `get_format_by_file_with_mappings(path, formats_exts, formats_names)` for explicit mappings. |
| `Tokenizer` class | covered natively | Use `Tokenizer::new()` or `Tokenizer::with_options(options)` and `generate_maps(source_id, content, format)`. Exact JavaScript package export shape is not implemented. |
| `Detector` class | covered natively | Use `Detector::new(options)` for stateful in-memory source detection or `detect_source_files(files, options)` for batch detection. Exact JavaScript constructor shape is not implemented. |
| `Statistic` class | covered natively | Use `Statistic::new()`, `match_source(...)`, `clone_found(...)`, and `get_statistic()` for upstream-style statistics accumulation. |
| `MemoryStore` class | covered natively | Use `MemoryStore<T>::new()`, `namespace(...)`, `get(...)`, `set(...)`, and `close()` for the native in-memory store concept. |
| Validators, subscribers, custom stores, custom reporters | option-surface only | CLI/config options are preserved where practical; dynamic npm loading is intentionally out of scope for the first release. |

## Server API

| Upstream API | Rust status | Notes |
| --- | --- | --- |
| `jscpd-server` binary | partial | Native binary covers help, startup, core HTTP routes, and MCP smoke contracts through `scripts/compat-server.sh`. Exact Streamable HTTP SDK edge cases remain follow-up. |
| `startServer`, `JscpdServer`, `JscpdServerService` exports | partial | Native server modules exist, but JavaScript export shape parity is not implemented. |

## Remaining API Gaps

- Decide later whether a JavaScript package wrapper is worth shipping. The
  current recommendation is to keep the first release native-only.
- Keep `jscpd(args)` / `jscpd_with_exit_callback(args, callback)` as the native
  embeddable path; add a JavaScript wrapper only if publishing an npm package
  becomes an explicit target.
- Keep `Tokenizer`, `Detector`, `Statistic`, and `MemoryStore` native and
  detection-oriented for now; add exact JavaScript iterator/object-shape
  wrappers only if an npm/API compatibility release is chosen.
- Keep custom store/reporter/provider APIs out of the release path until a real
  integration requires native hooks.
