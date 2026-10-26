#!/usr/bin/env bash
# ==============================================================================
# Cuddle (File Workbench) - Desktop Launcher & Application Installer
# Automates binary, icon, and .desktop launcher registration across Linux DEs.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Color output helpers (Midnight Cyber Dark theme aesthetic)
C_RESET=$'\033[0m'
C_BOLD=$'\033[1m'
C_TEAL=$'\033[38;2;45;212;191m'
C_CYAN=$'\033[38;2;56;189;248m'
C_GREEN=$'\033[38;2;52;211;153m'
C_AMBER=$'\033[38;2;251;191;36m'
C_RED=$'\033[38;2;248;113;113m'
C_MUTED=$'\033[38;2;148;163;184m'

log_info() {
    echo -e "${C_TEAL}${C_BOLD}[INFO]${C_RESET} $*"
}

log_step() {
    echo -e "${C_CYAN}${C_BOLD}[STEP]${C_RESET} $*"
}

log_success() {
    echo -e "${C_GREEN}${C_BOLD}[SUCCESS]${C_RESET} $*"
}

log_warn() {
    echo -e "${C_AMBER}${C_BOLD}[WARN]${C_RESET} $*"
}

log_error() {
    echo -e "${C_RED}${C_BOLD}[ERROR]${C_RESET} $*" >&2
}

# Default parameters
INSTALL_MODE="user"
TARGET_DE="auto"
USE_DEBUG="false"
BUILD_MODE="auto" # "auto" (build if missing), "always" (--build), "never" (--no-build)
CUSTOM_BIN=""

print_usage() {
    cat <<EOF
${C_BOLD}Cuddle Desktop Launcher & Application Installer${C_RESET}

${C_BOLD}USAGE:${C_RESET}
    bash scripts/install.sh [OPTIONS]

${C_BOLD}OPTIONS:${C_RESET}
    --user             Install for current user only (Default, no root required)
                       Binary: ~/.local/bin/cuddle
                       Desktop: ~/.local/share/applications/
                       Icons:   ~/.local/share/icons/hicolor/
    --system           Install system-wide (requires sudo / root)
                       Binary: /usr/local/bin/cuddle
                       Desktop: /usr/local/share/applications/
                       Icons:   /usr/local/share/icons/hicolor/
    --de <NAME>        Target Desktop Environment: auto (default), cosmic, gnome, kde, xfce, generic
    --build            Force compile production release binary before installing (pnpm tauri build)
    --no-build         Do not auto-build; fail if release binary is not found
    --debug            Use debug build (src-tauri/target/debug/fileworkbench, requires dev server)
    --bin <PATH>       Use custom binary at specified path
    -h, --help         Show this help message

${C_BOLD}EXAMPLES:${C_RESET}
    # Standard user installation (auto-builds release binary if missing)
    bash scripts/install.sh

    # CachyOS + COSMIC DE specific install
    bash scripts/install.sh --de cosmic

    # Force re-compile production release binary before installing
    bash scripts/install.sh --build

    # System-wide installation
    sudo bash scripts/install.sh --system
EOF
}

# Parse command line options
while [[ $# -gt 0 ]]; do
    case "$1" in
        --user)
            INSTALL_MODE="user"
            shift
            ;;
        --system)
            INSTALL_MODE="system"
            shift
            ;;
        --de)
            TARGET_DE="${2:-auto}"
            shift 2
            ;;
        --build)
            BUILD_MODE="always"
            shift
            ;;
        --no-build)
            BUILD_MODE="never"
            shift
            ;;
        --debug)
            USE_DEBUG="true"
            shift
            ;;
        --bin)
            CUSTOM_BIN="${2:-}"
            shift 2
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

build_release_binary() {
    log_step "Building production release binary (pnpm tauri build)..."
    log_info "Compiling and bundling self-contained assets (no localhost server needed)..."
    cd "${ROOT_DIR}"
    if command -v pnpm &>/dev/null; then
        pnpm tauri build
    elif command -v npm &>/dev/null; then
        npm run tauri build
    else
        log_error "Neither pnpm nor npm found. Please install Node.js and pnpm."
        exit 1
    fi
}

echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"
echo -e "${C_TEAL}${C_BOLD}   Cuddle (File Workbench) - Desktop Launcher Installer                       ${C_RESET}"
echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"

# 1. Locate Binary
log_step "Locating application executable..."
SOURCE_BIN=""

