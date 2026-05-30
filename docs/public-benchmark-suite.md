# Public Benchmark Suite

The release benchmark suite uses popular public repositories cloned under
`.bench/repos`. These clones are generated local state and are ignored by git.

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
| `kubernetes` | `https://github.com/kubernetes/kubernetes.git` | `master` | `go` |
| `rust` | `https://github.com/rust-lang/rust.git` | `main` | `rust` |

Usage:

```bash
LIST=1 scripts/public-bench-suite.sh
CASES=react,next RUNS=3 scripts/public-bench-suite.sh
CHECK_COMPAT=1 CASES=react scripts/public-bench-suite.sh
```

Default behavior clones missing repositories with `--depth=1`, runs Rust and
upstream `jscpd` through `scripts/bench.sh`, and writes raw benchmark output to
`.bench/results`. Set `UPDATE=1` to refresh existing clones.

Before publication, run the suite on the selected cases and copy the measured
averages into `docs/compat-baseline.md` or release notes with commit hashes.
