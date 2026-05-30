#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKTREE_ROOT="${JUNIOR_WORKTREE_ROOT:-${TMPDIR:-/tmp}/jscpd-rs-junior}"

usage() {
  cat <<'USAGE'
usage: scripts/junior-worktree.sh <task-slug> [base-ref]

Create an isolated git worktree for a single junior task.

Environment:
  JUNIOR_WORKTREE_ROOT  parent directory for helper worktrees
                        default: ${TMPDIR:-/tmp}/jscpd-rs-junior

Example:
  scripts/junior-worktree.sh yaml-comments
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage >&2
  exit 2
fi

slug="$1"
base_ref="${2:-HEAD}"
safe_slug="$(printf '%s' "$slug" | tr -cs '[:alnum:]_.-' '-' | sed 's/^-//; s/-$//')"
if [[ -z "$safe_slug" ]]; then
  printf 'invalid task slug: %s\n' "$slug" >&2
  exit 2
fi

timestamp="$(date +%Y%m%d%H%M%S)"
branch="junior/${safe_slug}-${timestamp}"
path="$WORKTREE_ROOT/$safe_slug"

if [[ -e "$path" ]]; then
  printf 'worktree path already exists: %s\n' "$path" >&2
  printf 'remove it with git worktree remove %q after reviewing any work.\n' "$path" >&2
  exit 1
fi

mkdir -p "$WORKTREE_ROOT"
cd "$ROOT"
git worktree add -b "$branch" "$path" "$base_ref"

cat <<EOF
created junior worktree
  path:   $path
  branch: $branch
  base:   $base_ref

Next:
  cd "$path"
  pi --no-session --tools read,edit,bash -p '<bounded junior task prompt>'

Review from main checkout:
  git -C "$path" status --short
  git -C "$path" diff

Cleanup after review:
  git worktree remove "$path"
  git branch -D "$branch"
EOF
