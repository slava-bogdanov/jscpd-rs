# Junior Workflow

This project can use a local junior agent for bounded implementation tasks. The
main agent remains responsible for scope, architecture, review, verification,
commits, and pushes.

## Rules

- Run at most one junior agent process at a time.
- Do not run junior work in the main checkout. Use a dedicated git worktree.
- Give small implementation tasks with an example, exact files, and exact tests.
- Do not delegate architecture, compatibility policy, broad reviews, or release
  decisions.
- Do not let a junior touch external repositories such as `/home/dev/dream`.
- Treat junior output as a patch proposal. Review every diff before merging.
- Prefer `pi` for one-shot tasks because `--no-session` and `--tools` make the
  allowed surface explicit. Use `opencode` only as fallback.

## Worktree Loop

Create a worktree:

```bash
scripts/junior-worktree.sh <task-slug>
```

Run the junior from the printed worktree path:

```bash
cd "${JUNIOR_WORKTREE_ROOT:-${TMPDIR:-/tmp}/jscpd-rs-junior}/<task-slug>"
pi --no-session --tools read,edit,bash -p '<prompt from docs/junior-task-template.md>'
```

Review from the main checkout:

```bash
git -C "${JUNIOR_WORKTREE_ROOT:-${TMPDIR:-/tmp}/jscpd-rs-junior}/<task-slug>" status --short
git -C "${JUNIOR_WORKTREE_ROOT:-${TMPDIR:-/tmp}/jscpd-rs-junior}/<task-slug>" diff
```

If the patch is useful, apply it deliberately from the main checkout using
`git diff`, `git apply`, cherry-pick, or a manual patch. After review:

```bash
git worktree remove "${JUNIOR_WORKTREE_ROOT:-${TMPDIR:-/tmp}/jscpd-rs-junior}/<task-slug>"
git branch -D junior/<branch-name>
```

## Good Junior Tasks

- Add one unit test following an existing nearby test.
- Add one small reporter or output-format test after the implementation exists.
- Add support for one comment style in a generic/native tokenizer.
- Add or update a small fixture for one format.
- Port one small function from upstream when the target shape is already clear.
- Run `scripts/check-format.sh <format> <target>` and report exact output.

## Prompt Notes

- Prefer implementation or test tasks over broad read-only scouting. A scout
  prompt that is too procedural may produce a task plan instead of executing the
  checks.
- For read-only fact gathering, explicitly say: "Execute the checks now; do not
  write instructions or a plan."
- Keep fact-gathering prompts to exact files and exact commands. If there are
  many commands, give a shell loop instead of asking the junior to design one.
- Do not use junior output as evidence until the main agent verifies the exact
  commands or diff.

## Bad Junior Tasks

- Review a whole commit or subsystem.
- Decide whether compatibility is acceptable.
- Refactor detector/tokenizer architecture.
- Change generated format registry by hand.
- Touch multiple unrelated files.
- Run broad benchmarks or modify local third-party repositories.

## Review Checklist

- The diff only touches allowed files.
- The implementation follows an existing local pattern.
- Tests are exact enough to catch broken structure, not only `contains` checks.
- Commands in the report match commands actually run.
- `cargo fmt`, `cargo test`, and relevant scripts pass from the main checkout.
- Any accepted patch is committed by the main agent, not by the junior.
