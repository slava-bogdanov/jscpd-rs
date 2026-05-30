#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${TARGET:-$ROOT/jscpd/fixtures/javascript}"
TMP_ROOT="${TMP_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/jscpd-rs-server.XXXXXX")}"
RUST_PORT="${RUST_PORT:-39981}"
UPSTREAM_PORT="${UPSTREAM_PORT:-39982}"
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

check_server_cli() {
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

  printf 'ok %-18s\n' "$label CLI"
}

check_server_cli rust "$ROOT/target/release/jscpd-server"
check_server_cli upstream node "$ROOT/jscpd/apps/jscpd-server/bin/jscpd-server"

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

check_server_http() {
  local label="$1"
  local port="$2"
  local dir="$TMP_ROOT/$label-http"
  mkdir -p "$dir"

  http_json "$dir/root.json" 200 "http://127.0.0.1:$port/"
  http_json "$dir/health.json" 200 "http://127.0.0.1:$port/api/health"
  http_json "$dir/stats.json" 200 "http://127.0.0.1:$port/api/stats"
  http_json "$dir/check-json.json" 200 \
    -H 'Content-Type: application/json' \
    -d '{"code":"function sample() {\n  const a = 1;\n  const b = 2;\n  const c = 3;\n  const d = 4;\n  const e = 5;\n  return a + b + c + d + e;\n}","format":"javascript"}' \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/check-form.json" 200 \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    --data-urlencode $'code=function sample() {\n  const a = 1;\n  const b = 2;\n  const c = 3;\n  const d = 4;\n  const e = 5;\n  return a + b + c + d + e;\n}' \
    --data-urlencode 'format=javascript' \
    "http://127.0.0.1:$port/api/check"
  http_json "$dir/not-found.json" 404 "http://127.0.0.1:$port/api/unknown?ignored=true"

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
  http_json "$dir/mcp-tools.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"tools/list","id":2}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-stats-tool.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_statistics","arguments":{}},"id":3}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-resource.json" 200 \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Content-Type: application/json' \
    -H "mcp-session-id: $session_id" \
    -d '{"jsonrpc":"2.0","method":"resources/read","params":{"uri":"jscpd://statistics"},"id":4}' \
    "http://127.0.0.1:$port/mcp"
  http_json "$dir/mcp-get.json" 405 "http://127.0.0.1:$port/mcp"

  node --input-type=module - "$label" "$dir" <<'NODE'
import fs from 'node:fs';
import path from 'node:path';

const [label, dir] = process.argv.slice(2);
const read = (file) => JSON.parse(fs.readFileSync(path.join(dir, file), 'utf8'));

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
}

const notFound = read('not-found.json');
assert(notFound.error === 'NotFound', `${label} not found error`);
assert(notFound.statusCode === 404, `${label} not found statusCode`);
assert(notFound.message === 'Route GET /api/unknown not found', `${label} not found message`);

const init = read('mcp-init.json');
assert(init.result?.serverInfo?.name === 'jscpd-server', `${label} mcp initialize`);
const tools = read('mcp-tools.json');
assert(tools.result?.tools?.some((tool) => tool.name === 'check_duplication'), `${label} mcp tools`);
const statsTool = read('mcp-stats-tool.json');
assert(statsTool.result?.content?.[0]?.text?.includes('statistics'), `${label} mcp stats tool`);
const resource = read('mcp-resource.json');
assert(resource.result?.contents?.[0]?.uri === 'jscpd://statistics', `${label} mcp resource`);

function assert(condition, message) {
  if (!condition) {
    console.error(message);
    process.exit(1);
  }
}
NODE

  printf 'ok %-18s\n' "$label HTTP/MCP"
}

check_server_http rust "$RUST_PORT"
check_server_http upstream "$UPSTREAM_PORT"
