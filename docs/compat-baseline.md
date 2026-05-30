# Compatibility Baseline

Baseline date: 2026-05-30.

Default gate:

```bash
STRICT=coverage scripts/compat-matrix.sh
```

Coverage means every upstream duplicate fragment must be covered by the Rust
report. Exact clone pair ordering is diagnostic only because multi-way duplicate
groups can choose different equivalent pairs.

## Current Matrix

| Target | Format | Gate | Notes |
| --- | --- | --- | --- |
| `jscpd/fixtures` | `javascript` | pass | exact summary parity |
| `jscpd/fixtures` | `typescript` | pass | exact summary parity |
| `jscpd/fixtures` | `jsx` | pass | token totals differ slightly; fragments covered |
| `jscpd/fixtures` | `tsx` | pass | token totals differ slightly; fragments covered |
| `jscpd/packages` | `javascript` | pass | no clones in either implementation |
| `jscpd/packages` | `typescript` | pass | 33/33 upstream starts covered |
| `/home/dev/dream` | `javascript` | pass | 131/131 upstream fragments covered; one exact pair differs in generated `.next` chunks |
| `/home/dev/dream` | `typescript` | pass | 204/204 upstream starts covered |
| `/home/dev/dream` | `tsx` | pass | 13/13 upstream fragments covered; Rust currently reports extra findings |

## Known Deltas

- JS/TS/JSX/TSX use native Rust/Oxc tokenization, so token totals can differ
  from Prism while fragment coverage remains green.
- Long-tail formats are now discoverable through the upstream-synchronized
  registry, but most use generic tokenization and do not carry parity claims.
- Non-native generic formats use coarse whitespace tokenization; weak mode
  strips only best-effort common comment spans.
- Extensionless names such as `Makefile` and `Dockerfile` require
  `--formats-names`, matching upstream behavior.
- Known upstream bug candidates are tracked in `docs/upstream-bugs.md`.

## Benchmark Sanity

Recent local sanity checks:

| Target | Format | Rust avg | Upstream avg | Approx speedup |
| --- | --- | ---: | ---: | ---: |
| `/home/dev/dream` | `tsx` | `0.0358s` | `0.568s` | `16x` |
| `jscpd/packages` | `typescript` | `0.0143s` | `0.831s` | `58x` |
