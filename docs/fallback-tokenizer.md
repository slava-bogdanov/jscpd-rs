# Prism Fallback Tokenizer Design

The first release keeps native Rust tokenizers for the hot path
(`javascript`, `typescript`, `jsx`, `tsx`, `json`) and tracks all other upstream
formats through the synchronized format registry.

The planned fallback is guarded by the `prism-fallback` Cargo feature. Its Rust
entrypoint is:

```rust
tokenizer_fallback::tokenize_with_fallback(content, format, options)
    -> anyhow::Result<Option<Vec<TokenMap>>>
```

Expected behavior:

- return `Ok(Some(maps))` when a Prism-compatible runtime can tokenize the
  requested non-native format;
- return `Ok(None)` when the format should fall back to the generic tokenizer;
- preserve upstream token fields needed by the detector: `format`, token type,
  value, `loc`, and byte `range`;
- keep source map grouping compatible with upstream for embedded formats such as
  Markdown, Vue, Svelte, and Astro;
- never replace the native Oxc path for JS/TS/JSX/TSX unless explicitly enabled
  for compatibility diagnostics.

The current feature only reserves the interface. It deliberately does not embed
or execute a JavaScript runtime yet.
