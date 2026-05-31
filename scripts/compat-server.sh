#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${TARGET:-$ROOT/jscpd/fixtures/javascript}"
TMP_ROOT="${TMP_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/jscpd-rs-server.XXXXXX")}"
RUST_PORT="${RUST_PORT:-39981}"
UPSTREAM_PORT="${UPSTREAM_PORT:-39982}"
RUST_STORE_WARNING_PORT="${RUST_STORE_WARNING_PORT:-39983}"
UPSTREAM_STORE_WARNING_PORT="${UPSTREAM_STORE_WARNING_PORT:-39984}"
RUST_BARE_HOST_PORT="${RUST_BARE_HOST_PORT:-39986}"
UPSTREAM_BARE_HOST_PORT="${UPSTREAM_BARE_HOST_PORT:-39987}"
RUST_LOCALHOST_PORT="${RUST_LOCALHOST_PORT:-39988}"
UPSTREAM_LOCALHOST_PORT="${UPSTREAM_LOCALHOST_PORT:-39989}"
RUST_CONFIG_PORT="${RUST_CONFIG_PORT:-39990}"
UPSTREAM_CONFIG_PORT="${UPSTREAM_CONFIG_PORT:-39991}"
MIN_TOKENS="${MIN_TOKENS:-40}"
MIN_LINES="${MIN_LINES:-5}"
MAX_SIZE="${MAX_SIZE:-1mb}"
RUST_PID=""
UPSTREAM_PID=""

cleanup() {
  if [[ -n "$RUST_PID" ]]; then
    kill "$RUST_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "$UPSTREAM_PID" ]]; then
    kill "$UPSTREAM_PID" >/dev/null 2>&1 || true
  fi
  if [[ "${KEEP:-0}" != "1" ]]; then
    rm -rf "$TMP_ROOT"
  fi
}
trap cleanup EXIT

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
fi

if command -v corepack >/dev/null 2>&1; then
  corepack prepare pnpm@10.28.0 --activate >/dev/null
fi

cd "$ROOT"
cargo build --release --bin jscpd-server >/dev/null

if [[ ! -d "$ROOT/jscpd/node_modules" ]]; then
  pnpm --dir "$ROOT/jscpd" install --frozen-lockfile
fi

if [[ ! -f "$ROOT/jscpd/apps/jscpd-server/dist/bin/jscpd-server.js" ]]; then
  pnpm --dir "$ROOT/jscpd" build
fi

printf 'target: %s\n' "$TARGET"
printf 'rust port: %s, upstream port: %s\n' "$RUST_PORT" "$UPSTREAM_PORT"
printf 'tmp: %s\n\n' "$TMP_ROOT"

run_command() {
  local code_file="$1"
  local stdout_file="$2"
  local stderr_file="$3"
  shift 3

  set +e
  "$@" >"$stdout_file" 2>"$stderr_file"
  local code=$?
  set -e
  printf '%s' "$code" >"$code_file"
}

require_contains() {
  local file="$1"
  local needle="$2"
  local label="$3"

  if ! grep -Fq -- "$needle" "$file"; then
    printf '%s missing expected text: %s\n' "$label" "$needle" >&2
    printf '%s contents:\n' "$file" >&2
    sed -n '1,120p' "$file" >&2
    return 1
  fi
}

check_exit_code() {
  local code_file="$1"
  local expected="$2"
  local label="$3"
  local actual
  actual="$(<"$code_file")"
  if [[ "$actual" != "$expected" ]]; then
    printf '%s exit code mismatch: got %s expected %s\n' "$label" "$actual" "$expected" >&2
    return 1
  fi
}

check_server_store_warning() {
  local label="$1"
  local store_warning_port="$2"
  shift 2
  local cmd=("$@")
  local dir="$TMP_ROOT/$label-cli"
  mkdir -p "$dir"

  run_command "$dir/store-warning.code" "$dir/store-warning.stdout" "$dir/store-warning.stderr" \
    timeout 3 "${cmd[@]}" "$TARGET" --store leveldb --port "$store_warning_port"
  check_exit_code "$dir/store-warning.code" 124 "$label --store warning"
  require_contains "$dir/store-warning.stderr" \
    "store name leveldb not installed." \
    "$label --store warning stderr"
  require_contains "$dir/store-warning.stdout" \
    "JSCPD server running on" \
    "$label --store warning stdout"

  printf 'ok %-18s\n' "$label store warning"
}

