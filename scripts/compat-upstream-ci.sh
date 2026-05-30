#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT"

MIN_TOKENS=50 \
MIN_LINES=5 \
MAX_SIZE=100kb \
STRICT="${STRICT:-coverage}" \
  scripts/compat.sh jscpd/fixtures
