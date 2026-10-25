# FileWorkbench (Cuddle) Development Handover Document

<p align="center">
  <strong>English</strong> | <a href="FileWorkbench-Handover.ja.md">日本語</a>
</p>

**Document Date:** 2026-10-25  
**Status:** Phase 5 Completed, Ready for Phase 6  
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
* [x] **Phase 2.2 (plocate Index Integration & Fast Filename Search): Completed**
  * Sub-millisecond filename search via `plocate --null` direct argument array invocation (no shell strings) (`src-tauri/src/search_filename.rs`)
  * Automatic fallback to recursive `walkdir` traversal when `plocate` is missing, unindexed, or errors
  * Search boundary constraints (`scope_raw_base64`) and case-insensitive (`ignore_case`) / basename (`basename_only`) controls
  * Automated TypeScript bindings synchronization via `ts-rs` (`SearchFilenameOptions.ts`, `SearchResultDto.ts`)
  * Tauri IPC command `search_files_by_name` exposed
  * Automated testing: 30 Cargo tests 100% pass, 5 Vitest tests 100% pass, Clippy zero warnings
* [x] **Phase 2.3 (Tantivy + lindera-tantivy Japanese Full-text Search Engine): Completed**
  * Tantivy 0.26 and Lindera 5 (`embed-ipadic` morphological tokenizer) Japanese full-text search engine implemented (`src-tauri/src/search_fulltext.rs`)
  * Linux XDG Base Directory specification compliance `$XDG_DATA_HOME/Cuddle/index/` persistent index directory and in-memory (`new_in_ram`) ephemeral engine
  * Safe incremental updates based on `item_id` (deduplication `delete_term` + `add_document`) and explicit document deletion
  * Highlighted preview snippet extraction for search hits via `SnippetGenerator`
  * Automated TypeScript bindings synchronization via `ts-rs` (`FulltextHitDto.ts`, `FulltextSearchOptions.ts`)
  * Tauri IPC command `search_fulltext` exposed and registered in `invoke_handler`
  * Automated tests: 35 Cargo tests 100% pass, 5 Vitest tests 100% pass, Clippy zero warnings
* [x] **Phase 2.4 (Text Extraction Isolated Worker: Plaintext/MD Direct + Memory-Bounded Isolated Process): Completed**
  * Two-tier text extraction architecture implemented (`src-tauri/src/extractor.rs`)
  * Tier 1 direct in-process extraction for text/code files with Mozilla `chardetng` and `encoding_rs` automatic character encoding detection (UTF-8, Shift_JIS, EUC-JP)
  * 10 MiB size quota guard and bounded character truncation (`truncated`) for OOM prevention
  * Tier 2 isolated worker (PDF): Linux Landlock LSM read-only original protection, RLIMIT_AS (256 MiB) memory ceiling, and 30s timeout execution
  * Safe handling and warning notifications when `pdftotext` is missing
  * Unified extraction pipeline into Tantivy + Lindera Japanese full-text search index (`index_file_with_text`)
  * Automated TypeScript bindings synchronization via `ts-rs` (`ExtractedTextDto.ts`, `ExtractTextOptions.ts`)
  * Tauri IPC commands `extract_file_text`, `index_file_with_text` exposed
  * Full bilingual dictionary (`extractor.*`) and UI extractor card integration
  * Automated tests: 45 Cargo tests 100% pass, 5 Vitest tests 100% pass, Clippy zero warnings
