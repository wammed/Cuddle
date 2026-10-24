# Linux File Management & Search Application: Integrated Basic Design Document (Definitive Edition)

<p align="center">
  <strong>English</strong> | <a href="FileWorkbench-Design-ver0.9.ja.md">日本語</a>
</p>

**Document Version:** 0.9 (Open Issues Resolved & Bilingual Specifications Definitive Edition)  
**Revision Date:** 2026-10-24  
**Target Audience:** Planning, Architecture, Implementation, and QA Teams  
**Document Purpose:** This document integrates all decisions from v0.5 through v0.8 and establishes definitive specifications addressing remaining pre-implementation technical challenges (Landlock minimum allowed paths, Linux non-UTF-8 binary filename & IPC boundary specifications, `workspace-temp` same-volume priority placement with pre-capacity checks, text extraction & SQLite synchronization policies, virtual scrolling for UI performance) as well as mandatory policies for "100% bilingual (EN/JA) documentation" and "fully switchable bilingual UI throughout the entire application."

---

## 1. Project Overview

### 1.1 Purpose

To develop a high-reliability file management and search workbench for Linux with its own unique visual branding. Moving far beyond a conventional file browser, it provides an integrated, highly dependable working environment featuring:

* High-speed filename search, full-text search, directory tree, and file browser
* A working-copy architecture that strictly protects original files
* Safety-first file operations and phased changes reflected back to originals
* Backups, comprehensive operation history, UNDO, and session recovery
* Index differential update queues and repair mechanisms
* A safe pipeline runner for invoking external processes (isolated via Linux Landlock LSM)
* Bulk rename, external editor-integrated rename (vidir-style), Dry-run validation, and bulk attribute modifications
* Configurable information density, display layouts, and explicit-action rich preview
* Complete English and Japanese bilingual support across all documentation and in-app user interface elements

### 1.2 Development & Validation Environment

* **Initial Validation Environment:** CachyOS + COSMIC DE (Target for official physical hardware verification and support guarantee in initial release; validation constraints documented in README).
* **Future Target Environments:** GNOME, KDE Plasma, etc. (Desktop-specific features abstracted; unverified environments not officially guaranteed in initial release).
* No desktop-environment-specific assumptions permitted inside the application's core logic.

### 1.3 Core Principles

1. Prioritize original file protection above all else.
2. Channel all file operations through unified safety verification rules.
3. Never execute the application with root privileges.
4. Never bypass OS access permissions; never perform privilege escalation.
5. Strictly decouple the success of file operations from index update success.
6. Never conflate the existence of operation history with actual UNDO availability.
7. Maintain loose coupling between UI, file operations, and search/indexing layers.
8. Prohibit user settings from disabling safety-critical warnings.
9. **Mandatory Bilingual Documentation:** All design documents, specifications, READMEs, and handover documents must be authored and maintained in both English and Japanese with identical content.
10. **Mandatory In-App Bilingual & Dynamic Switching:** All menus, warnings, messages, dialogs, and UI elements must support both English and Japanese, dynamically switchable via in-app settings without requiring an application restart.

---

## 2. Technical Stack & Architecture

### 2.1 Recommended Technologies & Approaches

| Domain | Selected Technology / Approach | Notes & Policy |
| :--- | :--- | :--- |
| **UI Framework** | Tauri 2 | Desktop integration and system security |
| **Core Processing** | Rust | Memory safety, asynchronous I/O, systems programming |
| **UI Implementation** | React + TypeScript + `@tanstack/react-virtual` | Advanced pane management, 60fps virtualized scrolling for up to 10,000 items |
| **Internationalization (i18n)** | `i18next` + `react-i18next` | Full English/Japanese bilingual support. Instant switching via settings without restart. Backend returns structured error codes and params |
| **Application Data Storage** | SQLite (WAL mode) | `PRAGMA synchronous = NORMAL` runtime; explicit checkpointing at critical boundaries |
| **Full-text Search Engine** | Tantivy + `lindera-tantivy` | Native Rust. Japanese morphological analysis (IPADIC) combined with n-gram |
| **Filename Search** | plocate / Direct Directory Traversal | Prioritize plocate; automatic fallback to direct scan when unavailable |
| **Configuration File** | TOML | Schema versioning and migration support (includes `[ui.language]`) |
| **Process Isolation (Sandbox)** | Linux Landlock (LSM) | Unprivileged access restrictions applied at process spawn. Per-preset `allowed_read_paths` declarations; `strace` profiling standard |
| **Text Extraction** | Phased Introduction (Text → Rich Text → Isolated Worker) | Memory-quota-restricted worker process to prevent OOM and main process crashes |
| **Backups** | Compressed tar archives | Contains session recovery metadata with automated integrity validation |
| **Periodic Updates** | systemd user timer | Primary option; background execution only when application is stopped |
| **Desktop Notifications** | OS/DE Notification Abstraction | Initial release verified on CachyOS + COSMIC DE; concurrent in-app notifications (all multilingual) |

