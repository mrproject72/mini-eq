#!/usr/bin/env bash
# dev-restart.sh — rebuild and restart mini-eq-rr, detached from the terminal.
#
# Usage:
#   ./dev-restart.sh            # debug build
#   ./dev-restart.sh --release  # release build
#   ./dev-restart.sh --no-build # just restart the existing binary
set -uo pipefail

cd "$(dirname "$0")"

PROFILE="debug"
DO_BUILD=1
# The app logs at INFO; without this the log file stays empty and the
# load/unload diagnostics below are meaningless.
export RUST_LOG="${RUST_LOG:-info}"
for arg in "$@"; do
  case "$arg" in
    --release)   PROFILE="release" ;;
    --no-build)  DO_BUILD=0 ;;
    -h|--help)
      sed -n '2,7p' "$0"; exit 0 ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

LOG=/tmp/eq_live.log

# -x is important: an unanchored pkill can match this very script's command
# line and kill the caller's shell.
if pgrep -x mini-eq-rr >/dev/null; then
  echo "stopping: $(pgrep -x mini-eq-rr | tr '\n' ' ')"
  pkill -x mini-eq-rr
  sleep 2
fi

if [[ "$DO_BUILD" == 1 ]]; then
  # Only the build needs pkg-config to find the vendored GTK/PipeWire deps.
  # Running does not: build.rs bakes deps/ into the binary's rpath.
  export PKG_CONFIG_PATH="$HOME/code/mini-eq-rr/deps/usr/lib/x86_64-linux-gnu/pkgconfig:${PKG_CONFIG_PATH:-}"
  echo "building ($PROFILE)..."
  # `cargo build` is debug by default; only --release exists as a profile flag.
  if [[ "$PROFILE" == "release" ]]; then
    CARGO_ARGS=(--release)
  else
    CARGO_ARGS=()
  fi
  if ! cargo build "${CARGO_ARGS[@]}"; then
    echo "build FAILED — old instance left stopped, nothing restarted." >&2
    exit 1
  fi
fi

BIN="./target/$PROFILE/mini-eq-rr"
if [[ ! -x "$BIN" ]]; then
  echo "no binary at $BIN — run without --no-build" >&2
  exit 1
fi

# setsid + redirect so closing the terminal does not take the app down.
setsid "$BIN" >"$LOG" 2>&1 </dev/null &
sleep 5

PID="$(pgrep -x mini-eq-rr || true)"
if [[ -z "$PID" ]]; then
  echo "did not stay up — last lines of $LOG:" >&2
  tail -20 "$LOG" >&2
  exit 1
fi

echo "running: PID $PID   log: $LOG"
echo "loads:   $(grep -c 'Loading filter-chain' "$LOG")"
echo "unloads: $(grep -c 'unloaded' "$LOG")"
WARN="$(grep -icE 'CRITICAL|panic|exceeds' "$LOG")"
echo "warnings: $WARN"
[[ "$WARN" == 0 ]] || echo "(check $LOG)"
