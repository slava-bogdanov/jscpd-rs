# Upstream Issue Drafts

These drafts are prepared from `docs/upstream-bugs.md` for filing issues in
`kucherenko/jscpd`. Re-verify each issue against current upstream `main` before
posting, and replace local absolute paths with a public fixture or reproduction
repository when needed.

## Draft 1: JavaScript tokenizer misparses template literals in minified code

Suggested title:

```text
JavaScript tokenizer misparses template literals and swallows following code
```

Suggested labels: `bug`, `javascript`, `tokenizer`.

Summary:

The Prism-backed JavaScript tokenizer can treat code after a nested template
literal as one large string token. A related case treats `//` inside a template
literal as a line comment that consumes the rest of a long minified line. This
causes valid duplicated JavaScript regions to be missed.

Repro shape:

```bash
FORMAT=javascript MIN_TOKENS=50 MIN_LINES=5 MAX_SIZE=1mb KEEP=1 \
  scripts/compat.sh /path/to/generated-nextjs-or-ssr-output
```

Observed symptoms:

- For a nested template expression like
  `` `${path}?url=${encodeURIComponent(url)}&w=${w}&q=${q}${url.startsWith("/")&&dpl?`&dpl=${dpl}`:""}` ``,
  upstream emits one very large `string` token for ordinary following module
  code.
- For a template literal like `` `${a}//${b}${c?":"+c:""}` ``, upstream emits a
  very large `comment` token starting at the `//` sequence inside the template.

Expected behavior:

The tokenizer should resume JavaScript tokenization after nested template
literals, and `//` inside template literal text should not start a line comment.

Impact:

Generated/minified SSR bundles can lose duplicate coverage because large parts
of the module body are hidden inside one incorrect token.

## Draft 2: `--blame` fails for paths inside a nested Git repository

Suggested title:

```text
--blame fails when scanned files are inside a nested Git repository or submodule
```

Suggested labels: `bug`, `blame`, `git`.

Summary:

When `jscpd` is launched from a parent repository and the scan target is inside a
nested Git repository or submodule, `--blame` invokes `git blame` from the
parent working directory with the nested file path. The parent repository does
not track that nested file as a normal file, so `git blame` exits with 128 and
the whole detection run fails.

Repro:

```bash
node jscpd/apps/jscpd/bin/jscpd jscpd/fixtures/javascript \
  --format javascript \
  --reporters json \
  --output /tmp/jscpd-upstream-blame \
  --silent \
  --noTips \
  --blame \
  --min-tokens 20 \
  --min-lines 3 \
  --max-size 1mb \
  --exitCode 0
```

Observed first error:

```text
Error: Command failed with exit code 128: /usr/bin/git blame -w jscpd/fixtures/javascript/file_4.js
fatal: no such path 'jscpd/fixtures/javascript/file_4.js' in HEAD
```

Expected behavior:

Blame should run from the scanned file's own repository/worktree, or blame
should fail per file without aborting the whole detection run.

## Draft 3: Clone ranges can extend through non-matching embedded/template blocks

Suggested title:

```text
Clone ranges can extend through non-matching embedded or template blocks
```

Suggested labels: `bug`, `detector`, `reporting`.

Summary:

Some reported clone ranges include neighboring source that does not match the
paired fragment. The issue is visible in fixture formats that contain large
block tokens or embedded markup.

Fixture repros:

```bash
FORMAT=pug MIN_TOKENS=20 MIN_LINES=3 MAX_SIZE=1mb KEEP=1 \
  scripts/compat.sh jscpd/fixtures/pug

FORMAT=haml MIN_TOKENS=20 MIN_LINES=3 MAX_SIZE=1mb KEEP=1 \
  scripts/compat.sh jscpd/fixtures/haml

FORMAT=aspnet MIN_TOKENS=20 MIN_LINES=3 MAX_SIZE=1mb KEEP=1 \
  scripts/compat.sh jscpd/fixtures/htmlembedded
```

Observed examples:

- Pug reports `file1.pug:1-274` against `file2.pug:1-266`, including a
  `style.` plain-text block whose CSS values differ.
- HAML reports `file1.haml:1-26` against `file2.haml:1-26`, including
  different silent-comment blocks.
- ASP.NET reports `file1.aspx:18-36` against `file2.aspx:18-43`, including an
  inserted email form group that is not present in the paired file.

Expected behavior:

Clone ranges should stop at the last matching token run, split around inserted
content, or document that specific ignored block types may intentionally extend
reported source ranges through non-matching text.

## Draft 4: Public benchmark clone ranges are sometimes overextended or reversed