check_server_host_binding() {
  local label="$1"
  local bare_host_port="$2"
  local localhost_port="$3"
  shift 3
  local cmd=("$@")
  local dir="$TMP_ROOT/$label-cli"
  mkdir -p "$dir"

  run_command "$dir/bare-host.code" "$dir/bare-host.stdout" "$dir/bare-host.stderr" \
    timeout 3 "${cmd[@]}" "$TARGET" --host --port "$bare_host_port"
  check_exit_code "$dir/bare-host.code" 124 "$label bare --host"
  require_contains "$dir/bare-host.stdout" \
    "JSCPD server running on http://true:$bare_host_port" \
    "$label bare --host stdout"

  run_command "$dir/localhost.code" "$dir/localhost.stdout" "$dir/localhost.stderr" \
    timeout 3 "${cmd[@]}" "$TARGET" --host=localhost --port "$localhost_port"
  check_exit_code "$dir/localhost.code" 124 "$label --host=localhost"
  require_contains "$dir/localhost.stdout" \
    "JSCPD server running on http://localhost:$localhost_port" \
    "$label --host=localhost stdout"

  printf 'ok %-18s\n' "$label host"
}

check_server_config_working_directory() {
  local label="$1"
  local port="$2"
  shift 2
  local cmd=("$@")
  local dir="$TMP_ROOT/$label-config"
  local log="$dir/server.log"
  local health="$dir/health.json"
  mkdir -p "$dir/src"
  cat >"$dir/src/a.js" <<'EOF_JS'
const alpha = 1;
const beta = 2;
const gamma = alpha + beta;
console.log(gamma);
EOF_JS
  cp "$dir/src/a.js" "$dir/src/b.js"
  cat >"$dir/.jscpd.json" <<'EOF_JSON'
{"path":["src"],"format":["javascript"],"minTokens":5,"minLines":1,"maxSize":"1mb"}
EOF_JSON

  (cd "$dir" && timeout 10 "${cmd[@]}" --config .jscpd.json --host 127.0.0.1 --port "$port") >"$log" 2>&1 &
  local pid=$!

  for _ in $(seq 1 100); do
    if curl -fsS "http://127.0.0.1:$port/api/health" >"$health" 2>/dev/null; then
      kill "$pid" >/dev/null 2>&1 || true
      wait "$pid" >/dev/null 2>&1 || true
      node --input-type=module - "$health" "$dir" "$label" <<'NODE'
import assert from 'node:assert/strict';
import fs from 'node:fs';

const [healthPath, root, label] = process.argv.slice(2);
const health = JSON.parse(fs.readFileSync(healthPath, 'utf8'));
assert.equal(health.workingDirectory, root, `${label} config-only workingDirectory`);
NODE
      printf 'ok %-18s\n' "$label config cwd"
      return 0
    fi
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      printf '%s config-only server exited before becoming ready\n' "$label" >&2
      sed -n '1,160p' "$log" >&2
      return 1
    fi
    sleep 0.1
  done

  kill "$pid" >/dev/null 2>&1 || true
  wait "$pid" >/dev/null 2>&1 || true
  printf '%s config-only server did not become ready\n' "$label" >&2
  sed -n '1,160p' "$log" >&2
  return 1
}

