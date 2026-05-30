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
  local allowed_missing_coverage="${9:-}"
  local extra_args=()
  if (($# > 9)); then
    extra_args=("${@:10}")
  fi

  if [[ ! -e "$target" ]]; then
    printf 'skip %-28s missing target: %s\n' "$name" "$target"
    return 0
  fi

  printf '\n== %s ==\n' "$name"
  STRICT="$strict_mode" \
    ALLOW_MISSING_COVERAGE="$allowed_missing_coverage" \
    FORMAT="$format" \
    DETECTION_MODE="$detect_mode" \
    MIN_TOKENS="$min_tokens" \
    MIN_LINES="$min_lines" \
    MAX_SIZE="$max_size" \
    "$ROOT/scripts/compat.sh" "$target" "${extra_args[@]}"
}

cd "$ROOT"

run_case "fixtures javascript" "jscpd/fixtures" "javascript" 20 3
run_case "fixtures typescript" "jscpd/fixtures" "typescript" 20 3
run_case "fixtures json" "jscpd/fixtures/javascript" "json" 20 3
run_case "fixtures custom formats-exts" "jscpd/fixtures/custom" "" 50 5 \
  "$DEFAULT_MAX_SIZE" "" "coverage" "" --formats-exts "c:ccc,cc1"
run_case "fixtures ignore blocks" "jscpd/fixtures/ignore" "" 50 5 \
  "$DEFAULT_MAX_SIZE" "" "clone-count"
run_case "fixtures ignore-pattern" "jscpd/fixtures/ignore-pattern" "" 20 5 \
  "$DEFAULT_MAX_SIZE" "" "coverage" "" --ignore-pattern "import.*from\\s*'.*'"
run_case "fixtures ignoreCase off" "jscpd/fixtures/ignore-case" "" 50 5 \
  "$DEFAULT_MAX_SIZE" "" "clone-summary"
run_case "fixtures ignoreCase on" "jscpd/fixtures/ignore-case" "" 50 5 \
  "$DEFAULT_MAX_SIZE" "" "clone-summary" "" --ignoreCase
run_case "fixtures one-file" "jscpd/fixtures/one-file/one-file.js" "" 50 5
run_case "fixtures skipLocal off" "jscpd/fixtures/folder1" "" 50 5 \
  "$DEFAULT_MAX_SIZE" "" "coverage" "" "jscpd/fixtures/folder2"
run_case "fixtures skipLocal on" "jscpd/fixtures/folder1" "" 50 5 \
  "$DEFAULT_MAX_SIZE" "" "coverage" "" "jscpd/fixtures/folder2" --skipLocal
run_case "fixtures mixed-formats" "jscpd/fixtures/mixed-formats" "" 20 3
run_case "fixtures shebang" "jscpd/fixtures/shebang" "" 20 3
run_case "fixtures javascript strict" "jscpd/fixtures/javascript" "javascript" 20 3 "$DEFAULT_MAX_SIZE" "strict" "1"
run_case "fixtures typescript strict" "jscpd/fixtures" "typescript" 20 3 "$DEFAULT_MAX_SIZE" "strict" "1"
run_case "fixtures javascript weak" "jscpd/fixtures/javascript" "javascript" 20 3 "$DEFAULT_MAX_SIZE" "weak" "coverage"
run_case "fixtures jsx" "jscpd/fixtures" "jsx" 20 3
run_case "fixtures tsx" "jscpd/fixtures" "tsx" 20 3
run_case "fixtures markdown" "jscpd/fixtures/markdown" "markdown" 20 3
run_case "fixtures vue" "jscpd/fixtures" "vue" 20 3
run_case "fixtures svelte" "jscpd/fixtures" "svelte" 20 3
run_case "fixtures astro" "jscpd/fixtures" "astro" 20 3
run_case "fixtures css" "jscpd/fixtures/css" "css" 20 3
run_case "fixtures less" "jscpd/fixtures/css" "less" 20 3
run_case "fixtures scss" "jscpd/fixtures/css" "scss" 20 3
run_case "fixtures python" "jscpd/fixtures/python" "python" 20 3
run_case "fixtures go" "jscpd/fixtures/go" "go" 20 3
run_case "fixtures ruby" "jscpd/fixtures/ruby" "ruby" 20 3
run_case "fixtures php" "jscpd/fixtures/php" "php" 20 3
run_case "fixtures yaml" "jscpd/fixtures/yaml" "yaml" 20 3
run_case "fixtures sql" "jscpd/fixtures/sql" "sql" 20 3
run_case "fixtures toml" "jscpd/fixtures/toml" "toml" 20 3
run_case "fixtures bash" "jscpd/fixtures/shell" "bash" 20 3
run_case "fixtures swift" "jscpd/fixtures/swift" "swift" 20 3
run_case "fixtures powershell" "jscpd/fixtures/powershell" "powershell" 20 3
run_case "fixtures lua" "jscpd/fixtures/lua" "lua" 20 3
run_case "fixtures haskell" "jscpd/fixtures/haskell" "haskell" 20 3
run_case "fixtures haskell literate" "jscpd/fixtures/haskell-literate" "haskell" 20 3
run_case "fixtures clojure" "jscpd/fixtures/clojure" "clojure" 20 3
run_case "fixtures sass" "jscpd/fixtures/sass" "sass" 20 3
run_case "fixtures stylus" "jscpd/fixtures/stylus" "stylus" 20 3
run_case "fixtures rust" "jscpd/fixtures/rust" "rust" 20 3
run_case "fixtures dart" "jscpd/fixtures/dart" "dart" 20 3
run_case "fixtures solidity" "jscpd/fixtures/solidity" "solidity" 20 3
run_case "fixtures perl" "jscpd/fixtures/perl" "perl" 20 3
run_case "fixtures lisp" "jscpd/fixtures/commonlisp" "lisp" 20 3
run_case "fixtures ocaml" "jscpd/fixtures/mllike" "ocaml" 20 3
run_case "fixtures fsharp" "jscpd/fixtures/mllike" "fsharp" 20 3
run_case "fixtures objectivec" "jscpd/fixtures/objective-c" "objectivec" 20 3
run_case "fixtures c" "jscpd/fixtures/clike" "c" 20 3
run_case "fixtures z80 as c" "jscpd/fixtures/z80" "c" 20 3
run_case "fixtures cpp" "jscpd/fixtures/clike" "cpp" 20 3
run_case "fixtures c-header" "jscpd/fixtures/clike" "c-header" 20 3
run_case "fixtures cpp-header" "jscpd/fixtures/clike" "cpp-header" 20 3
run_case "fixtures java" "jscpd/fixtures/clike" "java" 20 3
run_case "fixtures csharp" "jscpd/fixtures/clike" "csharp" 20 3
run_case "fixtures kotlin" "jscpd/fixtures/clike" "kotlin" 20 3
run_case "fixtures scala" "jscpd/fixtures/clike" "scala" 20 3
run_case "fixtures groovy" "jscpd/fixtures/groovy" "groovy" 20 3
run_case "fixtures actionscript" "jscpd/fixtures/actionscript" "actionscript" 20 3
run_case "fixtures awk" "jscpd/fixtures/awk" "awk" 20 3
run_case "fixtures basic" "jscpd/fixtures/basic" "basic" 20 3
run_case "fixtures coffeescript" "jscpd/fixtures/coffeescript" "coffeescript" 20 3
run_case "fixtures crystal" "jscpd/fixtures/crystal" "crystal" 20 3
run_case "fixtures d" "jscpd/fixtures/d" "d" 20 3
run_case "fixtures elm" "jscpd/fixtures/elm" "elm" 20 3
run_case "fixtures erlang" "jscpd/fixtures/erlang" "erlang" 20 3
run_case "fixtures fortran" "jscpd/fixtures/fortran" "fortran" 20 3
run_case "fixtures gdscript" "jscpd/fixtures/gdscript" "gdscript" 20 3
run_case "fixtures graphql" "jscpd/fixtures/graphql" "graphql" 20 3
run_case "fixtures julia" "jscpd/fixtures/julia" "julia" 20 3
run_case "fixtures protobuf" "jscpd/fixtures/protobuf" "protobuf" 20 3
run_case "fixtures ada" "jscpd/fixtures/ada" "ada" 20 3
run_case "fixtures apex" "jscpd/fixtures/apex" "apex" 20 3
run_case "fixtures haxe" "jscpd/fixtures/haxe" "haxe" 20 3
run_case "fixtures r" "jscpd/fixtures/r" "r" 20 3
run_case "fixtures csv" "jscpd/fixtures/csv" "csv" 20 3
run_case "fixtures diff" "jscpd/fixtures/diff" "diff" 20 3
run_case "fixtures cmake" "jscpd/fixtures/cmake" "cmake" 20 3
run_case "fixtures hcl" "jscpd/fixtures/hcl" "hcl" 20 3
run_case "fixtures ignore" "jscpd/fixtures/gitignore" "ignore" 20 3
run_case "fixtures json5" "jscpd/fixtures/json5" "json5" 20 3
run_case "fixtures latex" "jscpd/fixtures/latex" "latex" 20 3
run_case "fixtures puppet" "jscpd/fixtures/puppet" "puppet" 20 3
run_case "fixtures qsharp" "jscpd/fixtures/qsharp" "qsharp" 20 3
run_case "fixtures racket" "jscpd/fixtures/racket" "racket" 20 3
run_case "fixtures sas" "jscpd/fixtures/sas" "sas" 20 3
run_case "fixtures scheme" "jscpd/fixtures/scheme" "scheme" 20 3
run_case "fixtures vhdl" "jscpd/fixtures/vhdl" "vhdl" 20 3
run_case "fixtures xquery" "jscpd/fixtures/xquery" "xquery" 20 3
run_case "fixtures verilog" "jscpd/fixtures/verilog" "verilog" 20 3
run_case "fixtures wgsl" "jscpd/fixtures/wgsl" "wgsl" 20 3
run_case "fixtures zig" "jscpd/fixtures/zig" "zig" 20 3
run_case "fixtures tcl" "jscpd/fixtures/tcl" "tcl" 20 3
run_case "fixtures turtle" "jscpd/fixtures/turtle" "turtle" 20 3
run_case "fixtures twig" "jscpd/fixtures/twig" "twig" 20 3
run_case "fixtures properties" "jscpd/fixtures/properties" "properties" 20 3
run_case "fixtures ini" "jscpd/fixtures/properties" "ini" 20 3
run_case "fixtures markup" "jscpd/fixtures/xml" "markup" 20 3
run_case "fixtures htmlmixed markup" "jscpd/fixtures/htmlmixed" "markup" 20 3
run_case "fixtures aspnet" "jscpd/fixtures/htmlembedded" "aspnet" 20 3 \
  "$DEFAULT_MAX_SIZE" "" "coverage" "aspnet:jscpd/fixtures/htmlembedded/file2.aspx:18-43"
run_case "fixtures vbnet" "jscpd/fixtures/vb" "vbnet" 20 3
run_case "fixtures txt" "jscpd/fixtures/text" "txt" 20 3
run_case "fixtures robotframework" "jscpd/fixtures/robotframework" "robotframework" 20 3
run_case "fixtures tap" "jscpd/fixtures/tap" "tap" 20 3
run_case "fixtures textile" "jscpd/fixtures/textile" "textile" 20 3
run_case "fixtures antlr4" "jscpd/fixtures/antlr4" "antlr4" 20 3
run_case "fixtures apl" "jscpd/fixtures/apl" "apl" 20 3
run_case "fixtures bicep" "jscpd/fixtures/bicep" "bicep" 20 3
run_case "fixtures brainfuck" "jscpd/fixtures/brainfuck" "brainfuck" 20 3
run_case "fixtures cfml" "jscpd/fixtures/cfml" "cfml" 20 3
run_case "fixtures cfscript" "jscpd/fixtures/cfscript" "cfscript" 20 3
run_case "fixtures dot" "jscpd/fixtures/dot" "dot" 20 3
run_case "fixtures eiffel" "jscpd/fixtures/eiffel" "eiffel" 20 3
run_case "fixtures gettext" "jscpd/fixtures/gettext" "gettext" 20 3
run_case "fixtures gherkin" "jscpd/fixtures/gherkin" "gherkin" 20 3
run_case "fixtures handlebars" "jscpd/fixtures/handlebars" "handlebars" 20 3
run_case "fixtures idris" "jscpd/fixtures/idris" "idris" 20 3
run_case "fixtures lilypond" "jscpd/fixtures/lilypond" "lilypond" 20 3
run_case "fixtures livescript" "jscpd/fixtures/livescript" "livescript" 20 3
run_case "fixtures linker-script" "jscpd/fixtures/linker-script" "linker-script" 20 3
run_case "fixtures llvm" "jscpd/fixtures/llvm" "llvm" 20 3
run_case "fixtures log" "jscpd/fixtures/log" "log" 20 3
run_case "fixtures nsis" "jscpd/fixtures/nsis" "nsis" 20 3
run_case "fixtures openqasm" "jscpd/fixtures/openqasm" "openqasm" 20 3
run_case "fixtures oz" "jscpd/fixtures/oz" "oz" 20 3
run_case "fixtures pascal" "jscpd/fixtures/pascal" "pascal" 20 3
run_case "fixtures prolog" "jscpd/fixtures/idl" "prolog" 20 3
run_case "fixtures plsql" "jscpd/fixtures/plsql" "plsql" 20 3
run_case "fixtures plant-uml" "jscpd/fixtures/plant-uml" "plant-uml" 20 3
run_case "fixtures powerquery" "jscpd/fixtures/powerquery" "powerquery" 20 3
run_case "fixtures purescript" "jscpd/fixtures/purescript" "purescript" 20 3
run_case "fixtures q" "jscpd/fixtures/q" "q" 20 3
run_case "fixtures rescript" "jscpd/fixtures/rescript" "rescript" 20 3
run_case "fixtures smalltalk" "jscpd/fixtures/smalltalk" "smalltalk" 20 3
run_case "fixtures smarty" "jscpd/fixtures/smarty" "smarty" 20 3
run_case "fixtures soy" "jscpd/fixtures/soy" "soy" 20 3
run_case "fixtures sparql" "jscpd/fixtures/sparql" "sparql" 20 3
run_case "fixtures tt2" "jscpd/fixtures/tt2" "tt2" 20 3
run_case "fixtures unrealscript" "jscpd/fixtures/unrealscript" "unrealscript" 20 3
run_case "fixtures velocity" "jscpd/fixtures/velocity" "velocity" 20 3
run_case "fixtures wolfram" "jscpd/fixtures/mathematica" "wolfram" 20 3
run_case "jscpd packages js" "jscpd/packages" "javascript" 50 5
run_case "jscpd packages ts" "jscpd/packages" "typescript" 50 5
run_case "dream javascript" "/home/dev/dream" "javascript" 50 5
run_case "dream typescript" "/home/dev/dream" "typescript" 50 5
run_case "dream tsx" "/home/dev/dream" "tsx" 50 5
