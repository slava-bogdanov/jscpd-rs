# Upstream Bug Candidates

These are compatibility findings that look like upstream `jscpd` issues rather
than Rust clone issues. Verify each against the current upstream main branch
before filing.

## Prism JS tokenizer swallows code after nested template literals

Status: observed on `jscpd` submodule during compatibility work.

Repro target:

```sh
FORMAT=javascript MIN_TOKENS=50 MIN_LINES=5 MAX_SIZE=1mb KEEP=1 scripts/compat.sh /home/dev/dream
```

Observed mismatch:

- Upstream reports a clone between:
  - `../dream/landing/.next/dev/server/chunks/ssr/[root-of-the-server]__04xj076._.js:166`
  - `../dream/landing/.next/standalone/.next/server/chunks/[root-of-the-server]__0b-rble._.js:18`
- The Rust/Oxc tokenizer also sees the equivalent clone in:
  - `../dream/landing/.next/standalone/.next/server/chunks/ssr/_0vnreey._.js:3`
- Upstream does not see that second candidate because Prism tokenizes a large
  minified JS range as one `string` token.

Concrete tokenization symptom in upstream tokenizer:

```text
line 3 column 284: string token length ~8194
starts with: `}f.__next_img_default=!0;let g=f},67161,...
ends before: `locale-option...
```

The source pattern around the start is a nested template expression in minified
JavaScript:

```js
return`${a.path}?url=${encodeURIComponent(b)}&w=${c}&q=${i}${b.startsWith("/")&&h?`&dpl=${h}`:""}`}...
```

Expected behavior: the tokenizer should continue parsing JavaScript after the
outer template literal instead of treating the following module body as a single
template/string token.

## Prism JS tokenizer treats `//` inside a template literal as a line comment

Status: observed on `jscpd` submodule during compatibility work.

Related repro target:

```sh
FORMAT=javascript MIN_TOKENS=50 MIN_LINES=5 MAX_SIZE=1mb KEEP=1 scripts/compat.sh /home/dev/dream
```

In another generated SSR chunk, upstream tokenizes a `//` sequence inside a
template literal as a comment that runs to the end of a very large minified
line.

Observed source pattern:

```js
return`${a}//${b}${c?":"+c:""}`
```

Concrete tokenization symptom in upstream tokenizer:

```text
line 3 column 7270: comment token length ~300232
starts with: //${b}${c?":"+c:""}...
contains later ordinary module code and localeConfig data
```

Expected behavior: the `//` text inside the template literal should remain a
template string segment and should not comment out the rest of the generated
line.

## `--blame` fails for files inside a nested Git repository when run from the parent repo

Status: observed on the `jscpd` submodule during compatibility work.

Repro from the Rust clone repository root, where `jscpd/` is a Git submodule:

```sh
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

Observed failure:

```text
Error: Command failed with exit code 128: /usr/bin/git blame -w jscpd/fixtures/javascript/file_4.js
fatal: no such path 'jscpd/fixtures/javascript/file_4.js' in HEAD
```

The failure comes from `blamer@1.0.7`, which invokes `git blame -w <path>` from
the current process directory. When the scanned path is inside a nested Git
repository or submodule, the parent repository does not track the nested file
path as a regular file, so `git blame` exits with 128.

Expected behavior: blame should run from the file's own repository/worktree, or
fail per file without aborting the entire detection run.

## Pug report overextends a clone into a non-matching `style.` block

Status: observed on the `jscpd` submodule during compatibility work. The Rust
clone currently mirrors this range behavior for compatibility.

Repro target:

```sh
FORMAT=pug MIN_TOKENS=20 MIN_LINES=3 MAX_SIZE=1mb KEEP=1 scripts/compat.sh jscpd/fixtures/pug
```

Observed upstream clone:

- `jscpd/fixtures/pug/file1.pug:1-274`
- `jscpd/fixtures/pug/file2.pug:1-266`

Those ranges include the `style.` multiline plain-text block. The upstream
tokenizer emits that block as one `multiline-plain-text` token:

```text
file1.pug: token 391, lines 49-274, length 5278, md5 prefix 8a231
file2.pug: token 391, lines 49-266, length 5115, md5 prefix 9eaf8
```

The token values differ: `file1.pug` contains extra `.clones-excellent` and
`.clones-fine` CSS blocks before `.clones-danger`, while `file2.pug` jumps
directly from `.stats` to `.clones-danger`.

Expected behavior: the reported clone range should stop before the non-matching
multiline token, or the tokenizer should split the `style.` content so only
matching CSS ranges are reported.

## HAML report overextends a clone into a non-matching comment block

Status: observed on the `jscpd` submodule during compatibility work. The Rust
clone currently mirrors this range behavior for compatibility.

Repro target:

```sh
FORMAT=haml MIN_TOKENS=20 MIN_LINES=3 MAX_SIZE=1mb KEEP=1 scripts/compat.sh jscpd/fixtures/haml
```

Observed upstream clone:

- `jscpd/fixtures/haml/file1.haml:1-26`
- `jscpd/fixtures/haml/file2.haml:1-26`

The ranges include a HAML silent-comment block whose visible source differs:

```haml
-# File-specific: user settings section
  .settings-section
    %h2 Account Settings
    %p Change your password and security preferences.
