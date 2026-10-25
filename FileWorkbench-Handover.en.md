# FileWorkbench (Cuddle) Development Handover Document

<p align="center">
  <strong>English</strong> | <a href="FileWorkbench-Handover.ja.md">日本語</a>
</p>

**Document Date:** 2026-10-25  
**Status:** Phase 8 (UI/UX Polish, Midnight Teal Theme, Fixed Action Bars) Completed, Ready for Phase 9  
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
* **Verification & Testing:**
  * 45 Vitest tests passing (100%), 139 Cargo unit tests passing (100%), TypeScript / Vite production build passing cleanly.

---

## 7. Development Guardrails

* **Root Execution Prohibited:** The application must strictly execute with non-privileged user rights (no `sudo` / `pkexec` privilege escalation).
* **Original File Protection:** In-place modifications to original files are categorically forbidden. Operations apply strictly to `workspace-temp/` copies and are reflected only upon explicit confirmation.
* **Safety Warnings Cannot Be Disabled:** Critical warnings regarding destructive actions, permissions, or conflicts must not be configurable or suppressible by the user.
* **No Arbitrary Shell Strings:** Processes are spawned strictly via argument arrays under Landlock; `sh -c` and `bash -c` are forbidden.
* **No Monolingual Hardcoding:** Never hardcode English or Japanese strings directly in UI components; always retrieve via `t('key')`.
