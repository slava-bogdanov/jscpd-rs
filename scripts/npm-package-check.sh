#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="${TMP_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/jscpd-rs-npm.XXXXXX")}"

cleanup() {
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT

cd "$ROOT"

fail() {
  printf 'npm package check failed: %s\n' "$*" >&2
  exit 1
}

if ! command -v npm >/dev/null 2>&1; then
  fail "npm is required"
fi
if ! command -v node >/dev/null 2>&1; then
  fail "node is required"
fi

cargo_version="$(
  cargo metadata --no-deps --format-version 1 \
    | node --input-type=module -e 'let data = ""; process.stdin.on("data", chunk => data += chunk); process.stdin.on("end", () => console.log(JSON.parse(data).packages[0].version));'
)"
npm_version="$(node -p 'require("./package.json").version')"
if [[ "$npm_version" != "$cargo_version" ]]; then
  fail "package.json version $npm_version does not match Cargo.toml version $cargo_version"
fi

PACK_DIR="$TMP_ROOT/pack"
INSTALL_DIR="$TMP_ROOT/install"
mkdir -p "$PACK_DIR" "$INSTALL_DIR"

npm pack --pack-destination "$PACK_DIR" --json >"$TMP_ROOT/npm-pack.json"
tarball="$(node --input-type=module - "$TMP_ROOT/npm-pack.json" <<'NODE'
import fs from 'node:fs';

const pack = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
console.log(pack[0].filename);
NODE
)"
tarball="$PACK_DIR/$tarball"

node --input-type=module - "$TMP_ROOT/npm-pack.json" <<'NODE'
import fs from 'node:fs';

const pack = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'))[0];
const files = pack.files.map((file) => file.path).sort();
const required = [
  'Cargo.toml',
  'Cargo.lock',
  'LICENSE',
  'README.md',
  'npm/bin/jscpd-rs.js',
  'npm/bin/jscpd-server.js',
  'npm/lib/run-binary.js',
  'npm/scripts/postinstall.js',
  'package.json',
  'src/main.rs',
  'src/bin/jscpd-server.rs',
];
const forbidden = [
  /^jscpd\//,
  /^target\//,
  /^report\//,
  /^scripts\//,
  /(^|\/)node_modules\//,
];

for (const path of required) {
  if (!files.includes(path)) {
    console.error(`npm package is missing required file: ${path}`);
    process.exit(1);
  }
}
for (const path of files) {
  const match = forbidden.find((pattern) => pattern.test(path));
  if (match) {
    console.error(`npm package includes forbidden file: ${path}`);
    process.exit(1);
  }
}
console.log(`npm package file count: ${files.length}`);
NODE

npm publish --dry-run --json >"$TMP_ROOT/npm-publish-dry-run.json"
node --input-type=module - "$TMP_ROOT/npm-publish-dry-run.json" "$npm_version" <<'NODE'
import fs from 'node:fs';

const [file, expectedVersion] = process.argv.slice(2);
const publish = JSON.parse(fs.readFileSync(file, 'utf8'));
if (publish.name !== 'jscpd-rs' || publish.version !== expectedVersion) {
  console.error(
    `unexpected npm publish dry-run package: ${publish.name}@${publish.version}`,
  );
  process.exit(1);
}
console.log(`npm publish dry-run: ${publish.name}@${publish.version}`);
NODE

(
  cd "$INSTALL_DIR"
  npm init -y >/dev/null
  npm install --no-audit --no-fund "$tarball"
  test "$("./node_modules/.bin/jscpd-rs" --version)" = "$cargo_version"
  test "$("./node_modules/.bin/jscpd" --version)" = "$cargo_version"
  test "$("./node_modules/.bin/jscpd-server" --version)" = "$cargo_version"
  "./node_modules/.bin/jscpd" --help | grep -Fq 'Usage: jscpd [options] <path ...>'
)

npx --yes --package "$tarball" jscpd-rs --version | grep -Fxq "$cargo_version"

printf 'npm package check complete: %s\n' "$tarball"
