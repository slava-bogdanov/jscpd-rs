#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_TAG="${RELEASE_TAG:-v0.1.0}"
CRATE_NAME="${CRATE_NAME:-jscpd-rs}"
EXPECTED_JSCPD_SHA="${EXPECTED_JSCPD_SHA:-50290cfd1b60b8d0d4c2929a1367328a1dddd074}"
RUN_RELEASE_CANDIDATE="${RUN_RELEASE_CANDIDATE:-1}"

cd "$ROOT"

fail() {
  printf 'prepublish check failed: %s\n' "$*" >&2
  exit 1
}

section() {
  printf '\n== %s ==\n' "$*"
}

section "clean git state"
status="$(git status --short)"
if [[ -n "$status" ]]; then
  printf '%s\n' "$status" >&2
  fail "working tree is not clean"
fi
git status --short --branch

section "jscpd submodule reference"
submodule_status="$(git submodule status jscpd)"
printf '%s\n' "$submodule_status"
submodule_sha="$(awk '{print $1}' <<<"$submodule_status")"
if [[ "$submodule_sha" == +* || "$submodule_sha" == -* || "$submodule_sha" == U* ]]; then
  fail "jscpd submodule is not at the recorded commit"
fi
submodule_sha="${submodule_sha# }"
if [[ -n "$EXPECTED_JSCPD_SHA" && "$submodule_sha" != "$EXPECTED_JSCPD_SHA" ]]; then
  fail "jscpd submodule is $submodule_sha, expected $EXPECTED_JSCPD_SHA"
fi

section "release tag availability"
if git tag -l "$RELEASE_TAG" | grep -Fxq "$RELEASE_TAG"; then
  fail "local tag $RELEASE_TAG already exists"
fi
if git ls-remote --tags origin "refs/tags/$RELEASE_TAG" | grep -q .; then
  fail "remote tag $RELEASE_TAG already exists"
fi
printf 'tag %s is available locally and on origin\n' "$RELEASE_TAG"

section "crate name availability"
cargo_search_output="$(cargo search "$CRATE_NAME" --limit 5)"
if grep -E "^${CRATE_NAME}[[:space:]=]" <<<"$cargo_search_output"; then
  fail "crate $CRATE_NAME appears to exist in cargo search output"
fi
printf 'cargo search found no exact %s crate\n' "$CRATE_NAME"

section "benchmark docs consistency"
required_benchmark_rows=(
  "0.189897s | 9.879855s | 52.03x"
  "0.245680s | 14.249817s | 58.00x"
  "0.076644s | 4.509250s | 58.83x"
)
benchmark_docs=(
  "README.md"
  "docs/compat-baseline.md"
  "docs/public-benchmark-suite.md"
  "CHANGELOG.md"
)
for doc in "${benchmark_docs[@]}"; do
  for row in "${required_benchmark_rows[@]}"; do
    if ! grep -Fq "$row" "$doc"; then
      fail "$doc is missing benchmark row fragment: $row"
    fi
  done
done
printf 'benchmark rows are present in README, compat baseline, public suite docs, and changelog\n'

if [[ "$RUN_RELEASE_CANDIDATE" == "1" ]]; then
  section "release candidate gate"
  scripts/release-candidate.sh
else
  section "release candidate gate"
  printf 'skipped because RUN_RELEASE_CANDIDATE=%s\n' "$RUN_RELEASE_CANDIDATE"
fi

section "package/install check"
scripts/package-check.sh

section "cargo publish dry run"
cargo publish --dry-run --locked

section "prepublish check complete"
printf 'ready for manual tag/publish step, pending explicit release approval\n'