### 2.2 Architectural Principles

The Rust core logic must never depend on UI implementation details.

* **UI Layer:** Rendering, user interactions, configuration changes, progress indicators, explicit preview, virtual scrolling, and i18n multilingual rendering.
* **Multilingual / i18n Layer:** UI strings managed centrally in resource catalogs (`locales/{en,ja}/*.json`). The Rust backend returns structured errors (`code` + `params`), which the frontend resolves into localized messages.
* **Application Layer:** Use-case orchestration, operation history tracking, process lifecycle management.
* **Safety Verification Layer:** `OperationPolicy` (constraint verification), `OperationExecutor` (validation, pre-execution re-validation, execution).
* **File Operations Layer:** Copy, move, rename, bulk actions, origin reflection, restoration.
* **Bulk Action Planning Layer:** `batch_planner`, `transaction_coordinator` (conflict checks, Dry-run, phased execution).
* **Search & Indexing Layer:** SQLite, Tantivy, plocate integration, text extraction workers.
* **Pipeline Layer:** Preset validation, Landlock-isolated process invocation, logging, artifact management.
* **Persistence Layer:** Configuration, operation history, recovery journals, index update queues.
* **IPC & Filename Boundary Decoupling:** To support arbitrary Linux byte sequence filenames (`OsString` / `[u8]`), the Rust core consistently holds `PathBuf`, while IPC/JSON boundaries utilize Base64/WTF-8 escaping alongside immutable `item_id` (UUID / inode+dev) dual-key mapping. High-volume transfers use streaming or pagination.

---

## 3. GUI & Screen Layout

### 3.1 Main Window 6 Panes

1. **Top-Left: Search Results** - Displays filename and full-text search results.
2. **Bottom-Left: Directory Tree** - Hierarchical folder navigation and destination selection.
3. **Center-Left: File Browser** - Item listing of the current directory.
4. **Center-Right: Workspace** - View, edit, and inspect diffs of active working copies.
5. **Right Sidebar: Selected Item Details & Actions** - Metadata display and contextual action triggers.
6. **Bottom Tab Panel:** - Rich preview, operation history, Git status, pipeline execution logs.

### 3.2 Layout Control & Interaction

* Pane widths (tree, browser, workspace) are freely adjustable. The right sidebar and bottom panel can be collapsed.
* Startup dimensions adapt to display metrics. Considering HiDPI, multi-monitor, and Wayland, pane layouts and dimensions are persisted and restored on subsequent launches.
* **Drag & Drop:** D&D is supported between Center-Left (Browser) and Center-Right (Workspace). Drop semantics (create working copy, move within workspace) are made explicit beforehand. **D&D will NEVER directly reflect changes to original files.**
* **Preview Behavior:** Previews launch strictly upon explicit user actions (such as right-click context menus). No automatic previews triggered by hover or left-click.
* **High-Volume Rendering:** File browser, workspace, and bulk action review dialogs employ `@tanstack/react-virtual` to maintain 60fps even with 10,000 items.
* **Dynamic Multilingual Switching:** One-click language switching (Japanese / English) available in the header and settings, immediately refreshing all active lists and dialogs without restart.

### 3.3 Information Density & Bottom Tabs

* Configurable information density, icons, labels, and confirmation dialog details. Critical safety warnings (permissions, overwrite conflicts, data loss) cannot be suppressed.
* **Bottom Tab Configuration:**
  * **Rich Preview:** Syntax highlighting for Markdown, HTML, source code.
  * **Operation History:** Operation results, UNDO availability, and reasons for unavailability (multilingual).
  * **Git Status:** Git repository status equivalent to `git status`.
  * **Pipeline Execution Logs:** Transient overlay panel (re-displayable from history).

