# Compatibility Baseline

Baseline date: 2026-05-31.

Latest full release gate:
`FULL=1 PUBLIC=1 scripts/release-gate.sh`
passed on 2026-05-31 at code commit `bac2203` as part of
`scripts/prepublish-check.sh`.

Latest public release gate:
`PUBLIC=1 PUBLIC_RUNS=3 scripts/release-gate.sh`
passed on 2026-05-31 at code commit `bac2203` as part of
`scripts/prepublish-check.sh`.

Default gate:

```bash
STRICT=coverage scripts/compat-matrix.sh
```

Coverage means every upstream duplicated line must be covered by the Rust report
for the same file, and Rust must not report fewer clones. Exact clone starts,
formats, fragment boundaries, source totals, line totals, and pair ordering are
diagnostic only because Rust may find a wider or split equivalent range while
compatibility is converging.

Reporter gate:

```bash
scripts/compat-reporters.sh
```

This smoke check runs Rust and upstream with
`json,csv,markdown,xml,sarif,badge,html`, verifies the expected report files,
parses JSON/SARIF payloads, checks stable artifact contracts, and compares the
root JSON report with the default coverage rule. Stable artifact checks include
CSV/Markdown line and clone summary columns, the upstream Markdown heading
prefix, exact XML output for the fixture, SARIF structure with normalized
paths, badge title/aria text, HTML report text and clone summaries, and
equality between each HTML JSON payload and its root JSON report. The aggregate
release gate also runs this reporter check against a no-duplicates JavaScript
fixture so empty JSON/CSV/Markdown/XML/SARIF/badge/HTML reports stay covered.

CLI gate:

```bash
scripts/compat-cli.sh
```

This smoke check compares Rust and upstream exit codes plus stable terminal
contracts for `--help`, `--version`, `--list`, `--debug`, `--exitCode`,
`--threshold`, invalid `--mode`, bare `--config`, `--store`, `--store-path`,
bare optional string flag crashes, `--formats-exts`, `--formats-names`,
malformed `--formats-exts`/`--formats-names` mappings, `--ignore-pattern`,
`--ignoreCase`, unknown reporters, explicit `time`
reporter fallback, terminal footer/tips, `xcode`, `ai`, `consoleFull`, and
`--verbose`.
The debug checks include cwd `.gitignore` expansion in the printed `ignore`
option and user-order preservation for explicit `--format` lists.

Config gate:

```bash
scripts/compat-config.sh
```

This smoke check runs both implementations from real `.jscpd.json` and
`package.json#jscpd` configs, including relative `path`, config `output`,
`silent`, JSON reporter setup, `exitCode`, and order-sensitive `formatsExts`
object mappings. It also verifies explicit `--config` files outside `cwd`,
`formatsNames` mappings for extensionless filenames,
`reportersOptions.badge` path/subject/status/color overrides, debug
option-surface preservation for `config`, `cache`, `listeners`, and
`tokensToSkip`, upstream-coerced string numeric config values for `minLines`,
`maxLines`, and `threshold`, and checks that
malformed `package.json` files emit a warning and do not prevent detection from
continuing. Malformed `.jscpd.json` files are checked separately: both
implementations fail before detection with an upstream-style `SyntaxError`
printed to stdout. Symlinked explicit config files are also checked so
`config`, relative `path`, and relative `ignore` resolution follow the symlink
location rather than the real target path.

Blame gate:

```bash
scripts/compat-blame.sh
```

This smoke check creates a temporary Git repository, commits a duplicated pair,
runs both implementations with `--blame --reporters json`, verifies that both
JSON reports include matching blame data on both duplicate fragments, and then
compares the reports with the default coverage rule.

Server gate:

```bash
scripts/compat-server.sh
```

This smoke check compares the native `jscpd-server` binary with upstream
`apps/jscpd-server`. It verifies exact server `--help` output, invalid or bare
`--port`, bare common optional flag error shapes, missing-store warning
fallback, bare and explicit `--host` startup output, rejects main-CLI-only
options that upstream server does not accept, config-only `workingDirectory`
semantics, starts both servers on local ports, and checks the root API info,
`/api/health`, `/api/stats`, JSON and urlencoded `/api/check`,
empty/missing/non-string field validation, large and special-character
snippets, JSON content-type headers, JSON syntax errors, upstream-style JSON
404 responses for missing routes and wrong API methods, MCP
initialize/session handling, `tools/list`,
`resources/list`, `get_statistics`, `check_duplication` with `recheck`,
`check_current_directory`, `jscpd://statistics`, repeated snippet isolation,
and `GET /mcp` method rejection. It also checks upstream-style MCP UUID-v4
session IDs, `Content-Type` rejection,
`DELETE /mcp` and `OPTIONS /mcp` JSON 404 responses, plus JSON-RPC
single-request and multi-request batch handling. Stable MCP SDK-shaped
responses for `initialize`, `tools/list`, `resources/list`, and batch
list/resource requests are compared exactly against upstream, with only the
package version normalized.