check_server_cli_contract() {
  local label="$1"
  shift
  local cmd=("$@")
  local dir="$TMP_ROOT/$label-cli"
  mkdir -p "$dir"

  run_command "$dir/help.code" "$dir/help.stdout" "$dir/help.stderr" "${cmd[@]}" --help
  check_exit_code "$dir/help.code" 0 "$label --help"
  require_contains "$dir/help.stdout" "Usage: jscpd-server [options] <path>" "$label --help stdout"
  require_contains "$dir/help.stdout" "Start jscpd as a server" "$label --help stdout"
  require_contains "$dir/help.stdout" "-p, --port [number]" "$label --help stdout"
  require_contains "$dir/help.stdout" "-H, --host [string]" "$label --help stdout"

  run_command "$dir/port-invalid.code" "$dir/port-invalid.stdout" "$dir/port-invalid.stderr" \
    "${cmd[@]}" --port abc "$TARGET"
  check_exit_code "$dir/port-invalid.code" 1 "$label invalid --port"
  require_contains "$dir/port-invalid.stderr" \
    "Failed to start server: Error: Invalid port number: abc" \
    "$label invalid --port stderr"

  run_command "$dir/port-bare.code" "$dir/port-bare.stdout" "$dir/port-bare.stderr" \
    "${cmd[@]}" --port
  check_exit_code "$dir/port-bare.code" 1 "$label bare --port"
  require_contains "$dir/port-bare.stderr" \
    "Failed to start server: Error: Invalid port number: true" \
    "$label bare --port stderr"

  for option_error in \
    "--config|TypeError [ERR_INVALID_ARG_TYPE]: The \"paths[0]\" argument must be of type string. Received type boolean (true)" \
    "--format|TypeError: cli.format.split is not a function" \
    "--ignore|TypeError: cli.ignore.split is not a function" \
    "--ignore-pattern|TypeError: cli.ignorePattern.split is not a function" \
    "--mode|Failed to start server: TypeError: mode is not a function"; do
    local option="${option_error%%|*}"
    local expected="${option_error#*|}"
    local slug="${option//-/}"
    run_command "$dir/bare-$slug.code" "$dir/bare-$slug.stdout" "$dir/bare-$slug.stderr" \
      timeout 3 "${cmd[@]}" --port 39985 "$TARGET" "$option"
    check_exit_code "$dir/bare-$slug.code" 1 "$label bare $option"
    require_contains "$dir/bare-$slug.stderr" "$expected" "$label bare $option stderr"
  done

  for option in \
    --list \
    -h \
    --reporters \
    --output \
    --debug \
    --verbose \
    --exitCode \
    --noTips \
    --skipComments \
    --formats-exts \
    --formats-names \
    --pattern \
    --blame \
    --silent \
    --threshold \
    --no-gitignore; do
    local slug="${option//-/}"
    run_command "$dir/unknown-$slug.code" "$dir/unknown-$slug.stdout" "$dir/unknown-$slug.stderr" \
      "${cmd[@]}" "$option"
    check_exit_code "$dir/unknown-$slug.code" 1 "$label unknown $option"
    require_contains "$dir/unknown-$slug.stderr" \
      "error: unknown option '$option'" \
      "$label unknown $option stderr"
  done

  printf 'ok %-18s\n' "$label CLI"
}

check_server_cli_contract rust "$ROOT/target/release/jscpd-server"
check_server_cli_contract upstream node "$ROOT/jscpd/apps/jscpd-server/bin/jscpd-server"
check_server_store_warning rust "$RUST_STORE_WARNING_PORT" "$ROOT/target/release/jscpd-server"
check_server_store_warning upstream "$UPSTREAM_STORE_WARNING_PORT" node "$ROOT/jscpd/apps/jscpd-server/bin/jscpd-server"
check_server_host_binding rust "$RUST_BARE_HOST_PORT" "$RUST_LOCALHOST_PORT" "$ROOT/target/release/jscpd-server"
check_server_host_binding upstream "$UPSTREAM_BARE_HOST_PORT" "$UPSTREAM_LOCALHOST_PORT" node "$ROOT/jscpd/apps/jscpd-server/bin/jscpd-server"
check_server_config_working_directory rust "$RUST_CONFIG_PORT" "$ROOT/target/release/jscpd-server"
check_server_config_working_directory upstream "$UPSTREAM_CONFIG_PORT" node "$ROOT/jscpd/apps/jscpd-server/bin/jscpd-server"

if ! diff -u "$TMP_ROOT/upstream-cli/help.stdout" "$TMP_ROOT/rust-cli/help.stdout"; then
  printf 'server --help output differs from upstream\n' >&2
  exit 1
fi

start_server() {
  local label="$1"
  local port="$2"
  shift 2
  local log="$TMP_ROOT/$label.log"

  "$@" "$TARGET" \
    --host 127.0.0.1 \
    --port "$port" \
    --format javascript \
    --min-tokens "$MIN_TOKENS" \
    --min-lines "$MIN_LINES" \
    --max-size "$MAX_SIZE" \
    >"$log" 2>&1 &
  local pid=$!

  for _ in $(seq 1 100); do
    if curl -fsS "http://127.0.0.1:$port/api/health" >/dev/null 2>&1; then
      printf '%s' "$pid"
      return 0
    fi
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      printf '%s server exited before becoming ready\n' "$label" >&2
      sed -n '1,160p' "$log" >&2
      return 1
    fi
    sleep 0.1
  done

  printf '%s server did not become ready\n' "$label" >&2
  sed -n '1,160p' "$log" >&2
  return 1
}

RUST_PID="$(start_server rust "$RUST_PORT" "$ROOT/target/release/jscpd-server")"
UPSTREAM_PID="$(start_server upstream "$UPSTREAM_PORT" node "$ROOT/jscpd/apps/jscpd-server/bin/jscpd-server")"

http_json() {
  local output="$1"
  local expected_code="$2"
  shift 2
  local code
  code="$(curl -sS -o "$output" -w '%{http_code}' "$@")"
  if [[ "$code" != "$expected_code" ]]; then
    printf 'HTTP code mismatch for %s: got %s expected %s\n' "$output" "$code" "$expected_code" >&2
    sed -n '1,160p' "$output" >&2
    return 1
  fi
}