```

versus:

```haml
-# File-specific: notification preferences
  .notifications-section
    %h2 Notification Preferences
    %p Manage how you receive alerts and updates.
```

Expected behavior: the reported clone should stop before the differing
commented block, or the tokenizer/report range logic should document that HAML
silent comments are ignored and may extend clone ranges through non-matching
source text.

## ASP.NET report overextends a clone through an inserted email form group

Status: observed on the `jscpd` submodule during compatibility work.

Repro target:

```sh
FORMAT=aspnet MIN_TOKENS=20 MIN_LINES=3 MAX_SIZE=1mb KEEP=1 scripts/compat.sh jscpd/fixtures/htmlembedded
```

Observed upstream clone:

- `jscpd/fixtures/htmlembedded/file1.aspx:18-36`
- `jscpd/fixtures/htmlembedded/file2.aspx:18-43`

The `file2.aspx` range includes an inserted email field group on lines 36-42:

```aspx
<div class="form-group">
<asp:Label ID="lblEmail" runat="server" AssociatedControlID="txtEmail" Text="Email:" />
<asp:TextBox ID="txtEmail" runat="server" CssClass="form-control" MaxLength="255" />
<asp:RequiredFieldValidator ID="rfvEmail" runat="server"
    ControlToValidate="txtEmail" ErrorMessage="Email is required"
    CssClass="text-danger" Display="Dynamic" />
</div>
```

Those controls are not present in the paired `file1.aspx` range. The upstream
Prism tokens also keep distinct values such as `lblEmail`, `txtEmail`,
`Email:`, `rfvEmail`, and `Email is required`, so this does not look like a
normal token normalization difference.

Expected behavior: the reported clone should stop before the inserted email
group, split around it, or report only the structurally duplicated subranges.

## React public benchmark reports overextended JavaScript clone ranges

Status: observed on the `jscpd` submodule during React public benchmark
compatibility work.

Repro target:

```sh
FORMAT=javascript MIN_TOKENS=50 MIN_LINES=5 MAX_SIZE=1mb KEEP=1 \
  scripts/compat.sh /home/dev/.cache/jscpd-rs/public-bench/repos/react/.