Package/install gate:

```bash
scripts/package-check.sh
```

This release-surface check verifies the crate package file list, rejects
accidental publication of the upstream `jscpd/` submodule, `target/`,
`node_modules`, and internal scripts, runs `cargo package --locked`, installs
the `jscpd` and `jscpd-server` binaries into a temporary Cargo root, and checks
the installed binaries' versions and the CLI binary's upstream-compatible
command name.

Native API smoke tests are covered by the Rust test suite. They verify the
path-based detector API, in-memory source API, upstream singular
`detectClonesAndStatistic` spelling, default options, supported format registry,
and default/custom format lookup helpers.

Upstream CI fixture gate:

```bash
scripts/compat-upstream-ci.sh
```

This mirrors upstream's CI smoke command, `jscpd ./fixtures`, with the upstream
defaults that matter for detection (`minTokens=50`, `minLines=5`,
`maxSize=100kb`). It uses the coverage-first comparison, so Rust may report
additional clones but must cover every upstream duplicated line.

Aggregate gate:

```bash
scripts/release-gate.sh
FULL=1 scripts/release-gate.sh
PUBLIC=1 scripts/release-gate.sh
```

The default run covers formatting, unit tests, shell syntax, package/install
verification, and fast CLI/config/reporter/blame/server compatibility checks.
`FULL=1` also runs the full coverage-first compatibility matrix. `PUBLIC=1`
runs the project-owned public benchmark suite with coverage compatibility
enabled, using `PUBLIC_CASES`, `PUBLIC_RUNS`, `PUBLIC_CHECK_COMPAT`, and
`PUBLIC_MIN_SPEEDUP` to override its defaults.
`FULL=1 PUBLIC=1 scripts/release-gate.sh` is required before publication.

Release candidate gate:

```bash
scripts/release-candidate.sh
```

This is the pre-publication gate: it runs
`cargo clippy --all-targets -- -D warnings`, the default release gate, the full
compatibility matrix with `STRICT=coverage`, and the public benchmark/coverage
suite with three timing runs on the default public cases.
The GitHub Actions workflow exposes the same path through the
`release_candidate` manual dispatch input.

CI gate:

```bash
.github/workflows/release-gate.yml
```

The GitHub Actions workflow checks out the upstream submodule, installs Rust
and Node, and runs the default release gate on pushes and pull requests. Manual
workflow dispatch exposes `full`, `public`, `release_candidate`, and
`public_runs` inputs for the pre-release full matrix, public benchmark, and
release-candidate gates.

Latest local prepublish check: `scripts/prepublish-check.sh` passed on
2026-05-31 at code commit `bac2203`, covering
`cargo clippy --all-targets -- -D warnings`, the default release gate, the full
coverage matrix, the public benchmark/coverage suite, package/install
verification, crate/tag availability checks, and `cargo publish --dry-run --locked`.

Latest GitHub Actions default release-gate check:
`push` passed on 2026-05-31 at code commit `bac2203`:
https://github.com/vv-bogdanov/jscpd-rs/actions/runs/26702488698

Recorded public benchmark baseline:

| Case | Commit | Format | Rust avg | Upstream avg | Speedup | Compat |
| --- | --- | --- | ---: | ---: | ---: | --- |
| `react` | `f0dfee3` | `javascript` | 0.192096s | 9.946562s | 51.78x | pass |
| `next` | `2bbb67b9` | `typescript` | 0.248940s | 14.282248s | 57.37x | pass |
| `prometheus` | `a0524ee` | `go` | 0.082703s | 4.609423s | 55.73x | pass |

## Current Matrix

