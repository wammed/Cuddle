# Cuddle - Build & Environment Setup Guide

<p align="center">
  <strong>English</strong> | <a href="BUILD.ja.md">日本語</a>
</p>

This document details the required system packages, toolchains, and build steps to compile and run **Cuddle** from source on Linux.

---

## 1. System Requirements & Verified Environments

* **Target OS:** Linux x86_64
* **Verified Environments:**
  * **CachyOS / Arch Linux** (Wayland, COSMIC DE / GNOME / KDE) - *Primary Reference Desktop*
  * **Ubuntu 24.04 LTS / 22.04 LTS**
  * **Fedora 40 / 39**
  * **Debian 12 (Bookworm)**
* **Kernel Requirements:**
  * Linux Kernel **5.13 or later** with Landlock LSM enabled (`CONFIG_SECURITY_LANDLOCK=y`).
  * To check Landlock support on your running kernel:
    ```bash
    # Check current boot LSM list
    cat /sys/kernel/security/lsm
    # Should include: landlock
    ```

---

## 2. Toolchain Prerequisites

Ensure the following tools are installed:

1. **Rust Toolchain (1.77 or later):**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustc --version
   cargo --version
   ```
2. **Node.js (v20 LTS or later) & pnpm:**
   ```bash
   # Install Node.js via your distribution package manager or nvm/fnm
   node --version

   # Install pnpm (v9 or later)
   corepack enable
   # or
   npm install -g pnpm
   pnpm --version
   ```

---

## 3. Native System Dependencies (Tauri 2 Linux)

Tauri 2 requires WebKitGTK and essential native development headers:

### Arch Linux / CachyOS
```bash
sudo pacman -S --needed \
  base-devel \
  webkit2gtk-4.1 \
  openssl \
  curl \
  wget \
  file \
  squashfs-tools \
  libappindicator-gtk3 \
  librsvg
```

### Ubuntu / Debian (22.04 / 24.04)
```bash
sudo apt update
sudo apt install -y \
  build-essential \
  pkg-config \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf \
  libssl-dev \
  libasound2-dev
```

### Fedora
```bash
sudo dnf install -y \
  @development-tools \
  pkg-config \
  webkit2gtk4.1-devel \
  openssl-devel \
  libappindicator-gtk3-devel \
  librsvg2-devel
```

---

## 4. Optional External CLI Tools for Pipelines

Cuddle includes pure-Rust builtin implementations for data cleansing, log masking, JSON minification, TSV/CSV conversions, and SRT time shifting.

For rich image, audio, video, and PDF pipeline presets, install the following external CLI utilities:

```bash
# Arch / CachyOS
sudo pacman -S --needed imagemagick ffmpeg qpdf poppler perl-image-exiftool plocate

# Ubuntu / Debian
sudo apt install -y imagemagick ffmpeg qpdf poppler-utils libimage-exiftool-perl plocate

# Fedora
sudo dnf install -y ImageMagick ffmpeg qpdf poppler-utils perl-Image-ExifTool plocate
```

*Note: Cuddle probes tools automatically on startup. Any missing external tool will be safely marked as "Not Installed", and pure-Rust presets remain fully operational without them.*

---

## 5. Development & Running Locally

1. **Clone the repository:**
   ```bash
   git clone https://github.com/wammed/Cuddle.git
   cd Cuddle
   ```

2. **Install frontend packages:**
   ```bash
   pnpm install
   ```

3. **Launch in development mode:**
   ```bash
   pnpm run tauri dev
   ```
   *This starts the Vite local dev server and compiles the Tauri 2 Rust desktop wrapper.*

---

## 6. Running Test Suites

### Backend Rust Unit & Integration Tests (174 tests)
```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

### Frontend Vitest & TypeScript Tests (67 tests)
```bash
pnpm test -- --run
```

### TypeScript Type-Checking & Vite Production Build Verification
```bash
pnpm run build
```

---

## 7. Compiling Production Binaries

To produce an optimized native release binary:

```bash
pnpm run tauri build
```

The resulting standalone executable will be generated at:
```
src-tauri/target/release/fileworkbench
```

You can run the binary directly:
```bash
./src-tauri/target/release/fileworkbench
```

---

## 8. Desktop Launcher & System Integration (.desktop & Icons)

Cuddle provides automated installation scripts that deploy the binary, register standard FreeDesktop `.desktop` launchers, install icons across all standard resolutions (32/64/128/512px + scalable SVG), and trigger desktop database/icon cache reloads.

### 8.1 CachyOS + COSMIC DE (Wayland) Dedicated Preset (Recommended)

Dedicated preset tuned for CachyOS and the System76 COSMIC Desktop Environment (`cosmic-comp`, `cosmic-app-library`, `cosmic-launcher`). Guarantees Wayland `app_id` (`com.wammed.cuddle`) matching, dock pinning, and application search indexing:

```bash
# User-level installation (Recommended: ~/.local, no root required)
./scripts/install-cosmic.sh

# System-wide installation (/usr/local, requires sudo)
sudo ./scripts/install-cosmic.sh --system
```

* **Verification:** Press `Super` to open COSMIC App Library, search for `Cuddle`, or right-click the icon to select "Pin to Dock".

### 8.2 Presets for Major Desktop Environments

Tailored scripts are provided for various desktop environments:

* **GNOME (Wayland / X11):**
  ```bash
  ./scripts/install-gnome.sh
  ```
  Configures `StartupWMClass=com.wammed.cuddle` for window grouping and updates GNOME Shell application caches.

* **KDE Plasma 5 / 6 (Wayland / X11):**
  ```bash
  ./scripts/install-kde.sh
  ```
  Runs `kbuildsycoca6` / `kbuildsycoca5` to rebuild system configuration and Kickoff / KRunner menu caches.

* **Universal Auto-Detect Installer (XFCE, Cinnamon, MATE, etc.):**
  ```bash
  ./scripts/install.sh
  ```
  Automatically detects current `$XDG_CURRENT_DESKTOP` and `$XDG_SESSION_TYPE`, applying the optimal configuration.

### 8.3 Uninstallation

To cleanly remove all registered launchers, multi-resolution icons, and binaries:

```bash
# Remove from user directory (~/.local)
./scripts/uninstall.sh

# Remove from system directory (/usr/local)
sudo ./scripts/uninstall.sh --system
```

---

## 9. Troubleshooting & FAQ

### Landlock ABI Warning
* **Symptom:** UI displays "Landlock LSM is not supported by current kernel".
* **Solution:** Ensure your kernel was compiled with Landlock support (`CONFIG_SECURITY_LANDLOCK=y`) and that `lsm=...,landlock` is in your kernel boot parameters (`/etc/default/grub` or systemd-boot). Cuddle will continue to protect files via isolated staging even if Landlock is unavailable, but sandbox process confinement will be degraded.

### Non-UTF-8 Filesystem Encoding
* Cuddle natively preserves raw filesystem byte strings via Dual-Key architecture (`raw_base64` + `display_name`). Non-UTF-8 filenames are safely escaped without application crashes.

### Typography & Fonts
* Cuddle does not bundle heavyweight font files. If icons appear broken, ensure a standard Nerd Font or system symbol font is installed:
  * Arch: `sudo pacman -S ttf-nerd-fonts-symbols noto-fonts-cjk`
  * Ubuntu: `sudo apt install fonts-noto-cjk fonts-font-awesome`

