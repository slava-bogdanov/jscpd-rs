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

printf '\n== CLI compatibility ==\n'
scripts/compat-cli.sh

printf '\n== config compatibility ==\n'
scripts/compat-config.sh

printf '\n== reporter compatibility ==\n'
scripts/compat-reporters.sh

printf '\n== blame compatibility ==\n'
scripts/compat-blame.sh

printf '\n== upstream CI fixture compatibility ==\n'
scripts/compat-upstream-ci.sh

if [[ "${FULL:-0}" == "1" ]]; then
  printf '\n== full compatibility matrix ==\n'
  STRICT="${STRICT:-coverage}" scripts/compat-matrix.sh
else
  printf '\nSkipping full compatibility matrix. Run FULL=1 scripts/release-gate.sh before publication.\n'
fi