---

## 4. Safety Architecture for File Operations

### 4.1 Common Rules & Validation Items

All file operations (UI, search results, pipelines, bulk actions, UNDO, origin reflection) must pass through unified safety rules (`OperationPolicy` / `OperationExecutor`).

* **Validation Items:** Path existence & identity, symlink resolution targets, home boundary verification, OS permissions, mount points / external storage, existing file collisions, destructive operation confirmations, original modification checks post-working copy, disk capacity verification (minimum 1.5x original size free space required), and TOCTOU mitigations (re-validation immediately prior to execution).
* In addition to path strings, file identity metadata (inode/dev) and directory file descriptors are used wherever feasible.

### 4.2 Links & Mount Point Constraints

* Symlink destinations are resolved and validated against home boundary constraints.
* Hard links must NEVER be used as substitutes for working copies (independent content must be guaranteed).
* **Forbidden Zones for Origin Reflection:** Origin paths residing on mount points, external storage devices, or outside `$HOME` are limited to copy, inspect, and edit operations; **reflecting modifications back to originals in these zones is categorically prohibited.**

### 4.3 Confirmation Levels

1. Concise / 2. Standard / 3. Detailed / 4. Strict (configurable per operation type). Settings only alter verbosity; mandatory verifications and destructive confirmations can never be bypassed.

---

## 5. Working Copies & Origin Reflection

### 5.1 Directory Logical Structure & Placement Policy

Edits, conversions, and bulk operations are performed exclusively against independent working copies, never directly against original files. Deliverables and recovery data are strictly segregated.

```text
FileWorkbench-data/ (Location: $HOME/.local/share/FileWorkbench/ or on the same volume as the original)
├── workspace-temp/  # All working copies, organized by batch subdirectories
├── artifacts/       # Validated successful deliverables (staging area)
└── recovery/        # Journals and snapshots required for UNDO and crash recovery
```

* **`workspace-temp/` Physical Placement Policy (Cross-device / EXDEV Mitigation & Reflink Maximization):**
  * **Same-Volume Priority Placement:** When a temporary directory can be allocated on the same filesystem/mount point as the original files, a volume-specific workspace (e.g., `<volume-root>/.fileworkbench-temp/`) is used by preference. This maximizes atomic `rename(2)` and CoW Reflink, preventing unnecessary copy latencies and `$HOME` disk exhaustion caused by cross-filesystem (EXDEV) operations.
  * **Fallback:** If write permissions or creation fail, fallback to standard `$HOME/.local/share/FileWorkbench/workspace-temp/` (applying multi-stage copy + verify + delete original workflow).
  * **Pre-Execution Capacity Check:** Total original size × 1.5 free capacity must be verified before working copies are created.

### 5.2 Origin Reflection Mechanism (Strictly Independent Operation)

Reflecting changes back to original files is decoupled from internal pipeline and bulk processing. It is **executed exclusively when explicitly invoked by the user as an independent operation after deliverables are finalized.**

* **Method A:** Overwrite original directly.
* **Method B:** Rename original with a timestamped `.origin` suffix (e.g., `report_2610240900.origin.pdf`), then place the artifact.
* Within the same filesystem, atomic `rename(2)` and temporary file replacement are utilized.
* Across filesystems, a multi-stage "copy + verify checksum + delete source" procedure is executed under crash-resilient journaling.

### 5.3 Conflict Detection & Reflink

* **Conflict Detection:** Records original SHA-256, size, and modification time (mtime) at working copy creation. If the original was externally modified, reflection halts with a warning. Forced overwrites are prohibited; alternative save paths (e.g., `.edit.pdf`) are offered.
* **Reflink (CoW):** Prioritized for large files (initial threshold 16 MiB, dynamically tunable). Provides instant cloning on Btrfs, XFS, etc. Falls back to standard copy when unsupported. Reflink alone does not constitute an independent backup.

---

## 6. Backups, Session Recovery & UNDO

### 6.1 Backups & Data Lifecycle

* **Session Start:** Record recovery initial-state manifest.
* **Session End:** Display interactive confirmation prompt allowing the user to create a session-end backup.
* Backup Creation: Saved as compressed tar archives and validated for integrity.
* Unfinished Operations: Even if the user declines a backup, recovery journals and data are preserved until operation integrity is achieved.
* **Storage Location & Retention:** `$HOME/.local/share/FileWorkbench/backups/` (default: 5 archives, max: 10).
* **Storage Shortage:** Automatic deletion is never performed; execution is cancelled or the user manually selects targets for deletion.