http_json_with_headers() {
  local output="$1"
  local headers="$2"
  local expected_code="$3"
  shift 3
  local code
  code="$(curl -sS -D "$headers" -o "$output" -w '%{http_code}' "$@")"
  if [[ "$code" != "$expected_code" ]]; then
    printf 'HTTP code mismatch for %s: got %s expected %s\n' "$output" "$code" "$expected_code" >&2
    sed -n '1,160p' "$output" >&2
    return 1
  fi
}

check_server_http() {
  local label="$1"
  local port="$2"
  local dir="$TMP_ROOT/$label-http"
  mkdir -p "$dir"

  node --input-type=module - "$dir/check-large-payload.json" "$dir/check-special-payload.json" "$dir/check-isolation-payload.json" <<'NODE'
import fs from 'node:fs';

const [largePath, specialPath, isolationPath] = process.argv.slice(2);
const largeCode = Array.from({ length: 100 }, (_, i) => `const variable${i} = ${i};`).join('\n');
const specialCode = [
  `const str = "Hello, ${String.fromCodePoint(0x4e16, 0x754c)}! ${String.fromCodePoint(0x1f30d)}";`,
  'const regex = /[a-z]+/gi;',
  'const template = `${str}`;',
].join('\n');
const isolationCode = [
  'function uniqueSnippetIsolation() {',
  '  const isolationValue1 = "alpha";',
  '  const isolationValue2 = "beta";',
  '  const isolationValue3 = "gamma";',
  '  const isolationValue4 = "delta";',
  '  const isolationValue5 = "epsilon";',
  '  const isolationValue6 = "zeta";',
  '  return isolationValue1 + isolationValue2;',
  '}',
].join('\n');

fs.writeFileSync(largePath, JSON.stringify({ code: largeCode, format: 'javascript' }));
fs.writeFileSync(specialPath, JSON.stringify({ code: specialCode, format: 'javascript' }));
fs.writeFileSync(isolationPath, JSON.stringify({ code: isolationCode, format: 'javascript' }));
NODE

  http_json_with_headers "$dir/root.json" "$dir/root.headers" 200 "http://127.0.0.1:$port/"
  http_json_with_headers "$dir/health.json" "$dir/health.headers" 200 "http://127.0.0.1:$port/api/health"
  http_json_with_headers "$dir/stats.json" "$dir/stats.headers" 200 "http://127.0.0.1:$port/api/stats"
  http_json "$dir/check-json.json" 200 \
    -H 'Content-Type: application/json' \
    -d '{"code":"function sample() {\n  const a = 1;\n  const b = 2;\n  const c = 3;\n  const d = 4;\n  const e = 5;\n  return a + b + c + d + e;\n}","format":"javascript"}' \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-form.json" 200 \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    --data-urlencode $'code=function sample() {\n  const a = 1;\n  const b = 2;\n  const c = 3;\n  const d = 4;\n  const e = 5;\n  return a + b + c + d + e;\n}' \
    --data-urlencode 'format=javascript' \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-large.json" 200 \
    -H 'Content-Type: application/json' \
    -d @"$dir/check-large-payload.json" \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-special.json" 200 \
    -H 'Content-Type: application/json' \
    -d @"$dir/check-special-payload.json" \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-isolation-1.json" 200 \
    -H 'Content-Type: application/json' \
    -d @"$dir/check-isolation-payload.json" \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-isolation-2.json" 200 \
    -H 'Content-Type: application/json' \
    -d @"$dir/check-isolation-payload.json" \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-missing-code.json" 400 \
    -H 'Content-Type: application/json' \
    -d '{}' \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-empty-code.json" 400 \
    -H 'Content-Type: application/json' \
    -d '{"code":"   ","format":"javascript"}' \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-non-string-code.json" 400 \
    -H 'Content-Type: application/json' \
    -d '{"code":123,"format":"javascript"}' \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-non-string-format.json" 400 \
    -H 'Content-Type: application/json' \
    -d '{"code":"console.log(1);","format":123}' \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-missing-format.json" 400 \
    -H 'Content-Type: application/json' \
    -d '{"code":"console.log(1);"}' \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-empty-format.json" 400 \
    -H 'Content-Type: application/json' \
    -d '{"code":"console.log(1);","format":"   "}' \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-invalid-json.json" 400 \
    -H 'Content-Type: application/json' \
    -d 'invalid-json' \
    "http://127.0.0.1:$port/api/check"
  http_json_with_headers "$dir/not-found.json" "$dir/not-found.headers" 404 "http://127.0.0.1:$port/api/unknown?ignored=true"
  http_json "$dir/wrong-method-get-check.json" 404 \
    -X GET \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/wrong-method-get-recheck.json" 404 \
    -X GET \
    "http://127.0.0.1:$port/api/recheck"
  http_json "$dir/wrong-method-post-stats.json" 404 \
    -X POST \
    "http://127.0.0.1:$port/api/stats"
  http_json "$dir/wrong-method-post-health.json" 404 \
    -X POST \
    "http://127.0.0.1:$port/api/health"
  http_json "$dir/wrong-method-put-check.json" 404 \
    -X PUT \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/wrong-method-delete-stats.json" 404 \
    -X DELETE \
    "http://127.0.0.1:$port/api/stats"

  local mcp_headers="$dir/mcp-headers.txt"
  http_json "$dir/mcp-init.json" 200 \
    -D "$mcp_headers" \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"compat-server","version":"1.0.0"}},"id":1}' \
    "http://127.0.0.1:$port/mcp"
  local session_id
  session_id="$(awk 'tolower($1)=="mcp-session-id:" {print $2}' "$mcp_headers" | tr -d '\r' | tail -n 1)"
  if [[ -z "$session_id" ]]; then
    printf '%s MCP initialize did not return mcp-session-id\n' "$label" >&2
    sed -n '1,80p' "$mcp_headers" >&2
    return 1
  fi
  if [[ ! "$session_id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]; then
    printf '%s MCP initialize returned non-UUID-v4 session id: %s\n' "$label" "$session_id" >&2
    return 1
  fi
  local mcp_tools_headers="$dir/mcp-tools-headers.txt"
  http_json "$dir/mcp-tools.json" 200 \
    -D "$mcp_tools_headers" \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"tools/list","id":2}' \
    "http://127.0.0.1:$port/mcp"
  local tools_session_id
  tools_session_id="$(awk 'tolower($1)=="mcp-session-id:" {print $2}' "$mcp_tools_headers" | tr -d '\r' | tail -n 1)"
  if [[ "$tools_session_id" != "$session_id" ]]; then
    printf '%s MCP tools/list did not echo mcp-session-id\n' "$label" >&2
    sed -n '1,80p' "$mcp_tools_headers" >&2
    return 1
  fi
  http_json "$dir/mcp-resources-list.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"resources/list","id":11}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-stats-tool.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_statistics","arguments":{}},"id":3}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-check-duplication-recheck.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"check_duplication","arguments":{"code":"function test() { console.log('\''hello'\''); }","format":"javascript","recheck":true}},"id":15}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-check-current-directory.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"check_current_directory","arguments":{}},"id":16}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-resource.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"resources/read","params":{"uri":"jscpd://statistics"},"id":4}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-init-form-content-type.json" 400 \
    -H 'Accept: application/json, text/event-stream' \
    -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"compat-server","version":"1.0.0"}},"id":17}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-tools-text-content-type.json" 415 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: text/plain' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"tools/list","id":18}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-no-accept.json" 406 \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"tools/list","id":5}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-invalid-json.json" 400 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d 'invalid-json' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-missing-method.json" 400 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","id":6}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-unknown-method.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"unknown/method","id":7}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-unknown-resource.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"resources/read","params":{"uri":"jscpd://missing"},"id":8}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-unknown-tool.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"missing","arguments":{}},"id":9}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-missing-code.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"check_duplication","arguments":{"format":"javascript"}},"id":10}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-batch-single.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '[{"jsonrpc":"2.0","method":"tools/list","id":12}]' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-batch-multiple.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '[{"jsonrpc":"2.0","method":"tools/list","id":13},{"jsonrpc":"2.0","method":"resources/list","id":14}]' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-batch-empty.body" 202 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '[]' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-get.json" 405 "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-delete.json" 404 \
    -X DELETE \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-options.json" 404 \
    -X OPTIONS \
    "http://127.0.0.1:$port/mcp"

  node --input-type=module - "$label" "$dir" <<'NODE'
