# jscpd-rs Cloning Plan

## Upstream Review

The reference implementation is a TypeScript monorepo:

- `apps/jscpd` owns the CLI, option/config merging, store setup, reporters, and
  top-level execution.
- `packages/finder` discovers files, applies `.gitignore`/ignore/size/line
  filters, coordinates detection across files, and hosts most reporters.
- `packages/core` contains the detector, in-memory store, statistics, validators,
  and Rabin-Karp based clone search.
- `packages/tokenizer` maps file names/extensions to formats and tokenizes
  supported languages. Current upstream supports 223 formats and special
  block-aware tokenization for Vue, Svelte, Astro, and Markdown.

Upstream testing is conventional package-level Vitest behind `pnpm test`,
orchestrated by Turbo. The public CI workflow builds, lints, runs tests, and
then smoke-runs `./apps/jscpd/bin/jscpd ./fixtures`. Upstream does not keep a
separate public benchmark suite or a pinned set of large repositories; the
README only documents small fixture-based output-size timing/token examples and
a note about tokenizer speed on unspecified real projects.

The core flow is:

1. Parse CLI/config into options.
2. Find supported files with glob, ignore, size, line, symlink, and gitignore
   filters.
3. Tokenize each source.
4. Convert token windows of `minTokens` into hashes.
5. Use a per-format store to find matching windows and grow adjacent matches
   into clones.
6. Validate by `minLines` and optional validators.
7. Emit statistics and reports.

## MVP Scope

The first Rust MVP intentionally implements the minimum vertical slice needed to
measure whether a Rust clone has enough performance upside to continue:

- CLI with common jscpd flags.
- Partial `.jscpd.json` support.
- File discovery with the Rust `ignore` crate for `.gitignore`.
- Upstream-synchronized extension/name format registry.
- Language-agnostic non-whitespace tokenizer.
- Numeric rolling window hashing and in-memory per-format store.
- Clone growth and `minLines` validation.
- Console, consoleFull, AI, JSON, CSV, Markdown, XML PMD CPD, SARIF, badge,
  HTML, Xcode, silent, and threshold reporters.
- Benchmark script against upstream on the same target path.

Known MVP gaps:

- Tokenization is not language-compatible with upstream yet.
- The upstream format registry is synchronized, but most long-tail formats still
  use generic tokenization rather than Prism-compatible tokenization.
- `strict/mild/weak` are still converging overall. `strict` now preserves
  whitespace tokens in the native JS/TS/Oxc path and the generic tokenizer;
  `weak` strips common comment spans for generic formats.
- Terminal timing/tips/progress/verbose behavior is partially aligned with
  upstream, including clone progress and detector event output.
- Blame data is populated from native `git blame -w`. Store options currently
  match the local upstream missing-store fallback. Dynamic external stores are
  not implemented yet.
- `cache`, config `listeners`, and `tokensToSkip` are parsed for option-surface
  compatibility, but upstream currently does not consume them in runtime code.
- No full parity for non-native syntax-specific token streams yet.
- Markdown front matter and fenced code blocks are extracted into embedded
  format maps, with coverage parity on the current upstream fixture.

## Growth Plan

1. Compatibility harness: run upstream and Rust on shared fixtures, compare clone
   counts, locations, statistics, reports, and exit behavior.
2. CLI/config parity: harden remaining flags, config merging rules, exit codes,
   threshold behavior, and list output.
3. Tokenizer backend: replace the MVP tokenizer with maintained crates and
   language-aware token streams. Prefer existing parsers/tokenizers over custom
   grammars.
4. Reporters: polish remaining report details and terminal UX.
5. Advanced sources: full non-native tokenizer parity, dynamic external stores,
   dynamic external reporters, and stricter `strict`/`mild`/`weak` parity.
6. Performance work: parallel file reads/tokenization, compact hash storage,
   faster hashers where compatible, memory profiling, and optional external
   store backends.

## Current Benchmark

Command:

```bash
FORMAT=typescript RUNS=5 scripts/bench.sh jscpd/packages
```

Result on this workspace:

- Rust MVP: `0.108s` average.
- Upstream `jscpd`: `0.818s` average.
- Same file count for this run: 297 TypeScript files.

Broader command:

```bash
RUNS=3 scripts/bench.sh jscpd/packages
```

Result on this workspace:

- Rust MVP: `0.130s` average.
- Upstream `jscpd`: `0.937s` average.
- This broader run is not fully apples-to-apples yet: the MVP supports fewer
  formats than upstream.

Initial signal: continuing makes sense, but the next milestone must measure
speed while closing tokenization/report compatibility gaps.

## Compatibility Gate

The project now uses a coverage-first compatibility rule for ongoing cloning
work:

- Rust must not miss duplicated lines reported by upstream `jscpd` for the same
  file, format, input, and options.
- Rust may report additional duplicates while compatibility is converging.
- Missing upstream line coverage is a blocking compatibility failure.
- Extra Rust duplicates are tracked as diagnostics and fixed when they represent
  likely false positives or user-visible report noise.
- Exact clone pair and fragment-boundary overlap is diagnostic only: when three
  or more equivalent fragments exist, upstream and Rust may choose different
  pairs or wider/split ranges while still covering the same duplicated lines.
- Exact 1:1 parity remains a useful quality metric, but it is not the default
  gate for deciding whether the clone is viable.

Use the compatibility harness with `STRICT=coverage` to enforce this rule.
Use `scripts/compat-matrix.sh` for the current JS/TS-focused release matrix.

