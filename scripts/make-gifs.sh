#!/usr/bin/env bash
# make-gifs.sh — record animated GIFs for release demo assets
#
# Usage:
#   ./scripts/make-gifs.sh [output-dir]          # automated (ffmpeg)
#   ./scripts/make-gifs.sh --peek [output-dir]   # peek-controlled (requires Peek open + positioned)
#
# Requirements: xdotool, ffmpeg
#   --peek mode also requires: peek (open and positioned over the app window)
# The app must be running before you start this script.
#
# Output (version-prefixed, e.g. v0.9.1-01-chat-stream.gif):
#   01-chat-stream      Typing a message and watching the response stream
#   02-theme-switch     Switching themes in Settings → Appearance
#   03-command-palette  Opening and using the command palette
#   04-comparison       Side-by-side model comparison view

set -euo pipefail

# ─── Args ─────────────────────────────────────────────────────────────────────
USE_PEEK=false
if [[ "${1:-}" == "--peek" ]]; then
  USE_PEEK=true
  shift
fi
OUT="${1:-gifs}"

# ─── Config ───────────────────────────────────────────────────────────────────
WINDOW_W=1280
WINDOW_H=820
FPS=12              # frames per second — higher = smoother but larger file
MAX_COLORS=128      # GIF palette size (max 256)

# Delays — increase on slower machines
DELAY_VIEW=0.9
DELAY_DIALOG=0.6

# ─── Settings nav layout (shared with take-screenshots.sh) ────────────────────
NAV_X=156
NAV_Y0=80
NAV_STEP=36
nav_y() { echo $(( NAV_Y0 + $1 * NAV_STEP )); }

# ─── Version from tauri.conf.json ─────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONF="$SCRIPT_DIR/../src-tauri/tauri.conf.json"
VERSION=$(grep -m1 '"version"' "$CONF" | sed 's/.*"version": *"\([^"]*\)".*/\1/')
PREFIX="v${VERSION}-"
echo "Version: $VERSION  mode: $( $USE_PEEK && echo peek || echo ffmpeg )"

# ─── Output dir + previous version management ─────────────────────────────────
archive_to_previous() {
  local files=("$@")
  mkdir -p "$OUT/previous"
  rm -f "$OUT/previous/"*.gif
  mv "${files[@]}" "$OUT/previous/"
}

