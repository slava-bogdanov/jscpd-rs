# jscpd-rs

Fast native Rust clone of [`jscpd`](https://github.com/kucherenko/jscpd).

The project goal is practical upstream compatibility with much lower runtime
cost: the same CLI/config/reporting workflows should work, while the detector
stays native Rust and does not embed or spawn JavaScript for core behavior.

## Status

This is pre-release software. The first release target is a coverage-first
compatible CLI replacement for common `jscpd` workflows:

- command-line and config compatibility for the upstream option surface;
- coverage-first duplicate compatibility: Rust must not miss duplicated lines
  reported by upstream on the same inputs/options;
- native built-in reporters: `ai`, `console`, `consoleFull`, `csv`, `html`,
  `json`, `markdown`, `silent`, `sarif`, `threshold`, `xcode`, `xml`, and
  `badge`;
- upstream-synchronized format registry with native JS/TS/JSX/TSX tokenization
  and generic native tokenization for long-tail formats;
- native blame support through Git;
- initial native Rust library API for running detection from paths or prepared
  in-memory sources.

Dynamic npm reporters, stores, listeners, and plugins are intentionally out of
scope for the first release. Unknown external reporters/stores keep
upstream-style warnings and continue where upstream continues.

## Install

From this repository:

```bash
cargo install --path . --bin jscpd --locked
```

Build without installing:

```bash
cargo build --release --bin jscpd
cargo build --release --bin jscpd-server
```

## Usage

```bash
jscpd /path/to/source
jscpd --format typescript --min-tokens 50 --min-lines 5 src
jscpd --reporters json,html --output report src
jscpd --threshold 5 --exitCode 1 src
```

The CLI intentionally uses the upstream command name and help shape:

```bash
jscpd --help
jscpd --list
```

Start the native REST server:

```bash
jscpd-server /path/to/source --host 127.0.0.1 --port 3000
curl http://127.0.0.1:3000/api/health
```

The current server exposes `/`, `/api/health`, `/api/stats`, `/api/check`,
`/api/recheck`, and `/mcp`. The MCP endpoint supports the upstream server's core
tools and statistics resource over native JSON-RPC HTTP.

## Library API

The crate exposes the detector core for native integrations:

```rust
let options = jscpd_rs::cli::Options::default();
let result = jscpd_rs::detect_clones_and_statistics(&options)?;
let clones = result.clones;
```

`detect_source_files` accepts in-memory `SourceFile` values, which is the
foundation for the upstream-style snippet/server workflow.

## Compatibility Gates

Fast local gate:

```bash
scripts/release-gate.sh
```

Package/install gate:

```bash
scripts/package-check.sh
```

Full compatibility matrix:

```bash
FULL=1 scripts/release-gate.sh
```

Public benchmark and coverage gate:

```bash
PUBLIC=1 PUBLIC_RUNS=3 scripts/release-gate.sh
```

Release candidate gate:

```bash
scripts/release-candidate.sh
```

The GitHub Actions workflow runs the fast gate on pushes and pull requests.
Manual workflow runs can enable the full compatibility matrix and public
benchmark suite before a release, or set `release_candidate=true` to run the
full release-candidate gate in CI.

Latest recorded public benchmark baseline:

| Repo | Format | Rust avg | Upstream avg | Speedup |
| --- | --- | ---: | ---: | ---: |
| React | JavaScript | 0.186378s | 10.012629s | 53.72x |
| Next.js | TypeScript | 0.240852s | 14.316217s | 59.44x |
| Prometheus | Go | 0.078731s | 4.542469s | 57.70x |

See [docs/compat-baseline.md](docs/compat-baseline.md) for the current gate
baseline, [docs/release-readiness.md](docs/release-readiness.md) for component
status, [docs/release-checklist.md](docs/release-checklist.md) for the
publication checklist, and [docs/release-decisions.md](docs/release-decisions.md)
for approved first-release compatibility decisions.

## Known First-Release Deviations

The first release is native-only and coverage-first. These differences from the
JavaScript package are intentional unless a real workflow proves otherwise:

- dynamic npm reporters, stores, listeners, and plugins are not loaded;
- token totals and exact clone pair ordering may differ from Prism-based
  upstream reports while duplicated upstream lines remain covered;
- HTML reports are self-contained and practically compatible, not pixel-perfect;
- the Rust crate exposes a native API, not the upstream JavaScript package API.

## Development

The upstream repository is checked out as the `jscpd/` git submodule and is the
executable specification for behavior.

```bash
git submodule update --init --recursive
cargo test
```

Useful focused checks:

```bash
scripts/compat-cli.sh
scripts/compat-config.sh
scripts/compat-reporters.sh
STRICT=coverage scripts/compat-matrix.sh
```

Known upstream bug candidates and intentional compatibility exceptions are
tracked in [docs/upstream-bugs.md](docs/upstream-bugs.md). GitHub-ready issue
drafts are prepared in
[docs/upstream-issue-drafts.md](docs/upstream-issue-drafts.md).