* [x] **Phase 3.1 (6-Region Main Layout UI Integration & @tanstack/react-virtual Virtual Scrolling): Completed**
  * Integrated 6-region resizable split layout via `react-resizable-panels` (`src/App.tsx`)
  * Left column (Search Results + Directory Tree), center column (File Browser + Workbench), right column (Detail Inspector sidebar), bottom column (Preview / History / Security / Logs)
  * Standard density (row height 42px: icons, filename, size, modified date, inode, original protection pill) 60fps virtualized scrolling via `@tanstack/react-virtual` (`src/components/VirtualFileList.tsx`)
  * Specialized modular panels (`Header`, `SearchPanel`, `FolderTreePanel`, `FileBrowserPanel`, `WorkspacePanel`, `DetailsSidebarPanel`, `BottomTabPanel`)
  * Collapsible toggling for right sidebar and bottom panel, plus one-click layout reset
  * Dark Velvet Slate (`#070b12`) glassmorphism aesthetic with subtle glow accents
  * Full bilingual dictionary expansion (`panels.*`, `browser.*`, `tree.*`, `details.*`) with instant language toggle
  * Automated testing: 45 Cargo tests 100% pass, 6 Vitest tests 100% pass, Clippy zero warnings, Vite build success
* [x] **Phase 3.2 (Safety Operation Layer OperationPolicy Engine & Conflict Detection): Completed**
  * Safety operation policy engine implemented (`src-tauri/src/policy.rs`)
  * Dynamic detection of forbidden zones (outside `$HOME`, mount boundaries, and external storage) via `/proc/mounts` parsing (`is_external_or_mount_boundary`, `is_in_home_directory`)
  * Absolute original protection guardrail for reflection operations (`validate_reflection` enforcing `ERR_FORBIDDEN_ZONE`)
  * Original snapshot capture (`OriginalSnapshotDto`: inode, dev, size, mtime, SHA-256)
  * TOCTOU external mutation and race detection engine (`verify_conflict`: inode mismatch, size deviation, mtime drift, strict SHA-256 validation)
  * Disk space guardrail via `libc::statvfs` enforcing 1.5x original size lower threshold (`check_disk_space`)
  * Automated TypeScript bindings synchronization via `ts-rs` (`OriginalSnapshotDto.ts`, `PolicyCheckResultDto.ts`)
  * Tauri IPC commands `check_operation_policy`, `create_original_snapshot`, `verify_original_conflict` exposed
  * Full bilingual dictionary expansion (`policy.*`, `errors.ERR_*`) and Details Sidebar real-time safety evaluation & snapshot verification UI integration
  * Automated testing: 54 Cargo tests 100% pass, 8 Vitest tests 100% pass, Clippy zero warnings, Vite build success
* [x] **Phase 4.1 (Working Copy Isolation & Same-Volume Preferred Placement): Completed**
  * Working copy isolation engine implemented (`src-tauri/src/workspace.rs`)
  * Preferred local placement in `.cuddle-temp/` on the same volume with Linux `ioctl(FICLONE)` Reflink (CoW) instantaneous cloning
  * Graceful streaming copy fallback to `$XDG_CACHE_HOME/Cuddle/staging/` when non-CoW or volume is read-only
  * Pre-flight disk space safety verification (enforcing 1.5x minimum free space multiplier)
  * Dual-key consistency and staged item state tracking (`StagedItemDto`: original info, staged path, clone method `Reflink` | `StandardCopy`, status `Staged` | `Modified` | `Reflected`, original snapshot)
  * Automated TypeScript bindings synchronization via `ts-rs` (`StagedItemDto.ts`, `CloneMethod.ts`, `StageStatus.ts`)
  * Tauri IPC commands `stage_file`, `unstage_file`, `list_staged_items`, `clear_staged_items` exposed
  * UI integration in `WorkspacePanel.tsx` and `VirtualFileList.tsx` with Reflink badges, per-item unstage, and clear-all actions
  * Full bilingual dictionary expansion (`workspace.*`, `errors.ERR_STAGE_*`)
  * Automated testing: 61 Cargo tests 100% pass, 10 Vitest tests 100% pass, Clippy zero warnings, Vite build success
