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
| `jscpd/fixtures/javascript` | `javascript` / `strict` | pass | exact summary parity |
| `jscpd/fixtures` | `typescript` / `strict` | pass | exact summary parity |
| `jscpd/fixtures/javascript` | `javascript` / `weak` | pass | clone and line summary parity; token totals differ slightly |
| `jscpd/fixtures` | `jsx` | pass | token totals differ slightly; fragments covered |
| `jscpd/fixtures` | `tsx` | pass | token totals differ slightly; fragments covered |
| `jscpd/fixtures/markdown` | `markdown` | pass | 18/18 upstream fragments line-covered; Rust reports wider/split ranges |
| `jscpd/fixtures` | `vue` | pass | 18/18 upstream fragments line-covered; exact starts differ for wider markup/scss ranges |
| `jscpd/fixtures` | `svelte` | pass | 6/6 upstream fragments line-covered; exact start differs for wider css range |
| `jscpd/fixtures` | `astro` | pass | 8/8 upstream fragments line-covered; exact starts differ for wider markup/css ranges |
| `jscpd/fixtures/python` | `python` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/go` | `go` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/ruby` | `ruby` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/php` | `php` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/yaml` | `yaml` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/sql` | `sql` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/toml` | `toml` | pass | 2/2 upstream fragments line-covered |
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
  strips best-effort common comment spans, including `#`, `//`, `/* */`,
  `<!-- -->`, SQL-style `--`, and Lisp/INI-style `;` comments where those
  prefixes are comments in the upstream Prism grammar.
- `--mode strict` now preserves Prism-style `empty` and `new_line` whitespace
  tokens in the native JS/TS/Oxc path and the generic tokenizer. The
  JavaScript fixture has exact strict-mode summary parity.
- Extensionless names such as `Makefile` and `Dockerfile` require
  `--formats-names`, matching upstream behavior.
- Custom extension and filename mappings are supported through
  `--formats-exts`/`formatsExts` and `--formats-names`/`formatsNames`.
- `skipLocal` follows the upstream configured-root validator: clones are skipped
  only when both fragments are inside the same input path.
- The upstream workflow option surface for `blame`, `store`, `storePath`,
  `cache`, `executionId`, `noTips`, `listeners`, and `tokensToSkip` is parsed
  from CLI/config where applicable. The default `executionId` is generated as a
  UTC RFC3339 timestamp, matching the upstream workflow shape. `--blame`
  populates clone fragment blame data from native `git blame -w` output when
  available.
- `cache`, config `listeners`, and `tokensToSkip` are intentionally treated as
  option-surface compatibility only for now: the upstream CLI/reference code
  defines or merges these fields, but does not consume them in the detection,
  tokenizer, reporter, or store runtime.
- `--store <name>` currently follows the upstream missing-store fallback shape:
  it warns that the store package is not installed and continues with in-memory
  detection. Dynamic loading of external store packages remains an
  implementation gap.
- `--debug` is a dry run like upstream: it prints options and discovered files,
  then exits before clone detection and reporter execution.
- `--list` follows the upstream output shape: a `Supported formats:` header
  followed by comma-separated formats.
- Non-silent runs print clone progress for non-`ai` reporters, then reporter
  output, then a `time:` footer. Tips are printed by default and suppressed by
  `--noTips`, matching the upstream workflow shape.
- `--verbose` prints upstream-style format-filter skip messages and detector
  events for `START_DETECTION`, `CLONE_FOUND`, and `CLONE_SKIPPED`.
- Unknown reporter names emit the upstream-style install warning. Dynamic
  loading of external reporter packages is not implemented yet.
- `reportersOptions.badge` supports the upstream-style `subject`, `status`,
  `color`, and `path` overrides for the built-in badge reporter.
- Known upstream bug candidates are tracked in `docs/upstream-bugs.md`.

## Benchmark Sanity

Recent local sanity checks:

| Target | Format | Rust avg | Upstream avg | Approx speedup |
| --- | --- | ---: | ---: | ---: |
| `/home/dev/dream` | `tsx` | `0.0358s` | `0.568s` | `16x` |
| `jscpd/packages` | `typescript` | `0.0143s` | `0.831s` | `58x` |

## Additional Mode Checks

```bash
DETECTION_MODE=strict FORMAT=javascript MIN_TOKENS=20 MIN_LINES=3 MAX_SIZE=1mb \
  STRICT=coverage scripts/compat.sh jscpd/fixtures/javascript
```

The default matrix also includes strict JavaScript/TypeScript and weak
JavaScript mode checks so mode regressions are gated directly.
