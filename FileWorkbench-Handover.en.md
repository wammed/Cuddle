# FileWorkbench (Cuddle) Development Handover Document

<p align="center">
  <strong>English</strong> | <a href="FileWorkbench-Handover.ja.md">日本語</a>
</p>

**Document Date:** 2026-10-24  
**Status:** Phase 2.1 Completed, Ready for Phase 2.2  
**Target Audience:** Development, Architecture, and QA Teams; Next Working Session Maintainers  

---

## 1. Project Overview

* **Project Name:** Cuddle
* **Description:** File Workbench
* **Official Icon:** `Cuddle/images/Cuddle-Icon.svg`
* **README Banner:** `Cuddle/images/Cuddle-Banner.jpeg`
* **Purpose:** A high-reliability original-protection file management, working environment, and search workbench for Linux with its own unique visual branding.
* **Core Principles & Mandatory Policies:**
  1. **Strict Protection of Original Files:** Originals are never modified in-place; all transformations and edits are performed strictly on independent working copies (`workspace-temp/`).
  2. **Complete Separation of Internal Processing and Origin Reflection:** Pipelines and bulk operations do not overwrite originals; changes are reflected back exclusively via independent user actions after deliverables are finalized.
  3. **Kernel-Level Landlock LSM Sandbox:** When invoking external binaries (ImageMagick, ffmpeg, cwebp, etc.), Landlock rulesets are applied at process launch to physically block writes outside the workspace.
  4. **Deterministic Rollback (RollbackAll):** If any single item fails during bulk operations, the entire batch fails and working copies are restored to their exact initial state.
  5. **100% Bilingual Documentation (Mandatory):** All documentation—including specifications, architecture designs, READMEs, task tracking, and handover documents—must be authored and maintained in both English and Japanese with identical content.
  6. **100% Bilingual In-App UI with Dynamic Switching (Mandatory):** All menus, warnings, messages, dialogs, and text elements must be fully localized in both English and Japanese, switchable dynamically from within app settings without requiring an application restart.
* **Target Environment:** CachyOS + COSMIC DE (Wayland, HiDPI) - Initial release official verification target.

---

## 2. Key Documentation Index

