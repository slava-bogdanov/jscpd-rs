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
- native blame support through Git.

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

The GitHub Actions workflow runs the fast gate on pushes and pull requests.
Manual workflow runs can enable the full compatibility matrix and public
benchmark suite before a release.

Latest recorded public benchmark baseline:

| Repo | Format | Rust avg | Upstream avg | Speedup |
| --- | --- | ---: | ---: | ---: |
| React | JavaScript | 0.184628s | 10.059620s | 54.49x |
| Next.js | TypeScript | 0.241178s | 14.238566s | 59.04x |
| Prometheus | Go | 0.075945s | 4.508015s | 59.36x |

See [docs/compat-baseline.md](docs/compat-baseline.md) for the current gate
baseline and [docs/release-decisions.md](docs/release-decisions.md) for approved
first-release compatibility decisions.

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
tracked in [docs/upstream-bugs.md](docs/upstream-bugs.md).
