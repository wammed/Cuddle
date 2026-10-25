#!/usr/bin/env bash
# ==============================================================================
# Cuddle Installer - CachyOS + COSMIC DE (Wayland) Dedicated Preset
# Optimized for CachyOS Linux and System76 COSMIC Desktop Environment (cosmic-comp).
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

C_RESET=$'\033[0m'
C_BOLD=$'\033[1m'
C_TEAL=$'\033[38;2;45;212;191m'
C_CYAN=$'\033[38;2;56;189;248m'
C_GREEN=$'\033[38;2;52;211;153m'

echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"
echo -e "${C_TEAL}${C_BOLD}   Cuddle - CachyOS + COSMIC Desktop Environment (Wayland) Installer         ${C_RESET}"
echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"
echo -e "  • Target OS:      ${C_CYAN}CachyOS / Arch Linux (x86_64-v3/v4 / Wayland)${C_RESET}"
echo -e "  • Target DE:      ${C_CYAN}COSMIC DE (cosmic-comp, cosmic-app-library, cosmic-launcher)${C_RESET}"
echo -e "  • App ID:         ${C_CYAN}com.wammed.cuddle${C_RESET}"
echo ""

# Execute main installer with explicit cosmic profile
bash "${SCRIPT_DIR}/install.sh" --de cosmic "$@"

echo ""
echo -e "${C_GREEN}${C_BOLD}[COSMIC DE Optimization Note]${C_RESET}"
echo -e "  • The application entry is mapped to App ID ${C_BOLD}com.wammed.cuddle${C_RESET}."
echo -e "  • Press ${C_BOLD}Super${C_RESET} to open COSMIC App Library / Launcher and type ${C_TEAL}\"Cuddle\"${C_RESET}."
echo -e "  • You can right-click the icon in App Library to \"Pin to Dock\" seamlessly."
echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"