* [x] **Phase 4.2 (In-Workspace Fast Swap, Diff Preview & Safe Original Recovery): Completed**
  * Fast text diff computation engine implemented using `similar` (v2.6) crate (`src-tauri/src/diff.rs`)
  * Leading bytes heuristic for binary file detection with safe non-text diff representation
  * Structured diff output (`FileDiffResultDto`, `DiffLineDto`, `DiffChangeType`) tracking old and new line numbers
  * One-click pristine original recovery engine (`reset_staged_file`: purges mutated staging copy, re-clones from original snapshot, resets status)
  * Automated TypeScript bindings synchronization via `ts-rs` (`DiffChangeType.ts`, `DiffLineDto.ts`, `FileDiffResultDto.ts`)
  * Tauri IPC commands `get_file_diff`, `reset_staged_file` exposed
  * Integrated real-time Unified Diff preview in `BottomTabPanel.tsx` (Insert: green, Delete: red, Equal: muted)
  * One-click "Reset to Original" button in preview header connected to live backend reset
  * Full bilingual dictionary expansion (`diff.*`, `errors.ERR_DIFF_FAILED`)
  * Automated testing: 69 Cargo tests 100% pass, 12 Vitest tests 100% pass, Clippy zero warnings, Vite build success
* [x] **Phase 4.3 (Safe Original Reflection Transaction, Conflict Guard & TOCTOU Race Defense): Completed**
  * Safe original reflection transaction engine implemented (`src-tauri/src/workspace.rs`)
  * Pre-flight rigorous multi-guardrails:
    * Dynamic forbidden zone enforcement (`validate_reflection` blocks writes outside `$HOME`, mount boundaries, or external storage)
    * TOCTOU conflict detection (`verify_conflict` automatically aborts upon external SHA-256 mismatch, mtime variance, inode change, or size drift)
    * Destination disk space availability check
  * POSIX `fs::rename` atomic replacement using sibling temporary file (`.cuddle_reflect_<uuid>.tmp`) within the identical parent directory/filesystem
  * Automatic post-reflection snapshot refreshment (SHA-256 update) and staging status transition to `StageStatus::Reflected`
  * Tauri IPC command `reflect_staged_file` exposed and registered in `invoke_handler`
  * Added "Reflect Changes to Original" button in `BottomTabPanel.tsx` diff header with safety confirmation modal dialog displaying original/staged paths, diff statistics, and warnings
  * UI execution integration in `App.tsx` with automatic directory re-scanning to synchronize size and timestamps
  * Full bilingual dictionary expansion (`reflection.*`)
  * Automated testing: 71 Cargo tests 100% pass, 13 Vitest tests 100% pass, Clippy zero warnings, Vite build success
* [x] **Phase 5.1 (Normalization & Batch Rename Rules Engine with Dry-Run): Completed**
  * Implemented bulk renaming & normalization engine in `batch.rs` (Regex substitution, Unicode NFC/NFD/NFKC/NFKD normalization, fullwidth/halfwidth conversion, sequence numbering, datetime stamping, prefix/suffix)
  * Real-time collision detection (`ConflictDuplicate` within batch, `ConflictExistsOnDisk` in directory, `InvalidFilename` character safety)
  * Instant generation and validation of virtual mapping plans (`BatchPlanDto`, `PlannedOperationDto`)
  * Implemented responsive `BulkRenameModal.tsx` with rule builders, dry-run summary badges, preview table, and execution guardrails
  * `ts-rs` automated TypeScript type synchronization (`BatchPlanDto.ts`, `PlannedOperationDto.ts`, `RenameRule.ts`, `BulkRenameOptions.ts`, etc.)
  * Full bilingual dictionary expansion (`bulk_rename.*`)
* [x] **Phase 5.2 (Two-Phase Rename & Deterministic Atomic Rollback): Completed**
  * Safe resolution of file swaps and duplicate sequential numbering via temporary UUID intermediate parking (Phase 1) followed by target commitment (Phase 2)
  * Comprehensive deterministic atomic rollback (RollbackAll) restoring initial staging states if any single step encounters an error
  * Real-time workspace and browser synchronization via `WorkspaceManager::update_staged_path`
