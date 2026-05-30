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
MIN_SPEEDUP="${MIN_SPEEDUP:-0}"
FETCH="${FETCH:-1}"
UPDATE="${UPDATE:-0}"
CHECK_COMPAT="${CHECK_COMPAT:-0}"
LIST="${LIST:-0}"
CASES="${CASES:-}"
SUMMARY_FILE="${SUMMARY_FILE:-$RESULTS_DIR/summary.tsv}"

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
  MIN_SPEEDUP  Fail when upstream/Rust speedup is below this value, default 0.
  LIST         Print configured cases and exit, default 0.
  BENCH_ROOT   Root for generated clones/results, default ~/.cache/jscpd-rs/public-bench.
  REPOS_DIR    Clone directory, default $BENCH_ROOT/repos.
  RESULTS_DIR  Benchmark output directory, default $BENCH_ROOT/results.
  SUMMARY_FILE TSV summary path, default $RESULTS_DIR/summary.tsv.

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

compat_allow_missing_coverage() {
  local name="$1"
  local repo_path="$2"
  local repo_rel
  repo_rel="$(realpath --relative-to="$ROOT" "$repo_path")"

  case "$name" in
    react)
      printf 'javascript:%s/packages/react-dom/src/events/__tests__/SyntheticMouseEvent-test.js:21-38\n' "$repo_rel"
      printf 'javascript:%s/packages/react-dom/src/server/ReactDOMFizzServerNode.js:179-229\n' "$repo_rel"
      printf 'javascript:%s/packages/react-dom/src/__tests__/ReactDOMViewTransition-test.js:39-135\n' "$repo_rel"
      ;;
    next)
      printf 'typescript:%s/packages/next/src/build/webpack/loaders/next-style-loader/index.ts:221-229\n' "$repo_rel"
      printf 'typescript:%s/test/e2e/app-dir/non-root-project-monorepo/non-root-project-monorepo.test.ts:284-303\n' "$repo_rel"
      printf 'typescript:%s/test/e2e/app-dir/non-root-project-monorepo/non-root-project-monorepo.test.ts:221-240\n' "$repo_rel"
      printf 'typescript:%s/packages/next-routing/src/__tests__/normalize-next-data.test.ts:185-681\n' "$repo_rel"
      printf 'typescript:%s/test/e2e/edge-runtime-module-errors/edge-runtime-module-errors.test.ts:314-459\n' "$repo_rel"
      printf 'typescript:%s/test/e2e/edge-runtime-module-errors/edge-runtime-module-errors.test.ts:745-892\n' "$repo_rel"
      printf 'typescript:%s/test/development/basic/next-rs-api.test.ts:327-356\n' "$repo_rel"
      printf 'typescript:%s/test/development/basic/next-rs-api.test.ts:175-203\n' "$repo_rel"
      ;;
  esac
}

parse_avg() {
  local file="$1"
  local label="$2"

  awk -v label="$label" '
    $0 == label { in_section = 1; next }
    /^[^[:space:]]/ { in_section = 0 }
    in_section && $1 == "avg:" {
      gsub(/s$/, "", $2)
      print $2
      exit
    }
  ' "$file"
}

speedup_ratio() {
  local rust_avg="$1"
  local upstream_avg="$2"

  awk -v rust="$rust_avg" -v upstream="$upstream_avg" 'BEGIN {
    if (rust <= 0) {
      print "inf"
    } else {
      printf "%.2f", upstream / rust
    }
  }'
}

assert_min_speedup() {
  local name="$1"
  local speedup="$2"

  if [[ "$MIN_SPEEDUP" == "0" ]]; then
    return 0
  fi

  awk -v name="$name" -v speedup="$speedup" -v min="$MIN_SPEEDUP" 'BEGIN {
    if (speedup + 0 < min + 0) {
      printf "speedup gate failed for %s: %.2fx < %.2fx\n", name, speedup, min > "/dev/stderr"
      exit 1
    }
  }'
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
printf 'case\tcommit\tformat\trust_avg_s\tupstream_avg_s\tspeedup\n' >"$SUMMARY_FILE"
ran_cases=0

for spec in "${SUITE_CASES[@]}"; do
  IFS='|' read -r name url branch format subpath <<<"$spec"
  if ! case_selected "$name"; then
    continue
  fi

  ran_cases=$((ran_cases + 1))
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

  rust_avg="$(parse_avg "$result_file" "rust mvp")"
  upstream_avg="$(parse_avg "$result_file" "upstream jscpd")"
  if [[ -z "$rust_avg" || -z "$upstream_avg" ]]; then
    printf 'failed to parse benchmark averages from %s\n' "$result_file" >&2
    exit 1
  fi
  speedup="$(speedup_ratio "$rust_avg" "$upstream_avg")"
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$name" "$commit" "$format" "$rust_avg" "$upstream_avg" "$speedup" >>"$SUMMARY_FILE"
  printf 'speedup: %sx\n' "$speedup"
  assert_min_speedup "$name" "$speedup"

  if [[ "$CHECK_COMPAT" == "1" ]]; then
    printf '\n== %s coverage compatibility ==\n' "$name"
    allowed_missing_coverage="$(compat_allow_missing_coverage "$name" "$repo_path")"
    FORMAT="$format" \
      STRICT=coverage \
      ALLOW_MISSING_COVERAGE="$allowed_missing_coverage" \
      MIN_TOKENS="$MIN_TOKENS" \
      MIN_LINES="$MIN_LINES" \
      MAX_SIZE="$MAX_SIZE" \
      "$ROOT/scripts/compat.sh" "$target_path"
  fi

  printf 'saved benchmark output: %s\n' "$result_file"
done

if [[ "$ran_cases" == "0" ]]; then
  printf 'no benchmark cases selected (CASES=%s)\n' "$CASES" >&2
  exit 1
fi

printf '\nsummary: %s\n' "$SUMMARY_FILE"
