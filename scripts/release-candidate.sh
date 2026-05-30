#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT"

FULL="${FULL:-1}"
PUBLIC="${PUBLIC:-1}"
PUBLIC_RUNS="${PUBLIC_RUNS:-3}"
PUBLIC_CHECK_COMPAT="${PUBLIC_CHECK_COMPAT:-1}"
PUBLIC_MIN_SPEEDUP="${PUBLIC_MIN_SPEEDUP:-10}"
PUBLIC_CASES="${PUBLIC_CASES:-react,next,prometheus}"
STRICT="${STRICT:-coverage}"
RUN_CLIPPY="${RUN_CLIPPY:-1}"

print_plan() {
  cat <<EOF
release candidate gate:
  RUN_CLIPPY=$RUN_CLIPPY
  FULL=$FULL
  STRICT=$STRICT
  PUBLIC=$PUBLIC
  PUBLIC_CASES=$PUBLIC_CASES
  PUBLIC_RUNS=$PUBLIC_RUNS
  PUBLIC_CHECK_COMPAT=$PUBLIC_CHECK_COMPAT
  PUBLIC_MIN_SPEEDUP=$PUBLIC_MIN_SPEEDUP

commands:
  cargo clippy --all-targets -- -D warnings
  FULL=$FULL STRICT=$STRICT PUBLIC=$PUBLIC PUBLIC_CASES=$PUBLIC_CASES PUBLIC_RUNS=$PUBLIC_RUNS PUBLIC_CHECK_COMPAT=$PUBLIC_CHECK_COMPAT PUBLIC_MIN_SPEEDUP=$PUBLIC_MIN_SPEEDUP scripts/release-gate.sh
EOF
}

print_plan

if [[ "${DRY_RUN:-0}" == "1" ]]; then
  exit 0
fi

if [[ "$RUN_CLIPPY" == "1" ]]; then
  printf '\n== cargo clippy --all-targets -- -D warnings ==\n'
  cargo clippy --all-targets -- -D warnings
fi

printf '\n== release gate with full matrix and public suite ==\n'
FULL="$FULL" \
  STRICT="$STRICT" \
  PUBLIC="$PUBLIC" \
  PUBLIC_CASES="$PUBLIC_CASES" \
  PUBLIC_RUNS="$PUBLIC_RUNS" \
  PUBLIC_CHECK_COMPAT="$PUBLIC_CHECK_COMPAT" \
  PUBLIC_MIN_SPEEDUP="$PUBLIC_MIN_SPEEDUP" \
  scripts/release-gate.sh