mkdir -p "$OUT"
shopt -s nullglob
current=("$OUT/${PREFIX}"*.gif)
any_versioned=("$OUT"/v*.gif)
if [[ ${#current[@]} -gt 0 ]]; then
  echo "→ Archiving existing v${VERSION} GIFs to ${OUT}/previous/"
  archive_to_previous "${current[@]}"
elif [[ ${#any_versioned[@]} -gt 0 ]]; then
  echo "→ Archiving previous version GIFs to ${OUT}/previous/"
  archive_to_previous "${any_versioned[@]}"
fi
shopt -u nullglob
echo "GIFs → $(realpath "$OUT")/${PREFIX}*.gif"
echo ""

# ─── Find window ──────────────────────────────────────────────────────────────
get_wid() {
  local wid=""
  for _ in $(seq 1 10); do
    wid=$(xdotool search --name "Linux Claude Desktop" 2>/dev/null | head -1)
    [[ -n "$wid" ]] && echo "$wid" && return
    sleep 0.5
  done
  echo ""
}

WID=$(get_wid)
if [[ -z "$WID" ]]; then
  echo "ERROR: cannot find app window."
  echo "  Start the app first, then re-run this script."
  exit 1
fi

# Resize and position consistently
xdotool windowsize "$WID" "$WINDOW_W" "$WINDOW_H"
xdotool windowmove "$WID" 80 80
xdotool windowfocus --sync "$WID"
xdotool windowraise "$WID"
sleep 0.5

# Get absolute screen position (needed for ffmpeg x11grab)
WIN_GEOM=$(xdotool getwindowgeometry "$WID" 2>/dev/null)
WIN_X=$(echo "$WIN_GEOM" | grep -oP 'Position: \K[0-9]+')
WIN_Y=$(echo "$WIN_GEOM" | grep -oP 'Position: [0-9]+,\K[0-9]+')
DISPLAY_NUM="${DISPLAY:-:0}"

echo "Window: ID=$WID  ${WINDOW_W}×${WINDOW_H}  at (${WIN_X},${WIN_Y})"
if $USE_PEEK; then
  echo "Mode: peek — position the Peek overlay over the app window, then press Enter to begin."
  read -r
else
  echo "Mode: ffmpeg x11grab on ${DISPLAY_NUM}+${WIN_X},${WIN_Y}"
fi
echo ""
echo "Recording scenes…"

# ─── Helpers ──────────────────────────────────────────────────────────────────
focus_win() {
  xdotool windowraise "$WID"
  xdotool windowfocus --sync "$WID"
}

win_click() {
  local rel_x=$1 rel_y=$2
  focus_win
  xdotool mousemove --window "$WID" "$rel_x" "$rel_y"
  sleep 0.1
  xdotool click 1
}

# Raise + focus before every key so the WebView processes the shortcut
key() {
  focus_win
  xdotool key "$@"
}
type_text() {
  focus_win
  xdotool type --clearmodifiers --delay 60 "$1"
}

# Convert raw ffmpeg recording to optimised GIF
to_gif() {
  local src="$1" dst="$2"
  ffmpeg -y -i "$src" \
    -vf "fps=${FPS},scale=${WINDOW_W}:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=${MAX_COLORS}[p];[s1][p]paletteuse=dither=bayer" \
    "$dst" 2>/dev/null
  rm -f "$src"
}

# record_scene <name> <duration_seconds> <action_function>
#   Starts recording, sleeps a lead-in, calls the action function, sleeps a
#   lead-out, then stops and converts.
LEAD_IN=0.4    # seconds before actions start (captures initial state)
LEAD_OUT=0.8   # seconds after actions end (captures final state)

record_scene() {
  local name="${PREFIX}${1}.gif"
  local duration=$2
  local action_fn=$3
  local tmp="/tmp/lcd-gif-$$.mkv"

  echo "  → ${name} (${duration}s)"

  if $USE_PEEK; then
    peek --start
    sleep "$LEAD_IN"
    $action_fn
    sleep "$LEAD_OUT"
    peek --stop
    # Peek saves to ~/Videos or prompts for location — user handles the file
    echo "    ✓ saved by Peek (move to ${OUT}/${name})"
  else
    ffmpeg -y \
      -f x11grab -r "$FPS" \
      -video_size "${WINDOW_W}x${WINDOW_H}" \
      -i "${DISPLAY_NUM}+${WIN_X},${WIN_Y}" \
      -t "$duration" \
      -c:v ffv1 "$tmp" 2>/dev/null &
    FFMPEG_PID=$!

    sleep "$LEAD_IN"
    $action_fn
    sleep "$LEAD_OUT"

    # If ffmpeg hasn't hit the time limit yet, let it run to full duration
    wait "$FFMPEG_PID" 2>/dev/null || true

    to_gif "$tmp" "$OUT/$name"
    local size
    size=$(du -h "$OUT/$name" | cut -f1)
    echo "    ✓ ${name}  (${size})"
  fi
}

# ─── Scene definitions ────────────────────────────────────────────────────────

scene_chat_stream() {
  key "Escape"
  sleep 0.2
  key "ctrl+n"
  sleep 0.5
  # Click the message input (centred at bottom of chat area)
  win_click 760 750
  sleep 0.2
  type_text "What is the Linux philosophy?"
  sleep 0.4
  key "Return"
  sleep 2.5   # wait for streaming response to begin
}

scene_theme_switch() {
  key "ctrl+comma"
  sleep "$DELAY_VIEW"
  win_click "$NAV_X" "$(nav_y 1)"   # Appearance
  sleep "$DELAY_VIEW"
  # Click Light theme option (first theme card / left column)
  # Theme grid: padding ~20px, card ~160px wide, first card centre x ~100 (offset from content area x=56)
  win_click 156 300
  sleep 0.6
  # Click back to default dark theme (second card)
  win_click 336 300
  sleep 0.5
  key "Escape"
}

scene_command_palette() {
  key "Escape"
  sleep 0.2
  key "ctrl+n"
  sleep 0.3
  key "ctrl+p"
  sleep "$DELAY_DIALOG"
  type_text "new chat"
  sleep 0.6
  key "Escape"
}

scene_comparison() {
  key "Escape"
  sleep 0.3
  key "ctrl+shift+m"
  sleep "$DELAY_VIEW"
  # Brief scroll to show both model panels are live
  win_click 640 400
  sleep 0.5
}

# ─── Run scenes ───────────────────────────────────────────────────────────────
record_scene "01-chat-stream"      7  scene_chat_stream
record_scene "02-theme-switch"     6  scene_theme_switch
record_scene "03-command-palette"  5  scene_command_palette
record_scene "04-comparison"       4  scene_comparison

# ─── Summary ──────────────────────────────────────────────────────────────────
echo ""
echo "Done! Saved to ${OUT}/"
if ! $USE_PEEK; then
  for f in "$OUT/${PREFIX}"*.gif; do
    printf "  %5s  %s\n" "$(du -h "$f" | cut -f1)" "$(basename "$f")"
  done
fi
shopt -s nullglob; prev_gifs=("$OUT/previous/"*.gif); shopt -u nullglob
if [[ -d "$OUT/previous" ]] && [[ ${#prev_gifs[@]} -gt 0 ]]; then
  prev_ver=$(basename "${prev_gifs[0]}" | sed 's/\(v[^-]*\)-.*/\1/')
  echo ""
  echo "  Previous ($prev_ver) archived in ${OUT}/previous/"
fi
echo ""
echo "Tips:"
echo "  • Increase FPS (currently ${FPS}) for smoother motion at the cost of file size"
echo "  • Adjust MAX_COLORS (currently ${MAX_COLORS}) for quality vs size trade-off"
echo "  • If theme cards are in the wrong position, adjust win_click coords in scene_theme_switch"
echo "  • For Peek mode: ./scripts/make-gifs.sh --peek"
