# Release Checklist

This checklist is the publication runbook for the first Rust release candidate.
Policy decisions live in `docs/release-decisions.md`; current evidence lives in
`docs/compat-baseline.md` and `docs/release-readiness.md`.

## Current Release Candidate Evidence

Latest full local prepublish gate:

```bash
scripts/prepublish-check.sh
```

Passed on 2026-05-31 at code commit `4ae0412`. This includes
`scripts/release-candidate.sh`, package/install verification, crate/tag
availability checks, and `cargo publish --dry-run --locked`. Later documentation-only
commits may reuse this evidence if they do not change code, scripts, package
metadata, or benchmark configuration.

GitHub Actions default `release-gate` must pass on the exact pushed commit
being published. Check the current run in GitHub Actions after the final push;
the publish blocker below is the authoritative gate.

Latest GitHub Actions default release-gate:

```text
push
```

Passed on 2026-05-31 at code commit `4ae0412`:
https://github.com/vv-bogdanov/jscpd-rs/actions/runs/26703182279

Recorded public benchmark baseline for this release candidate:

| Case | Commit | Format | Rust avg | Upstream avg | Speedup | Compat |
| --- | --- | --- | ---: | ---: | ---: | --- |
| `react` | `f0dfee3` | `javascript` | 0.190427s | 10.012867s | 52.58x | pass |
| `next` | `2bbb67b9` | `typescript` | 0.256117s | 14.616985s | 57.07x | pass |
| `prometheus` | `a0524ee` | `go` | 0.080734s | 4.609130s | 57.09x | pass |

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
  `docs/public-benchmark-suite.md` contain the same recorded public benchmark
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
scripts/prepublish-check.sh
```

The script checks clean git state, the reviewed `jscpd` submodule reference,
local and remote tag availability, exact crate-name availability through
`cargo search`, benchmark-number consistency across release docs, the full
release-candidate gate, package/install validation, and
`cargo publish --dry-run --locked`. Set `RUN_RELEASE_CANDIDATE=0` only when the
same code commit already has fresh local and CI release-candidate evidence.

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
