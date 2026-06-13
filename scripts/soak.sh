#!/usr/bin/env bash
# Memory soak harness (W.1).
#
# Runs animaEngine under Xvfb with a 16-entity synthetic scene (mixed
# asset types, behaviours on so the render loop stays busy), samples
# RSS / decoded bytes / texture count / frame p95 every INTERVAL
# seconds via the in-app soak emitter (ANIMA_SOAK_METRICS), then
# regresses RSS against time and writes a verdict report.
#
# Usage:
#   scripts/soak.sh [DURATION_SECS] [INTERVAL_SECS] [RSS_SLOPE_KIB_PER_MIN]
#   DURATION default 1800 (30 min), INTERVAL 60, threshold 512 KiB/min.
#
# Exit status: 0 if the RSS slope is below the threshold (flat enough),
# 1 if it drifts above it — so CI can gate on it. A run that produces
# too few samples to regress also fails (1).
#
# Output: build/soak-report-<date>.md and the raw build/soak-<date>.csv.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

DURATION="${1:-1800}"
INTERVAL="${2:-60}"
SLOPE_THRESHOLD="${3:-512}" # KiB per minute

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p build
CSV="$REPO/build/soak-${STAMP}.csv"
REPORT="$REPO/build/soak-report-${STAMP}.md"

# ── 1) Build ─────────────────────────────────────────────────────────
log "Building (debug — a leak shows regardless of optimisation)…"
cargo build --locked

BIN="$REPO/target/debug/anima_engine"
[[ -x "$BIN" ]] || die "binary missing at $BIN"

# Generate the demo assets the synthetic scene references, by letting a
# throwaway launch create them (idempotent if they already exist).
[[ -d "$REPO/assets/demo/ghost" ]] || log "Demo assets will be generated on first launch."

# ── 2) Scratch session + 16-entity synthetic config ─────────────────
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/anima-soak.XXXXXX")"
trap 'rm -rf "$SCRATCH"' EXIT
mkdir -p "$SCRATCH/config/animaengine" "$SCRATCH/cache" "$SCRATCH/data"
CONF="$SCRATCH/config/animaengine/config.toml"

{
  echo "version = 2"
  echo ""
  echo "[global]"
  echo "always_on_top = true"
  echo "transparent = true"
  echo "playback_enabled = true"
  echo "window_width = 1280"
  echo "window_height = 720"
  ASSETS=(ghost slime heart star cat)
  for i in $(seq 0 15); do
    asset="${ASSETS[$((i % 5))]}"
    x=$(( (i * 73) % 1200 ))
    y=$(( (i * 137) % 640 ))
    echo ""
    echo "[[characters]]"
    echo "id = \"soak_$i\""
    echo "name = \"Soak $i\""
    echo "asset_type = \"png_sequence\""
    echo "asset_path = \"assets/demo/$asset\""
    echo "x = ${x}.0"
    echo "y = ${y}.0"
    echo "fps = 12.0"
    echo "z_index = $i"
    case $((i % 4)) in
      0) echo '[characters.behavior]'; echo 'type = "walk_around"'; echo 'speed = 80.0' ;;
      1) echo '[characters.behavior]'; echo 'type = "bounded_wander"'; echo 'speed = 120.0' ;;
      2) echo '[characters.behavior]'; echo 'type = "bounce"'; echo 'amplitude_px = 40.0'; echo 'period_sec = 1.5'; echo 'axis = "vertical"' ;;
      3) echo '[characters.behavior]'; echo 'type = "idle"' ;;
    esac
  done
} > "$CONF"

log "Synthetic config: 16 entities at $CONF"

# ── 3) Xvfb ──────────────────────────────────────────────────────────
OWN_XVFB=0
if [[ -z "${DISPLAY:-}" ]]; then
  command -v Xvfb >/dev/null 2>&1 || die "Xvfb not found (install xvfb)"
  Xvfb :99 -screen 0 1280x720x24 >/dev/null 2>&1 &
  XVFB_PID=$!
  OWN_XVFB=1
  export DISPLAY=:99
  sleep 2
  log "Started Xvfb on :99 (pid $XVFB_PID)"
fi

# ── 4) Run the soak ─────────────────────────────────────────────────
log "Soaking for ${DURATION}s, sampling every ${INTERVAL}s…"
XDG_CONFIG_HOME="$SCRATCH/config" \
XDG_CACHE_HOME="$SCRATCH/cache" \
XDG_DATA_HOME="$SCRATCH/data" \
ANIMA_SOAK_METRICS="$CSV" \
ANIMA_SOAK_INTERVAL_SECS="$INTERVAL" \
RUST_LOG="anima_engine=info" \
  timeout --preserve-status -k 5s "${DURATION}s" "$BIN" >"$SCRATCH/app.log" 2>&1 || true

if [[ $OWN_XVFB -eq 1 ]]; then
  kill "${XVFB_PID}" 2>/dev/null || true
fi

[[ -s "$CSV" ]] || die "no metrics written — check $SCRATCH/app.log"
log "Collected $(($(wc -l < "$CSV") - 1)) samples → $CSV"

# ── 5) Regress RSS vs time, write report ────────────────────────────
# Least-squares slope of rss_kib over elapsed_secs (→ KiB/min), plus
# first/last RSS. awk keeps the harness dependency-free.
awk -F, -v thr="$SLOPE_THRESHOLD" -v report="$REPORT" -v csv="$CSV" -v dur="$DURATION" -v iv="$INTERVAL" '
  NR == 1 { next }                       # header
  {
    n++; x=$1; y=$2;
    sx+=x; sy+=y; sxx+=x*x; sxy+=x*y;
    if (n==1) { first=y; firstt=x }
    last=y; lastt=x;
    dec=$3; tex=$4; p95=$5;
  }
  END {
    if (n < 3) {
      printf("FAIL: only %d samples, need >= 3 to regress\n", n);
      exit 2;
    }
    denom = (n*sxx - sx*sx);
    slope_per_sec = (denom != 0) ? (n*sxy - sx*sy) / denom : 0;
    slope_per_min = slope_per_sec * 60.0;
    verdict = (slope_per_min <= thr) ? "FLAT" : "DRIFT";
    delta = last - first;

    printf("# Soak report\n\n") > report;
    printf("- Samples: %d over %ds (interval %ds)\n", n, dur, iv) >> report;
    printf("- RSS first/last: %d / %d KiB (delta %d KiB)\n", first, last, delta) >> report;
    printf("- RSS slope: %.2f KiB/min (threshold %d)\n", slope_per_min, thr) >> report;
    printf("- Decoded bytes (final): %d\n", dec) >> report;
    printf("- Texture count (final): %d\n", tex) >> report;
    printf("- Frame p95 (final): %s us\n", p95) >> report;
    printf("- **Verdict: %s**\n", verdict) >> report;
    printf("\nRaw samples: %s\n", csv) >> report;

    printf("Verdict: %s (slope %.2f KiB/min, threshold %d)\n", verdict, slope_per_min, thr);
    exit (verdict == "FLAT") ? 0 : 1;
  }
' "$CSV"
STATUS=$?

log "Report: $REPORT"
cat "$REPORT"
exit $STATUS