### 6.2 Operation History & UNDO

* Records operation type, target paths, identifiers, execution timestamp, result, UNDO availability/reason, and recovery data state.
* **Separation of History and UNDO:** The existence of a history record is never conflated with actual UNDO availability if underlying restoration data is absent.
* In disk shortage scenarios, older operation data is purged if verified backups exist; otherwise, recent UNDO data is prioritized.

### 6.3 Session Recovery

Working file states, session ID, recovery baseline, and operation history are mandatory for session restoration. UNDO payload data is optional. Pane layouts revert to defaults; active selections and search queries are not restored. Conflicts with live files are strictly re-verified upon recovery.

---

## 7. Search & Indexing

### 7.1 Filename Search & Full-text Search

* **Filename Search:** Prioritizes plocate; falls back to direct scanning when unavailable. plocate database updates rely on OS services; the application never requests root privileges.
* **Full-text Scope:** Text and supported document formats within `$HOME`. Binary files and scanned PDF documents requiring OCR index metadata only. Cache directories are excluded by default.
* **Exclusion Policy:** Target zones retain full-text index; excluded directories retain metadata only; outside `$HOME` retains filename information only.

### 7.2 Indexing Technology & Text Extraction

* **Metadata & State Management:** SQLite (WAL mode).
* **Full-text Engine:** Tantivy + `lindera-tantivy` (morphological IPADIC, `Mode::Normal`, `LowerCaser`). Morphological matching is prioritized with n-gram partial matching acting as auxiliary scoring.
* **Phased Text Extraction & Process Isolation:**
  * **Phase 1:** Plain text, source code, configuration files (direct Rust native extraction).
  * **Phase 2:** Markdown (`pulldown-cmark`), HTML (`html2text`) rich text.
  * **Phase 3:** Text-layer PDFs (`pdf-extract`), Office documents. Executed within an isolated worker process bounded by a memory quota (e.g., 256 MiB) to prevent main process crashes or OOM. OCR extracts metadata only without rasterizing images.
* **SQLite Synchronization & Update Integrity:**
  * Runtime standard: `PRAGMA synchronous = NORMAL;` (optimal balance of crash safety and I/O performance in WAL mode).
  * Explicit `PRAGMA wal_checkpoint(TRUNCATE);` executed at critical milestones (origin reflection commits, batch commits, checkpoint generation) to ensure on-disk persistence.
  * SQLite and Tantivy updates are synchronized via a persistent retry/repair queue. If indexing fails after a successful file operation, the file operation is marked successful and an indexing repair job is enqueued.

### 7.3 Periodic Indexing

Managed via systemd user timer, executing exclusively when the application is stopped. Mutual exclusion locks prevent concurrent runs. While the application is active, background sweeps are paused and only delta updates from user file operations are processed.

---

## 8. Pipeline Runner (Landlock-Isolated Batch Processing)

### 8.1 Purpose & Constraints

Executes predefined presets (image conversion, PDF merging, metadata extraction, etc.) using selected files as inputs.

* **Initial Release Constraints:** Single-step execution (one external command per preset). No multi-step chaining, conditional branching, or arbitrary shell execution (`sh -c` / `bash -c`).
* **Input Modes:** `per_file`, `batch`, `stdin`.
* **Argument Expansion:** Typed placeholders (`InputFile`, `OutputFile`, `InputList`, `OutputDirectory`, `WorkspaceDirectory`, `Literal`) expanded directly into structured argument arrays.

### 8.2 Landlock Lightweight Sandbox (Definitive Specification)

Because external binaries execute under user privileges, **Linux kernel Landlock LSM access rules are applied immediately before child process execution**.

* **Read / Write Permissions:**
  * Dedicated `workspace-temp/` subdirectory for the current job
  * Dedicated `artifacts/` staging area for the current job
  * Dedicated isolated temporary directory (`workspace-temp/proc-tmp-<pid>/` passed via environment variable `TMPDIR`)