import fs from 'node:fs';
import path from 'node:path';

const [label, dir] = process.argv.slice(2);
const read = (file) => JSON.parse(fs.readFileSync(path.join(dir, file), 'utf8'));
const readText = (file) => fs.readFileSync(path.join(dir, file), 'utf8');

for (const file of ['root.headers', 'health.headers', 'stats.headers', 'not-found.headers']) {
  assert(
    /content-type:\s*application\/json/i.test(readText(file)),
    `${label} ${file} JSON content-type`,
  );
}

const root = read('root.json');
assert(root.name === 'jscpd-server', `${label} root name`);
assert(root.endpoints?.['POST /api/check'], `${label} root check endpoint`);
assert(root.endpoints?.['POST /mcp'], `${label} root mcp endpoint`);

const health = read('health.json');
assert(['ready', 'initializing'].includes(health.status), `${label} health status`);
assert(typeof health.workingDirectory === 'string', `${label} health workingDirectory`);

const stats = read('stats.json');
assert(stats.statistics?.total, `${label} stats total`);
assert(typeof stats.timestamp === 'string', `${label} stats timestamp`);

for (const file of ['check-json.json', 'check-form.json']) {
  const body = read(file);
  assert(Array.isArray(body.duplications), `${label} ${file} duplications`);
  assert(typeof body.statistics?.totalDuplications === 'number', `${label} ${file} totalDuplications`);
  assert(typeof body.statistics?.percentageDuplicated === 'number', `${label} ${file} percentageDuplicated`);
  assertNoSnippetCodebase(body, file);
}

