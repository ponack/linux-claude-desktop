#!/usr/bin/env bash
# take-screenshots.sh — capture consistent UI screenshots for release assets
#
# Usage:
#   ./scripts/take-screenshots.sh [output-dir]
#
# Requirements: xdotool, scrot
# The app must be running before you start this script.
#
# Output (version-prefixed, e.g. v0.9.0-01-chat.png):
#   01-chat                  Active chat conversation
#   02-settings-general      Settings → General
#   03-settings-appearance   Settings → Appearance (themes, custom CSS)
#   04-settings-accessibility Settings → Accessibility (font size, contrast, TTS/STT)
#   05-comparison            Side-by-side model comparison
#   06-computer-use          Computer Use view
#   07-extensions            Desktop Extensions catalog
#   08-extensions-install    Extensions install dialog (GitHub)
#   09-command-palette       Command palette

set -euo pipefail

# ─── Config ───────────────────────────────────────────────────────────────────
OUT="${1:-screenshots}"
WINDOW_W=1280
WINDOW_H=820

# Delays — increase on slower machines
DELAY_VIEW=0.9      # after switching views
DELAY_DIALOG=0.6    # after opening dialogs/modals
DELAY_LAUNCH=3.5    # waiting for app to appear (used if you launch via APP_BIN below)

# ─── Settings nav layout (derived from CSS) ───────────────────────────────────
# Sidebar collapsed width: 56px  →  Settings nav starts at x=56
# Settings nav width: 200px      →  nav centre x = 56+100 = 156
# Nav padding-top: 16px, header: ~46px  →  items start at y=62
# Each nav item: ~36px tall (8px padding × 2 + ~18px line + 2px gap)
#
# Section order (index → label):
#  0=General  1=Appearance  2=Prompts  3=Projects  4=Integrations
#  5=Schedules  6=Endpoints  7=Routing  8=Knowledge  9=Data & Usage
#  10=Accessibility  11=Computer Use  12=About
NAV_X=156
NAV_Y0=80      # centre-y of item 0 (General)
NAV_STEP=36    # pixels per item

nav_y() { echo $(( NAV_Y0 + $1 * NAV_STEP )); }

# ─── Version from tauri.conf.json ─────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONF="$SCRIPT_DIR/../src-tauri/tauri.conf.json"
VERSION=$(grep -m1 '"version"' "$CONF" | sed 's/.*"version": *"\([^"]*\)".*/\1/')
PREFIX="v${VERSION}-"
echo "Version: $VERSION"

# ─── Previous version management ──────────────────────────────────────────────
archive_to_previous() {
  local files=("$@")
  mkdir -p "$OUT/previous"
  rm -f "$OUT/previous/"*.png
  mv "${files[@]}" "$OUT/previous/"
}