Suggested title:

```text
Some clone ranges are overextended or reversed on large public repositories
```

Suggested labels: `bug`, `detector`, `reporting`.

Summary:

Large-repository runs show reported fragments whose line ranges extend across
neighboring non-matching tests, table entries, or declarations. Several ranges
also have reversed start/end ordering.

Repro shapes:

```bash
FORMAT=javascript MIN_TOKENS=50 MIN_LINES=5 MAX_SIZE=1mb KEEP=1 \
  scripts/compat.sh /path/to/react

FORMAT=typescript MIN_TOKENS=50 MIN_LINES=5 MAX_SIZE=1mb STRICT=coverage KEEP=1 \
  scripts/compat.sh /path/to/next

FORMAT=go MIN_TOKENS=50 MIN_LINES=5 MAX_SIZE=1mb STRICT=coverage KEEP=1 \
  scripts/compat.sh /path/to/prometheus
```

Observed examples:

- React reports ranges such as `ReactDOMFizzServerNode.js:229-179` and clone
  fragments that continue into neighboring test bodies.
- Next.js reports broad ranges around inline snapshots and multi-test files,
  including reversed endpoints like `459-314` and `892-745`.
- Prometheus reports broad table-test ranges where one early case is stretched
  through unrelated later cases.

Expected behavior:

Reported fragments should keep start/end ordering stable and should stop at the
actual matching token run instead of stretching one match across unrelated
neighboring source.

## Draft 5: Config string `minTokens` can corrupt token-window indexing

Suggested title:

```text
String minTokens values from config can corrupt detector token windows
```

Suggested labels: `bug`, `config`.

Summary:

Runtime config loaded from `.jscpd.json` or `package.json#jscpd` is merged
without the CLI numeric parser. Numeric-looking strings for some fields continue
through JavaScript coercion, but `minTokens` is later used in token-window
indexing with `+` before numeric subtraction. A value such as `"5"` can produce
string-concatenated indices and eventually an undefined token frame.

Expected behavior:

Config numeric fields should be parsed and validated before detector execution,
or invalid string values should fail with a clear configuration error.

## Draft 6: Public option fields are exposed but unused at runtime

Suggested title:

```text
Some public option fields are exposed but unused at runtime
```

Suggested labels: `bug`, `options`, `documentation`.

Summary:

The option surface exposes fields that look user-facing but are not consumed by
the runtime:

- `cache` is defined and defaulted, but there is no `--cache` CLI option and no
  detector/tokenizer read of `options.cache`.
- `listeners` is normalized to an array, but runtime subscriptions come only
  from built-in verbose/progress handling.
- `tokensToSkip` appears in the options interface but is not consumed by
  tokenizer or detector code.

Expected behavior:

These fields should either be documented as reserved/no-op, removed from the
public option surface, or wired to actual runtime behavior.

## Draft 7: Optional CLI values can produce TypeErrors or accidental behavior

Suggested title:

```text
Bare optional CLI flags can produce TypeErrors or accidental behavior
```

Suggested labels: `bug`, `cli`.

Summary:

Several Commander options accept optional values. When passed without a value,
Commander supplies boolean `true`, and later runtime code either crashes with a
TypeError or continues with surprising semantics.

Repro examples:

```bash
node jscpd/apps/jscpd/bin/jscpd jscpd/fixtures/javascript \
  --threshold \
  --silent \
  --noTips \
  --min-tokens 20 \
  --min-lines 3 \
  --max-size 1mb

node jscpd/apps/jscpd/bin/jscpd jscpd/fixtures/javascript \
  --exitCode \
  --silent \
  --noTips \
  --min-tokens 20 \
  --min-lines 3 \
  --max-size 1mb

node jscpd/apps/jscpd/bin/jscpd jscpd/fixtures/custom \
  --formats-exts javascript \
  --silent \
  --noTips \
  --min-tokens 20 \
  --min-lines 3 \
  --max-size 1mb
```

Observed behavior:

- Bare `--threshold` becomes `Number(true)`, so the threshold is treated as
  `1%`.
- Bare `--exitCode` stores boolean `true`, which Node rejects as
  `process.exitCode` when clones are found.
- Bare string flags such as `--ignore`, `--reporters`, `--mode`, `--format`,
  `--formats-exts`, and `--formats-names` later crash because boolean `true`
  is used as a string.
- Malformed mapping values like `--formats-exts javascript` crash during option
  conversion because the parser assumes each entry contains `:`.

Expected behavior:

Flags that require values should declare required values, validate the bare flag
case explicitly, or normalize bare flags to documented defaults before option
conversion.