const large = read('check-large.json');
assert(Array.isArray(large.duplications), `${label} large snippet duplications`);
assert(large.statistics?.totalLines === 100, `${label} large snippet totalLines`);
assertNoSnippetCodebase(large, 'check-large.json');

const special = read('check-special.json');
assert(Array.isArray(special.duplications), `${label} special chars duplications`);
assert(typeof special.statistics?.totalLines === 'number', `${label} special chars totalLines`);
assertNoSnippetCodebase(special, 'check-special.json');

const isolation1 = read('check-isolation-1.json');
const isolation2 = read('check-isolation-2.json');
assert(Array.isArray(isolation1.duplications), `${label} isolation first duplications`);
assert(Array.isArray(isolation2.duplications), `${label} isolation second duplications`);
assertNoSnippetCodebase(isolation1, 'check-isolation-1.json');
assertNoSnippetCodebase(isolation2, 'check-isolation-2.json');
assert(
  JSON.stringify(isolation2.duplications) === JSON.stringify(isolation1.duplications),
  `${label} repeated snippets should not detect previous snippets`,
);
assert(
  isolation2.statistics?.totalDuplications === isolation1.statistics?.totalDuplications,
  `${label} repeated snippet duplication count should stay stable`,
);

const missingCode = read('check-missing-code.json');
assert(missingCode.error === 'ValidationError', `${label} missing code error`);
assert(missingCode.statusCode === 400, `${label} missing code statusCode`);
assert(missingCode.message === 'Missing required field: code', `${label} missing code message`);

const emptyCode = read('check-empty-code.json');
assert(emptyCode.error === 'ValidationError', `${label} empty code error`);
assert(emptyCode.statusCode === 400, `${label} empty code statusCode`);
assert(emptyCode.message === 'Field "code" cannot be empty', `${label} empty code message`);

const nonStringCode = read('check-non-string-code.json');
assert(nonStringCode.error === 'ValidationError', `${label} non-string code error`);
assert(nonStringCode.statusCode === 400, `${label} non-string code statusCode`);
assert(nonStringCode.message === 'Field "code" must be a string', `${label} non-string code message`);

const nonStringFormat = read('check-non-string-format.json');
assert(nonStringFormat.error === 'ValidationError', `${label} non-string format error`);
assert(nonStringFormat.statusCode === 400, `${label} non-string format statusCode`);
assert(nonStringFormat.message === 'Field "format" must be a string', `${label} non-string format message`);

const missingFormat = read('check-missing-format.json');
assert(missingFormat.error === 'ValidationError', `${label} missing format error`);
assert(missingFormat.statusCode === 400, `${label} missing format statusCode`);
assert(missingFormat.message === 'Missing required field: format', `${label} missing format message`);

const emptyFormat = read('check-empty-format.json');
assert(emptyFormat.error === 'ValidationError', `${label} empty format error`);
assert(emptyFormat.statusCode === 400, `${label} empty format statusCode`);
assert(emptyFormat.message === 'Field "format" cannot be empty', `${label} empty format message`);

const invalidJson = read('check-invalid-json.json');
assert(invalidJson.error === 'SyntaxError', `${label} invalid JSON error`);
assert(invalidJson.statusCode === 400, `${label} invalid JSON statusCode`);
assert(invalidJson.message === 'Unexpected token \'i\', "invalid-json" is not valid JSON', `${label} invalid JSON message`);

const notFound = read('not-found.json');
assert(notFound.error === 'NotFound', `${label} not found error`);
assert(notFound.statusCode === 404, `${label} not found statusCode`);
assert(notFound.message === 'Route GET /api/unknown not found', `${label} not found message`);

