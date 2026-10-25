#!/usr/bin/env bash
# ==============================================================================
# Cuddle Installer - KDE Plasma 5/6 (Wayland / X11) Dedicated Preset
# Optimized for KDE Plasma (KRunner, Kickoff, KSycoca cache refresh).
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

C_RESET="\033[0m"
C_BOLD="\033[1m"
C_TEAL="\033[38;2;45;212;191m"
C_CYAN="\033[38;2;56;189;248m"
C_GREEN="\033[38;2;52;211;153m"

echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"
echo -e "${C_TEAL}${C_BOLD}   Cuddle - KDE Plasma (5/6) Desktop Environment Installer                    ${C_RESET}"
echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"
echo -e "  • Target DE:      ${C_CYAN}KDE Plasma (KWin / Wayland / X11)${C_RESET}"
echo -e "  • App ID:         ${C_CYAN}com.wammed.cuddle${C_RESET}"
echo ""

bash "${SCRIPT_DIR}/install.sh" --de kde "$@"

echo ""
echo -e "${C_GREEN}${C_BOLD}[KDE Plasma Optimization Note]${C_RESET}"
echo -e "  • Executed kbuildsycoca6/kbuildsycoca5 to refresh Kickoff and KRunner index."
echo -e "  • Press ${C_BOLD}Alt+Space${C_RESET} or ${C_BOLD}Meta${C_RESET} to search for \"Cuddle\" immediately."
echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"