| Target | Format | Gate | Notes |
| --- | --- | --- | --- |
| `jscpd/fixtures` | `javascript` | pass | exact summary parity |
| `jscpd/fixtures` | `typescript` | pass | exact summary parity |
| `jscpd/fixtures/javascript` | `json` | pass | exact clone and line summary parity |
| `jscpd/fixtures` | auto, upstream CI defaults | pass | 422/422 upstream fragments line-covered; Rust reports a few extra generic/SFC ranges |
| `jscpd/fixtures/custom` | auto + `--formats-exts c:ccc,cc1` | pass | exact clone and line summary parity |
| `jscpd/fixtures/ignore` | auto | pass | clone-summary gate; inline `style` attributes produce upstream-compatible CSS source buckets; ignored blocks produce 0 clones |
| `jscpd/fixtures/ignore-pattern` | auto + `--ignore-pattern` | pass | exact clone and line summary parity |
| `jscpd/fixtures/ignore-case` | auto | pass | clone-summary gate; no clones without `--ignoreCase` |
| `jscpd/fixtures/ignore-case` | auto + `--ignoreCase` | pass | clone-summary gate; 1 clone with case folding |
| `jscpd/fixtures/one-file/one-file.js` | auto | pass | exact summary parity for intra-file clones |
| `jscpd/fixtures/folder1` + `jscpd/fixtures/folder2` | auto | pass | exact clone and line summary parity without `--skipLocal` |
| `jscpd/fixtures/folder1` + `jscpd/fixtures/folder2` | auto + `--skipLocal` | pass | exact clone and line summary parity with local clones skipped |
| `jscpd/fixtures/mixed-formats` | auto | pass | upstream JS-in-HTML clone line-covered; Rust reports a wider cross-file JS range |
| `jscpd/fixtures/shebang` | auto | pass | exact clone and line summary parity for extensionless bash/python shebang files |
| `jscpd/fixtures/javascript` | `javascript` / `strict` | pass | exact clone and line summary parity; token totals differ |
| `jscpd/fixtures` | `typescript` / `strict` | pass | exact clone and line summary parity; token totals differ |
| `jscpd/fixtures/javascript` | `javascript` / `weak` | pass | clone and line summary parity; token totals differ slightly |
| `jscpd/fixtures` | `jsx` | pass | exact clone and line summary parity; token totals differ slightly |
| `jscpd/fixtures` | `tsx` | pass | exact clone and line summary parity; token totals differ slightly |
| `jscpd/fixtures/markdown` | `markdown` | pass | exact clone/start and duplicated-line parity; source line and token totals differ |
| `jscpd/fixtures` | `vue` | pass | exact upstream fragment/start coverage; Rust still reports duplicate extra script/template clones |
| `jscpd/fixtures` | `svelte` | pass | 6/6 upstream fragments line-covered; exact start differs for wider css range |
| `jscpd/fixtures` | `astro` | pass | exact upstream fragment/start coverage; Rust still reports duplicate extra embedded clones |
| `jscpd/fixtures/pug` | `pug` | pass | exact clone and line summary parity; upstream overextended `style.` range is mirrored |
| `jscpd/fixtures/haml` | `haml` | pass | exact clone and line summary parity; upstream overextended silent-comment range is mirrored |
| `jscpd/fixtures/css` | `css` | pass | exact clone coverage; token totals differ |
| `jscpd/fixtures/css` | `less` | pass | exact clone and line summary parity |
| `jscpd/fixtures/css` | `scss` | pass | exact clone and line summary parity |
| `jscpd/fixtures/python` | `python` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/go` | `go` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/ruby` | `ruby` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/php` | `php` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/yaml` | `yaml` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/sql` | `sql` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/toml` | `toml` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/shell` | `bash` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/swift` | `swift` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/powershell` | `powershell` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/lua` | `lua` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/haskell` | `haskell` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/haskell-literate` | `haskell` | pass | exact clone and line summary parity |
| `jscpd/fixtures/clojure` | `clojure` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/sass` | `sass` | pass | 6/6 upstream fragments line-covered |
| `jscpd/fixtures/stylus` | `stylus` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/rust` | `rust` | pass | exact summary parity; 76/76 upstream fragments line-covered |
| `jscpd/fixtures/dart` | `dart` | pass | exact summary parity; 4/4 upstream fragments line-covered |
| `jscpd/fixtures/solidity` | `solidity` | pass | 4/4 upstream fragments line-covered; Rust reports one extra clone |
| `jscpd/fixtures/perl` | `perl` | pass | exact summary parity; 8/8 upstream fragments line-covered |
| `jscpd/fixtures/commonlisp` | `lisp` | pass | exact clone and line summary parity |
| `jscpd/fixtures/mllike` | `ocaml` | pass | exact clone and line summary parity |
| `jscpd/fixtures/mllike` | `fsharp` | pass | exact clone and line summary parity |
| `jscpd/fixtures/objective-c` | `objectivec` | pass | exact clone and line summary parity |
| `jscpd/fixtures/clike` | `c` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/z80` | `c` | pass | exact clone and line summary parity |
| `jscpd/fixtures/clike` | `cpp` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/clike` | `c-header` | pass | exact clone and line summary parity |
| `jscpd/fixtures/clike` | `cpp-header` | pass | exact clone and line summary parity |
| `jscpd/fixtures/clike` | `java` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/clike` | `csharp` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/clike` | `kotlin` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/clike` | `scala` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/groovy` | `groovy` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/actionscript` | `actionscript` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/awk` | `awk` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/basic` | `basic` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/coffeescript` | `coffeescript` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/crystal` | `crystal` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/d` | `d` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/elm` | `elm` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/erlang` | `erlang` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/fortran` | `fortran` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/gdscript` | `gdscript` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/graphql` | `graphql` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/julia` | `julia` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/protobuf` | `protobuf` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/ada` | `ada` | pass | exact summary parity; 6/6 upstream fragments line-covered |
| `jscpd/fixtures/apex` | `apex` | pass | exact summary parity; includes embedded SOQL as `sql` |
| `jscpd/fixtures/haxe` | `haxe` | pass | exact summary parity; 8/8 upstream fragments line-covered |
| `jscpd/fixtures/r` | `r` | pass | exact summary parity; 4/4 upstream fragments line-covered |
| `jscpd/fixtures/csv` | `csv` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/diff` | `diff` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/cmake` | `cmake` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/hcl` | `hcl` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/gitignore` | `ignore` | pass | exact clone and line summary parity |
| `jscpd/fixtures/json5` | `json5` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/latex` | `latex` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/puppet` | `puppet` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/qsharp` | `qsharp` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/racket` | `racket` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/sas` | `sas` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/scheme` | `scheme` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/vhdl` | `vhdl` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/xquery` | `xquery` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/verilog` | `verilog` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/wgsl` | `wgsl` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/zig` | `zig` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/tcl` | `tcl` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/turtle` | `turtle` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/twig` | `twig` | pass | exact upstream fragment/start and line summary parity; token totals differ slightly |
| `jscpd/fixtures/properties` | `properties` | pass | exact clone and line summary parity |
| `jscpd/fixtures/properties` | `ini` | pass | exact clone and line summary parity |
| `jscpd/fixtures/xml` | `markup` | pass | 6/6 upstream fragments line-covered; Rust skips empty XML/XSD inputs |
| `jscpd/fixtures/htmlmixed` | `markup` | pass | exact clone and line summary parity; upstream also reports embedded script/style sources |
| `jscpd/fixtures/htmlembedded` | `aspnet` | pass | 9/10 upstream fragments line-covered; one documented upstream range overextends through an inserted email block |
| `jscpd/fixtures/vb` | `vbnet` | pass | exact clone and line summary parity |
| `jscpd/fixtures/text` | `txt` | pass | exact clone and line summary parity |
| `jscpd/fixtures/robotframework` | `robotframework` | pass | 4/4 upstream fragments line-covered; upstream reports final newline as one-past-content |
| `jscpd/fixtures/tap` | `tap` | pass | exact clone and line summary parity for embedded YAML diagnostics |
| `jscpd/fixtures/textile` | `textile` | pass | exact clone summary parity |
| `jscpd/fixtures/antlr4` | `antlr4` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/apl` | `apl` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/bicep` | `bicep` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/brainfuck` | `brainfuck` | pass | 8/8 upstream fragments line-covered |
| `jscpd/fixtures/cfml` | `cfml` | pass | exact clone and line summary parity |
| `jscpd/fixtures/cfscript` | `cfscript` | pass | exact clone and line summary parity |
| `jscpd/fixtures/dot` | `dot` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/eiffel` | `eiffel` | pass | exact clone and line summary parity |
| `jscpd/fixtures/gettext` | `gettext` | pass | 2/2 upstream fragments line-covered; Rust reports extra covered ranges |
| `jscpd/fixtures/gherkin` | `gherkin` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/handlebars` | `handlebars` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/idris` | `idris` | pass | 4/4 upstream fragments line-covered |
| `jscpd/fixtures/lilypond` | `lilypond` | pass | 6/6 upstream fragments line-covered |
| `jscpd/fixtures/livescript` | `livescript` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/linker-script` | `linker-script` | pass | exact clone and line summary parity |
| `jscpd/fixtures/llvm` | `llvm` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/log` | `log` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/nsis` | `nsis` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/openqasm` | `openqasm` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/oz` | `oz` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/pascal` | `pascal` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/idl` | `prolog` | pass | exact clone and line summary parity |
| `jscpd/fixtures/plsql` | `plsql` | pass | exact clone and line summary parity |
| `jscpd/fixtures/plant-uml` | `plant-uml` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/powerquery` | `powerquery` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/purescript` | `purescript` | pass | exact clone and line summary parity |
| `jscpd/fixtures/q` | `q` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/rescript` | `rescript` | pass | exact clone and line summary parity |
| `jscpd/fixtures/smalltalk` | `smalltalk` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/smarty` | `smarty` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/soy` | `soy` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/sparql` | `sparql` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/tt2` | `tt2` | pass | exact clone and line summary parity |
| `jscpd/fixtures/unrealscript` | `unrealscript` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/velocity` | `velocity` | pass | 2/2 upstream fragments line-covered |
| `jscpd/fixtures/mathematica` | `wolfram` | pass | exact clone and line summary parity |
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
  format maps. YAML quoted scalars are kept whole and fenced gap whitespace is
  preserved enough for exact upstream Markdown clone/start and duplicated-line
  parity, while source line and token totals still differ.
- Vue, Svelte, and Astro now split embedded template/script/style/frontmatter
  regions into format maps. CSS-like style blocks skip internal whitespace
  tokens so Vue SCSS starts align with upstream, while other embedded generic
  block maps still preserve internal whitespace where it is needed for
  coverage. Their fixtures are line-covered, with remaining wider ranges from
  generic markup/style tokenization.
- Plain `markup` now extracts top-level `<script>` and `<style>` blocks into
  embedded JavaScript/TypeScript/CSS-like maps. This covers upstream mixed HTML
  fixture clones, though Rust may report a wider equivalent embedded range.
- Pug and HAML mirror Prism's multiline block behavior for fixture parity:
  `pug` keeps non-`script` dot blocks as one token, and `haml` keeps silent
  comment blocks as one token. The overextended upstream report ranges remain
  listed in `docs/upstream-bugs.md`.
- Non-native generic formats use coarse whitespace tokenization; weak mode
  strips best-effort common comment spans, including `#`, `//`, `/* */`,
  `<!-- -->`, SQL-style `--`, and Lisp/INI-style `;` comments where those
  prefixes are comments in the upstream Prism grammar.