for (const [file, method, route] of [
  ['wrong-method-get-check.json', 'GET', '/api/check'],
  ['wrong-method-get-recheck.json', 'GET', '/api/recheck'],
  ['wrong-method-post-stats.json', 'POST', '/api/stats'],
  ['wrong-method-post-health.json', 'POST', '/api/health'],
  ['wrong-method-put-check.json', 'PUT', '/api/check'],
  ['wrong-method-delete-stats.json', 'DELETE', '/api/stats'],
]) {
  const body = read(file);
  assert(body.error === 'NotFound', `${label} ${method} ${route} error`);
  assert(body.statusCode === 404, `${label} ${method} ${route} statusCode`);
  assert(body.message === `Route ${method} ${route} not found`, `${label} ${method} ${route} message`);
}

const init = read('mcp-init.json');
assert(init.result?.serverInfo?.name === 'jscpd-server', `${label} mcp initialize`);
const tools = read('mcp-tools.json');
assert(tools.result?.tools?.some((tool) => tool.name === 'check_duplication'), `${label} mcp tools`);
for (const tool of tools.result?.tools ?? []) {
  assert(tool.inputSchema?.$schema === 'http://json-schema.org/draft-07/schema#', `${label} mcp ${tool.name} schema`);
  assert(tool.execution?.taskSupport === 'forbidden', `${label} mcp ${tool.name} execution`);
}
const resourcesList = read('mcp-resources-list.json');
assert(resourcesList.result?.resources?.some((resource) => resource.uri === 'jscpd://statistics'), `${label} mcp resources list`);
const statsTool = read('mcp-stats-tool.json');
assert(statsTool.result?.content?.[0]?.text?.includes('statistics'), `${label} mcp stats tool`);
const checkDuplicationRecheck = read('mcp-check-duplication-recheck.json');
assert(checkDuplicationRecheck.id === 15, `${label} mcp check_duplication recheck id`);
assert(checkDuplicationRecheck.result?.content?.[0]?.text?.includes('duplications'), `${label} mcp check_duplication recheck duplications`);
assert(checkDuplicationRecheck.result?.content?.[0]?.text?.includes('totalDuplications'), `${label} mcp check_duplication recheck statistics`);
const checkCurrentDirectory = read('mcp-check-current-directory.json');
assert(checkCurrentDirectory.id === 16, `${label} mcp check_current_directory id`);
assert(checkCurrentDirectory.result?.content?.[0]?.text?.includes('statistics'), `${label} mcp check_current_directory statistics`);
assert(checkCurrentDirectory.result?.content?.[0]?.text?.includes('timestamp'), `${label} mcp check_current_directory timestamp`);
const resource = read('mcp-resource.json');
assert(resource.result?.contents?.[0]?.uri === 'jscpd://statistics', `${label} mcp resource`);
assert(!Object.hasOwn(resource.result?.contents?.[0] ?? {}, 'mimeType'), `${label} mcp resource read content mimeType`);

const initFormContentType = read('mcp-init-form-content-type.json');
assert(initFormContentType.error?.code === -32000, `${label} mcp init form content-type code`);
assert(initFormContentType.error?.message === 'Bad Request: No valid session ID provided', `${label} mcp init form content-type message`);

const toolsTextContentType = read('mcp-tools-text-content-type.json');
assert(toolsTextContentType.error?.code === -32000, `${label} mcp tools text content-type code`);
assert(toolsTextContentType.error?.message === 'Unsupported Media Type: Content-Type must be application/json', `${label} mcp tools text content-type message`);
assert(toolsTextContentType.id === null, `${label} mcp tools text content-type id`);

const noAccept = read('mcp-no-accept.json');
assert(noAccept.error?.code === -32000, `${label} mcp no accept code`);
assert(noAccept.error?.message === 'Not Acceptable: Client must accept both application/json and text/event-stream', `${label} mcp no accept message`);

const mcpInvalidJson = read('mcp-invalid-json.json');
assert(mcpInvalidJson.error === 'SyntaxError', `${label} mcp invalid JSON error`);
assert(mcpInvalidJson.statusCode === 400, `${label} mcp invalid JSON statusCode`);
assert(mcpInvalidJson.message === 'Unexpected token \'i\', "invalid-json" is not valid JSON', `${label} mcp invalid JSON message`);

const missingMethod = read('mcp-missing-method.json');
assert(missingMethod.error?.code === -32700, `${label} mcp missing method code`);
assert(missingMethod.error?.message === 'Parse error: Invalid JSON-RPC message', `${label} mcp missing method message`);
assert(missingMethod.id === null, `${label} mcp missing method id`);