* **Read-only Permissions (Standard Whitelist):**
  * Dynamic linker (`/lib64/ld-linux-x86-64.so.2`, etc.) and system shared libraries/binaries (`/usr`, `/lib`, `/lib64`, `/bin`, `/opt`)
  * Explicitly specified input file paths
  * Configuration and font paths (`/etc`, `/usr/share/fonts`, `$HOME/.cache/fontconfig`, `$HOME/.local/share/fonts`)
  * **Per-Preset Extension:** Binary-specific Read-only paths can be explicitly declared via `allowed_read_paths` in TOML presets.
* **Pre-profiling Procedure:** Commands adopted in presets are profiled using `strace -f -e trace=file <command>` during development to establish precise read whitelists.
* **Blocked Access:**
  * All other `$HOME` directories (both read and write denied).
  * Direct writes to original files are physically impossible under Landlock.
* **Unsupported Environments:** On kernels prior to Linux 5.13 or environments lacking Landlock, execution is blocked or requires explicit high-level user consent. CI environments use Linux 5.13+ runners.

### 8.3 Mandatory Safety Flow for User Pipelines

Executed strictly in the following sequence regardless of user skill setting:

```text
[Risk Explanation] → [1st Confirmation] → [Dry-Run] → [Results & Risk Review] → [2nd Confirmation] → [Execution]
```

1. **Risk Explanation:** Display binary name, resolved absolute path, argument list, and I/O paths.
2. **1st Confirmation:** User authorizes Dry-Run execution.
3. **Dry-Run:** Validates argument expansion, output collisions, and disk capacity without running the command.
4. **2nd Confirmation:** Presents predicted outputs and requires re-confirmation.
5. **Execution:** Runs binary with Landlock restrictions reading from `workspace-temp/` and writing to `artifacts/` staging.

### 8.4 Logs & Artifact Management

* Displayed in bottom panel via a transient overlay (`all` / `hidden` / `file_only`). UI presentation settings are decoupled from persistent log storage.
* Artifacts are registered as regular workspace files upon validation. Origin reflection is decoupled and executed through dedicated reflection workflows.

---

## 9. Bulk File Operations

### 9.1 Core Principles & Rollback Policy

Bulk rename, editor-integrated rename, and bulk attribute changes execute strictly against **working copies inside `workspace-temp/`**, never against originals.

* **RollbackAll Policy:** **If even a single item fails, the entire batch fails and all working copies and uncommitted artifacts are rolled back to their exact initial state.**
* Artifacts are not published until all items succeed. Reflecting changes to originals is strictly decoupled from bulk operations.
* Default limits: 10,000 items maximum, validation concurrency of 4 (configurable).

### 9.2 Recommended Rust Architecture & Data Types

```rust
pub struct BatchPlan {
    pub batch_id: BatchId,
    pub workspace_id: WorkspaceId,
    pub items: Vec<PlannedOperation>,
    pub failure_policy: FailurePolicy,   // Fixed to RollbackAll
    pub artifact_policy: ArtifactPolicy, // Fixed to PublishOnFullSuccess
}

pub struct PlannedOperation {
    pub item_id: ItemId,                // Immutable ID (UUID / inode+dev)
    pub original_path: PathBuf,          // Rust core retains raw bytes
    pub display_name: String,            // Lossless UTF-8 / WTF-8 representation
    pub raw_base64: String,              // Base64 string for non-UTF-8 safety
    pub target_name: Option<String>,
}

pub enum BatchStatus {
    Planned,
    Validated,
    Preparing,
    Running,
    Verifying,
    Committing,
    Succeeded,
    RollingBack,
    FailedRolledBack,   // Rollback verified and complete
    RecoveryRequired,   // Rollback failed; requires manual intervention
}
```

### 9.3 SQLite Journal Schema

* `batches`: Overall batch state, plan hash, failure policy, error summary.
* `batch_operations`: Individual operation sequence, type, input/output hashes, status.
* `recovery_checkpoints`: Recovery checkpoints, manifest hash, restoration status.
* `batch_artifacts`: Staging path, final path, publication status.

### 9.4 Bulk Action Specifications

* **Bulk Rename:** Regular expressions (Rust `regex`), sequential numbering, timestamps (`YYYYMMDD_HHmmss`), extension preservation. Name swaps and chained renames execute through temporary intermediate names to prevent collisions.
* **Editor-Integrated Rename (vidir-style) Binary Safety:**
  * Outputs structured JSON containing immutable `item_id`, `display_name`, and `raw_base64`.
  * Matches files by `item_id` upon saving; line deletions are treated as exclusions, NEVER as file deletions. Initial release limited to single directories.