* [x] **Phase 5.3 (Rollback Journal & Atomic One-Click Undo): Completed**
  * Thread-safe bulk rename history management via `BatchJournalManager`
  * Dynamic reverse plan generation and two-phase atomic execution for one-click rollback (`undo_last_bulk_rename`)
  * Strict separation between internal disk UUID prefixes and user-facing display names (`display_name`) ensuring pristine restoration
  * Header undo button (`RotateCcw`) integration and floating toast notification banner for instant visual confirmation
  * Automated testing: 96 Cargo tests 100% pass, 20 Vitest tests 100% pass, Clippy zero warnings, Vite build success

---

## 5. Phase Operational Protocol & Mandatory Rules

All future development must strictly adhere to the following protocol:

1. **Pre-Phase Decision Gate:**  
   Before beginning any subphase (Phase X.Y), present all architectural decisions, specifications, and options to the user. Do not begin implementation without explicit user consensus.
2. **Strict Bilingual Implementation:**  
   * All UI text, system messages, warnings, and error dialogs must be declared in both `locales/en/*.json` and `locales/ja/*.json`. No hardcoded monolingual strings permitted.
   * All documentation (READMEs, design documents, handover notes) must be written and synchronized in both English and Japanese.
3. **Color Themes & Pipeline Presets Notification Protocol:**
   * **Color Themes:** Notify and form user consensus at the start of the final UI polishing phase (Phases 8–9).
   * **Pipeline Presets:** Notify and form user consensus at the recommended timing (Phase 7: Pipeline Runner).
4. **Mandatory Manual Testing on Major Phase Transitions:**  
   At major phase boundaries (e.g. Phase 1 → 2, Phase 2 → 3, Phase 3 → 4, Phase 4 → 5, Phase 5 → 6), the assistant must present a concrete manual verification checklist on real hardware/GUI/CLI and confirm functionality before proceeding to the next major phase.

---

## 6. Next Session Startup Guide (Phase 6 Tasks)

Having completed Major Phase 5 (Advanced Bulk Operations & Normalization) manual verification, the project transitions to Major Phase 6 (Backups, Operation History, UNDO & Session Restoration) **after user consensus on the pre-phase decision gate**.

### 6.1 Phase 6.1 Architecture & Design Decisions (Operation History, Persistent Journal & Separation Model)
* **Persistent Operation History & Journal:**
  * Log operations (bulk rename, reflection, staging, resets) with timestamp, inode, operation type, paths, and execution status.
  * Explicit determination of undoability (verifying whether backing data exists and is safe to revert).
* **History Inspector UI:**
  * Localized timeline view in the bottom "History" panel tab.
  * Direct invocation of undo or restoration from history entries.

### 6.2 Phase 6.2 Architecture & Design Decisions (Session Restoration Engine)
* **Workspace Staging Session Manifest:**
  * Persist active `workspace-temp/` staging states across application restarts or unexpected terminations.
  * Validate external origin conflicts (TOCTOU) and auto-restore staged working copies upon app startup.

### 6.3 Phase 6.3 Architecture & Design Decisions (Exit Confirmation & Safe Tar Archiving)
* **Exit Guardrail Flow:**
  * Intercept window closing when unreflected modified working copies exist.
  * Safe options: "Archive & Exit (tar.zst/tar.gz)", "Discard & Exit", or "Cancel".

---

## 7. Development Guardrails

* **Root Execution Prohibited:** The application must strictly execute with non-privileged user rights (no `sudo` / `pkexec` privilege escalation).
* **Original File Protection:** In-place modifications to original files are categorically forbidden. Operations apply strictly to `workspace-temp/` copies and are reflected only upon explicit confirmation.
* **Safety Warnings Cannot Be Disabled:** Critical warnings regarding destructive actions, permissions, or conflicts must not be configurable or suppressible by the user.
* **No Arbitrary Shell Strings:** Processes are spawned strictly via argument arrays under Landlock; `sh -c` and `bash -c` are forbidden.
* **No Monolingual Hardcoding:** Never hardcode English or Japanese strings directly in UI components; always retrieve via `t('key')`.
