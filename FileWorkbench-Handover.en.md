# FileWorkbench (Cuddle) Development Handover Document

<p align="center">
  <strong>English</strong> | <a href="FileWorkbench-Handover.ja.md">日本語</a>
</p>

**Document Date:** 2026-10-29  
**Status:** All Phases (Phase 1–14) fully implemented and verified. Open-source distribution ready (MIT License / README / BUILD / CI Actions complete); Phase 14 (Architectural Decisions & Scale Verification: full alignment between Settings menu items and config.toml, config.template.toml complete update, Standard / Strict safety confirmation levels, endorsement of SQLite WAL + BatchJournal state model, and validated batch IPC architecture) fully tested and verified (218 Cargo / 127 Vitest 100% pass).  
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
| [`README.ja.md`](file:///home/susie/git/wammed/Cuddle/README.ja.md) | [`README.md`](file:///home/susie/git/wammed/Cuddle/README.md) | Project Overview, Features Matrix, and Quick Start Guide |
| [`BUILD.ja.md`](file:///home/susie/git/wammed/Cuddle/BUILD.ja.md) | [`BUILD.md`](file:///home/susie/git/wammed/Cuddle/BUILD.md) | Build Environment Setup, Dependencies, Testing, and Release Guide |
| [`LICENSE`](file:///home/susie/git/wammed/Cuddle/LICENSE) | [`LICENSE`](file:///home/susie/git/wammed/Cuddle/LICENSE) | MIT License Open Source Terms & Conditions |
| [`FileWorkbench-Design-ver0.9.ja.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Design-ver0.9.ja.md) | [`FileWorkbench-Design-ver0.9.en.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Design-ver0.9.en.md) | Integrated Basic Design Document (Definitive Edition) |
| [`FileWorkbench-Handover.ja.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Handover.ja.md) | [`FileWorkbench-Handover.en.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Handover.en.md) | Development Handover Document (this document) |
| [`roadmap.ja.md`](file:///home/susie/.gemini/antigravity-ide/brain/45a8eaa8-509d-4ed8-9e6c-255c45bac425/roadmap.ja.md) | [`roadmap.en.md`](file:///home/susie/.gemini/antigravity-ide/brain/45a8eaa8-509d-4ed8-9e6c-255c45bac425/roadmap.en.md) | Detailed Implementation Roadmap (9 Phases / 28 Subphases) |
| [`NEXT_IMPLEMENTATION_TASKS.ja.md`](file:///home/susie/git/wammed/Cuddle/NEXT_IMPLEMENTATION_TASKS.ja.md) | [`NEXT_IMPLEMENTATION_TASKS.en.md`](file:///home/susie/git/wammed/Cuddle/NEXT_IMPLEMENTATION_TASKS.en.md) | Next Implementation Tasks & Roadmap (18 Items from Design Spec ver 0.9 / Phases 10–14) |
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
* [x] **Phase 6.1 (Operation History, Persistent SQLite WAL Journal & Undo Separation): Completed**
  * Persistent SQLite WAL operation history manager with synchronous NORMAL mode using `rusqlite` bundled (`src-tauri/src/history.rs`)
  * Automatic recording for all key file operations: Staging, Unstaging, Bulk Rename, Undo Rename, Reset to Original, Reflect to Original, and Clear Workspace
  * Automated history trimming retaining the most recent 50 operations
  * Tauri IPC commands `get_operation_history` and `clear_operation_history`
  * Overhauled `BottomTabPanel.tsx` "Operation History" tab: Structured timeline featuring timestamps, color-coded operation badges, original and staged file paths, status indicators, and operation details
  * History clearing action with immediate UI synchronization and global status bar record count integration
  * Complete bilingual dictionary sync (`history.*`) in English and Japanese
  * Automated testing: 100 Cargo tests 100% pass, 22 Vitest tests 100% pass, Clippy zero warnings, Vite build success
* [x] **Phase 6.2 (Session Restoration Engine & TOCTOU Conflict Detection): Completed**
  * Persistent session manifest via SQLite WAL `session_items` table (synchronized automatically on stage, unstage, rename, undo, reset, reflect, and clear)
  * Automated session restoration engine (`restore_workspace_session`) executing on application boot
  * Staging file verification on disk and strict TOCTOU conflict checking against original files (SHA-256 hash, mtime, file size)
  * Safe transition to `StageStatus::Conflict` and warning badge (`conflict-pill-sm`) when external modifications are detected
  * `ts-rs` automated TypeScript type synchronization (`StageStatus.ts` updated with `"Conflict"`)
  * Full bilingual dictionary expansion (`workspace.status_conflict`, `workspace.restored_session`, `workspace.conflict_alert`)
  * Automated testing: 102 Cargo tests 100% pass, 23 Vitest tests 100% pass, Clippy zero warnings, Vite build success
* [x] **Phase 6.3 (Interactive Exit Dialog & Workspace tar.gz Archiving): Completed**
  * Detection of unreflected staged items (`has_unreflected_staged_items`) and dynamic default backup destination generation (`get_default_backup_path`: `$XDG_DATA_HOME/Cuddle/backups/backup_YYYYMMDD_HHMMSS.tar.gz`)
  * High-performance tar.gz archive generation engine (`create_workspace_backup`) bundling all staged files and `manifest.json` (original paths, staged file names, clone methods, timestamps) using `flate2` + `tar`
  * User decision-gate compliance: default backup path with one-click reset alongside full custom path editing (automatic parent directory creation)
  * Automatic logging into SQLite WAL operation history upon backup completion (`OperationType::Backup`)
  * `ExitBackupModal.tsx` implementation: 3-way safe exit actions ("Archive & Exit", "Discard & Exit", "Continue Working"), error alerts, and destination input
  * `WorkspacePanel.tsx` header integration: "Backup / Exit" action button (`Archive` icon)
  * `ts-rs` automated TypeScript type synchronization (`BackupResultDto.ts`)
  * Full bilingual dictionary expansion (`backup.*`) in English and Japanese
  * Automated testing: 105 Cargo tests 100% pass, 31 Vitest tests 100% pass, Clippy zero warnings, Vite build success
* [x] **Phase 7.1 (Sandboxed External Tool Execution, Probe & Pure-Rust Transforms): Completed**
  * Implemented `src-tauri/src/pipeline.rs` providing SandboxedExecutor, ToolDetector, and BuiltinPipelineEngine
  * **Landlock LSM Isolation:** Original paths blocked from read/write; sub-processes restricted exclusively to `workspace-temp/` directory
  * **Safety Guardrails:** RLIMIT_AS (512 MiB) virtual memory limit, 30-second SIGKILL timeout protection, 10 MiB output truncation guards, direct argument array execution (eliminating `sh -c` shell injections)
  * **Pure-Rust Builtin Transforms:** Zero-dependency, immediate processing even without external tools (log secret masking, CSV cleansing, JSON minify, TSV/CSV conversion, SQL dump slimming, SRT timecode shift, SHA-256 checksum generation)
  * **Tauri IPC Commands:** Exposed `probe_external_tools`, `execute_sandboxed_command`, `run_builtin_text_transform`
  * **UI Integration:** Added "External Tool Ecosystem (Phase 7.1)" card inside `BottomTabPanel.tsx` Security & Settings tab (displaying ImageMagick, FFmpeg, qpdf, pdfimages, exiftool, pandoc status badges, versions, paths, and rescan action)
  * **Dark Theme Compliance:** Zero white backgrounds, adhering strictly to Dark Velvet Slate palette
  * **Automated Type Sync:** `ts-rs` generation of `ExternalToolStatusDto.ts`, `CommandExecutionOptionsDto.ts`, `CommandExecutionResultDto.ts`, `BuiltinTransformResultDto.ts`
* [x] **Phase 7.2 (30 Pipeline Presets Definition, DAG Execution Engine & UI Integration): Completed**
  * **30 Presets Definition:** Implemented all 30 presets in `PipelineRegistry`: Images 1-8, Audio/Video 9-13, Text 14-20, Documents 21-24, File Organization 25-27, and Dev/Sec 28-30.
  * **Hybrid Execution Engine:** Pure-Rust high-speed transforms combined with Landlock LSM sandboxed external tool dispatch (ImageMagick, FFmpeg, qpdf, etc.).
  * **Original File Protection:** Transforms only apply to working copies inside `workspace-temp/`, safely updating staging paths while leaving original files untouched.
  * **Operation History Integration:** Automatically records completed pipeline runs into SQLite WAL journal as `OperationType::Pipeline`.
  * **UI Integration (`PipelineModal.tsx`):** 30 preset browser, category pills (All/Image/Media/Text/Document/File/Dev), search box, parameter sliders/inputs, tool availability checks, execution result summary, and double-click prevention.
  * **Dark Mode Enforcement:** Completely eliminated pale/whitish tints on secondary buttons ("Original Snapshot Recorded", "Text Extraction & Indexing") by establishing deep Dark Velvet Slate surfaces (`rgba(15, 23, 42, 0.9)`, text `#e2e8f0`, border `rgba(255, 255, 255, 0.14)`). Strictly prohibited white backgrounds in textboxes, modals, and popups.
  * **Bilingual Synchronization:** Full English and Japanese dictionaries synchronized (`pipeline.*`).
  * **Automated Type Sync:** `ts-rs` generation of `PipelineCategory.ts`, `PipelinePresetDto.ts`, `PipelineParameterDef.ts`, `ExecutePipelineRequestDto.ts`, `PipelineExecutionResultDto.ts`, and `PipelineExecutionItemResultDto.ts`.
* [x] **Phase 7.3 (Real-time Progress UI & Cancellation Controls): Completed**
  * **Asynchronous Progress Streaming:** Dispatched `"pipeline-progress"` Tauri events in real time as each staged file begins, succeeds, or fails.
  * **Job-based Cancellation Management:** Implemented `PipelineCancellationManager` with atomic cancellation flags accessible via Tauri command `cancel_pipeline`.
  * **Immediate Process Termination (SIGKILL):** Interrupted external child processes (ImageMagick, FFmpeg, etc.) immediately when cancellation is requested, preventing runaway resource usage.
  * **UI Integration (`PipelineModal.tsx` & `App.css`):**
    * Displayed real-time progress bar with percentage, item count (e.g. `1 / 5`), and active target filename.
    * Converted "Execute Pipeline" button during execution into a prominent, dark crimson "Cancel Execution" button (`StopCircle` icon).
    * Rendered amber warning banner (`AlertTriangle`) when execution is cancelled by user, detailing successfully processed files prior to termination.
  * **Dark Mode Enforcement:** Guaranteed zero white surfaces across progress bars, cancel buttons, and warning banners, maintaining high contrast Dark Velvet Slate styling.
  * **Bilingual Synchronization:** Full English and Japanese dictionaries synchronized with all new keys.
  * **Verification:** 130 Cargo tests 100% pass, 38 Vitest tests 100% pass, Clippy zero warnings, Vite build success.

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
   At major phase boundaries (e.g. Phase 1 → 2, Phase 2 → 3, Phase 3 → 4, Phase 4 → 5, Phase 5 → 6, Phase 6 → 7), the assistant must present a concrete manual verification checklist on real hardware/GUI/CLI and confirm functionality before proceeding to the next major phase.

---

## 6. Next Session Startup Guide (Phase 7 Tasks)

With Major Phase 6 (Backups, Operation History, UNDO & Session Restoration) fully implemented and verified via automated tests, the project transitions to Major Phase 7 (External Tool Integration & Pipeline Runner) **following real-hardware manual verification checklist confirmation and pre-phase decision gate consensus**.

### 7.1 Phase 7.1 Architecture & Design Decisions (External Command Execution Engine)

* **Process Isolation & Landlock LSM / cgroups Confinement:**
  * Execute external media/document utilities (ImageMagick, FFmpeg, pandoc) isolated with read-write access restricted strictly to `workspace-temp/`.
  * Protective bounds on memory, CPU, and execution timeout (30 seconds).
  * Argument array execution completely devoid of shell string interpolation (`sh -c`).

### 7.2 Phase 7.2 Architecture & Design Decisions (30 Pipeline Presets & DAG Workflow)

* **Agreed 30 Pipeline Presets Taxonomy:**
  * **Images & Visual Processing (1–8):**
    1. Web Distribution Optimization (Strip Exif $\rightarrow$ Resize to max 1920px $\rightarrow$ WebP Q80 $\rightarrow$ Append `_web`)
    2. Social & OGP Card Generation (1200x630 crop $\rightarrow$ Padding $\rightarrow$ JPEG Q85)
    3. App Icon & Favicon Pack (Square crop $\rightarrow$ Multi-resolution 16/32/180/512px $\rightarrow$ PNG)
    4. Watermarked Preview Generation (1200px downscale $\rightarrow$ Bottom-right 30% watermark composite $\rightarrow$ JPEG)
    5. Retina Asset Set (Input @2x $\rightarrow$ 50% scale @1x generation $\rightarrow$ `name@2x.png` / `name.png`)
    6. Pixel Art / Sprite Sheet Upscaling (Nearest-neighbor 400% enlargement $\rightarrow$ Transparent PNG)
    7. High-Res CMYK Print Prep (300 DPI metadata $\rightarrow$ sRGB to CMYK conversion $\rightarrow$ Lossless TIFF)
    8. Artistic Monochrome Film Effect (Grayscale $\rightarrow$ S-curve contrast $\rightarrow$ Subtle film grain $\rightarrow$ JPEG)
  * **Audio, Video & Rich Media (9–13):**
    9. Web Animated GIF from Video (Time segment trim $\rightarrow$ Width 480px $\rightarrow$ Palette optimization $\rightarrow$ GIF)
    10. Podcast Audio Mastering (Loudness normalization -16 LUFS $\rightarrow$ Silence trimming $\rightarrow$ 128kbps MP3 $\rightarrow$ ID3 tags)
    11. Audio Extraction & Transcription Prep (Strip video $\rightarrow$ Downmix mono $\rightarrow$ 16kHz sampling $\rightarrow$ WAV)
    12. Smartphone Video SNS Reformat (9:16 center crop $\rightarrow$ H.264/AAC encode $\rightarrow$ FastStart optimization)
    13. Discord/Slack File Size Shrink (720p downscale $\rightarrow$ 2-pass bitrate control targeting < 8MB $\rightarrow$ MP4)
  * **Text, Logs & Data Cleansing (14–20 / Pure Rust In-Memory Engine):**
    14. Public Log Sanitization (Mask IP addresses $\rightarrow$ Mask emails $\rightarrow$ Mask API tokens $\rightarrow$ Date prefix)
    15. Legacy CSV UTF-8 Cleansing (Shift_JIS to UTF-8 $\rightarrow$ CRLF to LF $\rightarrow$ Trim column whitespace)
    16. JSON Formatting & Minification (JSON syntax validation $\rightarrow$ Whitespace/newline stripping $\rightarrow$ `.min.json`)
    17. TSV/CSV Conversion & Header Snake_case (Delimiter conversion $\rightarrow$ Header snake_casing $\rightarrow$ Quote escaping)
    18. Markdown Tech Spec Linter/Formatter (Fullwidth/halfwidth normalization $\rightarrow$ Table & blank line cleanup)
    19. SQL Dump Minification (Strip comments & blank lines $\rightarrow$ Combine bulk INSERT statements $\rightarrow$ gzip)
    20. Subtitle (SRT) Timecode Shift & Clean (Batch timecode offset $\rightarrow$ Tag cleanup $\rightarrow$ UTF-8 without BOM)
  * **Documents & PDF Workflows (21–24):**
    21. Scanned Document Enhancement & Binarization (Auto-deskew $\rightarrow$ Grayscale $\rightarrow$ Adaptive binarization $\rightarrow$ PDF)
    22. Multi-PDF Merge & Table of Contents (Uniform A4 size $\rightarrow$ Filename sequence merge $\rightarrow$ Document bookmarks)
    23. Secure Document PDF Distribution (Scrub metadata $\rightarrow$ Apply print/copy permission passwords $\rightarrow$ Linearized PDF)
    24. PDF Raster Image Extractor (Extract embedded images $\rightarrow$ Deduplicate SHA hashes $\rightarrow$ Sequential export)
  * **File Organization & Smart Renaming (25–27):**
    25. Camera Media Auto-Directory Sorting (Extract Exif timestamp $\rightarrow$ `YYYYMMDD_HHMMSS` $\rightarrow$ Sort into `YYYY/MM/`)
    26. Cluttered Downloads Normalizer (Decode URL encodings $\rightarrow$ Spaces to hyphens $\rightarrow$ Lowercase conversion)
    27. Asset Zero-Padding Sequencer (Detect numeric tail $\rightarrow$ 4-digit zero padding `img_0001.png` $\rightarrow$ Conflict collision avoidance)
  * **Development, Distribution & Security (28–30):**
    28. Clean Source Distribution Packaging (Exclude `.git`, `node_modules`, `.DS_Store` $\rightarrow$ LF conversion $\rightarrow$ ZIP)
    29. Tamper-Evident Checksum Archive (Generate SHA-256 for all staged items $\rightarrow$ Bundle `checksums.txt` $\rightarrow$ tar.gz)
    30. Encrypted Archive Backup (Tar bundling $\rightarrow$ AES-256 encryption $\rightarrow$ Output `.enc`)
* **Tool Availability Probe:**
  * Dynamic detection of required tools (`magick`, `ffmpeg`, `qpdf`, `pdfimages`, `exiftool`) in `$PATH`. Clean guidance and fallback for missing tools. Pure-Rust presets (14–20, 26–29) work instantly without external prerequisites.

### 7.3 Phase 7.3 (Real-time Progress UI & Cancellation Controls): Completed

* **Streaming Asynchronous Feedback & UI Integration:**
  * Real-time file processing status tracking (pending, running, succeeded, failed) via progress indicators and percentage counters (Tauri event `pipeline-progress`).
  * Immediate SIGKILL process interruption and safe RollbackAll termination via `PipelineCancellationManager`.
  * Comprehensive UI integration in `PipelineModal.tsx` including animated progress bar, active file name, crimson cancellation button, and abort warning banner.

---

## 8. Phase 8 (UI/UX Polish, Virtual Scrolling, Layout Optimization & Cyber Theme): Completed

### 8.1 User Agreement & Decision Gate

1. **Color Theme Selected:** `Midnight Teal / Cyber Dark` (Deep oceanic canvas `#060c11` / `#09141b` accented with cyber teal `#14b8a6` and neon mint `#2dd4bf`).
2. **Action Bar Concealment Fix:** `Fixed Bottom Action Bar Layout` (Footer action buttons are permanently pinned at the bottom of all modals and dialogs with `flex-shrink: 0; margin-top: auto;` while the upper content/banner areas scroll independently, ensuring action buttons never disappear regardless of window size).

### 8.2 Implementation Details

* **Fixed Bottom Action Bar Architecture:**
  * Standardized `.dialog-footer` and `.dialog-actions-bar` across all modals (`.reflect-confirm-dialog`, `.bulk-rename-modal`, `.exit-backup-dialog`, `.pipeline-modal-dialog`) with `flex-shrink: 0; margin-top: auto; position: relative; z-index: 10;`.
  * Enforced `flex: 1; min-height: 0; overflow-y: auto;` on modal body containers (`.dialog-body`, `.bulk-rename-body`, `.pipeline-config-scroll`), preventing layout overflow and ensuring action buttons remain visible even with tall error banners or preview tables.
* **Midnight Teal / Cyber Dark Global Theme:**
  * Tokenized design system palette: `--bg-canvas` (`#060c11`), `--bg-surface-base` (`#09141b`), `--bg-surface-input` (`#050a0e`), `--accent-brand` (`#14b8a6`), `--accent-brand-bright` (`#2dd4bf`).
  * Injected sleek 6px global custom scrollbars (`::-webkit-scrollbar`) with cyber teal thumb styling.
  * Overhauled formerly washed-out buttons ("Original Snapshot Recorded", "Text Extraction & Indexing") to rich cyber dark cards with high-contrast mint typography.
  * Completely eradicated white backgrounds across all inputs, popups, and modal dialogs.
* **Virtual Scrolling (`@tanstack/react-virtual`):**
  * Powered `FileBrowserPanel` and `WorkspacePanel` through `VirtualFileList` for smooth 60fps rendering of up to 10,000 files with minimal memory footprint.
* **Keyboard Shortcuts & Accessibility (Phase 8.4):**
  * `Ctrl+F` / `Cmd+F`: Immediate focus and text selection in global search input (`#global-search-input`).
  * `Ctrl+R` / `Cmd+R`: Open Bulk Rename modal (when staged files exist).
  * `Ctrl+P` / `Cmd+P`: Open Pipeline modal (when staged files exist).
  * `Ctrl+Z` / `Cmd+Z`: Undo last bulk rename operation (when undo history exists; native text undo is preserved in textareas/inputs).
  * `Escape`: Safely close active modal dialogs.
* **FHD Window Geometry (1920x1080):**
  * Configured default window size in `src-tauri/tauri.conf.json` to `width: 1920, height: 1080` (minWidth 1024, minHeight 700).
* **Two-Row Responsive Header Architecture:**
  * Redesigned `Header.tsx` into a 2-row layout. Row 1 hosts branding, navigation mode switcher (Start ↔ Workbench), Landlock status badge, Settings button (⚙️), and language switch. Row 2 hosts the active directory path chip and panel layout controls, completely eliminating button wrapping and label truncation even under long Japanese text.
* **Settings & Preferences Modal (`SettingsModal.tsx`):**
  * Integrated dedicated Settings modal accessible via the header gear icon (⚙️). Covers 30-theme live switcher, language toggle, workspace staging info, 1.5x disk guardrail status, Landlock LSM kernel isolation monitor, keyboard shortcuts cheat sheet, and `config.toml` path information.
* **30-Theme Curated Dark Palette Presets (`themePresets.ts`):**
  * `Cyber` (8: Midnight Teal, Matrix, Quantum Abyss, Circuit, Void, Biohazard, Glitch, Subzero)
  * `Neon` (8: Synthwave, Cyberpunk 2077, Hotline Miami, Laser Pink, Acid Lime, Electric Blue, Ultra Violet, Solar Flare)
  * `Modern` (8: Tokyo Night, Nord Frost, Dracula, Catppuccin, One Dark Pro, Gruvbox Dark, Monokai Pro, Rosé Pine)
  * `Classic` (6: Dark Charcoal, Midnight Navy, Obsidian Black, Industrial Slate, Dark Espresso, Emerald Night)
  * Strictly adhering to dark mode (zero white backgrounds). Instant CSS variables live switching and `localStorage` persistence.
* **Configuration File Template (`config.template.toml`):**
  * Created clean, extensively commented template in repository root documenting all config fields, guardrail enforcement, and available theme keys.
* **Start Page Launchpad (`StartPage.tsx`):**
  * Built a rich welcome dashboard featuring 3 key action cards (Folder Explorer, 30-Pipeline Runner, Bulk Renamer), quick jump locations, and real-time security & system health indicators.
* **2-Row Multi-Line Panel Headers & Dedicated Action Toolbars (Zero Overflow):**
  * **Root Cause of Previous Overflow:** The File Browser header placed title, long path bar, and staging buttons on one row (38px fixed height), while the Workspace header packed title, protected badge, and 5 action buttons (Pipeline, Rename, Undo, Backup, Clear All) into one row. This required an excessive panel width of ~880px (equivalent to near-fullscreen WQHD 2560px) to prevent elements from wrapping, squashing, and overflowing.
  * **File Browser Panel (`FileBrowserPanel.tsx`):**
    * Row 1: `FolderOpen` icon, "File Browser" title, and right-aligned action buttons ("Stage Selected", "Stage All", "Refresh").
    * Row 2: "Navigate Up" button and full-width path bar (`browser-path-toolbar-row`) with inline click-to-edit support.
  * **Workspace Panel (`WorkspacePanel.tsx`):**
    * Row 1: `Briefcase` icon, "Staging Workspace" title, `workspace-temp/` badge, and right-aligned "Clear All" action.
    * Row 2: Dedicated operations toolbar (`workspace-action-toolbar-row`) grouping "30 Presets Pipeline", "Bulk Rename", "Undo Rename", and "Exit Backup".
  * **CSS Hardening (`App.css`):**
    * Enforced `white-space: nowrap; flex-shrink: 0;` on all toolbar and action buttons.
    * Enabled graceful wrapping (`flex-wrap: wrap`) so headers remain stable down to 400px panel widths (compact and standard FHD displays).
* **Git Status Dashboard Tab (Phase 8.5):**
  * Implemented high-precision Git backend in `src-tauri/src/git.rs` executing `git rev-parse --show-toplevel`, `git status --porcelain=v2 --branch`, and `git log -1` via direct argument arrays without shell interpolation.
  * Formatted output into structured DTOs (`GitStatusDto`, `GitFileItemDto`) with branch badge, upstream sync state (ahead / behind), clean repository status, latest commit card (hash, message, author, timestamp), and categorizes changed files into 4 clear cards: Staged (`staged`), Unstaged (`unstaged`), Untracked (`untracked`), and Conflicts (`conflicts`) with color-coded status pills (M, A, D, R, U, ?).
  * Generated TypeScript bindings via `ts-rs` (`GitStatusDto.ts`, `GitFileItemDto.ts`) with 6 Cargo unit tests passing.
* **Harmonized Bottom Tab Naming (Phase 8.5):**
  * Resolved discrepancy where the tab was previously mislabeled "Security & Settings"; harmonized to the formal name specified in design documentation: "Security & Tools Diagnostic" (Japanese: "セキュリティ & ツール診断") across both `ja` and `en` translation dictionaries.
* **Rich Preview & Right-Click Context Menu (Phase 8.6):**
  * **Strict Explicit Preview Policy:** Completely eliminated hover-triggered or left-click-triggered file previewing to avoid inadvertent I/O. Previews are explicitly initiated via right-click custom context menu ("Show Preview" / "プレビューを表示") or via guided cards in the preview tab.
  * **Syntax Highlighting (`syntaxHighlight.ts`):** Fast, zero-dependency code syntax highlighter featuring line numbers, tokenized coloring (keywords, strings, numbers, comments, functions, types, punctuation) across Rust, TypeScript, JavaScript, Python, Shell, SQL, CSS, JSON, TOML, and YAML under a dark Prism theme.
  * **Safe Markdown Rendering:** Cleanly renders Markdown headings (H1-H4), blockquotes, bullet/numbered lists, inline code chips, syntax-highlighted code blocks, bold text, and hyperlinks.
  * **HTML Live Preview:** Dual-mode preview switching between an isolated sandboxed iframe render and raw highlighted source code.
  * **Custom Context Menu (`ContextMenu.tsx`):** Sleek cyber dark contextual menu offering "Show Preview", "Copy File Path", "Add to Workbench", "Unstage", and "Delete". Features automatic boundary flip and click-outside/Escape dismissal.
* **3-Level Display Density Settings (Phase 8.7):**
  * **Three Density Configurations:**
    * Compact (`compact`: 32px row height / 14px icon / 11px font)
    * Normal (`normal`: 42px row height / 16px icon / 12px font)
    * Spacious (`spacious`: 52px row height / 20px icon / 13px font)
  * **System Integration (`density.ts`):** Dynamically updates root CSS variables (`--list-row-height`, `--list-icon-size`, `--list-font-size`, `--list-padding-y`) and persists choices to `localStorage`.
  * **Virtual Scrolling Integration:** Synchronized with `@tanstack/react-virtual` row height estimation for seamless, flicker-free list re-rendering.
  * **Settings Modal Controls (`SettingsModal.tsx`):** Prominently placed 3-pill toggle button group at the top of the Appearance settings tab.
* **User-Created Pipelines & Mandatory Design 8.3 Safety Flow (Phase 8.8):**
  * **Configuration File & Settings Menu Templates (`config.template.toml`, `SettingsModal.tsx`):**
    * Added `[[custom_pipelines]]` template sections in `config.template.toml` with detailed explanations of `{input}` (working copy absolute path) and `{output}` (staged destination path) placeholders, with examples for WebP lossless compression and ExifTool metadata stripping.
    * Added a dedicated "Custom Pipelines" tab to `SettingsModal.tsx` featuring 1-click TOML template clipboard copying, preset auto-fill buttons (WebP / ExifTool), inline creation & editing forms, deletion, and immediate persistence via `save_app_config`.
  * **Pipeline Modal Integration (`PipelineModal.tsx`):**
    * Added "Custom" (`custom`) category filter pill.
    * Integrated custom pipeline definitions from `AppConfig` into the preset browser with distinct amber `ShieldAlert` safety badges.
  * **Mandatory Design 8.3 Safety Flow Wizard:**
    * Directly executing user-defined pipelines without verification is strictly blocked; a mandatory 2-step confirmation and pre-flight Dry-Run wizard is enforced:
      * **Step 1: [Risk Explanation]** Displays the executable binary, expanded argument template, target file counts, and Linux Landlock LSM kernel isolation constraints protecting original files.
      * **1st Confirmation:** User reviews constraints and clicks "1st Confirmation: Start Dry-Run Verification".
      * **Step 2: [Dry-Run Pre-flight]** Backend executes `validate_custom_pipeline_dry_run` to verify binary existence in PATH, argument expansions, destination filename collision detection, supported file extensions, and disk space guardrails (>= 1.5x).
      * **Step 3: [Results & Risk Re-presentation]** Shows resolved binary path, interactive table of input files -> expanded commands -> proposed outputs, conflict warnings, and re-presents kernel-level protection guarantees.
      * **2nd Confirmation:** "2nd Confirmation: Execute under Landlock Isolation" activates only when Dry-Run succeeds with zero fatal conflicts.
      * **Step 4: [Execution under Landlock]** Executes the custom tool inside isolated sandboxed processes with original directory write blocking, real-time progress events, and cancellation support.
* **Context Menu Portal Rendering & Action Button Spacing Optimization (Phase 8.9):**
  * **Unclipped Context Menu Portal (`ContextMenu.tsx`):**
    * Migrated context menu rendering to `ReactDOM.createPortal(..., document.body)` to eliminate container clipping caused by `overflow: hidden` and `transform: translateY(...)` within `VirtualFileList` and `.browser-table-body`.
    * Implemented smart flip logic (`y - menuHeight`) to automatically open upward when near the viewport bottom (near `BottomTabPanel`), as well as horizontal flip (`x - menuWidth`) near the right edge, paired with `z-index: 99999` to ensure top-layer visibility.
  * **Generous Spacing for Action Buttons (Preview / Copy / Stage) (`App.css`, `VirtualFileList.tsx`):**
    * Expanded action column width (`col-header.col-actions`, `row-col-actions`) from `32px` to `72px` (more than double).
    * Introduced a clean `gap: 6px` between action buttons and optimized button dimensions to `25px × 25px` with dedicated color accents for Preview (teal) and Staging/Copy (sky blue) and hover scaling.
    * Added uniform `padding-right: 12px` gutter across Filename, Size, MTime, and INODE columns, completely eliminating cramped layouts.
    * Integrated responsive button dimensions (22px / 25px / 28px) and column widths (62px / 72px / 82px) corresponding to the 3-level density settings (compact, normal, spacious).
* **Font Family & Font Size Settings Menu (Phase 8.10):**
  * **Typography Management Foundation (`src/utils/typography.ts`):**
    * Unified typography management module covering UI Font Family (Sans-serif), Monospace Code Font (Monospace / Paths), and Base Font Size / Scale (11px to 18px / 79% to 129%).
    * **UI Font Presets (6 presets):** Default (Inter / Noto Sans), Modern (Inter / Noto Sans JP), System Native (Segoe UI / Roboto), Japanese Gothic (BIZ UDP / Yu Gothic), Rounded Gothic (M PLUS Rounded), Custom Font.
    * **Monospace Font Presets (7 presets):** Default (JetBrains / Fira / Cascadia), JetBrains Mono, Fira Code, Cascadia Code, Japanese Mono (BIZ UDGothic), Symbols Nerd Font, Custom Font.
    * **Custom Font Direct Input:** Accepts arbitrary locally installed font names with guaranteed fallback font stack protection.
    * **Dynamic CSS Variable Injection & Persistence:** Injects `--font-sans`, `--font-mono`, `--app-base-font-size`, and `--font-size-scale` into `:root` in real time and persists to `localStorage` (`cuddle-typography-settings`). Restored automatically upon application startup.
    * **Instant Reset:** One-click reset to factory defaults (Inter 14px / JetBrains Mono).
  * **Settings Modal UI Integration (`SettingsModal.tsx`):**
    * Added dedicated Typography Card (`.settings-typography-card`) within the Appearance tab.
    * Base font size slider (11px to 18px) alongside quick-select pill buttons (11px, 12px, 13px, 14px, 15px, 16px, 18px).
    * Preset pill button selectors and custom font text inputs for both UI and Monospace fonts.
    * Live preview specimen card displaying both UI text sample and monospace code block sample with immediate visual reflection.
  * **Bilingual Dictionary Synchronization (`typography.*`):**
    * Full parity across Japanese and English locales (`ja/translation.json`, `en/translation.json`).
  * **Design Cohesion (Zero White Backgrounds):**
    * Dark velvet cyber background (`#06090e`, `rgba(0,0,0,0.4)`) with cyber teal (`#2dd4bf`) accents maintained.
* **Verification & Testing:**
  * 50 Vitest tests passing (100%, added `Typography.test.ts`), 139 Cargo unit tests passing (100%), TypeScript / Vite production build passing cleanly.

---

## 9. Phase 9 (Remaining Core Features & Source-Only Distribution): Completed

### 9.1 User Decisions (Decision Gate)

1. **Distribution Policy:** For initial public release, avoid pre-built binary packages (deb, AppImage, Flatpak, etc.) to minimize licensing and compliance complexities. **Publish strictly as open source code accompanied by exhaustive build guides and prerequisite instructions (README / BUILD)**.
2. **Editor-Integrated Rename (vidir style) Specifications:**
   * **Interface:** Dual-mode support featuring both an embedded multi-line in-app editor and external editor spawning (`$VISUAL`, `$EDITOR`, VS Code, nano, gedit, etc.).
   * **Format:** Clean tab-separated values format (`[item_id]<TAB>[Filename]`) adhering to Design 9.4.
   * **Safety Guarantees:** Correlated strictly via `item_id`. Deleted lines are preserved as unchanged (NEVER interpreted as file deletion). Two-Phase atomic rename and rollback journal (UNDO) fully enforced.

### 9.2 Implementation Details (Phase 9.1: vidir-Style Editor Integration): Completed

* **Backend Engine (`src-tauri/src/batch.rs`, `src-tauri/src/lib.rs`):**
  * `generate_vidir_buffer`: Generates tab-separated text representation from target items with safety guidance headers.
  * `parse_vidir_buffer`: Parses user-edited text buffer, guarantees that omitted lines remain `OperationStatus::Unchanged`, and detects duplicate collisions (`ConflictDuplicate`), on-disk collisions (`ConflictExistsOnDisk`), and illegal paths (`InvalidFilename`).
  * `launch_external_editor_vidir`: Spawns external editor with temporary TSV file (`~/.cache/Cuddle/vidir_<uuid>.tsv`) via direct argument arrays without shell expansion, waits for editor completion, auto-parses output, and securely deletes the temporary buffer.
  * Exposed Tauri IPC commands: `generate_vidir_text`, `parse_vidir_text`, `launch_external_editor_vidir`.
  * Added 2 Rust unit tests (`test_vidir_buffer_generation_and_parsing`, `test_vidir_collision_detection`), passing 100%.
* **Frontend UI Integration (`BulkRenameModal.tsx`, `App.css`):**
  * Added clean top toggle tab bar: "Rule Builder (Regex, Seq, DateTime)" vs "Editor Integration (vidir)".
  * **Embedded vidir Editor:** Dedicated monospace text area with instant Dry-Run collision re-computation on edit.
  * **External Editor Controls:** "Open in External Editor" action button with launching spinner, custom editor command override input, and reload buffer button.
  * **Safety Badge:** Highlighting "Deleted lines are preserved as unchanged (files are NEVER deleted)".
  * Seamlessly synchronized with summary counters, preview diff table, and fixed-bottom execution button.
* **Bilingual Synchronization (`ja/translation.json`, `en/translation.json`):**
  * Full parity across Japanese and English locales for all `bulk_rename.tab_*` and `bulk_rename.vidir_*` keys.
* **Verification & Testing:**
  * 51 Vitest unit tests passing (100%), 141 Cargo unit tests passing (100%), TypeScript / Vite production build passing cleanly.

### 9.3 Implementation Details (Phase 9.2: Batch Attribute Change chmod - Octal & Symbolic): Completed

* **Backend Engine (`src-tauri/src/chmod.rs`, `src-tauri/src/lib.rs`):**
  * `parse_octal_mode`: Strictly parses 3-digit and 4-digit octal permission strings (e.g., `644`, `0755`, `4755`).
  * `parse_symbolic_mode`: Parses POSIX chmod compatible comma-separated clauses (`u+x`, `go-w`, `a+r`, `u=rwx,go=rx`) as well as shorthand forms (`+x`, `-w`).
  * `compute_target_mode`: Applies operators (`+`, `-`, `=`) against existing `st_mode` to calculate target permissions.
  * `has_special_permission_bits`: Detects setuid (`04000`), setgid (`02000`), and sticky bit (`01000`) and triggers safety alerts.
  * `generate_chmod_plan`: Produces complete Dry-Run preview plan including before/after octal & symbolic strings, status flags, special bit detection, and warnings.
  * `execute_chmod_plan`: Safely applies permissions to target files with reverse-order rollback on failure to maintain atomicity; commits result to SQLite WAL operation history.
  * Exposed Tauri IPC commands: `generate_chmod_plan`, `execute_chmod_plan`.
  * Added 10 Rust unit tests; all 151 Cargo unit tests passing 100%.
* **Frontend UI Integration (`ChmodModal.tsx`, `App.css`, Panel components):**
  * **Modal Dialog:** Tab switcher between Octal mode (`tab_octal`) and Symbolic mode (`tab_symbolic`).
  * **Quick Presets:**
    * Octal: `644`, `755`, `600`, `700`, `664`, `775`
    * Symbolic: `+x`, `-x`, `u+x`, `go-w`, `a+r`
  * **Interactive Permission Matrix:** 3x3 checkbox grid (Owner/Group/Others × Read/Write/Execute) bidirectionally synced with octal value.
  * **Special Bits Warning Banner:** Prominent amber banner when setuid/setgid/sticky bits are detected.
  * **Dry-Run Comparison Table:** Original mode (0644 / -rw-r--r--), target mode (0755 / -rwxr-xr-x), status badge, and item path.
  * **Panel & Context Menu Integration:**
    * `WorkspacePanel.tsx`: Added "Change Attributes (chmod)" action button in header; context menu integration.
    * `FileBrowserPanel.tsx`: Added context menu action to trigger attribute change for individual/multiple files.
* **Bilingual Synchronization (`ja/translation.json`, `en/translation.json`):**
  * Added all `chmod.*` keys with full English and Japanese parity.

### 9.4 Implementation Details (Phase 9.3: Batch File Timestamps & Workspace tar/tar.gz Export): Completed

* **Backend Engine (`src-tauri/src/timestamp.rs`, `src-tauri/src/export_tar.rs`, `src-tauri/src/lib.rs`):**
  * **Batch Timestamp Change (`timestamp.rs`):**
    * High-precision file timestamp management via `filetime = "0.2"` and `kamadak-exif = "0.6"`.
    * **4 Modification Modes (`TimestampModeSpec`):**
      1. Current System Time (`CurrentTime` / touch)
      2. Absolute Datetime (`Absolute` / supports `YYYY-MM-DD HH:mm:ss` and major ISO formats)
      3. Relative Time Shift (`RelativeOffset` / offset in seconds, positive for future, negative for past)
      4. EXIF Shooting Datetime Sync (`ExifSync` / extracts `DateTimeOriginal` from JPEG/TIFF Exif metadata and updates mtime)
    * `generate_timestamp_plan`: Computes a comprehensive Dry-Run verification plan (`TimestampPlanDto`) including current mtime, planned mtime, change status, and warnings.
    * `execute_timestamp_plan`: Safely updates modification timestamps (mtime) with automatic reverse-order rollback on failure to maintain batch atomicity; logs operation to SQLite WAL history (`OperationType::Timestamp`).
    * Exposed Tauri IPC commands: `generate_timestamp_plan`, `execute_timestamp_plan`.
  * **Workspace tar / tar.gz Export (`export_tar.rs`):**
    * Exports working files from `workspace-temp/` into standalone archives without touching originals.
    * **2 Archive Formats (`TarExportFormat`):** Gzip-compressed tar (`TarGz`: `.tar.gz` / recommended) and uncompressed POSIX tar (`Tar`: `.tar`).
    * **3 Export Scopes (`TarExportScope`):**
      1. Modified files only (`ModifiedOnly`: default recommended)
      2. All staged workspace files (`AllStaged`)
      3. Selected files only (`SelectedOnly`)
    * Automatically creates parent directories, calculates uncompressed vs archive byte sizes, and records archive creation to SQLite WAL history (`OperationType::TarExport`).
    * Exposed Tauri IPC command: `export_workspace_tar`.
  * TypeScript types auto-generated via `ts-rs` (`TimestampModeSpec.ts`, `PlannedTimestampItemDto.ts`, `TimestampPlanDto.ts`, `TimestampExecutionResultDto.ts`, `TarExportFormat.ts`, `TarExportScope.ts`, `TarExportOptionsDto.ts`, `TarExportResultDto.ts`).
  * Added 16 Rust unit tests; all 167 Cargo unit tests passing 100%.
* **Frontend UI Integration (`TimestampModal.tsx`, `TarExportModal.tsx`, `App.css`, Panels):**
  * **Timestamp Modal (`TimestampModal.tsx`):**
    * 4-mode tab switcher (Touch / Absolute / Relative Shift / EXIF Sync).
    * Quick shift preset buttons (+1h, -1h, +30m, -30m, +1d, -1d).
    * Dry-run preview table (item name, original timestamp, arrow, target timestamp, status badge).
    * Fixed-bottom action bar, rollback notice, and feedback banners.
  * **Tar Export Modal (`TarExportModal.tsx`):**
    * Scope selection cards (Modified only / All staged / Selected only, with live count badges).
    * Format selection cards (.tar.gz / .tar, automatically updates destination filename extension).
    * Destination path input with reset generator and parent directory auto-create note.
    * Post-export summary banner with compressed size and uncompressed size statistics.
  * **Unified Aesthetics (Zero White Backgrounds):**
    * Midnight Teal / Cyber Dark theme integration across all dialog cards, inputs, and tables.
  * **Panel Integrations:**
    * `WorkspacePanel.tsx`: Added "Change Timestamps (touch)" (`Clock` icon) and "Export tar" (`FolderArchive` icon) toolbar buttons.
    * `VirtualFileList.tsx`: Added "Change Timestamps (touch)" to context menu.
    * `FileBrowserPanel.tsx`: Context menu support for touch/timestamp modification.
* **Bilingual Synchronization (`ja/translation.json`, `en/translation.json`):**
  * Full parity across Japanese and English locales for all `timestamp.*` and `tar_export.*` keys.
* **Verification & Testing:**
  * 65 Vitest unit tests passing (100%, added `TimestampModal.test.tsx` and `TarExportModal.test.tsx`), 167 Cargo unit tests passing (100%), TypeScript and Vite production builds passing cleanly.

### 9.6 Implementation Details (Phase 9.5: Original Reflection Modes & Selective Undo): Completed

* **Original Reflection Modes (Design 5.2 Independent Explicit Operations):**
  * **2 Reflection Modes (`workspace::ReflectionMode`):**
    * **Mode A (`Overwrite`):** Creates a staging temp file (`.cuddle_reflect_<uuid>.tmp`) in the original file's parent directory, verifies copy integrity, and performs atomic replacement using POSIX `fs::rename(2)`.
    * **Mode B (`BackupOrigin` / Recommended):** Safely renames the original file to a timestamped `.origin` file (e.g. `report_YYYYMMDD_HHMMSS.origin.pdf`, with collision-resistant sequence suffixes) before placing the deliverable. Automatically rolls back from the backup if deliverable placement fails, physically preventing original data loss.
  * **Real-time Backup Path Preview:**
    * Backend IPC `get_origin_backup_preview` computes and returns the planned backup path on the fly.
  * **Modal UI Integration (`BottomTabPanel.tsx`, `App.css`):**
    * Radio card selection UI for Mode A vs Mode B styled in Midnight Cyber Dark aesthetic.
    * When Mode B is selected, displays the planned backup path preview in vibrant neon teal.
    * Returns `ReflectExecutionResultDto` (updated `StagedItemDto`, `backup_path`, and executed `mode`), updating toast notifications and activity logs.
* **Selective Undo (Design 6.2):**
  * **Tight SQLite WAL Operation History Integration (`history.rs`, `batch.rs`, `lib.rs`):**
    * Implemented integrated IPC command `undo_history_operation(operation_id)` to pinpoint and revert specific operations with `can_undo == true`.
    * **Dispatched Inversion for 3 Core Operations:**
      1. **Bulk Rename (`BulkRename`):** `batch::get_journal_manager().undo_batch_by_id(&operation_id)` inverts and renames the specific batch back to original display names and disk paths.
      2. **Permissions (`chmod`):** Parses recorded prior octal mode from `details` and restores exact permissions using `fs::set_permissions`.
      3. **Timestamps (`touch_mtime`):** Parses recorded prior epoch millisecond timestamp (`[orig_ms:...]`) from `details` and restores mtime with high precision using `filetime::set_file_mtime`.
    * Updates SQLite WAL record to `status = 'Undone'`, `can_undo = false`, and inserts an audit undo entry (`UndoBulkRename`, `UndoChmod`, `UndoTouchMtime`).
  * **UI Integration ("Operation History" Tab):**
    * Added action column (`.th-actions`) to the history table.
    * Displays "Revert (UNDO)" (`RotateCcw` icon in teal) on rows where `can_undo == true`.
    * Includes confirmation dialog with target filename and async execution with loading spinner.
    * Automatically refreshes file browser and history records upon completion.
* **Bilingual Synchronization (`ja/translation.json`, `en/translation.json`):**
  * Full parity across Japanese and English locales for all `reflection.mode_*`, `reflection.backup_preview_label`, `history.col_actions`, `history.undo_*`, and `history.type_undo_*` keys.
* **Verification & Testing:**
  * 67 Vitest unit tests passing (100%, added reflection Mode A/B switching and selective undo tests).
  * 174 Cargo unit tests passing (100%, added `test_workspace_manager_reflect_mode_b_backup_origin`, `test_selective_undo_by_batch_id`, `test_undo_history_chmod_and_touch`).
  * TypeScript type check and Vite production bundle passing cleanly.

### 9.7 Implementation Details (Phase 9.6: Desktop Launcher Automation & Multi-DE Installers/Uninstaller): Completed

* **Multi-Desktop Environment (DE) Installation Scripts (`scripts/`):**
  * **CachyOS + COSMIC DE (Wayland) Dedicated Preset (`scripts/install-cosmic.sh`):**
    * Tailored for CachyOS / Arch Linux and the System76 COSMIC Desktop Environment (`cosmic-comp`, `cosmic-app-library`, `cosmic-launcher`).
    * Configures Wayland `app_id` (`com.wammed.cuddle`), `StartupWMClass=com.wammed.cuddle`, and `X-CosmicAppId=com.wammed.cuddle` to ensure flawless COSMIC App Library recognition, search indexing, and Dock pinning.
  * **GNOME Dedicated Preset (`scripts/install-gnome.sh`):**
    * Tailored for GNOME 40-47 (Wayland/X11) with window grouping via `StartupWMClass` and GNOME Shell search cache refreshing.
  * **KDE Plasma Dedicated Preset (`scripts/install-kde.sh`):**
    * Tailored for KDE Plasma 5/6, running `kbuildsycoca6` / `kbuildsycoca5` to rebuild system configuration and Kickoff / KRunner menu caches.
  * **Universal Auto-Detect Installer (`scripts/install.sh`):**
    * Automatically analyzes `$XDG_CURRENT_DESKTOP`, `$XDG_SESSION_TYPE`, and binary paths.
    * Supports both `--user` (default: `~/.local/`, rootless) and `--system` (`/usr/local/`, requires sudo).
    * Deploys icons across all standard resolutions (32x32, 64x64, 128x128, 512x512 PNG) and scalable vector SVG (`images/Cuddle-Icon.svg`) to the `hicolor` tree and `pixmaps`.
    * Generates FreeDesktop-compliant desktop entries (`com.wammed.cuddle.desktop` and symlink `cuddle.desktop`), verified cleanly via `desktop-file-validate`.
    * Invokes `update-desktop-database` and `gtk-update-icon-cache` for instant desktop launcher updates.
  * **Comprehensive Uninstaller (`scripts/uninstall.sh`):**
    * Cleanly purges deployed binaries, symlinks, `.desktop` files, multi-resolution icons, and pixmaps, followed by cache rebuilds.
* **Documentation Synchronization:**
  * `BUILD.ja.md` / `BUILD.md`: Added Section 8 "Desktop Launcher & System Integration (.desktop & Icons)".
  * `README.ja.md` / `README.md`: Added one-liner installation commands to the Quick Start section.

### 9.8 Implementation Details (Full-text Search Index On-Demand UI & Section 7.3 Compliant systemd Periodic Timer): Completed

* **Full-text Search Index On-Demand UI (`SearchPanel.tsx`, `App.css`, Panel Components):**
  * Added dedicated index management dropdown (`Database` icon) directly adjacent to the search input field.
  * **Real-time Statistics & Scope Indicators:**
    * Displays active document count, index status badge (Ready), and current target directory indicator (current folder path or global).
  * **4 Explicit Index Operation Modes (`IndexOperationMode`):**
    1. **Incremental Update (`IncrementalUpdate` / Recommended):** Compares files against recorded index modification times (`mtime`) and extracts only newly added or modified files. Unchanged files are skipped at high speed.
    2. **Index Directory (`IndexDirectory`):** Recursively traverses and indexes text, source code, and documents within the currently opened directory.
    3. **Full Rebuild (`FullRebuild`):** Clears existing index documents and completely rescans and rebuilds full-text search indexes from scratch.
    4. **Reset / Clear (`ResetClear`):** Safely deletes all documents from the Tantivy full-text index (resets to initial state).
  * **Dynamic Feedback & Completion Summary Banner:**
    * Shows animated spinner (`Loader2`) and disables dropdown buttons during execution to prevent concurrent operations.
    * Renders an immediate completion summary banner showing processed count, new/updated count, skipped count, error count, and elapsed milliseconds.
  * **Cyber Dark Styling Parity:** Eliminates all pure white backgrounds, adhering strictly to Dark Velvet Slate styling with color-coded iconography (neon green, cyan, amber, rose).
* **Backend Index Operation Engine (`src-tauri/src/extractor.rs`, `src-tauri/src/search_fulltext.rs`):**
  * `search_fulltext.rs`:
    * `clear_all_documents`: Atomically deletes all index documents and commits changes.
    * `get_total_documents`: Rapidly queries active document count.
    * `get_indexed_mtime`: Tantivy 0.26 query (`item_id` / `raw_base64`) retrieving indexed `mtime` for incremental validation.
    * `INIT_MUTEX` double-checked locking around `get_or_init_engine()` eliminates concurrent test race conditions on Tantivy index file locks.
  * `extractor.rs`:
    * `run_index_operation`: Unified worker executing `IncrementalUpdate`, `IndexDirectory`, `FullRebuild`, and `ResetClear`.
    * Handles skipped files, PDF isolated workers (Landlock LSM + 256 MiB memory ceiling), and fault-tolerant continuous batch processing.
* **Section 7.3 Architecture: Offline Periodic Indexing (`src-tauri/src/periodic_index.rs`):**
  * **Kernel Exclusive Lock Control (`libc::flock`):**
    * Uses `flock(LOCK_EX | LOCK_NB)` on `$XDG_DATA_HOME/Cuddle/cuddle.lock`.
    * Cuddle GUI holds `GUI_LOCK` for its entire runtime.
    * Headless periodic updater (`--index-update`) probes the lock on startup and exits immediately with code 0 if GUI is running (physically preventing Tantivy index corruption and lock contention).
  * **Headless CLI Subcommands:**
    * `cuddle --index-update [--quiet]`: Executes headless incremental full-text index updating.
    * `cuddle --install-systemd-timer`: Generates, writes, and enables user systemd units (`cuddle-index.service` and `cuddle-index.timer`).
    * `cuddle --uninstall-systemd-timer`: Stops, disables, and deletes systemd user timer units.
    * `cuddle --systemd-timer-status`: Outputs systemd user timer status, active state, and next scheduled trigger.
  * **systemd User Units (`systemd/cuddle-index.service`, `systemd/cuddle-index.timer`):**
    * Configured with `Nice=19` and `IOSchedulingClass=idle` to ensure zero background impact on interactive desktop workflows.
    * `OnCalendar=*-*-* 02:30:00` with `Persistent=true` ensures missed runs during laptop suspend/sleep are caught up cleanly.
    * Integrated into installers (`scripts/install.sh`, `scripts/install-kde.sh`) and cleanly removed in `scripts/uninstall.sh`.
* **Tauri IPC Commands & TypeScript DTO Synchronization:**
  * Exposed and registered `get_index_stats`, `run_index_operation`, `get_systemd_timer_status`, `install_systemd_timer`, `uninstall_systemd_timer`.
  * Auto-generated TypeScript types via `ts-rs` (`IndexStatsDto.ts`, `RunIndexOptions.ts`, `IndexOperationMode.ts`, `IndexOperationResultDto.ts`, `SystemdTimerStatusDto.ts`).
* **Bilingual Synchronization (`ja/translation.json`, `en/translation.json`):**
  * 100% parity across Japanese and English locales for all `search.index_*` keys.
* **Verification & Testing:**
  * 71 Vitest unit tests passing (100%, added `SearchPanel.test.tsx`).
  * 184 Cargo unit tests passing (100%, added `test_run_index_operations_workflow`, `test_app_lock_exclusive`, etc.).
  * Zero Cargo clippy warnings, TypeScript type checks and Vite production builds passing cleanly.

---

## 10. Development Guardrails

* **Root Execution Prohibited:** The application must strictly execute with non-privileged user rights (no `sudo` / `pkexec` privilege escalation).
* **Original File Protection:** In-place modifications to original files are categorically forbidden. Operations apply strictly to `workspace-temp/` copies and are reflected only upon explicit confirmation.
* **Safety Warnings Cannot Be Disabled:** Critical warnings regarding destructive actions, permissions, or conflicts must not be configurable or suppressible by the user.
* **No Arbitrary Shell Strings:** Processes are spawned strictly via argument arrays under Landlock; `sh -c` and `bash -c` are forbidden.
* **No Monolingual Hardcoding:** Never hardcode English or Japanese strings directly in UI components; always retrieve via `t('key')`.

---

## 11. Next Implementation Tasks & Backlog (Phases 10–14)

Based on comprehensive cross-verification between the Master Design Specification ([`FileWorkbench-Design-ver0.9.en.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Design-ver0.9.en.md)) and the current codebase, the 18 identified items have been triaged and detailed in [`NEXT_IMPLEMENTATION_TASKS.en.md`](file:///home/susie/git/wammed/Cuddle/NEXT_IMPLEMENTATION_TASKS.en.md).

### Phase Structure & Recommended Implementation Flow

1. **Phase 10: Quick UX & Interaction Wins & Ultra-Rich Visual Polish [Completed & Verified by User]**
   * [2-2] Pane Width & Layout Persistence across Sessions (`react-resizable-panels` / `localStorage`): **Done & Verified**
   * [2-1] Drag & Drop (D&D) between Browser & Workspace (D&D staging): **Done & Verified**
   * [2-4] Pipeline Log Display Filter (`all` / `hidden` / `file_only`): **Done & Verified**
   * [2-5] Rich Extension Icons (Rust, TS, Python, Media specific icons & colors): **Done & Verified**
   * [2-6] Inner Rim Light & 3-Layer Depth Shadows: **Done & Verified**
   * [2-7] Spotlight Cursor Tracking & Physical Spring Press: **Done & Verified**
   * [2-8] Storage Safety Gauge, Inline Heat-Bar & Permission Chips: **Done & Verified**
   * [2-9] Tabular Figures & Interactive Breadcrumb Trail: **Done & Verified**
   * [2-10] Ambient Aurora Mesh Background & Subtle Noise Grain (Theme Reactive & High Vibrancy): **Done & Verified**
2. **Phase 11: Safety, Conflict Resolution & Recovery Refinements [Implemented & Verified by Manual Testing]**
   * [3-2] Alternate Filename UI on Original Conflict (e.g., `.edit.pdf`) & non-destructive save: **Done & Verified via Manual Testing**
     * Implemented backend `reflect_staged_item_as_copy`, `get_alternate_copy_preview`, collision-safe numeric suffixes (`.edit.1.ext`).
     * Added conflict-triggered alternate copy save box to reflection dialog in `BottomTabPanel`.
   * [3-3] Backup Retention Management (default 5, max 10) & Prune UI: **Done & Verified via Manual Testing**
     * Added `backup_retention_count` to `config.rs` and `list_backups`, `delete_backup`, `prune_old_backups`, `restore_workspace_backup` to `backup.rs`.
     * Added retention indicator, auto-prune toggle, and archive list with delete buttons to `ExitBackupModal` and `SettingsModal`.
     * Added "Resume from Backup" switch and archive selection dialog to Start Dashboard (`StartPage.tsx`).
   * [4-3] Warning / Consent Modal on Non-Landlock Kernels: **Done & Verified via Manual Testing**
     * Created `CriticalSecurityModal` blocking application use until consent is provided on unsupported kernels, with choice persistence.
   * [Window Close Safety Intercept & Header Exit Button]: **Done & Verified via Manual Testing**
     * Implemented `onCloseRequested` interception in Tauri 2 for Zero Data Loss and prominent Header exit button.
3. **Phase 12: Pipeline Flexibility & External Tool Integrations [Done & Verified via Manual Testing]**
   * [4-2] Custom `allowed_read_paths` in Pipeline Presets (Landlock integration): **Done & Verified via Manual Testing**
     * Added `allowed_read_paths: Vec<String>` to `CustomPipelineDefDto`. Dynamically registered to `LandlockSandbox::allow_read` during dry-run and execution.
     * Added allowed read paths input and Landlock Read badges to `SettingsModal`, as well as Risk Specification display in `PipelineModal`.
   * [4-1] Input Modes (`batch`, `stdin`) & Typed Placeholders (`{InputList}`, `{WorkspaceDirectory}`): **Done & Verified via Manual Testing**
     * Introduced `ExecutionInputMode` (`PerFile`, `BatchList`, `StandardInput`).
     * Typed placeholder expansion for `{input}`, `{output}`, `{InputList}`, `{WorkspaceDirectory}`.
     * Child process stdin streaming via `SandboxedExecutor`.
   * [2-3] Desktop Environment Native Notification Abstraction: **Done & Verified via Manual Testing**
     * Native notification dispatch via `notify.rs` (`notify_desktop`) conforming to FreeDesktop Desktop Notifications Spec.
     * Automated pipeline completion alerts, `notify_user` command, and frontend `notifyUser` utility.
   * [Refinement] Dynamic Nerd Font Statusbar Detection & Active Indicator: **Done & Verified via Manual Testing**
     * Added `isNerdFontConfigured`, `detectSystemNerdFont`, `getNerdFontStatus` in `src/utils/typography.ts`.
     * Real-time UI updates via `cuddle:typography-changed` custom event, 3-state display (`Active` / `Detected` / `Recommended`), and quick shortcut to SettingsModal.
     * Implemented typed placeholder expansion (`{input}`, `{output}`, `{InputList}` safe argument splatting, `{WorkspaceDirectory}`).
     * Extended `SandboxedExecutor` with `stdin_input: Option<String>` for streaming file paths to child stdin with EOF closing.
     * Added input mode select and quick-insert placeholder pills in `SettingsModal`.
   * [2-3] Desktop Environment Native Notification Abstraction (FreeDesktop D-Bus): **Done & Verified**
     * Added `notify.rs` module providing FreeDesktop notification dispatch via `notify-send` (`notify_desktop` & Tauri command `notify_user`).
     * Automatically dispatches notifications upon preset and custom pipeline completion. Frontend `notifyUser` helper implemented.
4. **Phase 13: Search, Extraction & Indexing Evolution [Done & Approved via Manual Testing]**
   * [1-4] Structured Markdown (`pulldown-cmark`) / HTML Text Extraction: **Done & Verified**
     * Parsed Markdown via `pulldown-cmark` (v0.12) stripping formatting tokens. Stripped HTML tags, `<script>`, `<style>` blocks, and restored HTML entities into clean prose text.
   * [1-3] Tier 3: Office Document Extraction (`.docx`, `.xlsx`): **Done & Verified**
     * Built zero-dependency in-memory safe ZIP reader (`read_zip_file_entry`) parsing EOCD and Central Directory records with `flate2::DeflateDecoder`.
     * Parsed `word/document.xml` and `xl/sharedStrings.xml` via `quick-xml` (v0.41).
     * Enforced strict ZipBomb guardrails (100:1 ratio limit, 10MB entry size limit, 256MB memory cap).
   * [1-2] Combined Morphological (IPADIC) + n-gram Search: **Done & Verified**
     * Added `body_ngram` field using Tantivy's built-in `NgramTokenizer::all_ngrams(2, 3)`.
     * Boosted multi-field scoring (`display_name: 3.0`, `body: 2.0`, `body_ngram: 0.8`), prioritizing exact morphological matches while guaranteeing full recall for OOV words and substrings.
   * [1-1] Index Update / Repair Queue for Discrepancies: **Done & Verified**
     * Created SQLite WAL `index_repair_queue` table with retry tracking and auto-drain. Exposed Tauri commands `process_index_repair_queue` and `get_index_repair_queue_count`.
     * Added pending count badge and "Process Index Repair Queue" action to `SearchPanel` dropdown with bilingual notification banners.
   * [1-5] Per-Rule Indexing Policy (Metadata-only / Filename-only): **Done & Verified**
     * Supported `IndexingPolicy::MetadataOnly` for excluded directories (`.git`, `node_modules`), indexing file name, mtime, and size while keeping body empty.
5. **Phase 14: Architectural Decisions & Scale Verification [Done & Verified]**
   * [3-1] Safety Confirmation Levels (Standard / Strict, Brief omitted): **Done & Verified**
     * Implemented 2-tier model in `src-tauri/src/config.rs` (`ConfirmationLevel` / `SecuritySection`), `config.template.toml`, and `SettingsModal` (General Tab).
   * [3-4] Endorsement of SQLite WAL + Journal State Model: **Done & Verified**
     * Endorsed SQLite WAL `session_items` / `operation_history` / `index_repair_queue` and `BatchJournalManager` as superior in reliability and simplicity with zero data loss on unexpected exits.
   * [5-1] Alignment between Settings Menu & config.toml + Template Update: **Done & Verified**
     * Established 4-section architecture (`[app]`, `[workspace]`, `[security]`, `[[custom_pipelines]]`). SettingsModal changes (language, theme, density, confirmation level, backup retention, custom pipelines) now immediately persist to `config.toml`.
     * Fully updated `config.template.toml` and in-app template generator with complete section definitions and typed pipeline examples.
   * [6-1] Validated Batch IPC + Virtual Scrolling Architecture: **Done & Verified**
     * Validated 10,000+ item directory transfer benchmarks under Tauri 2 IPC and `@tanstack/react-virtual`, maintaining 60fps scrolling and avoiding unnecessary pagination API state complexity.

All planned phases (Phase 1–14) are fully implemented and verified. For architectural specifications and task mappings, refer to [`NEXT_IMPLEMENTATION_TASKS.en.md`](file:///home/susie/git/wammed/Cuddle/NEXT_IMPLEMENTATION_TASKS.en.md).