* **Dry-run Validation:** Validates source/target paths, permissions, collisions, symlink targets, and capacity before execution.
* **Bulk Attribute Changes:** Octal (`644`, etc.) and symbolic (`u+x`, etc.) modes. Ownership changes limited to current user capabilities (privilege escalation prohibited). Special bits (setuid/setgid/sticky) trigger explicit warnings; recursive application disabled by default. ACLs and extended attributes excluded from initial release.
* **Frontend Performance (Virtual Scrolling & Streaming):**
  * Employs `@tanstack/react-virtual` to render up to 10,000 items, previews, and diffs at 60fps.
  * Tauri IPC transfers use chunked streaming or pagination to prevent UI thread starvation.

---

## 10. Crash Recovery, Exception Handling & Testing

### 10.1 Crash Recovery State Machine

Multi-stage changes require persistent journaling and physical file cross-verification:
`PREPARED → BACKUP_READY → APPLYING → VERIFYING → COMMITTED`

* Unfinished journals detected at startup are reconciled against live filesystem state. Inconsistencies halt recovery and prompt user intervention without automatic overwrites or deletions.
* Recovery payloads for incomplete operations are never purged due to expiration.

### 10.2 Bulk Operation Fault Injection Matrix (Mandatory Pass Matrix)

| ID | Test Scenario | Acceptance Criteria |
| :--- | :--- | :--- |
| **B-01** | All items succeed | All operations succeed; artifacts finalized and published |
| **B-02** | 1 item fails midway | Entire batch fails; working copies roll back to initial state |
| **B-03** | First item fails | Zero partial changes; status set to `FailedRolledBack` |
| **B-04** | Final item fails | All preceding operations safely undone |
| **B-05** | Checkpoint creation fails | Aborts before starting; working copies untouched |
| **B-06** | Disk capacity exhausted | Uncommitted artifacts purged; restoration to initial state |
| **B-07** | Re-crash during recovery | Recovery resumes safely on subsequent startup |
| **B-08** | SQLite write fails | Not marked success; journal mismatch detected |
| **B-09** | Crash during publication | Partial publication prevented on startup |
| **B-10** | Out-of-bounds write attempt | Blocked at OS level via Landlock |
| **B-11** | Input modified post-check | Pre-execution re-validation halts operation |
| **B-12** | Recovery backup corrupted | Marked `RecoveryRequired`; prompts manual intervention |
| **B-13** | Multiple recovery runs | Idempotent; completes safely without data destruction |
| **B-14** | Symlink / Hard link edge cases | Accidental destruction of original link target prevented |
| **B-15** | Cancellation during execution | Stops at safe boundary; rolls back to initial state |

---

## 11. Configuration Management (TOML)

* **Sections:** `[ui]`, `[confirmation]`, `[workspace]`, `[backup]`, `[search]`, `[index]`, `[index.schedule]`, `[undo]`, `[exclude]`, `[pipeline]`.
* **`[ui]` Section:**
  * `language = "auto"` (`"auto"`, `"en"`, `"ja"`. Defaults to system locale; switchable immediately in UI without restart).
* Bidirectional synchronization between settings UI and TOML. Schema versioning, pre-migration backups, unknown-key preservation, safe fallback for invalid values.
* Invariant safety rules (root prohibition, origin reflection decoupling, Landlock enforcement) cannot be disabled.
* `[pipeline]` section allows defining per-preset `allowed_read_paths` arrays.

---

## 12. Decisions & Resolution of Open Issues

### 12.1 Finalized Architecture Decisions

