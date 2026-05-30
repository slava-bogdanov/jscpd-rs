#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STRICT_MODE="${STRICT:-coverage}"
DEFAULT_MAX_SIZE="${MAX_SIZE:-1mb}"

run_case() {
  local name="$1"
  local target="$2"
  local format="$3"
  local min_tokens="$4"
  local min_lines="$5"
  local max_size="${6:-$DEFAULT_MAX_SIZE}"
  local detect_mode="${7:-}"
  local strict_mode="${8:-$STRICT_MODE}"

  if [[ ! -e "$target" ]]; then
    printf 'skip %-28s missing target: %s\n' "$name" "$target"
    return 0
  fi

  printf '\n== %s ==\n' "$name"
  STRICT="$strict_mode" \
    FORMAT="$format" \
    DETECTION_MODE="$detect_mode" \
    MIN_TOKENS="$min_tokens" \
    MIN_LINES="$min_lines" \
    MAX_SIZE="$max_size" \
    "$ROOT/scripts/compat.sh" "$target"
}

cd "$ROOT"

run_case "fixtures javascript" "jscpd/fixtures" "javascript" 20 3
run_case "fixtures typescript" "jscpd/fixtures" "typescript" 20 3
run_case "fixtures javascript strict" "jscpd/fixtures/javascript" "javascript" 20 3 "$DEFAULT_MAX_SIZE" "strict" "1"
run_case "fixtures typescript strict" "jscpd/fixtures" "typescript" 20 3 "$DEFAULT_MAX_SIZE" "strict" "1"
run_case "fixtures javascript weak" "jscpd/fixtures/javascript" "javascript" 20 3 "$DEFAULT_MAX_SIZE" "weak" "coverage"
run_case "fixtures jsx" "jscpd/fixtures" "jsx" 20 3
run_case "fixtures tsx" "jscpd/fixtures" "tsx" 20 3
run_case "fixtures markdown" "jscpd/fixtures/markdown" "markdown" 20 3
run_case "fixtures vue" "jscpd/fixtures" "vue" 20 3
run_case "fixtures svelte" "jscpd/fixtures" "svelte" 20 3
run_case "fixtures astro" "jscpd/fixtures" "astro" 20 3
run_case "fixtures python" "jscpd/fixtures/python" "python" 20 3
run_case "fixtures go" "jscpd/fixtures/go" "go" 20 3
run_case "fixtures ruby" "jscpd/fixtures/ruby" "ruby" 20 3
run_case "fixtures php" "jscpd/fixtures/php" "php" 20 3
run_case "fixtures yaml" "jscpd/fixtures/yaml" "yaml" 20 3
run_case "fixtures sql" "jscpd/fixtures/sql" "sql" 20 3
run_case "fixtures toml" "jscpd/fixtures/toml" "toml" 20 3
run_case "fixtures bash" "jscpd/fixtures/shell" "bash" 20 3
run_case "jscpd packages js" "jscpd/packages" "javascript" 50 5
run_case "jscpd packages ts" "jscpd/packages" "typescript" 50 5
run_case "dream javascript" "/home/dev/dream" "javascript" 50 5
run_case "dream typescript" "/home/dev/dream" "typescript" 50 5
run_case "dream tsx" "/home/dev/dream" "tsx" 50 5
