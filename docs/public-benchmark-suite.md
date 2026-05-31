# Public Benchmark Suite

The release benchmark suite uses popular public repositories cloned under
`${XDG_CACHE_HOME:-$HOME/.cache}/jscpd-rs/public-bench/repos` by default. These
clones are generated local state outside this git repo.

Keep benchmark repositories outside this project tree unless you intentionally
disable parent `.gitignore` effects. Upstream `jscpd` respects parent ignore
files and can silently skip repo-internal benchmark directories that are
gitignored.

Upstream `jscpd` does not currently ship a public-repository performance suite.
Its monorepo scripts expose `pnpm test`, CI runs build/lint/test plus
`./apps/jscpd/bin/jscpd ./fixtures`, and the README benchmark note is based on
the local `fixtures/` directory. This suite is therefore a project-owned release
gate for product-scale speed checks.

Configured cases:

| Case | Repository | Branch | Format |
| --- | --- | --- | --- |
| `react` | `https://github.com/facebook/react.git` | `main` | `javascript` |
| `next` | `https://github.com/vercel/next.js.git` | `canary` | `typescript` |
| `vscode` | `https://github.com/microsoft/vscode.git` | `main` | `typescript` |
| `prometheus` | `https://github.com/prometheus/prometheus.git` | `main` | `go` |
| `rust` | `https://github.com/rust-lang/rust.git` | `main` | `rust` |

Usage:

```bash
LIST=1 scripts/public-bench-suite.sh
CASES=react,next RUNS=3 scripts/public-bench-suite.sh
CHECK_COMPAT=1 CASES=react scripts/public-bench-suite.sh
MIN_SPEEDUP=10 CASES=react,next RUNS=3 scripts/public-bench-suite.sh
UPSTREAM_TIMEOUT=600s CASES=vscode RUNS=1 scripts/public-bench-suite.sh
PUBLIC=1 PUBLIC_CASES=react,next PUBLIC_RUNS=3 scripts/release-gate.sh
```

Default behavior clones missing repositories with `--depth=1`, runs Rust and
upstream `jscpd` through `scripts/bench.sh`, and writes raw benchmark output to
`$BENCH_ROOT/results`. It also writes a TSV summary to
`$BENCH_ROOT/results/summary.tsv` with case, commit, format, Rust average,
upstream average, speedup, and compatibility status. Set `UPDATE=1` to refresh
existing clones. Set `MIN_SPEEDUP` to make the suite fail when any selected
case falls below the required upstream/Rust speedup. Each upstream timing run is
bounded by `UPSTREAM_TIMEOUT` (`600s` by default) so optional stress cases cannot
hang a release gate indefinitely; set `RUST_TIMEOUT` or `UPSTREAM_TIMEOUT` to an
empty value to disable that side's timeout.

When `CHECK_COMPAT=1` is enabled, the suite runs the same coverage-first report
comparison used by the fixture gates. `react`, `next`, and `prometheus` include
narrow allowlists for upstream overextended ranges documented in
`docs/upstream-bugs.md`; those entries are printed as ignored line-coverage
exceptions in the comparison output. New public benchmark misses should be fixed
or documented before they are added to this allowlist.

Recorded release-candidate measurements on May 31, 2026:

```bash
scripts/release-candidate.sh
```

| Case | Commit | Format | Rust avg | Upstream avg | Speedup | Compat |
| --- | --- | --- | ---: | ---: | ---: | --- |
| `react` | `f0dfee3` | `javascript` | 0.192096s | 9.946562s | 51.78x | pass |
| `next` | `2bbb67b9` | `typescript` | 0.248940s | 14.282248s | 57.37x | pass |
| `prometheus` | `a0524ee` | `go` | 0.082703s | 4.609423s | 55.73x | pass |

`kubernetes` was also checked as a Go stress case, but upstream `jscpd` ran out
of memory with the default Node heap, so it is intentionally not part of the
default release suite.

`vscode` is configured as an optional TypeScript stress case, but it is not part
of the default release suite yet. On May 31, 2026, Rust completed the timing run
in `1.464358s` at commit `e4074382`, while upstream was still running after
more than nine minutes and the exploratory run was stopped before compatibility
comparison. Keep it behind an explicit `CASES=vscode` run until we decide on a
separate slow-suite policy.

Before publication, rerun the suite on the selected cases and copy the measured
averages into `docs/compat-baseline.md` or release notes with commit hashes.
