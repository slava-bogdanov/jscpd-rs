# Release Checklist

This checklist is the publication runbook for the first Rust release candidate.
Policy decisions live in `docs/release-decisions.md`; current evidence lives in
`docs/compat-baseline.md` and `docs/release-readiness.md`.

## Current Release Candidate Evidence

Latest full local release-candidate gate:

```bash
scripts/release-candidate.sh
```

Passed on 2026-05-31 at code commit `04563ac`. Later documentation-only
commits may reuse this evidence if they do not change code, scripts, package
metadata, or benchmark configuration.

GitHub Actions default `release-gate` must pass on the exact pushed commit
being published. Check the current run in GitHub Actions after the final push;
the publish blocker below is the authoritative gate.

Public benchmark summary from the latest release-candidate run:

| Case | Commit | Format | Rust avg | Upstream avg | Speedup | Compat |
| --- | --- | --- | ---: | ---: | ---: | --- |
| `react` | `f0dfee3` | `javascript` | 0.185054s | 9.880061s | 53.39x | pass |
| `next` | `2bbb67b9` | `typescript` | 0.246471s | 14.149835s | 57.41x | pass |
| `prometheus` | `a0524ee` | `go` | 0.068917s | 4.609267s | 66.88x | pass |

## Publish Blockers

Before publishing, all of these must be true:

- `git status --short` is clean.
- `git submodule status jscpd` points at the reviewed upstream reference.
- `scripts/release-candidate.sh` passes on the exact code commit being tagged.
- GitHub Actions `release-gate` passes on the pushed commit.
- `scripts/package-check.sh` passes and the package file list excludes
  `jscpd/`, `target/`, `node_modules/`, and `scripts/`.
- `cargo publish --dry-run --locked` passes for the exact package manifest and
  include list being published.
- `README.md`, `docs/compat-baseline.md`, and
  `docs/public-benchmark-suite.md` contain the same latest public benchmark
  numbers.
- For the first publication, the `jscpd-rs` crate name is still available or
  already owned by this project, and `v0.1.0` does not already exist locally or
  on the remote.
- `docs/upstream-bugs.md` contains concrete repro commands for upstream issues
  we plan to file.
- `docs/upstream-issue-drafts.md` contains reviewed issue drafts ready to
  verify against current upstream and post.
- `CHANGELOG.md` contains the exact release notes for the version being tagged.

## First-Release Scope

Treat these as release scope:

- `jscpd` and `jscpd-server` binaries with upstream-compatible command names.
- CLI/config option surface covered by `scripts/compat-cli.sh` and
  `scripts/compat-config.sh`.
- Coverage-first duplicate parity: Rust must not miss duplicated upstream lines
  for the same inputs and options.
- Built-in native reporters: `ai`, `console`, `consoleFull`, `csv`, `html`,
  `json`, `markdown`, `silent`, `sarif`, `threshold`, `xcode`, `xml`, and
  `badge`.
- Native file discovery, format registry, JS/TS/JSX/TSX tokenization, generic
  long-tail tokenization, blame, native API, and native server endpoints listed
  in `docs/release-readiness.md`.

## Intentional First-Release Deviations

These are not publication blockers for the first release:

- Dynamic npm reporters, stores, listeners, and plugins are not loaded. The
  compatible option surface and upstream-style missing-package warnings are the
  release contract.
- Exact token totals, pair ordering, and boundaries are diagnostic while
  upstream duplicated lines remain covered.
- HTML output is self-contained and practically compatible, not pixel-perfect.
- The Rust crate exposes a native Rust API, not the upstream JavaScript package
  API.
- Persistent store/cache backends remain demand-driven until benchmark data
  shows the in-memory path is insufficient.
- Full Prism grammar parity for every long-tail format is not attempted; promote
  formats only when coverage gates show missed upstream lines.

## Pre-Tag Commands

Run from the repository root:

```bash
git status --short
git submodule status jscpd
git tag -l 'v0.1.0'
git ls-remote --tags origin 'refs/tags/v0.1.0'
cargo search jscpd-rs --limit 5
scripts/release-candidate.sh
scripts/package-check.sh
cargo publish --dry-run --locked
```

Then push the exact release commit and verify the GitHub Actions
`release-gate` result. Use the workflow dispatch `release_candidate` input for a
full CI-side release-candidate run when needed.

For the first publication candidate checked on 2026-05-31, local and remote
`v0.1.0` tag lookups returned no entries. `cargo search jscpd-rs --limit 5`
returned no exact package, and the sparse crates.io index path
`https://index.crates.io/js/cp/jscpd-rs` returned 404.

## Post-Tag Smoke

After tagging or publishing, install the package into a temporary Cargo root and
check the binaries:

```bash
cargo install --path . --bin jscpd --root /tmp/jscpd-rs-install --force --locked
cargo install --path . --bin jscpd-server --root /tmp/jscpd-rs-install --force --locked
/tmp/jscpd-rs-install/bin/jscpd --version
/tmp/jscpd-rs-install/bin/jscpd --help
/tmp/jscpd-rs-install/bin/jscpd-server --version
```

## Next Release Themes

Track these after the first release candidate:

- Reduce noisy extra Rust findings where they are user-visible false positives.
- Add native persistent store/cache only if release-scale benchmark data needs
  it.
- Tighten MCP Streamable HTTP SDK edge cases if real MCP clients require them.
- Promote long-tail tokenizers only from concrete missed-coverage evidence.
- File upstream bug reports from `docs/upstream-issue-drafts.md`.