## Format Coverage Strategy

The format registry is generated from upstream `@jscpd/tokenizer` using
`scripts/sync-formats.mjs`. This keeps extension detection and `--list` aligned
with upstream while avoiding hand-maintained mapping drift.

Tokenizer strategy remains hybrid:

- native Rust/Oxc path for hot JS/TS formats;
- native Rust block splitting for Markdown, Vue, Svelte, and Astro embedded
  code/style/template regions;
- generic tokenizer for other recognized formats without parity claims;
- no embedded JavaScript runtime fallback. Formats that need real compatibility
  should get native Rust tokenizers and focused compat tests.

## Accepted Hard-Feature Decisions

The current release decisions are canonicalized in
`docs/release-decisions.md`. The summary below mirrors that file for quick
orientation.

These choices are part of the current cloning direction until a compatibility
gate proves they are insufficient:

- Dynamic npm reporters, stores, and plugins are post-MVP. The first release
  should implement popular built-in reporters and stores natively instead of
  embedding or casually spawning JavaScript from Rust.
- Reporter compatibility should be strict for machine-readable contracts such as
  JSON, XML, SARIF, CSV, and Markdown. HTML should stay practically compatible,
  but pixel-perfect parity is not a release blocker.
- Tokenizer compatibility stays hybrid. Use maintained Rust crates or Oxc for
  hot formats where they materially help; keep long-tail formats on generic
  tokenization until a fixture or public-repo gate shows missed upstream
  coverage.
- Node/Commander quirks are mirrored only when they are user-visible and covered
  by compatibility tests. The project should not import a JavaScript runtime to
  reproduce every incidental JS behavior.
- Blame should stay native (`git blame`/git library based) and fail per file
  where possible instead of inheriting upstream nested-repo failure modes.
- Store/cache work should stay native and demand-driven. Custom external stores
  remain documented gaps unless a real large-repo benchmark proves they are
  needed for release viability.

## Larger Local Repo Benchmarks

All commands below used TypeScript-only scanning to keep the comparison focused:

```bash
FORMAT=typescript RUNS=3 scripts/bench.sh <repo>
```

| Repo | Rust MVP | Upstream `jscpd` | Files | Rust clones | Upstream clones |
| --- | ---: | ---: | ---: | ---: | ---: |
| `/home/dev/.hermes/hermes-agent` | `0.447s` | `1.930s` | 316 | 93 | 475 |
| `/home/dev/dream` | `0.350s` | `1.877s` | 566 Rust / 572 upstream | 371 | 1371 |
| `/home/dev/infer` | `0.010s` | `0.290s` | 28 | 12 | 41 |

Broader all-format stress on `/home/dev/dream`:

- Rust MVP, `RUNS=2`: `0.600s` average.
- Upstream `jscpd`: first run took `70.32s`, then the benchmark was stopped.
- This broader run is not a fair compatibility comparison yet because upstream
  supports far more formats and emitted a warning for `excel-formula`.

Conclusion remains positive: the Rust path is consistently faster on larger
repos, but the next milestone must prioritize tokenizer/discovery parity before
the speedup can be treated as product-quality.

## Acceleration Pass

The first MVP was still too conservative: it used MD5 strings for token/window
hashes and prepared files sequentially. The current hot path now uses:

- zero-copy detection tokens: no per-token `String` allocation in detection;
- `xxh3_128` token hashes;
- numeric rolling window hashes instead of MD5 over concatenated strings;
- `rustc_hash::FxHashMap` for the in-memory window store;
- parallel file reads and line counting;
- parallel per-file tokenization/window preparation;
- nanosecond-resolution benchmark timing in `scripts/bench.sh`.

Updated TypeScript-only benchmark:

| Repo | Rust MVP before | Rust MVP now | Upstream `jscpd` | Approx speedup vs upstream |
| --- | ---: | ---: | ---: | ---: |
| `jscpd/packages` | `0.108s` | `0.019s` | `0.856s` | ~45x |
| `/home/dev/.hermes/hermes-agent` | `0.447s` | `0.085s` | `2.028s` | ~24x |
| `/home/dev/dream` | `0.350s` | `0.074s` | `1.955s` | ~26x |
| `/home/dev/infer` | `0.010s` | `0.009s` | `0.305s` | ~33x |

This is the first speed signal that is strong enough to justify continuing,
provided compatibility can be raised without destroying the margin.

## Core Stabilization Pass

The detector core was refactored again so later tokenizer/reporting work does
not have to revisit the detection hot path:

- introduced numeric `SourceId` and `FormatId` in the core;
- introduced `TokenStream` as the detector input contract;
- removed per-window `Frame` allocation;
- stores only first `Occurrence { source_id, token_start }` per window hash;
- streams rolling windows directly from token hashes;
- verifies matching windows on hash hits;
- shards detection by format and runs format shards in parallel.

Updated TypeScript-only benchmark after this pass:

| Repo | Rust after hash pass | Rust after core pass | Upstream `jscpd` | Approx speedup vs upstream |
| --- | ---: | ---: | ---: | ---: |
| `jscpd/packages` | `0.019s` | `0.011s` | `0.821s` | ~75x |
| `/home/dev/.hermes/hermes-agent` | `0.085s` | `0.038s` | `2.041s` | ~54x |
| `/home/dev/dream` | `0.074s` | `0.034s` | `1.939s` | ~57x |

Do not treat these as fixed release gates yet. Before publication, choose a
small set of popular public repositories and use them as the repeatable
benchmark/compatibility suite.