if [[ -n "${CUSTOM_BIN}" ]]; then
    if [[ -f "${CUSTOM_BIN}" && -x "${CUSTOM_BIN}" ]]; then
        SOURCE_BIN="${CUSTOM_BIN}"
    else
        log_error "Specified binary does not exist or is not executable: ${CUSTOM_BIN}"
        exit 1
    fi
elif [[ "${USE_DEBUG}" == "true" ]]; then
    log_warn "Installing DEBUG binary (--debug specified). Note: requires local Vite dev server (localhost:1420)."
    if [[ -f "${ROOT_DIR}/src-tauri/target/debug/fileworkbench" ]]; then
        SOURCE_BIN="${ROOT_DIR}/src-tauri/target/debug/fileworkbench"
    else
        log_error "Debug binary not found at src-tauri/target/debug/fileworkbench. Build it first."
        exit 1
    fi
else
    # Release build resolution
    RELEASE_BIN=""
    if [[ -f "${ROOT_DIR}/src-tauri/target/release/fileworkbench" ]]; then
        RELEASE_BIN="${ROOT_DIR}/src-tauri/target/release/fileworkbench"
    elif [[ -f "${ROOT_DIR}/src-tauri/target/release/cuddle" ]]; then
        RELEASE_BIN="${ROOT_DIR}/src-tauri/target/release/cuddle"
    fi

    if [[ "${BUILD_MODE}" == "always" ]] || { [[ -z "${RELEASE_BIN}" ]] && [[ "${BUILD_MODE}" == "auto" ]]; }; then
        if [[ -z "${RELEASE_BIN}" ]]; then
            log_info "Release binary not found. Triggering automatic production build..."
        else
            log_info "Forced rebuild requested (--build). Triggering production build..."
        fi
        build_release_binary
        if [[ -f "${ROOT_DIR}/src-tauri/target/release/fileworkbench" ]]; then
            RELEASE_BIN="${ROOT_DIR}/src-tauri/target/release/fileworkbench"
        elif [[ -f "${ROOT_DIR}/src-tauri/target/release/cuddle" ]]; then
            RELEASE_BIN="${ROOT_DIR}/src-tauri/target/release/cuddle"
        fi
    fi

    if [[ -n "${RELEASE_BIN}" ]]; then
        SOURCE_BIN="${RELEASE_BIN}"
    else
        log_error "No release binary found and auto-build was disabled (--no-build)."
        log_error "Please build Cuddle release binary or run installer without --no-build:"
        echo -e "    ${C_CYAN}pnpm run tauri build${C_RESET}"
        exit 1
    fi
fi

log_info "Using binary: ${SOURCE_BIN}"

# 2. Determine Installation Targets
if [[ "${INSTALL_MODE}" == "system" ]]; then
    if [[ $EUID -ne 0 ]]; then
        log_error "System-wide installation requires root privileges. Please run with sudo."
        exit 1
    fi
    BIN_DIR="/usr/local/bin"
    APP_DIR="/usr/local/share/applications"
    ICON_BASE="/usr/local/share/icons/hicolor"
    PIXMAPS_DIR="/usr/local/share/pixmaps"
else
    BIN_DIR="${HOME}/.local/bin"
    APP_DIR="${HOME}/.local/share/applications"
    ICON_BASE="${HOME}/.local/share/icons/hicolor"
    PIXMAPS_DIR="${HOME}/.local/share/pixmaps"
fi

mkdir -p "${BIN_DIR}" "${APP_DIR}" "${PIXMAPS_DIR}"

# 3. Detect Desktop Environment
CURRENT_DESKTOP="${XDG_CURRENT_DESKTOP:-unknown}"
SESSION_TYPE="${XDG_SESSION_TYPE:-unknown}"

if [[ "${TARGET_DE}" == "auto" ]]; then
    case "${CURRENT_DESKTOP,,}" in
        *cosmic*)
            DETECTED_DE="cosmic"
            ;;
        *gnome*|*ubuntu*)
            DETECTED_DE="gnome"
            ;;
        *kde*|*plasma*)
            DETECTED_DE="kde"
            ;;
        *xfce*)
            DETECTED_DE="xfce"
            ;;
        *)
            DETECTED_DE="generic"
            ;;
    esac
else
    DETECTED_DE="${TARGET_DE,,}"
fi

log_info "Target Desktop Environment: ${C_BOLD}${DETECTED_DE}${C_RESET} (Session: ${SESSION_TYPE}, Desktop: ${CURRENT_DESKTOP})"

# 4. Copy Executable Binary
log_step "Installing binary to ${BIN_DIR}/cuddle..."
install -m 755 "${SOURCE_BIN}" "${BIN_DIR}/cuddle"
# Provide fileworkbench alias symlink for compatibility
ln -sf "cuddle" "${BIN_DIR}/fileworkbench"