| Japanese Document | English Document | Description |
| :--- | :--- | :--- |
| [`FileWorkbench-Design-ver0.9.ja.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Design-ver0.9.ja.md) | [`FileWorkbench-Design-ver0.9.en.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Design-ver0.9.en.md) | Integrated Basic Design Document (Definitive Edition) |
| [`FileWorkbench-Handover.ja.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Handover.ja.md) | [`FileWorkbench-Handover.en.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Handover.en.md) | Development Handover Document (this document) |
| [`roadmap.ja.md`](file:///home/susie/.gemini/antigravity-ide/brain/45a8eaa8-509d-4ed8-9e6c-255c45bac425/roadmap.ja.md) | [`roadmap.en.md`](file:///home/susie/.gemini/antigravity-ide/brain/45a8eaa8-509d-4ed8-9e6c-255c45bac425/roadmap.en.md) | Detailed Implementation Roadmap (9 Phases / 28 Subphases) |
| [`IP_COMPLIANCE.ja.md`](file:///home/susie/git/wammed/Cuddle/IP_COMPLIANCE.ja.md) | [`IP_COMPLIANCE.md`](file:///home/susie/git/wammed/Cuddle/IP_COMPLIANCE.md) | Icon Design & IP Compliance Due Diligence Record |

---

## 3. Finalized Technology Stack & Architecture

| Domain | Technology | Finalized Specifications & Policies |
| :--- | :--- | :--- |
| **UI Framework** | **Tauri 2 + React + TypeScript** | Native Rust backend permits direct control of OS system calls (Landlock, inode, POSIX permissions) |
| **Internationalization (i18n)** | **`i18next` + `react-i18next`** | Full English/Japanese bilingual support. Instant switching in settings without restart. Backend returns structured error codes and params |
| **UI Rendering** | **`@tanstack/react-virtual`** | 60fps virtualized scrolling for up to 10,000 items in browser, workspace, and bulk review lists |
| **Metadata & History** | **SQLite (WAL mode)** | Runtime standard `PRAGMA synchronous = NORMAL;`; explicit `PRAGMA wal_checkpoint(TRUNCATE);` at origin reflection and batch commits |
| **Full-text Search** | **Tantivy + `lindera-tantivy`** | Japanese morphological analysis (IPADIC, Normal) combined with n-gram auxiliary scoring |
| **Filename Search** | **plocate / Direct Traversal** | Prioritizes plocate; automatic fallback to `walkdir` direct scan when unavailable |
| **Process Isolation** | **Linux Landlock (LSM)** | Unprivileged access restriction. Per-preset `allowed_read_paths` declarations; isolated `TMPDIR` (`workspace-temp/proc-tmp-<pid>/`) injected |
| **Text Extraction** | **Phased & Isolated Worker** | Text/MD/HTML extracted directly. PDF/Office handled in 256 MiB quota isolated worker processes to prevent main process crashes/OOM |
| **Working Copy Placement** | **Same-Volume Priority** | Volume-local workspace (e.g. `<origin-volume>/.fileworkbench-temp/`) prioritized to maximize atomic `rename(2)` and CoW Reflink. Mandatory 1.5x capacity pre-check |
| **Non-UTF-8 Filenames** | **Dual-Key Architecture** | Rust core retains `PathBuf` (`[u8]`). IPC/JSON boundaries utilize Base64 (`raw_base64`) + display string (`display_name`) with immutable `item_id` |
| **Bulk Operations** | **`RollbackAll` & Journaling** | Transactional `BatchPlan` / `BatchStatus` state machines. Automated passing of all B-01 through B-15 fault injection test cases |
| **UI/UX Styling** | **Vanilla CSS + Glassmorphism** | "Simple yet Ultra-Rich" core concept with dark velvet palette, subtle 16px frosted-glass blur, 1px hairline borders, and pulsing security status indicators |
| **Typography & Fonts** | **Pre-required (Zero Bundled Fonts)** | **Zero font bundling for Nerd Font**. UI uses `Inter` + Noto Sans CJK JP; Code uses `JetBrains Mono`; Glyphs dynamically fall back to system `Symbols Nerd Font` / `JetBrainsMono Nerd Font`. Core UI icons 100% guaranteed via built-in `lucide-react` SVG vectors |

---

## 4. Current Development Status & Progress

* [x] **Basic Design Phase:** Resolved all technical challenges and integrated bilingual policies into `ver 0.9` (both English & Japanese versions).
* [x] **Roadmap Definition Phase:** 9 phases and 28 subphases defined with test gates and user confirmation gates; officially approved (both English & Japanese versions).
* [x] **Phase 1.1 (Project Foundations & Infrastructure): Completed**
  * Tauri 2 + React + TypeScript scaffolding created
  * Rust Edition 2021 configured with core crates (`tokio`, `thiserror`, `anyhow`, `serde`, `serde_json`)
  * Frontend packages configured (`i18next`, `@tanstack/react-virtual`, `lucide-react`)
  * Full bilingual dictionaries (`en` / `ja`) implemented, dynamic language toggle verified, missing key test (Vitest 100% pass)
  * Rust unit tests passing (Cargo test 100% pass)
* [x] **Phase 1.2 (Landlock & Security PoC Verification): Completed**
  * Probed and confirmed host CachyOS kernel Landlock ABI version (detected: ABI V5)
  * Implemented `LandlockSandbox` builder with `pre_exec` syscall isolation
  * Injected isolated temporary directory (`TMPDIR`) per process
  * Automated testing verifying write prevention outside whitelist (`EACCES` enforced at kernel level)
  * Verified external CLI tool (`magick`) cannot tamper with protected original directory
  * Exposed `get_landlock_status` Tauri command with UI security status badge
* [x] **Phase 1.3 (IPC Type Definitions & Non-UTF-8 Dual-Key Infrastructure): Completed**
  * Implemented `FileItem` dual-key data model (`item_id` UUID + `raw_base64` raw bytes + `display_name` safe UTF-8)
  * Linux `OsStrExt` byte-level lossless roundtrip path conversion utilities
  * Automated TypeScript bindings generation using `ts-rs` (`src/types/`)
  * Unit tests validating 100% byte fidelity on non-UTF-8 paths (raw bytes `\xFF\xFE`)
  * Tauri IPC command `get_demo_items` and UI verification displaying UTF-8 and non-UTF-8 items safely without panics
* [x] **Phase 1.4 (Configuration Management - TOML Schema & Sync): Completed**
  * Linux XDG Base Directory specification compliance `$XDG_CONFIG_HOME/Cuddle/config.toml` (`dirs` + `toml` 0.8)
  * Mandatory safety guardrail enforcement (automatic denial of Landlock disablement and disk free multiplier clamped to safe 1.5x minimum)
  * Automated corruption self-healing and `.bak` snapshot generation with safe default recreation
  * Automated TypeScript bindings synchronization via `ts-rs` (`AppConfig.ts`, `AppSection.ts`, `WorkspaceSection.ts`, `SecuritySection.ts`)
  * Tauri IPC commands `get_app_config` and `save_app_config` exposed
  * UI configuration card implementation with full EN/JA bilingual parity
  * Automated tests: 18 Cargo tests 100% pass, 5 Vitest tests 100% pass, Clippy zero warnings
* [x] **Phase 2.1 (High-speed Directory Scanner & Inode Tracking): Completed**
  * `walkdir` (v2) directory scanner module supporting depth bounds (`max_depth`) and hidden file filtering (`src-tauri/src/scanner.rs`)
  * Physical file identity tracking (`inode: u64`, `dev: u64`) via Linux `MetadataExt` embedded in `FileItem` with `ts-rs` type synchronization
  * Cyclic symlink recursion prevention guard utilizing `(dev, ino)` hash set to safely detect and skip infinite loops
  * Non-UTF-8 path testing (`\xFF\xFE`) verifying lossless dual-key traversal and representation
  * Tauri IPC command `scan_dir` exposed
  * Automated testing: 24 Cargo tests 100% pass, 5 Vitest tests 100% pass, Clippy zero warnings
* [ ] **Phase 2.2 (plocate Index Integration & Fast Filename Search): Awaiting start**

---

## 5. Phase Operational Protocol & Mandatory Rules

All future development must strictly adhere to the following protocol:

1. **Pre-Phase Decision Gate:**  
   Before beginning any subphase (Phase X.Y), present all architectural decisions, specifications, and options to the user. Do not begin implementation without explicit user consensus.
2. **Strict Bilingual Implementation:**  
   * All UI text, system messages, warnings, and error dialogs must be declared in both `locales/en/*.json` and `locales/ja/*.json`. No hardcoded monolingual strings permitted.
   * All documentation (READMEs, design documents, handover notes) must be written and synchronized in both English and Japanese.
3. **Exit Criteria for Phase Transition:**  
   A phase is complete only when:
   * **100% of automated tests pass** (Cargo unit/integration tests and Vitest suites).
   * Translation key parity validation passes with zero missing keys in English or Japanese.
   * **All manual test checklist items are verified** (physical hardware, CLI, UI, language switching).
   * Zero blocking issues remain.
   * Formal test report is submitted and user authorization to transition to the next phase is granted.

---

## 6. Next Session Startup Guide (Phase 2.2 Tasks)

Strategic architectural plan and implementation roadmap for Phase 2.2:

### 6.1 Phase 2.2 Architecture & Design Decisions
* **plocate Integration & Automatic Fallback:**
  * When `plocate` binary is available on the system, invoke `plocate --null <pattern>` directly via argument array execution (no shell interpolation) for millisecond filename search.
  * When `plocate` is unavailable or index is unreadable, automatically fall back to direct directory traversal via `walkdir` engine (built in Phase 2.1).
* **Batch Conversion to `FileItem`:**
  * Convert each matching path to dual-key `FileItem` with physical `inode` and `dev` metadata.
* **Search Scope Confinement:**
  * Support confining search queries to `$HOME` or specified subdirectories to maintain safety and original protection boundaries.

### 6.2 Step-by-Step Implementation Guide
1. **Implement Filename Search Module (`src-tauri/src/search_filename.rs`):**
   * `search_filename(pattern: &str, scope: Option<&Path>) -> Result<Vec<FileItem>, SearchError>`
   * `plocate` probe and direct execution
   * Automatic fallback to `walkdir` scanner
2. **Expose Tauri IPC Commands & Tests:**
   * `search_files_by_name(pattern: String, scope_base64: Option<String>) -> Result<Vec<FileItem>, String>`
   * Test matrix covering normal matches, wildcard patterns, non-existent files, and fallback execution
3. **Validation Commands:**
   ```bash
   cargo test --lib search_filename::tests
   cargo clippy -- -D warnings
   pnpm test
   ```

---

## 7. Development Guardrails

* **Root Execution Prohibited:** The application must strictly execute with non-privileged user rights (no `sudo` / `pkexec` privilege escalation).
* **Original File Protection:** In-place modifications to original files are categorically forbidden. Operations apply strictly to `workspace-temp/` copies and are reflected only upon explicit confirmation.
* **Safety Warnings Cannot Be Disabled:** Critical warnings regarding destructive actions, permissions, or conflicts must not be configurable or suppressible by the user.
* **No Arbitrary Shell Strings:** Processes are spawned strictly via argument arrays under Landlock; `sh -c` and `bash -c` are forbidden.
* **No Monolingual Hardcoding:** Never hardcode English or Japanese strings directly in UI components; always retrieve via `t('key')`.
