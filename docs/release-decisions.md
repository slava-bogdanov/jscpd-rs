# Release Decisions

Approved on 2026-05-30.

These decisions define the first-release direction. They can change only when a
compatibility gate, public benchmark, or real user workflow proves the current
choice insufficient.

## Compatibility Gate

- The blocking rule is coverage-first parity: Rust must not miss duplicate
  lines reported by upstream `jscpd` for the same input and options.
- Rust may report additional duplicates while compatibility is converging.
  Extra findings remain visible in comparison output and should be reduced when
  they are false positives or noisy user-visible ranges.
- Exact 1:1 clone boundaries, pair ordering, and token totals are quality
  metrics, not the default release blocker.

## Runtime Strategy

- Do not embed a JavaScript runtime or spawn upstream JavaScript from Rust to
  implement tokenizers, reporters, stores, or plugins.
- Prefer native Rust implementations backed by maintained crates.
- If a format cannot be supported well without a large custom port, keep it on
  generic tokenization until a fixture or public-repo gate shows missed
  upstream coverage.

## Reporters

- Built-in reporters are first-release scope and should be implemented natively:
  `ai`, `console`, `consoleFull`, `csv`, `html`, `json`, `markdown`, `silent`,
  `sarif`, `threshold`, `xcode`, `xml`, and the commonly used badge reporter.
- Machine-readable reporter contracts are strict release scope: JSON, XML,
  SARIF, CSV, and Markdown should stay structurally compatible with upstream.
- HTML must remain practically compatible and self-contained. Pixel-perfect
  upstream parity is not a first-release blocker.
- Dynamic npm reporter loading is post-MVP. Unknown reporter names should keep
  the upstream-style warning and continue where upstream continues.

## Stores And Cache

- The default in-memory store is the first-release store path.
- `--store <name>` should keep the upstream missing-store fallback shape unless
  a native store backend is deliberately added.
- Dynamic npm store loading is post-MVP.
- Native persistent/cache stores are demand-driven. Add one only if public
  benchmark data shows the in-memory path is not enough for release viability.
- `cache`, config `listeners`, and `tokensToSkip` remain option-surface
  compatibility fields while upstream exposes but does not consume them in the
  current CLI runtime.

## Formats And Tokenization

- JS/TS/JSX/TSX stay on the Oxc-backed native path.
- Markdown, Vue, Svelte, Astro, Apex, and markup keep small native block
  splitters where that unlocks upstream coverage without a full grammar port.
- Long-tail formats use the upstream-synchronized registry plus generic
  tokenization by default.
- Promote a long-tail format only with a focused fixture, `STRICT=coverage`
  comparison, and a clear reason that generic tokenization is insufficient.

## CLI And Upstream Quirks

- Mirror upstream behavior when it is visible, common, and covered by tests.
- Crash-only Commander edge cases are documented first and mirrored only if a
  release gate or user workflow makes exact behavior necessary.
- Keep upstream bug candidates in `docs/upstream-bugs.md` so we can file them
  later with concrete repro commands.

## Blame

- Blame stays native through Git commands or a Git library.
- Prefer per-file failure isolation over inheriting upstream nested-repository
  failure modes.

## Performance

- Performance remains a product requirement. Public benchmark runs should be
  repeated before publication with pinned commits and recorded speedups.
- The aspirational target is 50x on representative cases, but release gating
  should use measured thresholds from the selected public benchmark suite.

## Approved Complex Feature Choices

These are the current choices for features that are expensive to clone exactly:
the user explicitly approved these tradeoffs on 2026-05-30.

- Dynamic npm reporters, stores, listeners, and plugins: do not implement for
  the first release. Keep option-surface compatibility, native built-ins, and
  upstream-style missing-package warnings.
- Long-tail Prism tokenizer parity: do not port all grammars eagerly. Keep the
  upstream-synchronized format registry plus generic tokenization, then promote
  formats only when fixtures or public-repo gates show missed upstream coverage.
- Extra Rust findings: acceptable while compatibility converges. The release
  blocker is missing upstream duplicated line coverage, not 1:1 pair identity.
- Node/Commander quirks: mirror visible behavior only when covered by gates.
  Document crash-only or incidental quirks in `docs/upstream-bugs.md`.
- HTML reporter: keep self-contained and practically compatible. Pixel-perfect
  parity is not a first-release blocker.
- Persistent cache/store backends: postpone until benchmark data shows the
  in-memory detector is insufficient on release-scale repositories.
- Blame failures: prefer native per-file isolation instead of inheriting
  upstream failure modes around nested or unusual Git repositories.