const unknownMethod = read('mcp-unknown-method.json');
assert(unknownMethod.error?.code === -32601, `${label} mcp unknown method code`);
assert(unknownMethod.error?.message === 'Method not found', `${label} mcp unknown method message`);

const unknownResource = read('mcp-unknown-resource.json');
assert(unknownResource.error?.code === -32602, `${label} mcp unknown resource code`);
assert(unknownResource.error?.message === 'MCP error -32602: Resource jscpd://missing not found', `${label} mcp unknown resource message`);

const unknownTool = read('mcp-unknown-tool.json');
assert(unknownTool.result?.isError === true, `${label} mcp unknown tool isError`);
assert(unknownTool.result?.content?.[0]?.text === 'MCP error -32602: Tool missing not found', `${label} mcp unknown tool text`);

const missingCodeTool = read('mcp-missing-code.json');
assert(missingCodeTool.result?.isError === true, `${label} mcp missing code isError`);
assert(missingCodeTool.result?.content?.[0]?.text?.includes('Invalid arguments for tool check_duplication'), `${label} mcp missing code text`);
assert(missingCodeTool.result?.content?.[0]?.text?.includes('Invalid input: expected string, received undefined'), `${label} mcp missing code validation`);

const batchSingle = read('mcp-batch-single.json');
assert(batchSingle.result?.tools?.some((tool) => tool.name === 'check_duplication'), `${label} mcp batch single result`);
assert(batchSingle.id === 12, `${label} mcp batch single id`);

const batchMultiple = read('mcp-batch-multiple.json');
assert(Array.isArray(batchMultiple), `${label} mcp batch multiple array`);
assert(batchMultiple.length === 2, `${label} mcp batch multiple length`);
assert(batchMultiple[0]?.result?.tools?.some((tool) => tool.name === 'check_duplication'), `${label} mcp batch multiple tools`);
assert(batchMultiple[1]?.result?.resources?.some((resource) => resource.uri === 'jscpd://statistics'), `${label} mcp batch multiple resources`);

for (const [file, method] of [
  ['mcp-delete.json', 'DELETE'],
  ['mcp-options.json', 'OPTIONS'],
]) {
  const body = read(file);
  assert(body.error === 'NotFound', `${label} mcp ${method} error`);
  assert(body.statusCode === 404, `${label} mcp ${method} statusCode`);
  assert(body.message === `Route ${method} /mcp not found`, `${label} mcp ${method} message`);
}

function assert(condition, message) {
  if (!condition) {
    console.error(message);
    process.exit(1);
  }
}

function assertNoSnippetCodebase(body, file) {
  for (const duplication of body.duplications ?? []) {
    assert(
      !String(duplication.codebaseLocation?.file ?? '').includes('<snippet>'),
      `${label} ${file} leaked snippet path into codebaseLocation`,
    );
  }
}
NODE

  printf 'ok %-18s\n' "$label HTTP/MCP"
}

check_server_http rust "$RUST_PORT"
check_server_http upstream "$UPSTREAM_PORT"

node --input-type=module - "$TMP_ROOT" <<'NODE'
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const [root] = process.argv.slice(2);
const read = (label, file) =>
  JSON.parse(fs.readFileSync(path.join(root, `${label}-http`, file), 'utf8'));

const normalizeInit = (body) => {
  const normalized = structuredClone(body);
  delete normalized.result.serverInfo.version;
  return normalized;
};

assert.deepStrictEqual(
  normalizeInit(read('rust', 'mcp-init.json')),
  normalizeInit(read('upstream', 'mcp-init.json')),
  'MCP initialize stable contract differs from upstream',
);
assert.deepStrictEqual(
  read('rust', 'mcp-tools.json'),
  read('upstream', 'mcp-tools.json'),
  'MCP tools/list contract differs from upstream',
);
assert.deepStrictEqual(
  read('rust', 'mcp-resources-list.json'),
  read('upstream', 'mcp-resources-list.json'),
  'MCP resources/list contract differs from upstream',
);
assert.deepStrictEqual(
  read('rust', 'mcp-batch-single.json'),
  read('upstream', 'mcp-batch-single.json'),
  'MCP single-request batch contract differs from upstream',
);
assert.deepStrictEqual(
  read('rust', 'mcp-batch-multiple.json'),
  read('upstream', 'mcp-batch-multiple.json'),
  'MCP multi-request batch contract differs from upstream',
);

console.log('ok MCP stable contract');
NODE
