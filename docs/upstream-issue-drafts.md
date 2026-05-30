# Upstream Issue Drafts

These drafts are prepared from `docs/upstream-bugs.md` for filing issues in
`kucherenko/jscpd`. Re-verify each issue against current upstream `main` before
posting. Drafts that use public repositories include pinned commits; the other
drafts use upstream fixtures or minimal inline reproductions.

Verification snapshot: the quick repros for Drafts 1, 2, 3, 5, 6, and 7 were
checked on 2026-05-31 against upstream submodule `50290cf`. Draft 4 is covered
by the public benchmark compatibility gate recorded in
`docs/compat-baseline.md`.

## Draft 1: JavaScript tokenizer treats `//` inside a template literal as a comment

Suggested title:

```text
JavaScript tokenizer treats `//` inside a template literal as a line comment
```

Suggested labels: `bug`, `javascript`, `tokenizer`.

Summary:

The Prism-backed JavaScript tokenizer can treat `//` inside a JavaScript
template literal as a line comment. The resulting comment token consumes the
rest of the physical line, including ordinary JavaScript after the template
literal. This can hide valid duplicated code behind one large comment token in
minified or bundled JavaScript.

Minimal tokenizer repro:

Run this from the upstream `jscpd` repository after building packages:

```bash
node - <<'NODE'
const { Tokenizer } = require('./packages/tokenizer/dist/index.js');
const { mild } = require('./packages/core/dist/index.js');

const code =
  'function h(a){let j="//";return`${j}`}function j(){let{protocol:a,hostname:b,port:c}=window.location;return`${a}//${b}${c?":"+c:""}`}function k(){return 1}\n';

const tokens = new Tokenizer()
  .generateMaps('repro.js', code, 'javascript', { minTokens: 1, mode: mild })[0]
  .tokens;

for (const token of tokens.filter((t) => t.type === 'comment' || t.value.includes('//'))) {
  console.log(`${token.type} ${JSON.stringify(token.value)} line=${token.loc.start.line} col=${token.loc.start.column}`);
}
NODE
```

Observed symptoms:

- The first `//` string literal is emitted correctly as a `string` token.
- The `//` inside the template literal is emitted as a `comment` token:

```text
string "\"//\"" line=1 col=21
comment "//${b}${c?\":\"+c:\"\"}`}function k(){return 1}" line=1 col=113
```

Expected behavior:

`//` inside template literal text should remain a template string segment and
should not comment out the rest of the generated line.

Impact:

Generated/minified SSR bundles can lose duplicate coverage because ordinary
module code after the template literal is hidden inside one incorrect comment
token.

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

Observed first error on the current submodule:

```text
Error: Command failed with exit code 128: /usr/bin/git blame -w jscpd/fixtures/javascript/file_2.mjs
fatal: no such path 'jscpd/fixtures/javascript/file_2.mjs' in HEAD
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
git clone https://github.com/facebook/react.git /tmp/jscpd-react
git -C /tmp/jscpd-react checkout f0dfee3
node apps/jscpd/bin/jscpd /tmp/jscpd-react \
  --format javascript \
  --reporters json \
  --output /tmp/jscpd-react-report \
  --silent \
  --noTips \
  --min-tokens 50 \
  --min-lines 5 \
  --max-size 1mb \
  --exitCode 0

git clone https://github.com/vercel/next.js.git /tmp/jscpd-next
git -C /tmp/jscpd-next checkout 2bbb67b9
node apps/jscpd/bin/jscpd /tmp/jscpd-next \
  --format typescript \
  --reporters json \
  --output /tmp/jscpd-next-report \
  --silent \
  --noTips \
  --min-tokens 50 \
  --min-lines 5 \
  --max-size 1mb \
  --exitCode 0

git clone https://github.com/prometheus/prometheus.git /tmp/jscpd-prometheus
git -C /tmp/jscpd-prometheus checkout a0524ee
node apps/jscpd/bin/jscpd /tmp/jscpd-prometheus \
  --format go \
  --reporters json \
  --output /tmp/jscpd-prometheus-report \
  --silent \
  --noTips \
  --min-tokens 50 \
  --min-lines 5 \
  --max-size 1mb \
  --exitCode 0
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

Minimal repro:

Run this from the upstream `jscpd` repository root:

```bash
tmp=$(mktemp -d)
JSCPD_REPO=$(pwd)
mkdir -p "$tmp/src"
printf 'function alpha(){\n  return [1,2,3,4,5,6,7,8,9,10].map((x)=>x+1).join(",");\n}\nfunction beta(){\n  return alpha();\n}\n' > "$tmp/src/a.js"
cp "$tmp/src/a.js" "$tmp/src/b.js"
printf '{"path":["src"],"format":["javascript"],"reporters":["json"],"silent":true,"minTokens":"5","minLines":1,"maxSize":"1mb","exitCode":0}\n' > "$tmp/.jscpd.json"

cd "$tmp"
node "$JSCPD_REPO/apps/jscpd/bin/jscpd" --config .jscpd.json --noTips
```

Observed first error:

```text
TypeError: Cannot read properties of undefined (reading 'range')
    at _RabinKarp.enlargeClone (.../packages/core/dist/index.js:100:49)
```

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
