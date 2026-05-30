#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT"

printf '== cargo fmt --check ==\n'
cargo fmt --check

printf '\n== cargo test ==\n'
cargo test

printf '\n== bash -n scripts/*.sh ==\n'
bash -n scripts/*.sh

printf '\n== package/install check ==\n'
scripts/package-check.sh

printf '\n== CLI compatibility ==\n'
scripts/compat-cli.sh

printf '\n== config compatibility ==\n'
scripts/compat-config.sh

printf '\n== reporter compatibility ==\n'
scripts/compat-reporters.sh

printf '\n== blame compatibility ==\n'
scripts/compat-blame.sh

printf '\n== server compatibility ==\n'
scripts/compat-server.sh

printf '\n== upstream CI fixture compatibility ==\n'
scripts/compat-upstream-ci.sh

if [[ "${FULL:-0}" == "1" ]]; then
  printf '\n== full compatibility matrix ==\n'
  STRICT="${STRICT:-coverage}" scripts/compat-matrix.sh
else
  printf '\nSkipping full compatibility matrix. Run FULL=1 scripts/release-gate.sh before publication.\n'
fi

if [[ "${PUBLIC:-0}" == "1" ]]; then
  printf '\n== public benchmark suite ==\n'
  CASES="${PUBLIC_CASES:-react,next,prometheus}" \
    RUNS="${PUBLIC_RUNS:-1}" \
    CHECK_COMPAT="${PUBLIC_CHECK_COMPAT:-1}" \
    MIN_SPEEDUP="${PUBLIC_MIN_SPEEDUP:-10}" \
    scripts/public-bench-suite.sh
else
  printf '\nSkipping public benchmark suite. Run PUBLIC=1 scripts/release-gate.sh before publication.\n'
fi
