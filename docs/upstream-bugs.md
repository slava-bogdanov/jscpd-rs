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
