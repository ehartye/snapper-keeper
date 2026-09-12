#!/usr/bin/env bash
# Exercise the real launcher without building or opening a native application.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/snk-launcher.XXXXXX")"
trap 'rm -rf -- "$FIXTURE"' EXIT
mkdir -p "$FIXTURE/bin"

cat > "$FIXTURE/bin/git" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$CASE_ROOT"
SH
cat > "$FIXTURE/bin/uname" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$CASE_ARCH"
SH
cat > "$FIXTURE/bin/pnpm" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'build\n' >> "$CASE_ROOT/stages"
[[ "$CASE_FAILURE" != build ]] || exit 17
target=""
while (( $# )); do
  if [[ "$1" == --target ]]; then
    target="$2"
    shift
  fi
  shift
done
output="${CARGO_TARGET_DIR:-$CASE_ROOT/target}"
[[ -z "$target" ]] || output="$output/$target"
bundle="$output/debug/bundle/macos/Snapper Keeper.app"
mkdir -p "$bundle/Contents/MacOS"
printf 'fresh\n' > "$bundle/Contents/MacOS/snapper-keeper-app"
printf '%s\n' "$bundle" > "$CASE_ROOT/built"
SH
cat > "$FIXTURE/bin/codesign" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
bundle="${!#}"
[[ -f "$bundle/Contents/MacOS/snapper-keeper-app" ]] || exit 18
[[ "$1" != --force ]] || {
  printf 'sign\n' >> "$CASE_ROOT/stages"
  [[ "$CASE_FAILURE" != sign ]] || exit 19
  printf '%s\n' "$bundle" > "$CASE_ROOT/signed"
}
SH
cat > "$FIXTURE/bin/open" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
bundle="${!#}"
printf '%s\n' "$bundle" > "$CASE_ROOT/launched"
cat "$bundle/Contents/MacOS/snapper-keeper-app" > "$CASE_ROOT/launched-content"
SH
chmod +x "$FIXTURE/bin/"*

failures=0
run_case() {
  local name="$1" arch="$2" failure="$3" override="$4" target expected exit_code=0
  local expected_exit expected_stages actual_stages
  export CASE_ROOT="$FIXTURE/$name" CASE_ARCH="$arch" CASE_FAILURE="$failure"
  case "$arch" in
    arm64) target=aarch64-apple-darwin ;;
    x86_64) target=x86_64-apple-darwin ;;
    *) target=unsupported ;;
  esac
  expected="$CASE_ROOT/target/$target/debug/bundle/macos/Snapper Keeper.app"
  mkdir -p "$CASE_ROOT/app/src-tauri" "$expected/Contents/MacOS"
  # A prior build must never hide build failures or be launched as fresh output.
  printf 'stale\n' > "$expected/Contents/MacOS/snapper-keeper-app"
  (
    unset CARGO_TARGET_DIR
    if [[ "$override" == yes ]]; then
      export CARGO_TARGET_DIR="$CASE_ROOT/custom target"
    fi
    PATH="$FIXTURE/bin:$PATH" bash "$SCRIPT_DIR/dev-macos.sh"
  ) > "$CASE_ROOT/output.log" 2>&1 || exit_code=$?

  if [[ "$failure" != none || "$target" == unsupported ]]; then
    case "$failure:$target" in
      build:*) expected_exit=17; expected_stages=build ;;
      sign:*) expected_exit=19; expected_stages=$'build\nsign' ;;
      none:unsupported) expected_exit=1; expected_stages='' ;;
    esac
    actual_stages="$(cat "$CASE_ROOT/stages" 2>/dev/null || true)"
    if (( exit_code == expected_exit )) &&
      [[ "$actual_stages" == "$expected_stages" ]] &&
      [[ ! -f "$CASE_ROOT/signed" && ! -f "$CASE_ROOT/launched" ]] &&
      { [[ "$target" != unsupported ]] ||
        [[ "$(cat "$CASE_ROOT/output.log")" == *"unsupported architecture: $arch"* ]]; }; then
      printf 'PASS %s (expected exit %s and failure stage; no launch)\n' "$name" "$expected_exit"
      return
    fi
  elif (( exit_code == 0 )) &&
    [[ "$(cat "$CASE_ROOT/built")" == "$expected" ]] &&
    [[ "$(cat "$CASE_ROOT/signed")" == "$expected" ]] &&
    [[ "$(cat "$CASE_ROOT/launched")" == "$expected" ]] &&
    [[ "$(cat "$CASE_ROOT/launched-content")" == fresh ]]; then
    printf 'PASS %s (built, signed and launched fresh expected bundle)\n' "$name"
    return
  fi
  printf 'FAIL %s (launcher exit %s)\n' "$name" "$exit_code" >&2
  cat "$CASE_ROOT/output.log" >&2
  failures=$((failures + 1))
}

run_case arm64-default arm64 none no
run_case intel-default x86_64 none no
run_case inherited-target-dir arm64 none yes
run_case failed-build arm64 build no
run_case failed-signing x86_64 sign no
run_case unsupported-architecture riscv64 none no
(( failures == 0 ))