```

After the Rust clone covers the real duplicated subranges, three upstream
fragments still look overextended rather than genuinely missed:

- `SyntheticMouseEvent-test.js:21-38`: upstream pairs
  `SyntheticClipboardEvent-test.js:20-34` with
  `SyntheticMouseEvent-test.js:21-38`. Lines 21-35 are duplicated setup code,
  but lines 36-38 already enter the `onMouseMove` test body and do not match
  the clipboard test's nested `describe`/`it` block.
- `ReactDOMFizzServerNode.js:179-229`: one upstream clone reports the Node
  fragment as `229-179` against `ReactDOMFizzServerEdge.js:92-165`, producing a
  reversed range. The Rust clone covers the surrounding real duplicated
  subranges, but not the reversed overextension gap.
- `ReactDOMViewTransition-test.js:39-135`: upstream pairs
  `ReactDOMSuspensePlaceholder-test.js:37-109` with
  `ReactDOMViewTransition-test.js:39-135`. Lines 39-111 cover the shared test
  helpers; lines 112-135 enter a ViewTransition-specific SuspenseList test and
  do not correspond to the SuspensePlaceholder range.

Expected behavior: clone fragments should stop at the last matching token range,
or the detector should split separate duplicated subranges instead of extending
through neighboring non-matching test code.

## Next.js TypeScript public benchmark overextended report ranges

Status: observed on the `next` public benchmark at commit `2bbb67b9` during
coverage-first compatibility work.

Repro target:

```sh
FORMAT=typescript MIN_TOKENS=50 MIN_LINES=5 MAX_SIZE=1mb STRICT=coverage KEEP=1 \
  scripts/compat.sh /home/dev/.cache/jscpd-rs/public-bench/repos/next/.
```

After matching the upstream `console-exit` template interpolation behavior and
TypeScript array-regex tokenization, the Rust clone covers `3900/3908` upstream
clone fragments on this benchmark. The remaining missing coverage is dominated
by upstream fragments that extend across unrelated neighboring test cases or
through reversed/oversized ranges:

- `next-style-loader/index.ts:221-229`: upstream tokenizes generated JS inside a
  template literal as ordinary TypeScript and reports a clone against
  `154-165`. A broad Rust experiment that tokenized code-like template raw text
  did cover this fragment, but it increased overall Next missing coverage and
  token volume, so it was rejected as too invasive.
- `non-root-project-monorepo.test.ts:221-240` and `284-303`: inline snapshot
  blocks with similar stack traces; upstream starts/ends inside snapshot text.
- `normalize-next-data.test.ts:185-681`: upstream pairs a 22-line later test
  block with a 497-line earlier range. Rust covers the actual smaller repeated
  route-normalization blocks around that area, but not the whole overextended
  range.
- `edge-runtime-module-errors.test.ts:314-459` and `745-892`: upstream contains
  several useful repeated subranges, but some reported pairs have reversed or
  overextended endpoints such as `459-314` and `892-745`.
- `next-rs-api.test.ts:175-203` and `327-356`: a real repeated config object
  body with an upstream start before the stable matching token run.

Expected behavior: clone fragments should be split at the actual matching token
runs and should not report reversed or multi-test overextended ranges.

## Prometheus Go public benchmark overextended report ranges

Status: observed on the `prometheus` public benchmark at commit `a0524ee`
during coverage-first compatibility work.

Repro target:

```sh
FORMAT=go MIN_TOKENS=50 MIN_LINES=5 MAX_SIZE=1mb STRICT=coverage KEEP=1 \
  scripts/compat.sh /home/dev/.cache/jscpd-rs/public-bench/repos/prometheus/.
