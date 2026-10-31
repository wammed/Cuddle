<p align="center">
  <img src="images/Cuddle-Banner.jpeg" alt="Cuddle Banner" width="100%" />
</p>

<p align="center">
  <strong>English</strong> | <a href="README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Linux%20(Wayland%20%2F%20X11)-teal.svg" alt="Platform" />
  <img src="https://img.shields.io/badge/Security-Linux%20Landlock%20LSM-emerald.svg" alt="Security" />
  <img src="https://img.shields.io/badge/Engine-Tauri%202%20%2B%20Rust-orange.svg" alt="Engine" />
  <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License" />
</p>

---

# Cuddle - Original-Protected File Workbench

**Cuddle** is a high-reliability, safety-first file operations workbench and search environment engineered specifically for Linux desktops (Wayland, COSMIC DE, GNOME, KDE).

Traditional file managers edit or process your original files directly in-place. If an operation fails, batch rename collides, or an external script runs amok, original files can be corrupted or permanently lost. **Cuddle fundamentally redefines this paradigm with absolute original protection.**

---

## Key Core Principles

1. **Absolute Original File Protection:**  
   Original files are **never modified in-place**. All transformations, renames, batch operations, and external script executions are planned and applied strictly inside an isolated staging workspace (`workspace-temp/`).
2. **Clear Separation of Staging and In-Place Reflection:**  
   Changes are reflected to original locations only when you explicitly trigger "Reflect to Original" with two-stage verification and TOCTOU (Time-of-Check to Time-of-Use) conflict detection.
3. **Linux Landlock LSM Kernel Isolation:**  
   External tools (ImageMagick, FFmpeg, cwebp, pdftotext, etc.) and user-defined pipelines execute under kernel-enforced sandboxing. Write access outside `workspace-temp/` and into `$HOME` is physically blocked at the Linux kernel level.
4. **Instant Staging via Reflink (CoW):**  
   Prioritizes staging on the same filesystem volume, leveraging Linux `ioctl(FICLONE)` copy-on-write to create near-instantaneous, zero-space duplicate snapshots.
5. **Deterministic Atomicity & Rollback (RollbackAll):**  
   Batch renames and permission/timestamp modifications follow strict Two-Phase operations. If even a single item fails, the entire batch is deterministically rolled back to its pre-operation state.
6. **Persistent SQLite WAL Operation History & Undo:**  
   Every staging, unstage, rename, touch, and pipeline action is safely recorded in an ACID SQLite WAL journal, supporting full batch UNDO.
7. **Zero Font Bundling & Pure Cyber Aesthetics:**  
   Adheres to clean system font fallbacks (Inter, Noto Sans CJK JP, JetBrains Mono, Symbols Nerd Font) without bloated asset packaging. Implements 30 switchable Midnight Teal and Cyber Dark palettes with zero white-flash backgrounds.
8. **100% Full Bilingual Parity:**  
   English and Japanese locales are dynamically switchable without requiring an application restart.

---

## Feature Matrix

| Feature | Description |
| :--- | :--- |
| **6-Region Adaptive Layout** | Search Results, Directory Tree, File Browser, Staged Workspace, Inspector Sidebar, and Bottom Tabs. |
| **Virtual Scrolling** | Powered by `@tanstack/react-virtual`, rendering up to 10,000 files smoothly at 60 fps with 3 density modes (Compact, Normal, Spacious). |
| **Bulk Rename & Normalization** | Regex substitution, Unicode normalization (NFC/NFD/NFKC/NFKD), full-width/half-width conversions, sequential numbers, date stamps, and live Dry-Run collision preview. |
| **vidir Editor Integration** | Edit batch filenames as tab-delimited text inside the app or launch external editors (`$EDITOR`, VS Code, nano). Deleted lines are safely preserved as unchanged. |
| **30 Built-in Pipeline Presets** | Fast image optimization (WebP, OGP, Favicon, Retina), audio/video mastering, PDF binarize & merge, text/log secret masking, and clean source code packaging. |
| **Custom Pipeline Runner** | Design Section 8.3 mandatory safety flow with 2-stage execution confirmation, pre-Dry-Run verification, and Landlock LSM confinement. |
| **Batch Permissions (chmod)** | Octal (e.g., 644, 755) and symbolic (`u+x`, `go-w`) permission editing with interactive 3x3 matrix and special bit warnings. |
| **Batch Timestamps (touch)** | Touch current time, absolute datetime, relative offset (+/- seconds/hours/days), and camera EXIF shooting datetime synchronization. |
| **Workspace tar Archive Export** | Export working files into uncompressed `.tar` or gzip-compressed `.tar.gz` archives with scope filtering (Modified Only, All Staged, Selected Only). |
| **Japanese Full-Text Search** | Tantivy 0.26 + Lindera morphological analysis (`ipadic`) with real-time text snippet extraction. |
| **plocate Integration** | Millisecond-level filename indexing and search with automatic fallback to filesystem tree scanning. |
| **Rich File Preview** | Syntax highlighting for 10+ programming languages, formatted Markdown rendering, and sandboxed HTML preview. |
| **Git Status Dashboard** | Embedded repository health, upstream divergence (ahead/behind), branch display, and categorized commit staging overview. |

---

## Quick Start & Building

To build and run Cuddle from source:

```bash
# Clone the repository
git clone https://github.com/wammed/Cuddle.git
cd Cuddle

# Install frontend dependencies
pnpm install

# Run in development mode (launches Tauri 2 desktop shell)
pnpm run tauri dev

# Build production binary & install desktop launcher (with icons and .desktop registration)
pnpm run tauri build
./scripts/install-cosmic.sh   # CachyOS + COSMIC DE (Recommended)
# or ./scripts/install.sh     # GNOME / KDE / XFCE auto-detection
```

For complete compilation instructions, Linux system prerequisites, external CLI recommendations, and desktop environment installation options, see **[BUILD.md](BUILD.md)** (or **[BUILD.ja.md](BUILD.ja.md)**).

---

## Documentation

* **[BUILD.md](BUILD.md)** / **[BUILD.ja.md](BUILD.ja.md)**: Detailed compilation guides, prerequisites, and system configuration.
* **[FileWorkbench-Design-ver0.9.en.md](FileWorkbench-Design-ver0.9.en.md)** / **[FileWorkbench-Design-ver0.9.ja.md](FileWorkbench-Design-ver0.9.ja.md)**: Comprehensive architectural design specifications.
* **[FileWorkbench-Handover.en.md](FileWorkbench-Handover.en.md)** / **[FileWorkbench-Handover.ja.md](FileWorkbench-Handover.ja.md)**: Engineering handover and implementation progress logs.
* **[IP_COMPLIANCE.md](IP_COMPLIANCE.md)**: Trademark, brand icon, and asset compliance verification.
* **[THIRD-PARTY-LICENSES.md](THIRD-PARTY-LICENSES.md)**: Third-party dependency licenses and attribution audit record.

---

## License

Cuddle is licensed under the [MIT License](LICENSE).
Copyright (c) 2026 wammed and Cuddle Contributors.

Third-party dependencies and assets remain subject to their respective licenses. See [THIRD-PARTY-LICENSES.md](THIRD-PARTY-LICENSES.md) for details.

Cuddle is distributed as source code. Precompiled binary packages are not distributed as part of this project.
