#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="${TMP_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/jscpd-rs-package.XXXXXX")}"

cleanup() {
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT

cd "$ROOT"

PACKAGE_LIST="$TMP_ROOT/package-list.txt"

cargo package --allow-dirty --no-verify --list >"$PACKAGE_LIST"

required_files=(
  "CHANGELOG.md"
  "Cargo.lock"
  "Cargo.toml"
  "LICENSE"
  "README.md"
  "src/bin/jscpd-server.rs"
  "src/lib.rs"
  "src/main.rs"
)

for file in "${required_files[@]}"; do
  if ! grep -Fxq "$file" "$PACKAGE_LIST"; then
    printf 'package is missing required file: %s\n' "$file" >&2
    exit 1
  fi
done

for pattern in '^jscpd/' '^target/' '(^|/)node_modules/' '^scripts/'; do
  if grep -Eq "$pattern" "$PACKAGE_LIST"; then
    printf 'package includes forbidden paths matching %s:\n' "$pattern" >&2
    grep -E "$pattern" "$PACKAGE_LIST" >&2
    exit 1
  fi
done

printf 'package file count: %s\n' "$(wc -l <"$PACKAGE_LIST" | tr -d ' ')"

cargo package --allow-dirty --locked

INSTALL_ROOT="$TMP_ROOT/install"
cargo install --path . --bin jscpd --root "$INSTALL_ROOT" --force --locked >/dev/null
cargo install --path . --bin jscpd-server --root "$INSTALL_ROOT" --force --locked >/dev/null

EXPECTED_VERSION="$(
  cargo metadata --no-deps --format-version 1 \
    | node --input-type=module -e 'let data = ""; process.stdin.on("data", chunk => data += chunk); process.stdin.on("end", () => console.log(JSON.parse(data).packages[0].version));'
)"
ACTUAL_VERSION="$("$INSTALL_ROOT/bin/jscpd" --version)"

if [[ "$ACTUAL_VERSION" != "$EXPECTED_VERSION" ]]; then
  printf 'installed binary version mismatch: got %s, expected %s\n' "$ACTUAL_VERSION" "$EXPECTED_VERSION" >&2
  exit 1
fi

if ! "$INSTALL_ROOT/bin/jscpd" --help | grep -Fq 'Usage: jscpd [options] <path ...>'; then
  printf 'installed binary help does not expose upstream-compatible command name\n' >&2
  exit 1
fi

if [[ "$("$INSTALL_ROOT/bin/jscpd-server" --version)" != "$EXPECTED_VERSION" ]]; then
  printf 'installed server binary version mismatch\n' >&2
  exit 1
fi

scripts/npm-package-check.sh
