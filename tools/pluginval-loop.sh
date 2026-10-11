#!/usr/bin/env bash
# Runs pluginval (strictness 5, in-process, GUI tests on) N times on a bundle and fails on the
# first run that is not SUCCESS. Catches intermittent editor hangs a single run misses.
# Usage: tools/pluginval-loop.sh [runs=6] [bundle=$CARGO_TARGET_DIR/bundled/GTS.vst3]
set -u
runs=${1:-6}
bundle=${2:-${CARGO_TARGET_DIR:-/d/cargo-target/riddim-rack}/bundled/GTS.vst3}
pv="$LOCALAPPDATA/pluginval/pluginval.exe"
logdir=${PV_LOGDIR:-${TMPDIR:-/tmp}}
for i in $(seq 1 "$runs"); do
  log="$logdir/pv_loop_$i.log"
  start=$(date +%s)
  "$pv" --strictness-level 5 --timeout-ms 60000 --validate-in-process --validate "$bundle" >"$log" 2>&1
  code=$?
  secs=$(( $(date +%s) - start ))
  if [ $code -ne 0 ] || ! grep -q "SUCCESS" "$log"; then
    echo "run $i/$runs: FAIL (exit $code, ${secs}s) -> $log"; tail -3 "$log"; exit 1
  fi
  echo "run $i/$runs: SUCCESS (${secs}s)"
done
echo "all $runs runs SUCCESS"