- CSS-like generic formats split common punctuation so practical stylesheet
  clones meet upstream token thresholds without carrying a full Prism port.
- Code-like and Prism-like generic formats split common punctuation and
  operator runs so practical language fixtures meet upstream default token
  thresholds without carrying a full Prism port. This includes long-tail
  fixture formats such as YAML, INI, markup, HAML, DOT, CSV, CMake, Clojure,
  CoffeeScript, Q#, SPARQL, and Robot Framework.
- Properties uses the same generic punctuation/operator split so dotted keys
  and assignments reach upstream clone thresholds without a dedicated lexer.
- Several upstream fixture directories are gated through upstream aliases:
  `gitignore` as `ignore`, `mathematica` as `wolfram`, `idl` as `prolog`, and
  `z80` as `c`.
- ASP.NET uses the code-like generic splitter and is gated with a narrow
  documented upstream range exception for `file2.aspx:18-43`, where upstream
  reports through an inserted email field block that is not present in the
  paired source.
- Apex extracts bracketed SOQL regions into an embedded `sql` map to match
  upstream's multi-format Apex reports.
- `--mode strict` now preserves Prism-style `empty` and `new_line` whitespace
  tokens in the native JS/TS/Oxc path and the generic tokenizer. The
  JavaScript fixture has exact strict-mode summary parity.