# 5. Install Icons in hicolor directory tree
log_step "Installing application icons to ${ICON_BASE}..."
SVG_SRC="${ROOT_DIR}/images/Cuddle-Icon.svg"
PNG_512="${ROOT_DIR}/src-tauri/icons/icon.png"
PNG_128="${ROOT_DIR}/src-tauri/icons/128x128.png"
PNG_64="${ROOT_DIR}/src-tauri/icons/64x64.png"
PNG_32="${ROOT_DIR}/src-tauri/icons/32x32.png"

# Scalable SVG
if [[ -f "${SVG_SRC}" ]]; then
    mkdir -p "${ICON_BASE}/scalable/apps"
    install -m 644 "${SVG_SRC}" "${ICON_BASE}/scalable/apps/com.wammed.cuddle.svg"
    install -m 644 "${SVG_SRC}" "${ICON_BASE}/scalable/apps/cuddle.svg"
    install -m 644 "${SVG_SRC}" "${PIXMAPS_DIR}/com.wammed.cuddle.svg"
    install -m 644 "${SVG_SRC}" "${PIXMAPS_DIR}/cuddle.svg"
fi

# 512x512 PNG
if [[ -f "${PNG_512}" ]]; then
    mkdir -p "${ICON_BASE}/512x512/apps"
    install -m 644 "${PNG_512}" "${ICON_BASE}/512x512/apps/com.wammed.cuddle.png"
    install -m 644 "${PNG_512}" "${ICON_BASE}/512x512/apps/cuddle.png"
    install -m 644 "${PNG_512}" "${PIXMAPS_DIR}/com.wammed.cuddle.png"
    install -m 644 "${PNG_512}" "${PIXMAPS_DIR}/cuddle.png"
fi

# 128x128 PNG
if [[ -f "${PNG_128}" ]]; then
    mkdir -p "${ICON_BASE}/128x128/apps"
    install -m 644 "${PNG_128}" "${ICON_BASE}/128x128/apps/com.wammed.cuddle.png"
    install -m 644 "${PNG_128}" "${ICON_BASE}/128x128/apps/cuddle.png"
fi

# 64x64 PNG
if [[ -f "${PNG_64}" ]]; then
    mkdir -p "${ICON_BASE}/64x64/apps"
    install -m 644 "${PNG_64}" "${ICON_BASE}/64x64/apps/com.wammed.cuddle.png"
    install -m 644 "${PNG_64}" "${ICON_BASE}/64x64/apps/cuddle.png"
fi

# 32x32 PNG
if [[ -f "${PNG_32}" ]]; then
    mkdir -p "${ICON_BASE}/32x32/apps"
    install -m 644 "${PNG_32}" "${ICON_BASE}/32x32/apps/com.wammed.cuddle.png"
    install -m 644 "${PNG_32}" "${ICON_BASE}/32x32/apps/cuddle.png"
fi

# 6. Generate and Install .desktop File
log_step "Generating .desktop launcher specification..."
DESKTOP_FILE="${APP_DIR}/com.wammed.cuddle.desktop"
SYMLINK_FILE="${APP_DIR}/cuddle.desktop"

# Base exec command: in COSMIC / Wayland, ensure WebKitGTK and Wayland backend cooperate
EXEC_COMMAND="${BIN_DIR}/cuddle %U"
EXTRA_WM_CLASS="StartupWMClass=com.wammed.cuddle"

case "${DETECTED_DE}" in
    cosmic)
        # COSMIC DE (cosmic-comp) strictly matches Wayland app_id and .desktop id
        EXTRA_WM_CLASS="StartupWMClass=com.wammed.cuddle
X-CosmicAppId=com.wammed.cuddle"
        ;;
    gnome)
        EXTRA_WM_CLASS="StartupWMClass=com.wammed.cuddle"
        ;;
    kde)
        EXTRA_WM_CLASS="StartupWMClass=com.wammed.cuddle
X-KDE-SubstituteUID=false"
        ;;
    *)
        EXTRA_WM_CLASS="StartupWMClass=com.wammed.cuddle"
        ;;
esac