mkdir -p "$OUT"
shopt -s nullglob
current=("$OUT/${PREFIX}"*.png)
any_versioned=("$OUT"/v*.png)
if [[ ${#current[@]} -gt 0 ]]; then
  echo "→ Archiving existing v${VERSION} screenshots to ${OUT}/previous/"
  archive_to_previous "${current[@]}"
elif [[ ${#any_versioned[@]} -gt 0 ]]; then
  echo "→ Archiving previous version screenshots to ${OUT}/previous/"
  archive_to_previous "${any_versioned[@]}"
fi
shopt -u nullglob
echo "Screenshots → $(realpath "$OUT")/${PREFIX}*.png"
echo ""

# ─── Helpers ──────────────────────────────────────────────────────────────────
get_wid() {
  local wid=""
  for _ in $(seq 1 10); do
    wid=$(xdotool search --name "Linux Claude Desktop" 2>/dev/null | head -1)
    [[ -n "$wid" ]] && echo "$wid" && return
    sleep 0.5
  done
  echo ""
}

# Click at coordinates relative to the app window (avoids screen-position guessing)
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

snap() {
  local name="${PREFIX}${1}"
  sleep "$DELAY_VIEW"
  scrot --window "$WID" --border "$OUT/$name"
  echo "  ✓ $name"
}

# ─── Find window ──────────────────────────────────────────────────────────────
WID=$(get_wid)
if [[ -z "$WID" ]]; then
  echo "ERROR: cannot find app window."
  echo "  Start the app first, then re-run this script."
  echo "  Or set: APP_BIN=/path/to/linux-claude-desktop; \$APP_BIN &"
  exit 1
fi

# Resize and position consistently
xdotool windowsize "$WID" "$WINDOW_W" "$WINDOW_H"
xdotool windowmove "$WID" 80 80
xdotool windowfocus --sync "$WID"
xdotool windowraise "$WID"
sleep 0.5
echo "Window: ID=$WID  ${WINDOW_W}×${WINDOW_H}"
echo ""
echo "Capturing…"

# ─── 01: Chat ─────────────────────────────────────────────────────────────────
echo "  → 01 Chat"
key "Escape"            # close any open view
sleep 0.2
key "ctrl+n"            # new chat
sleep 0.3
snap "01-chat.png"

# ─── 02: Settings → General ───────────────────────────────────────────────────
echo "  → 02 Settings > General"
key "ctrl+comma"
# General is the default active tab — no click needed
snap "02-settings-general.png"

# ─── 03: Settings → Appearance ────────────────────────────────────────────────
echo "  → 03 Settings > Appearance"
win_click "$NAV_X" "$(nav_y 1)"   # index 1 = Appearance
snap "03-settings-appearance.png"

# ─── 04: Settings → Accessibility (also contains TTS/STT) ────────────────────
echo "  → 04 Settings > Accessibility"
win_click "$NAV_X" "$(nav_y 10)"  # index 10 = Accessibility
snap "04-settings-accessibility.png"

# ─── 05: Comparison view ──────────────────────────────────────────────────────
echo "  → 05 Comparison"
key "Escape"
sleep 0.3
key "ctrl+shift+m"
snap "05-comparison.png"

# ─── 06: Computer Use ─────────────────────────────────────────────────────────
echo "  → 06 Computer Use"
key "Escape"
sleep 0.3
key "ctrl+shift+u"
snap "06-computer-use.png"

# ─── 07: Extensions catalog ───────────────────────────────────────────────────
echo "  → 07 Extensions"
key "Escape"
sleep 0.3
key "ctrl+shift+e"
snap "07-extensions.png"

# ─── 08: Extensions install dialog (GitHub card) ─────────────────────────────
# Extensions grid layout at 1280×820 with 56px collapsed sidebar:
#   Content area: 1224px wide, padding: 16px 20px
#   ~4 columns of 280px cards with 14px gap
#   Header (52px) + toolbar/search+tabs (~90px) + grid padding (16px) = 158px to first card
#   Install button: bottom-right of first card
#   Approx window-relative: x=316 y=300
# ── Tune these if the click misses ──
INSTALL_BTN_X=316
INSTALL_BTN_Y=300
echo "  → 08 Extensions install dialog"
win_click "$INSTALL_BTN_X" "$INSTALL_BTN_Y"
sleep "$DELAY_DIALOG"
snap "08-extensions-install.png"
key "Escape"   # close dialog

# ─── 09: Command palette ──────────────────────────────────────────────────────
echo "  → 09 Command palette"
key "Escape"
sleep 0.3
key "ctrl+n"
sleep 0.2
key "ctrl+p"
sleep "$DELAY_DIALOG"
snap "09-command-palette.png"
key "Escape"

# ─── Summary ──────────────────────────────────────────────────────────────────
echo ""
echo "Done! Saved to ${OUT}/"
for f in "$OUT/${PREFIX}"*.png; do
  printf "  %5s  %s\n" "$(du -h "$f" | cut -f1)" "$(basename "$f")"
done
shopt -s nullglob; prev_pngs=("$OUT/previous/"*.png); shopt -u nullglob
if [[ -d "$OUT/previous" ]] && [[ ${#prev_pngs[@]} -gt 0 ]]; then
  prev_ver=$(basename "${prev_pngs[0]}" | sed 's/\(v[^-]*\)-.*/\1/')
  echo ""
  echo "  Previous ($prev_ver) archived in ${OUT}/previous/"
fi
echo ""
echo "Tip: if Settings tabs or the Extensions install button are off, adjust"
echo "     NAV_Y0 / NAV_STEP (nav coords) or INSTALL_BTN_X/Y near the bottom"
echo "     of the script, then re-run."
