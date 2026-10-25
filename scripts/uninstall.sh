#!/usr/bin/env bash
# ==============================================================================
# Cuddle (FileWorkbench) Desktop Launcher & Binary Uninstaller
# Supports both user-level (~/.local) and system-wide (/usr/local) uninstalls.
# ==============================================================================
set -euo pipefail

# ANSI Styling
BOLD='\033[1m'
CYAN='\033[38;2;0;229;255m'
GREEN='\033[38;2;16;185;129m'
YELLOW='\033[38;2;245;158;11m'
RED='\033[38;2;239;68;68m'
RESET='\033[0m'

log_info()  { printf "${CYAN}ℹ %s${RESET}\n" "$*"; }
log_succ()  { printf "${GREEN}✔ %s${RESET}\n" "$*"; }
log_warn()  { printf "${YELLOW}⚠ %s${RESET}\n" "$*"; }
log_err()   { printf "${RED}✖ %s${RESET}\n" "$*" >&2; }

MODE="user"

print_help() {
  cat <<EOF
${BOLD}Usage:${RESET} $0 [OPTIONS]

Uninstalls Cuddle binaries, desktop launcher, and icon files.

${BOLD}Options:${RESET}
  --user        Uninstall from user directory ~/.local (Default)
  --system      Uninstall from system-wide /usr/local (Requires root/sudo)
  -h, --help    Show this help message
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --user)
      MODE="user"
      shift
      ;;
    --system)
      MODE="system"
      shift
      ;;
    -h|--help)
      print_help
      exit 0
      ;;
    *)
      log_err "Unknown option: $1"
      print_help
      exit 1
      ;;
  esac
done

if [[ "$MODE" == "system" ]]; then
  if [[ $EUID -ne 0 ]]; then
    log_err "System uninstall requires root permissions. Please run with sudo: sudo $0 --system"
    exit 1
  fi
  BIN_DIR="/usr/local/bin"
  DATA_DIR="/usr/local/share"
else
  BIN_DIR="${HOME}/.local/bin"
  DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
fi

APP_DIR="${DATA_DIR}/applications"
ICON_DIR="${DATA_DIR}/icons"
PIXMAPS_DIR="${DATA_DIR}/pixmaps"

log_info "Uninstalling Cuddle (${MODE} mode)..."

# 1. Remove binary and symlinks
for b in "cuddle" "fileworkbench"; do
  if [[ -f "${BIN_DIR}/${b}" || -L "${BIN_DIR}/${b}" ]]; then
    rm -f "${BIN_DIR}/${b}"
    log_succ "Removed binary: ${BIN_DIR}/${b}"
  fi
done

# 2. Remove .desktop files
for d in "com.wammed.cuddle.desktop" "cuddle.desktop"; do
  if [[ -f "${APP_DIR}/${d}" || -L "${APP_DIR}/${d}" ]]; then
    rm -f "${APP_DIR}/${d}"
    log_succ "Removed desktop entry: ${APP_DIR}/${d}"
  fi
done

# 3. Remove icons
for s in "32x32" "64x64" "128x128" "512x512"; do
  for name in "com.wammed.cuddle.png" "cuddle.png"; do
    target="${ICON_DIR}/hicolor/${s}/apps/${name}"
    if [[ -f "$target" ]]; then
      rm -f "$target"
      log_succ "Removed icon: ${target}"
    fi
  done
done

for name in "com.wammed.cuddle.svg" "cuddle.svg"; do
  target="${ICON_DIR}/hicolor/scalable/apps/${name}"
  if [[ -f "$target" ]]; then
    rm -f "$target"
    log_succ "Removed icon: ${target}"
  fi
done

for name in "com.wammed.cuddle.png" "cuddle.png"; do
  target="${PIXMAPS_DIR}/${name}"
  if [[ -f "$target" ]]; then
    rm -f "$target"
    log_succ "Removed pixmap: ${target}"
  fi
done

# 4. Refresh Desktop & Icon Caches
log_info "Refreshing desktop and icon caches..."

if command -v update-desktop-database >/dev/null 2>&1; then
  if [[ -d "${APP_DIR}" ]]; then
    update-desktop-database "${APP_DIR}" 2>/dev/null || true
    log_succ "Updated desktop database: ${APP_DIR}"
  fi
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  if [[ -d "${ICON_DIR}/hicolor" ]]; then
    gtk-update-icon-cache -f -t "${ICON_DIR}/hicolor" 2>/dev/null || true
    log_succ "Updated GTK icon cache: ${ICON_DIR}/hicolor"
  fi
elif command -v gtk4-update-icon-cache >/dev/null 2>&1; then
  if [[ -d "${ICON_DIR}/hicolor" ]]; then
    gtk4-update-icon-cache -f -t "${ICON_DIR}/hicolor" 2>/dev/null || true
    log_succ "Updated GTK4 icon cache: ${ICON_DIR}/hicolor"
  fi
fi

if command -v kbuildsycoca6 >/dev/null 2>&1; then
  kbuildsycoca6 --noincremental 2>/dev/null || true
elif command -v kbuildsycoca5 >/dev/null 2>&1; then
  kbuildsycoca5 --noincremental 2>/dev/null || true
fi

printf "\n${GREEN}${BOLD}✔ Cuddle has been completely uninstalled.${RESET}\n\n"