1. **6-pane main layout** with explicit preview invocations.
2. **Original protection & workspace segregation:** Consolidated in `workspace-temp/`; deliverables in `artifacts/`; recovery in `recovery/`.
3. **Decoupled origin reflection:** Executed only as an independent user action after batch completion. Categorically forbidden on mounts and outside `$HOME`.
4. **Landlock sandbox:** Child processes isolated under Landlock rulesets; non-workspace writes blocked at OS level.
5. **Mandatory pipeline safety flow:** "Explanation → Confirmation 1 → Dry-Run → Results Review → Confirmation 2 → Execution." Single-step, argument array expansion, zero arbitrary shell strings.
6. **RollbackAll bulk policy:** 1 failure triggers entire batch rollback. Deliverables unreleased until 100% success.
7. **Full-text search:** Tantivy + `lindera-tantivy` (IPADIC, Normal) + n-gram. Plain text extraction first.
8. **Session-end backups:** Interactive prompt on shutdown. Incomplete recovery data isolated and protected.
9. **Landlock path profiling & preset extensions:** `strace` profiling standard; TOML `allowed_read_paths`; isolated `TMPDIR`.
10. **`workspace-temp` same-volume priority:** Prioritized allocation on original volume to maximize atomic `rename(2)` and Reflink. Mandatory 1.5x capacity pre-check.
11. **Non-UTF-8 dual-key management:** Rust `PathBuf` internally; `raw_base64` / `WTF-8` + immutable `item_id` at IPC/JSON boundaries.
12. **Isolated text extraction worker & SQLite sync:** Worker process with memory quota (256 MiB) prevents OOM; SQLite WAL + `NORMAL` with checkpoints at critical commits.
13. **Virtual scrolling & streaming:** `@tanstack/react-virtual` and chunked IPC for 10,000 items at 60fps.
14. **100% Bilingual Operation (Docs & UI):** All documentation provided in both English and Japanese; all in-app UI, menus, warnings, and dialogs dynamically switchable between English and Japanese.

### 12.2 Resolved Specifications & Implementation Notes

Items previously marked as research topics in v0.8 are resolved as follows:

1. **Landlock Minimal Paths:** Dynamic linker, font caches, and isolated `TMPDIR` whitelisted; presets extend via `allowed_read_paths`. CI verified on Linux 5.13+ runners.
2. **Linux Filename Binary Problem:** Dual-key mapping (`item_id` + `raw_base64`) prevents panics and avoids file loss on line deletion.
3. **`workspace-temp` Placement:** Volume-local `.fileworkbench-temp/` prioritized for atomic renames and Reflink. 1.5x capacity pre-verification enforced.
4. **Text Extraction & Memory Protection:** Plain text and Markdown first; PDF/Office handled in isolated 256 MiB quota workers.
5. **SQLite Synchronization:** WAL + `NORMAL` standard; explicit `PRAGMA wal_checkpoint(TRUNCATE)` at origin reflection and batch commits.
6. **i18n Architecture:** Frontend uses `i18next`; backend returns structured error codes. Dynamic EN/JA switching supported in settings.

---

## 13. Implementation Roadmap

* **Phase 1: Project Foundations & Infrastructure** (Rust API, safety rules, TOML settings, 6-pane prototype, Landlock PoC, non-UTF-8/Base64 dual-key types, virtual scroll baseline, i18n localization infrastructure)
* **Phase 2: File Management & Origin Protection** (Browser, tree, `workspace-temp` management, same-volume detection, conflict detection, origin reflection, multilingual UI integration)
* **Phase 3: Search & Indexing** (SQLite metadata, plocate, Tantivy + lindera, isolated text worker, differential queue)
* **Phase 4: Backups & Session Recovery** (Shutdown interactive backup, recovery journals, session restoration)
* **Phase 5: Pipeline Runner** (Landlock isolation, TOML presets & `allowed_read_paths`, 2-phase confirmation, log overlay)
* **Phase 6: Advanced Bulk Operations** (BatchPlan, Dry-run, phased rename, editor integration, RollbackAll, virtual scroll integration)
* **Phase 7: Quality & Durability Verification** (systemd updates, COSMIC notification integration, B-01 through B-15 fault injection tests, Landlock CI verification, bilingual verification)

---

## 14. Acceptance Criteria

* All file operations and external commands pass unified safety verification and Landlock isolation.
* Direct writes to original files from external commands are physically impossible.
* All fault injection test scenarios (B-01 through B-15) pass completely.
* Operation execution and index updates are decoupled; repair queues function correctly upon indexing failures.
* Bulk operations with up to 10,000 items maintain 60fps UI performance (virtualized) and handle non-UTF-8 paths safely.
* All documentation is completely provided in both English and Japanese with zero discrepancy in content.
* All menus, warnings, messages, and dialogs throughout the application are completely available in both English and Japanese, switchable immediately from in-app settings.