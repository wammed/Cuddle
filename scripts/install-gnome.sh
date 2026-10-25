#!/usr/bin/env bash
# ==============================================================================
# Cuddle Installer - GNOME (Wayland / X11) Dedicated Preset
# Optimized for GNOME Shell (Dash to Dock, Activities, StartupWMClass).
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

C_RESET="\033[0m"
C_BOLD="\033[1m"
C_TEAL="\033[38;2;45;212;191m"
C_CYAN="\033[38;2;56;189;248m"
C_GREEN="\033[38;2;52;211;153m"

echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"
echo -e "${C_TEAL}${C_BOLD}   Cuddle - GNOME Desktop Environment Installer                               ${C_RESET}"
echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"
echo -e "  • Target DE:      ${C_CYAN}GNOME Shell (Mutter / Wayland / X11)${C_RESET}"
echo -e "  • App ID:         ${C_CYAN}com.wammed.cuddle${C_RESET}"
echo ""

bash "${SCRIPT_DIR}/install.sh" --de gnome "$@"

echo ""
echo -e "${C_GREEN}${C_BOLD}[GNOME Optimization Note]${C_RESET}"
echo -e "  • StartupWMClass is configured so GNOME Dash tracks running windows cleanly."
echo -e "  • Press ${C_BOLD}Super${C_RESET} to view Cuddle in Activities or search for \"Cuddle\"."
echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"
