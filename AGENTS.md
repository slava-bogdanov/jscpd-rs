# AGENTS.md

## Project Goal

This project is a high-performance Rust clone of
[`jscpd`](https://github.com/kucherenko/jscpd).

The goal is full practical compatibility with upstream `jscpd`: command-line
behavior, configuration formats, supported languages, reports, exit codes, and
integration workflows should match the reference implementation unless a
deliberate incompatibility is documented.

The upstream `jscpd` repository is kept in `jscpd/` as a git submodule and is the
primary reference for behavior.

## Engineering Principles

- Build a fast Rust implementation first: performance is a core product goal,
  not an afterthought.
- Prefer battle-tested crates over custom code. Keep project-specific logic as
  small as practical.
- For JS/TS syntax tokenization, prefer Oxc-backed token processing over a
  hand-rolled lexer. Keep only the glue needed for jscpd-compatible filtering,
  positions, hashing, and reporting.
- Keep it simple. Use KIS: straightforward data flow, small modules, and minimal
  abstraction until real complexity requires it.
- Use SOTA libraries and algorithms where they materially improve correctness,
  performance, maintainability, or ecosystem compatibility.
- Match upstream behavior before improving it. Optimizations must not silently
  change user-visible semantics.
- Treat the upstream project as executable specification: compare behavior
  against `jscpd/` when implementing CLI flags, config parsing, tokenization,
  detection logic, and reporters.
- Avoid rewriting mature infrastructure from scratch: prefer existing crates for
  CLI parsing, config formats, globbing, ignore files, syntax/token processing,
  serialization, reporting formats, concurrency, and diagnostics.
- Keep dependencies intentional: choose widely used, maintained crates with clear
  APIs and acceptable compile-time/runtime costs.
- Add focused compatibility tests as features are ported. Prefer fixtures based
  on upstream behavior.
- Document intentional deviations from upstream in the relevant code, tests, or
  project documentation.