cat > "${DESKTOP_FILE}" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Cuddle
GenericName=Safe Linux File Workbench
GenericName[ja]=安全志向 Linux ファイルワークベンチ
Comment=A safe Linux file workbench that hugs your originals tight
Comment[ja]=原本を抱きしめて守る安全志向Linuxファイルワークベンチ
Exec=${EXEC_COMMAND}
Icon=com.wammed.cuddle
Terminal=false
Categories=System;FileManager;FileTools;Core;
MimeType=inode/directory;
StartupNotify=true
${EXTRA_WM_CLASS}
Keywords=file;manager;workbench;batch;safe;rename;diff;cuddle;chmod;timestamp;backup;
Keywords[ja]=ファイル;管理;リネーム;バックアップ;安全;差分;原本;属性;タイムスタンプ;ワークベンチ;
Actions=NewWindow;

[Desktop Action NewWindow]
Name=Open New Window
Name[ja]=新しいウィンドウを開く
Exec=${BIN_DIR}/cuddle
EOF

chmod 644 "${DESKTOP_FILE}"
ln -sf "com.wammed.cuddle.desktop" "${SYMLINK_FILE}"

log_info "Installed: ${DESKTOP_FILE}"
log_info "Linked:    ${SYMLINK_FILE}"

# 7. Update Desktop Database & Icon Caches
log_step "Updating desktop database and icon caches for ${DETECTED_DE}..."

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${APP_DIR}" 2>/dev/null || true
fi

if command -v gtk4-update-icon-cache >/dev/null 2>&1; then
    gtk4-update-icon-cache -f -t "${ICON_BASE}" 2>/dev/null || true
elif command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${ICON_BASE}" 2>/dev/null || true
fi

# DE Specific Cache Invalidation
case "${DETECTED_DE}" in
    cosmic)
        # COSMIC inotifies ~/.local/share/applications, but update-desktop-database ensures index is fresh
        log_info "COSMIC DE: Triggered desktop and icon index refresh."
        ;;
    kde)
        if command -v kbuildsycoca6 >/dev/null 2>&1; then
            kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
        elif command -v kbuildsycoca5 >/dev/null 2>&1; then
            kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
        fi
        ;;
    *)
        if command -v xdg-desktop-menu >/dev/null 2>&1; then
            xdg-desktop-menu forceupdate 2>/dev/null || true
        fi
        ;;
esac

# 8. Configure Periodic Fulltext Indexing systemd User Timer (Section 7.3)
if [[ "${INSTALL_MODE}" == "user" ]] && command -v systemctl >/dev/null 2>&1; then
    log_step "Configuring periodic fulltext indexing systemd user timer (Section 7.3)..."
    USER_SYSTEMD_DIR="${HOME}/.config/systemd/user"
    mkdir -p "${USER_SYSTEMD_DIR}"
    install -m 644 "${ROOT_DIR}/systemd/cuddle-index.service" "${USER_SYSTEMD_DIR}/cuddle-index.service"
    install -m 644 "${ROOT_DIR}/systemd/cuddle-index.timer" "${USER_SYSTEMD_DIR}/cuddle-index.timer"
    systemctl --user daemon-reload 2>/dev/null || true
    if systemctl --user enable --now cuddle-index.timer 2>/dev/null; then
        log_info "Enabled systemd user timer: cuddle-index.timer (Daily 02:30 AM, lock-guarded)"
    fi
fi

# 9. Check PATH configuration
echo ""
log_success "Cuddle has been successfully installed to your desktop environment!"
echo -e "${C_MUTED}------------------------------------------------------------------------------${C_RESET}"
echo -e "  • ${C_BOLD}Executable:${C_RESET}       ${BIN_DIR}/cuddle"
echo -e "  • ${C_BOLD}Desktop Entry:${C_RESET}    ${DESKTOP_FILE}"
echo -e "  • ${C_BOLD}Icon Asset:${C_RESET}       ${ICON_BASE}/scalable/apps/com.wammed.cuddle.svg"
echo -e "  • ${C_BOLD}Environment:${C_RESET}      ${DETECTED_DE} (${SESSION_TYPE})"
echo -e "${C_MUTED}------------------------------------------------------------------------------${C_RESET}"

if [[ ":${PATH}:" != *":${BIN_DIR}:"* ]]; then
    log_warn "${BIN_DIR} is currently NOT in your \$PATH."
    echo -e "  To launch 'cuddle' directly from terminal, add the following to your ~/.bashrc or ~/.zshrc:"
    echo -e "    ${C_CYAN}export PATH=\"\$HOME/.local/bin:\$PATH\"${C_RESET}"
fi

echo -e "  You can now search for ${C_BOLD}\"Cuddle\"${C_RESET} in your Application Launcher or run: ${C_TEAL}cuddle${C_RESET}"
echo -e "${C_TEAL}${C_BOLD}==============================================================================${C_RESET}"
