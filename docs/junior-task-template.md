# Junior Task Template

Use this as the prompt body for `pi --no-session --tools read,edit,bash -p`.

```text
Junior helper task.
Goal: <one concrete deliverable>
Scope: edit only <exact file list>; read <exact reference files>.
Allowed actions:
- read <files>
- edit <files>
- run exactly `<verification command>` from <worktree path>
Verification: <exact command and expected high-level result>
Rules:
- stay in scope
- follow the nearest existing pattern
- do not change production logic unless explicitly requested
- do not edit generated files unless the task says so
- do not touch external repositories
- stop on blockers instead of guessing
Output: Russian report with Result, Evidence, Verification, Blockers.
```

## Example: Add One Test

```text
Junior helper task.
Goal: Add one unit test proving weak mode skips TOML hash comments in the generic tokenizer.
Scope: edit only src/tokenizer.rs, preferably only the #[cfg(test)] module.
Allowed actions:
- read src/tokenizer.rs
- edit src/tokenizer.rs
- run exactly `cargo test tokenizer::tests::weak_mode_skips_generic_toml_comments` from /tmp/jscpd-rs-junior/toml-comments
Verification: the exact cargo test above passes.
Rules:
- follow the existing weak_mode_skips_generic_comments test style
- do not change production tokenizer logic
- use format `toml`
- assert exact token slices for the non-comment tokens
- stop on blockers
Output: Russian report with Result, Evidence, Verification, Blockers.
```

## Example: Add One Format Smoke Fixture

```text
Junior helper task.
Goal: Add a tiny smoke fixture and test for format <format>.
Scope: edit only <fixture files> and <one test file>.
Allowed actions:
- read docs/format-porting.md
- read the nearest existing test
- edit only the files listed in Scope
- run exactly `scripts/check-format.sh <format> <target>` from /tmp/jscpd-rs-junior/<task>
Verification: the exact check-format command passes.
Rules:
- keep fixtures minimal
- do not claim upstream parity
- do not edit src/formats.rs by hand
- stop on blockers
Output: Russian report with Result, Evidence, Verification, Blockers.
```