```

The Rust clone reports more Go clones overall on this benchmark, but upstream
still has a small set of fragments whose line ranges extend beyond the matching
token run. Several are reversed ranges, and several table-driven test cases use
one early case as the paired fragment for many later cases, which makes the
reported early range span unrelated intervening cases. The public benchmark
gate allows these exact ranges while keeping them visible as ignored exceptions:

- `storage/remote/write_test.go:214-221` and `240-249`: upstream starts on the
  tail of a different config-call line; Rust starts at the following shared
  block and covers the real repeated assertions.
- `storage/remote/read_test.go:339-414`: upstream reports a reversed fragment
  (`414-339`) across multiple table entries; Rust covers the smaller repeated
  entries around that region.
- `discovery/marathon/marathon_test.go:325-478`: upstream reports a reversed
  table-test range twice.
- `discovery/hetzner/mock_test.go:58-457` and `464-517`: upstream pairs a very
  large mock implementation range with a later smaller block.
- `discovery/triton/triton.go:90-136` and `discovery/gce/gce.go:91-117`:
  upstream extends structurally similar config validation clones into
  neighboring declarations.
- `cmd/promtool/main_test.go:250-256` and `250-258`: upstream reports two
  overlapping partial ranges for the same test setup.
- `tsdb/head_read_test.go:73-94`, `122-171`, `122-213`, and `122-280`:
  upstream repeatedly pairs later table entries with broad earlier table-entry
  spans instead of the closest equivalent repeated case.
- `rules/group_test.go:42-67`: upstream includes adjacent setup lines around
  the repeated body.

Expected behavior: clone fragments should stop at the matching token run, keep
start/end ordering stable, and avoid stretching one table-driven test case
through unrelated neighboring cases.

## Option fields are exposed but unused at runtime

Status: observed on the `jscpd` submodule during compatibility work.

The option surface contains fields that look like user-facing workflow hooks,
but the current CLI/runtime does not consume them after option parsing or
defaulting:

- `cache` is defined in
  `jscpd/packages/core/src/interfaces/options.interface.ts`, defaults to `true`
  in `jscpd/packages/core/src/options.ts`, and is copied from the CLI object in
  `jscpd/apps/jscpd/src/options.ts`. There is no `--cache` CLI option and no
  runtime read of `options.cache` in core/finder/tokenizer.
- `listeners` is defined in the options interface and normalized to `[]` in
  `jscpd/apps/jscpd/src/options.ts`, but runtime subscribers are registered
  only from built-in `verbose` and progress rules.
- `tokensToSkip` appears only in the options interface. It is not consumed by
  tokenization or detector code.

Expected behavior: either document these fields as reserved/no-op, remove them
from the public option surface, or wire them to runtime behavior.

## Bare optional numeric CLI flags produce accidental behavior

Status: observed on the `jscpd` submodule during compatibility work.

Several Commander options are declared with optional numeric values, for example
`--threshold [number]` and `--exitCode [number]`. When the flag is passed
without a value, Commander supplies boolean `true`.

Repro:

```sh
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
```

Observed behavior:

- Bare `--threshold` is converted with `Number(true)`, so the threshold becomes
  `1%` and detection fails if duplication is at least 1%.
- Bare `--exitCode` stores boolean `true`; when clones are found, Node rejects
  that boolean as `process.exitCode` with `TypeError [ERR_INVALID_ARG_TYPE]`.

Expected behavior: require a numeric value, or explicitly document and normalize
the default value for bare flags.

Rust clone handling: bare `--threshold` and bare `--exitCode` are mirrored for
CLI compatibility. The `--exitCode` behavior remains an upstream bug candidate,
but preserving it is cheaper than leaving a visible CLI parity gap.

## Bare optional string CLI flags produce inconsistent failures

Status: observed on the `jscpd` submodule during compatibility work.

Several Commander string options are declared with optional values. When the
flag is passed without a value, Commander supplies boolean `true`, and later
runtime code either crashes with a type error or continues depending on whether
that option is used.

Repro shape:

```sh
node jscpd/apps/jscpd/bin/jscpd <flag> \
  --silent \
  --noTips \
  jscpd/fixtures/clike/file2.c \
  --min-tokens 20 \
  --min-lines 3 \
  --max-size 1mb
```

Observed first stdout lines:

| Flag | Exit | First line |
| --- | ---: | --- |
| `--config` | 1 | `TypeError [ERR_INVALID_ARG_TYPE]: The "paths[0]" argument must be of type string. Received type boolean (true)` |
| `--ignore` | 1 | `TypeError: cli.ignore.split is not a function` |
| `--ignore-pattern` | 1 | `TypeError: cli.ignorePattern.split is not a function` |
| `--reporters` | 1 | `TypeError: cli.reporters.split is not a function` |
| `--mode` | 1 | `TypeError: mode is not a function` |
| `--format` | 1 | `TypeError: cli.format.split is not a function` |
| `--formats-exts` | 1 | `TypeError: extensions.split is not a function` |
| `--formats-names` | 1 | `TypeError: extensions.split is not a function` |
| `--output` | 0 | continues when no file-writing reporter uses `output` |

`--output --reporters json` later fails when the JSON reporter passes boolean
`true` to filesystem path creation.

Expected behavior: require string values for these flags, or normalize bare
flags before option conversion.

Rust clone handling: only low-risk bare-value cases that upstream continues with
are mirrored by default. These crash-only edge cases remain documented until a
release gate makes them worth preserving exactly.
