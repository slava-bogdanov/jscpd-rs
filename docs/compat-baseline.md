# Compatibility Baseline

Baseline date: 2026-05-30.

Default gate:

```bash
STRICT=coverage scripts/compat-matrix.sh
```

Coverage means every upstream duplicated line must be covered by the Rust report
for the same file, and Rust must not report fewer clones. Exact clone starts,
formats, fragment boundaries, source totals, line totals, and pair ordering are
diagnostic only because Rust may find a wider or split equivalent range while
compatibility is converging.

## Current Matrix

| Target | Format | Gate | Notes |
| --- | --- | --- | --- |
| `jscpd/fixtures` | `javascript` | pass | exact summary parity |
| `jscpd/fixtures` | `typescript` | pass | exact summary parity |
| `jscpd/fixtures/javascript` | `json` | pass | exact clone and line summary parity |
| `jscpd/fixtures/javascript` | `javascript` / `strict` | pass | exact summary parity |
| `jscpd/fixtures` | `typescript` / `strict` | pass | exact summary parity |
| `jscpd/fixtures/javascript` | `javascript` / `weak` | pass | clone and line summary parity; token totals differ slightly |
| `jscpd/fixtures` | `jsx` | pass | token totals differ slightly; fragments covered |
| `jscpd/fixtures` | `tsx` | pass | token totals differ slightly; fragments covered |
| `jscpd/fixtures/markdown` | `markdown` | pass | 18/18 upstream fragments line-covered; Rust reports wider/split ranges |
| `jscpd/fixtures` | `vue` | pass | 18/18 upstream fragments line-covered; exact starts differ for wider markup/scss ranges |
| `jscpd/fixtures` | `svelte` | pass | 6/6 upstream fragments line-covered; exact start differs for wider css range |
| `jscpd/fixtures` | `astro` | pass | 8/8 upstream fragments line-covered; exact starts differ for wider markup/css ranges |
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
| `jscpd/fixtures/twig` | `twig` | pass | 6/6 upstream fragments line-covered |
| `jscpd/fixtures/properties` | `properties` | pass | exact clone and line summary parity |
| `jscpd/fixtures/properties` | `ini` | pass | exact clone and line summary parity |
| `jscpd/fixtures/xml` | `markup` | pass | 6/6 upstream fragments line-covered; Rust skips empty XML/XSD inputs |
| `jscpd/fixtures/htmlmixed` | `markup` | pass | exact clone and line summary parity; upstream also reports embedded script/style sources |
| `jscpd/fixtures/htmlembedded` | `aspnet` | pass | 9/10 upstream fragments line-covered; one documented upstream range overextends through an inserted email block |
| `jscpd/fixtures/vb` | `vbnet` | pass | exact clone and line summary parity |
| `jscpd/fixtures/text` | `txt` | pass | exact clone and line summary parity |
| `jscpd/fixtures/robotframework` | `robotframework` | pass | 4/4 upstream fragments line-covered; upstream reports final newline as one-past-content |
| `jscpd/fixtures/tap` | `tap` | pass | upstream YAML embedded block is covered by a wider TAP clone |
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
  format maps. The upstream Markdown fixture is line-covered, though exact
  starts still differ where Rust reports wider/split ranges.
- Vue, Svelte, and Astro now split embedded template/script/style/frontmatter
  regions into format maps. Their fixtures are line-covered, with expected
  wider ranges from generic markup/style tokenization.
- Non-native generic formats use coarse whitespace tokenization; weak mode
  strips best-effort common comment spans, including `#`, `//`, `/* */`,
  `<!-- -->`, SQL-style `--`, and Lisp/INI-style `;` comments where those
  prefixes are comments in the upstream Prism grammar.
- CSS-like generic formats split common punctuation so practical stylesheet
  clones meet upstream token thresholds without carrying a full Prism port.
- Code-like generic formats split common punctuation and operator runs so
  practical language fixtures meet upstream token thresholds without carrying
  a full Prism port.
- Properties uses the same generic punctuation/operator split so dotted keys
  and assignments reach upstream clone thresholds without a dedicated lexer.
- Several upstream fixture directories are gated through upstream aliases:
  `gitignore` as `ignore`, `mathematica` as `wolfram`, `idl` as `prolog`, and
  `z80` as `c`.
- The remaining upstream `formats.test.ts` fixture formats not in the green
  matrix are `pug` and `haml`; both are tracked as upstream bug candidates.
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