- Extensionless names such as `Makefile` and `Dockerfile` require
  `--formats-names`, matching upstream behavior.
- Custom extension and filename mappings are supported through
  `--formats-exts`/`formatsExts` and `--formats-names`/`formatsNames`.
- Relative `ignore`/`--ignore` patterns are normalized against each configured
  scan root and the current working directory, matching upstream behavior for
  absolute scan paths outside `cwd`.
- `--noSymlinks` skips symlink scan roots as well as symlinks found during tree
  walking, matching upstream's pre-glob path filtering.
- File discovery respects the current working directory `.gitignore`, scan-root
  `.gitignore` files, `.git/info/exclude`, and the global Git excludes file
  from `git config --global core.excludesFile`.
- `--max-size`/`maxSize` follows upstream `bytes.parse` semantics, including
  decimal `kb` through `pb` values, `parseInt` fallback for non-matching
  suffixes such as `1k`, and zero-file behavior for invalid limits.
- CLI `--min-lines`, `--min-tokens`, and `--max-lines` accept upstream-style
  `parseInt` numeric prefixes, so values such as `20.9` are treated as `20`;
  missing optional values are accepted like Commander `[number]` options.
- Bare optional values for `--threshold`, `--exitCode`, `--max-size`,
  `--pattern`, `--store`, and `--store-path` follow the local upstream runtime
  behavior where upstream continues instead of failing during CLI parsing.
