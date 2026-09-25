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
# The slope is fit over the STEADY STATE only — the first
# SOAK_WARMUP_FRAC (default 0.5) of the run is discarded before
# regressing. A cold start legitimately steps RSS once as the cache
# fills and every entity's assets are decoded/uploaded for the first
# time; fitting a single line across that one-time step reports a
# bogus positive slope even when memory then plateaus flat. A real
# leak keeps climbing through the back half and is still caught. The
# report shows the full-range first/last delta for transparency, but
# the verdict is the post-warmup slope.
#
# Exit status: 0 if the steady-state RSS slope is below the threshold
# (flat enough), 1 if it drifts above it — so CI can gate on it. A run
# with too few post-warmup samples to regress also fails.
#
# Output: build/soak-report-<date>.md, the raw build/soak-<date>.csv, and
# the app's log as build/soak-<date>.log.
#
# Safe to leave unattended for days: the CSV is one row per interval
# (about 10,000 rows for a week at 60 s), and the log is rotated at
# SOAK_LOG_MAX_MB (default 20) to one `.old`, so it never exceeds twice
# that whatever the log level. An uncapped log from a long unattended run
# is how this project has filled a disk before. The app logs at `warn`
# unless SOAK_RUST_LOG says otherwise — the metrics come from the CSV,
# not the log.

set -euo pipefail

# A private session bus. On the caller's own bus the soaked app asks for
# `com.animaengine.Anima` like any launch — so with an overlay already
# running, the single-instance handshake hands off to it and the soak
# measures nothing. CI wraps the script the same way.
# Its config lists no service directories, so nothing is auto-started on
# it: with the stock one, the app's portal probe launched
# xdg-desktop-portal and its desktop backend there.
if [[ -z "${ANIMA_SOAK_PRIVATE_BUS:-}" ]] && command -v dbus-run-session >/dev/null 2>&1; then
  exec env ANIMA_SOAK_PRIVATE_BUS=1 dbus-run-session \
    --config-file="$(dirname "${BASH_SOURCE[0]}")/soak-dbus.conf" -- "$0" "$@"
fi

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

DURATION="${1:-1800}"
INTERVAL="${2:-60}"
SLOPE_THRESHOLD="${3:-512}"            # KiB per minute
WARMUP_FRAC="${SOAK_WARMUP_FRAC:-0.5}" # discard this leading fraction before regressing

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p build
CSV="$REPO/build/soak-${STAMP}.csv"
REPORT="$REPO/build/soak-report-${STAMP}.md"
LOG="$REPO/build/soak-${STAMP}.log"
LOG_MAX_BYTES=$(( ${SOAK_LOG_MAX_MB:-20} * 1024 * 1024 ))

# stdin → $LOG, rotated to $LOG.old once it passes LOG_MAX_BYTES. Bytes,
# not characters, hence LC_ALL=C.
cap_log() {
  LC_ALL=C awk -v f="$LOG" -v max="$LOG_MAX_BYTES" '
    { print > f; fflush(f); n += length($0) + 1
      if (n > max) { close(f); system("mv -f \"" f "\" \"" f ".old\""); n = 0 } }'
}

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
XVFB_PID=""
# Stops Xvfb too: `die` used to leave it running.
cleanup() {
  [[ -n "$XVFB_PID" ]] && kill "$XVFB_PID" 2>/dev/null
  rm -rf "$SCRATCH"
}
trap cleanup EXIT
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
if [[ -z "${DISPLAY:-}" ]]; then
  command -v Xvfb >/dev/null 2>&1 || die "Xvfb not found (install xvfb)"
  Xvfb :99 -screen 0 1280x720x24 >/dev/null 2>&1 &
  XVFB_PID=$!
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
XDG_SESSION_TYPE=x11 \
ANIMA_SOAK_INTERVAL_SECS="$INTERVAL" \
RUST_LOG="${SOAK_RUST_LOG:-anima_engine=warn}" \
  timeout --preserve-status -k 5s "${DURATION}s" "$BIN" 2>&1 | cap_log || true

# Kept after the run, unlike the scratch dir: it is what "no metrics"
# below tells you to read.
[[ -s "$CSV" ]] || die "no metrics written — check $LOG"
log "Collected $(($(wc -l < "$CSV") - 1)) samples → $CSV"

# ── 5) Regress RSS vs time, write report ────────────────────────────
# Least-squares slope of rss_kib over elapsed_secs (→ KiB/min), fit over
# the post-warmup samples only (see header). Full-range first/last is
# kept for the report. awk keeps the harness dependency-free.
awk -F, -v thr="$SLOPE_THRESHOLD" -v report="$REPORT" -v csv="$CSV" -v logfile="$LOG" \
        -v dur="$DURATION" -v iv="$INTERVAL" -v warm="$WARMUP_FRAC" '
  BEGIN { cutoff = dur * warm }          # discard samples before this elapsed time
  NR == 1 { next }                       # header
  {
    n++; x=$1; y=$2;
    if (n==1) { first=y; firstt=x }
    last=y; lastt=x;
    dec=$3; tex=$4; p95=$5;
    # Steady-state accumulators: only samples past the warmup cutoff.
    if (x >= cutoff) {
      m++; sx+=x; sy+=y; sxx+=x*x; sxy+=x*y;
    }
  }
  END {
    delta = last - first;
    if (m < 3) {
      printf("FAIL: only %d post-warmup samples, need >= 3 to regress\n", m);
      exit 2;
    }
    denom = (m*sxx - sx*sx);
    slope_per_sec = (denom != 0) ? (m*sxy - sx*sy) / denom : 0;
    slope_per_min = slope_per_sec * 60.0;
    verdict = (slope_per_min <= thr) ? "FLAT" : "DRIFT";

    printf("# Soak report\n\n") > report;
    printf("- Samples: %d over %ds (interval %ds); regressed %d post-warmup (>=%.0f%%)\n", n, dur, iv, m, warm*100) >> report;
    printf("- RSS first/last: %d / %d KiB (full-range delta %d KiB)\n", first, last, delta) >> report;
    printf("- RSS steady-state slope: %.2f KiB/min (threshold %d)\n", slope_per_min, thr) >> report;
    printf("- Decoded bytes (final): %d\n", dec) >> report;
    printf("- Texture count (final): %d\n", tex) >> report;
    printf("- Frame p95 (final): %s us\n", p95) >> report;
    printf("- **Verdict: %s**\n", verdict) >> report;
    printf("\nRaw samples: %s\n", csv) >> report;
    printf("App log: %s (rotated past the cap to .old)\n", logfile) >> report;

    printf("Verdict: %s (steady-state slope %.2f KiB/min, threshold %d)\n", verdict, slope_per_min, thr);
    exit (verdict == "FLAT") ? 0 : 1;
  }
' "$CSV"
STATUS=$?

log "Report: $REPORT"
cat "$REPORT"
exit $STATUS
