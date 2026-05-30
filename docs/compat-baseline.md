# Compatibility Baseline

Baseline date: 2026-05-30.

Default gate:

```bash
STRICT=coverage scripts/compat-matrix.sh
```

Coverage means every upstream duplicated line must be covered by the Rust report
for the same file and format. Exact clone starts, fragment boundaries, and pair
ordering are diagnostic only because Rust may find a wider or split equivalent
range while compatibility is converging.

## Current Matrix

| Target | Format | Gate | Notes |
| --- | --- | --- | --- |
| `jscpd/fixtures` | `javascript` | pass | exact summary parity |
| `jscpd/fixtures` | `typescript` | pass | exact summary parity |
| `jscpd/fixtures` | `jsx` | pass | token totals differ slightly; fragments covered |
| `jscpd/fixtures` | `tsx` | pass | token totals differ slightly; fragments covered |
| `jscpd/fixtures/markdown` | `markdown` | pass | 18/18 upstream fragments line-covered; Rust reports wider/split ranges |
| `jscpd/fixtures` | `vue` | pass | 18/18 upstream fragments line-covered; exact starts differ for wider markup/scss ranges |
| `jscpd/fixtures` | `svelte` | pass | 6/6 upstream fragments line-covered; exact start differs for wider css range |
| `jscpd/fixtures` | `astro` | pass | 8/8 upstream fragments line-covered; exact starts differ for wider markup/css ranges |
| `jscpd/packages` | `javascript` | pass | no clones in either implementation |
| `jscpd/packages` | `typescript` | pass | 66/66 upstream fragments line-covered |
| `/home/dev/dream` | `javascript` | pass | 154/154 upstream fragments line-covered; one exact pair differs in generated `.next` chunks |
| `/home/dev/dream` | `typescript` | pass | 408/408 upstream fragments line-covered |
| `/home/dev/dream` | `tsx` | pass | 14/14 upstream fragments line-covered; Rust currently reports extra findings |

## Known Deltas

- JS/TS/JSX/TSX use native Rust/Oxc tokenization, so token totals can differ
  from Prism while fragment coverage remains green.
- Long-tail formats are now discoverable through the upstream-synchronized
  registry, but most use generic tokenization and do not carry parity claims.
- Markdown extracts YAML front matter and fenced code blocks into embedded
  format maps. The upstream Markdown fixture is line-covered, though exact
  starts still differ where Rust reports wider/split ranges.
- Vue, Svelte, and Astro now split embedded template/script/style/frontmatter
  regions into format maps. Their fixtures are line-covered, with expected
  wider ranges from generic markup/style tokenization.
- Non-native generic formats use coarse whitespace tokenization; weak mode
  strips only best-effort common comment spans.
- Extensionless names such as `Makefile` and `Dockerfile` require
  `--formats-names`, matching upstream behavior.
- Custom extension and filename mappings are supported through
  `--formats-exts`/`formatsExts` and `--formats-names`/`formatsNames`.
- Known upstream bug candidates are tracked in `docs/upstream-bugs.md`.

## Benchmark Sanity

Recent local sanity checks:

| Target | Format | Rust avg | Upstream avg | Approx speedup |
| --- | --- | ---: | ---: | ---: |
| `/home/dev/dream` | `tsx` | `0.0358s` | `0.568s` | `16x` |
| `jscpd/packages` | `typescript` | `0.0143s` | `0.831s` | `58x` |