- Bare optional values for `--ignore`, `--ignore-pattern`, `--reporters`,
  `--mode`, `--format`, `--formats-exts`, `--formats-names`, and file-writing
  `--output` paths now mirror upstream's Commander runtime TypeError shape
  instead of failing during CLI parsing, including the different
  `fs.mkdirSync` and `path.join` error strings used by different file
  reporters.
- Malformed CLI `--formats-exts`/`--formats-names` entries without `:` now
  preserve upstream's visible `Cannot read properties of undefined` TypeError
  instead of silently ignoring the entry.
- CLI `--threshold` follows JavaScript `Number(...)` parsing for values such as
  `0x10` and `nope`, matching upstream threshold reporter behavior.
- CLI/config `exitCode` keeps the raw Node-like value until clones are found.
  Integer strings such as `0x10` exit with the matching code, while invalid,
  fractional, or bare boolean values emit the same Node-style error after
  reports are written.
- Config `minLines`, `maxLines`, and `threshold` accept string numeric values
  that upstream coerces at runtime, including JavaScript-style threshold strings
  such as `0x10`. Config `minTokens` remains intentionally strict because
  upstream's string value path can corrupt token-window indexing and crash in
  detection.
- Invalid `--mode` values fail after CLI parsing with the upstream-style
  `Error: Mode ... does not supported yet.` message printed to stdout.
- If discovery, size, or line filters leave no files to detect, reporters are
  not run, matching upstream's `InFilesDetector` early return. Silent mode
  stays quiet; non-silent mode only prints the terminal footer.
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
- `--store <name>` currently follows the upstream missing-store fallback shape
  in both CLI and server entrypoints: it warns that the store package is not
  installed and continues with in-memory detection. Dynamic loading of external
  store packages remains an implementation gap.
- `--debug` is a dry run like upstream: it prints JS-style option fields and
  discovered files, then exits before clone detection and reporter execution.
- Explicit `--config` paths are resolved lexically like Node `path.resolve()`,
  without canonicalizing symlinks, so config-relative options use the visible
  config path's directory.
- `--list` follows the upstream output shape: a `Supported formats:` header
  followed by comma-separated formats.
- Non-silent runs print clone progress for non-`ai` reporters, then reporter
  output, then a `time:` footer. Tips are printed by default and suppressed by
  `--noTips`, matching the upstream workflow shape.
- Reporter normalization mirrors upstream append behavior: explicit `silent`
  or `threshold` reporters are not deduplicated when `--silent` or
  `--threshold` appends the same reporter.
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

Latest public benchmark suite checks, using repositories cloned outside the
project tree:

| Target | Commit | Format | Rust avg | Upstream avg | Approx speedup |
| --- | --- | --- | ---: | ---: | ---: |
| `facebook/react` | `f0dfee3` | `javascript` | `0.192096s` | `9.946562s` | `51.78x` |
| `vercel/next.js` | `2bbb67b9` | `typescript` | `0.248940s` | `14.282248s` | `57.37x` |
| `prometheus/prometheus` | `a0524ee` | `go` | `0.082703s` | `4.609423s` | `55.73x` |

## Additional Mode Checks

```bash
DETECTION_MODE=strict FORMAT=javascript MIN_TOKENS=20 MIN_LINES=3 MAX_SIZE=1mb \
  STRICT=coverage scripts/compat.sh jscpd/fixtures/javascript
```

The default matrix also includes strict JavaScript/TypeScript and weak
JavaScript mode checks so mode regressions are gated directly. Strict mode uses
the same coverage-first release rule; token totals remain diagnostic because
the native token stream may split whitespace differently from Prism while still
covering every upstream duplicated line.
