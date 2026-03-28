#!/usr/bin/env bash
# take-screenshots.sh — capture consistent UI screenshots for release assets
#
# Usage:
#   ./scripts/take-screenshots.sh [output-dir]
#
# Requirements: xdotool, scrot, dbus-send
# The app must already be installed/running (or adjust APP_BIN below).
#
# Captured views:
#   01-chat.png          — Chat view with a sample conversation loaded
#   02-sidebar.png       — Sidebar expanded
#   03-settings.png      — Settings > General
#   04-settings-voice.png — Settings > Voice tab
#   05-settings-accessibility.png — Settings > Accessibility tab
#   06-comparison.png    — Model comparison view
#   07-computer-use.png  — Computer Use view
#   08-extensions.png    — Extensions catalog
#   09-extensions-install.png — Extensions install dialog (GitHub)
#   10-command-palette.png — Command palette

set -euo pipefail

# --- Config ---
OUT="${1:-screenshots}"
WINDOW_W=1280
WINDOW_H=820
APP_BIN="${APP_BIN:-linux-claude-desktop}"  # override with full path if needed
DBUS_DEST="com.linux_claude_desktop.App"
DBUS_PATH="/com/linux_claude_desktop/App"
DBUS_IFACE="com.linux_claude_desktop.App"

# Delays (seconds) — increase if your machine is slow
DELAY_LAUNCH=3.0   # wait for app to appear after launch
DELAY_VIEW=0.8     # wait after switching views
DELAY_DIALOG=0.5   # wait after opening dialogs

mkdir -p "$OUT"
echo "Screenshots will be saved to: $(realpath "$OUT")"

# --- Helpers ---
dbus_call() {
  dbus-send --session --dest="$DBUS_DEST" --type=method_call \
    "$DBUS_PATH" "${DBUS_IFACE}.${1}" "${@:2}" 2>/dev/null
}

# Get the app window ID (retry up to 5s)
get_wid() {
  local wid=""
  for _ in $(seq 1 10); do
    wid=$(xdotool search --name "Linux Claude Desktop" 2>/dev/null | head -1)
    [[ -n "$wid" ]] && echo "$wid" && return
    sleep 0.5
  done
  echo ""
}

key() { xdotool key --window "$WID" "$@"; }

snap() {
  local name="$1"
  sleep "$DELAY_VIEW"
  scrot --window "$WID" --border "$OUT/$name"
  echo "  ✓ $name"
}

# --- Launch / focus app ---
echo "→ Launching / focusing app…"
if dbus_call "show" 2>/dev/null; then
  echo "  (app already running, brought to front)"
else
  echo "  (starting app…)"
  "$APP_BIN" &
  sleep "$DELAY_LAUNCH"
fi

WID=$(get_wid)
if [[ -z "$WID" ]]; then
  echo "ERROR: could not find app window. Is '$APP_BIN' in your PATH?"
  echo "Set APP_BIN=/path/to/linux-claude-desktop and re-run."
  exit 1
fi

# Resize and position window consistently
xdotool windowsize "$WID" "$WINDOW_W" "$WINDOW_H"
xdotool windowmove "$WID" 80 80
xdotool windowfocus --sync "$WID"
xdotool windowraise "$WID"
sleep 0.4

echo "→ Window: ID=$WID  size=${WINDOW_W}x${WINDOW_H}"
echo ""
echo "Capturing views…"

# --- 01: Chat (new chat, no sidebar collapse) ---
echo "  → Chat view"
key "ctrl+n"
sleep 0.3
# Send a sample question so the chat isn't empty
dbus_call "ask" "string:Tell me about the Linux philosophy in one sentence."
sleep 1.5
snap "01-chat.png"

# --- 02: Sidebar visible (wide window, sidebar expanded) ---
echo "  → Sidebar"
# Sidebar is always visible in chat view; already captured above, but do a clean new-chat
key "ctrl+n"
snap "02-sidebar.png"

# --- 03: Settings — General ---
echo "  → Settings"
key "ctrl+comma"
snap "03-settings.png"

# --- 04: Settings — Voice ---
echo "  → Settings > Voice"
# Click the Voice tab via keyboard: the tab order is General/Model/Voice/Accessibility/...
# Use xdotool to click the Voice tab text directly
sleep "$DELAY_VIEW"
WID_VOICE=$(xdotool search --name "Linux Claude Desktop" | head -1)
xdotool key --window "$WID_VOICE" Tab Tab Tab  # navigate to Voice tab
# Simpler: use xdotool click on the tab position
xdotool mousemove --window "$WID" 0 0  # reset mouse
# Voice tab is roughly at x=~190 in the sidebar nav — use findimage or fixed coords
# Settings nav is a left column; tabs are stacked vertically ~56px each starting at ~100px
# Order: General(0), Model(1), Provider(2), MCP(3), Voice(4), Accessibility(5), ...
# Approximate y positions at x=88 (center of left nav):
xdotool mousemove --window "$WID" 88 372  # Voice tab (approx y=100 + 4*56=324, center ~372)
xdotool click 1
sleep "$DELAY_VIEW"
snap "04-settings-voice.png"

# --- 05: Settings — Accessibility ---
echo "  → Settings > Accessibility"
xdotool mousemove --window "$WID" 88 428  # next tab down
xdotool click 1
sleep "$DELAY_VIEW"
snap "05-settings-accessibility.png"

# --- 06: Close settings, open Comparison ---
echo "  → Comparison view"
key "Escape"
sleep 0.3
key "ctrl+shift+m"
snap "06-comparison.png"

# --- 07: Computer Use ---
echo "  → Computer Use"
key "Escape"
sleep 0.3
key "ctrl+shift+u"
snap "07-computer-use.png"

# --- 08: Extensions catalog ---
echo "  → Extensions"
key "Escape"
sleep 0.3
key "ctrl+shift+e"
snap "08-extensions.png"

# --- 09: Extensions — install dialog (click GitHub Install button) ---
echo "  → Extensions install dialog"
sleep "$DELAY_VIEW"
# GitHub is the first card; Install button is bottom-right of the card
# With grid layout at 1280px, first card starts ~20px from left, ~190px from top (after header+toolbar)
# Card is ~280px wide, button is near bottom-right
xdotool mousemove --window "$WID" 316 330  # approx Install button on GitHub card
xdotool click 1
sleep "$DELAY_DIALOG"
snap "09-extensions-install.png"

# Close dialog
key "Escape"

# --- 10: Command palette ---
echo "  → Command palette"
key "Escape"
sleep 0.3
key "ctrl+n"
sleep 0.3
key "ctrl+p"
sleep "$DELAY_DIALOG"
snap "10-command-palette.png"

# Close palette
key "Escape"

echo ""
echo "Done! ${OUT}/ contains:"
ls -1 "$OUT"/*.png | while read -r f; do
  size=$(du -h "$f" | cut -f1)
  echo "  $size  $(basename "$f")"
done
echo ""
echo "Tip: review the screenshots and re-run individual steps if any look wrong."
echo "     Increase DELAY_VIEW / DELAY_LAUNCH at the top of the script on slow machines."
