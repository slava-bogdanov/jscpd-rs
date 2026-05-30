#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BENCH_ROOT="${BENCH_ROOT:-${XDG_CACHE_HOME:-$HOME/.cache}/jscpd-rs/public-bench}"
REPOS_DIR="${REPOS_DIR:-$BENCH_ROOT/repos}"
RESULTS_DIR="${RESULTS_DIR:-$BENCH_ROOT/results}"
RUNS="${RUNS:-3}"
MIN_TOKENS="${MIN_TOKENS:-50}"
MIN_LINES="${MIN_LINES:-5}"
MAX_SIZE="${MAX_SIZE:-1mb}"
FETCH="${FETCH:-1}"
UPDATE="${UPDATE:-0}"
CHECK_COMPAT="${CHECK_COMPAT:-0}"
LIST="${LIST:-0}"
CASES="${CASES:-}"

SUITE_CASES=(
  "react|https://github.com/facebook/react.git|main|javascript|."
  "next|https://github.com/vercel/next.js.git|canary|typescript|."
  "vscode|https://github.com/microsoft/vscode.git|main|typescript|."
  "prometheus|https://github.com/prometheus/prometheus.git|main|go|."
  "rust|https://github.com/rust-lang/rust.git|main|rust|."
)

usage() {
  cat <<'EOF'
usage: scripts/public-bench-suite.sh

Environment:
  CASES        Comma-separated case names to run, default all.
  RUNS         Benchmark runs per tool, default 3.
  FETCH        Clone missing repositories, default 1.
  UPDATE       Fetch/reset existing repositories, default 0.
  CHECK_COMPAT Run coverage compat after each benchmark, default 0.
  LIST         Print configured cases and exit, default 0.
  BENCH_ROOT   Root for generated clones/results, default ~/.cache/jscpd-rs/public-bench.
  REPOS_DIR    Clone directory, default $BENCH_ROOT/repos.
  RESULTS_DIR  Benchmark output directory, default $BENCH_ROOT/results.

Examples:
  LIST=1 scripts/public-bench-suite.sh
  CASES=react,next RUNS=3 scripts/public-bench-suite.sh
  CHECK_COMPAT=1 CASES=react scripts/public-bench-suite.sh
EOF
}

case_selected() {
  local name="$1"
  [[ -z "$CASES" ]] && return 0
  [[ ",$CASES," == *",$name,"* ]]
}

repo_path_for() {
  local name="$1"
  printf '%s/%s' "$REPOS_DIR" "$name"
}

ensure_repo() {
  local name="$1"
  local url="$2"
  local branch="$3"
  local repo_path
  repo_path="$(repo_path_for "$name")"

  if [[ -d "$repo_path/.git" ]]; then
    if [[ "$UPDATE" == "1" ]]; then
      git -C "$repo_path" fetch --depth=1 origin "$branch"
      git -C "$repo_path" checkout --detach FETCH_HEAD
    fi
    return 0
  fi

  if [[ "$FETCH" != "1" ]]; then
    printf 'missing %s; rerun with FETCH=1 or clone %s into %s\n' "$name" "$url" "$repo_path" >&2
    return 1
  fi

  mkdir -p "$REPOS_DIR"
  git clone --depth=1 --branch "$branch" "$url" "$repo_path"
}

print_cases() {
  for spec in "${SUITE_CASES[@]}"; do
    IFS='|' read -r name url branch format subpath <<<"$spec"
    printf '%-12s format=%-10s branch=%-8s subpath=%s url=%s\n' \
      "$name" "$format" "$branch" "$subpath" "$url"
  done
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ "$LIST" == "1" ]]; then
  print_cases
  exit 0
fi

mkdir -p "$RESULTS_DIR"

for spec in "${SUITE_CASES[@]}"; do
  IFS='|' read -r name url branch format subpath <<<"$spec"
  if ! case_selected "$name"; then
    continue
  fi

  ensure_repo "$name" "$url" "$branch"
  repo_path="$(repo_path_for "$name")"
  target_path="$repo_path/$subpath"
  commit="$(git -C "$repo_path" rev-parse --short HEAD)"
  result_file="$RESULTS_DIR/$name-$format-$commit.txt"

  printf '\n== %s (%s, %s) ==\n' "$name" "$format" "$commit"
  printf 'target: %s\n' "$target_path"
  FORMAT="$format" \
    RUNS="$RUNS" \
    MIN_TOKENS="$MIN_TOKENS" \
    MIN_LINES="$MIN_LINES" \
    MAX_SIZE="$MAX_SIZE" \
    "$ROOT/scripts/bench.sh" "$target_path" | tee "$result_file"

  if [[ "$CHECK_COMPAT" == "1" ]]; then
    printf '\n== %s coverage compatibility ==\n' "$name"
    FORMAT="$format" \
      STRICT=coverage \
      MIN_TOKENS="$MIN_TOKENS" \
      MIN_LINES="$MIN_LINES" \
      MAX_SIZE="$MAX_SIZE" \
      "$ROOT/scripts/compat.sh" "$target_path"
  fi

  printf 'saved benchmark output: %s\n' "$result_file"
done
