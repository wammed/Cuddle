# Cuddle (FileWorkbench) Next Implementation Tasks & Roadmap

<p align="center">
  <strong>English</strong> | <a href="NEXT_IMPLEMENTATION_TASKS.ja.md">日本語</a>
</p>

**Document Date:** 2026-10-27  
**Status:** Approved (Triaged and specified 18 items from Design Specification ver 0.9)  
**Audience:** Developers, Architects, Next Engineering Sessions  

---

## 1. Overview and Objectives

This document provides a systematic backlog and implementation guide for the **18 items identified during the cross-verification** between the Master Design Specification ([`FileWorkbench-Design-ver0.9.en.md`](FileWorkbench-Design-ver0.9.en.md)) and the current codebase (as of Phase 9.8 completion), plus **6 Core Visual Polish & Sensory Upgrade tasks** for a total of 24 items. It details the **prioritization, architectural design, affected files, and testing strategies** so that future engineering sessions can immediately start implementation without ambiguity.

### Non-Negotiable Core Principles

1. **Absolute Original Protection:** Direct in-place editing of original files is strictly prohibited. Operations must only run within working copies (`workspace-temp/`), reflecting to originals only upon explicit user confirmation.
2. **Kernel Isolation with Landlock LSM:** External commands and parsers must execute under restrictive Landlock rulesets (read-only system whitelist, dedicated isolated `TMPDIR`).
3. **Full Bilingual Synchronization (EN/JA):** All UI strings, logs, warnings, and documentation must exist in both English and Japanese with 100% parity.
4. **Dual-Key Non-UTF-8 Path Safety:** Non-UTF-8 paths with invalid byte sequences must be preserved safely via `FileItem` (`item_id` + `raw_base64` + safe `display_name`).
5. **Deterministic Safety and Rollback:** Any discrepancy or error triggers a safe fail-stop with deterministic rollback to the pre-operation state.

---

## 2. Task List and Priority Matrix

The 24 items are organized across **5 implementation phases (Phase 10 to 14)** based on dependency, risk, complexity, and user experience value.

| ID | Category | Task Name | Recommended Phase | Priority | Complexity | Status |
| :--- | :--- | :--- | :--- | :---: | :---: | :---: |
| **2-2** | UI / Layout | Pane Width & Layout Persistence across Sessions | **Phase 10** | **High** | Low | **Done & Verified** |
| **2-1** | UI / Workflow | Drag & Drop (D&D) between Browser & Workspace | **Phase 10** | **High** | Med | **Done & Verified** |
| **2-4** | UI / Logs | Pipeline Log Display Filter (`all` / `hidden` / `file_only`) | **Phase 10** | **High** | Low | **Done & Verified** |
| **2-5** | UI / Visual | Extension-Specific Rich Icons | **Phase 10** | **High** | Low | **Done & Verified** |
| **2-6** | UI / Visual | Lighting & Depth Layering (Inner Rim Light & Multi-layer Shadows) | **Phase 10** | **High** | Low | **Done & Verified** |
| **2-7** | UI / Visual | Cursor Spotlight Glow & Tactile Spring Feedback | **Phase 10** | **High** | Med | **Done & Verified** |
| **2-8** | UI / Visual | Storage Capacity Gauges & Visual Permissions/Size Metrics | **Phase 10** | **High** | Med | **Done & Verified** |
| **2-9** | UI / Visual | Tabular Precision Typography & Interactive Breadcrumb Trail | **Phase 10** | **High** | Low | **Done & Verified** |
| **2-10** | UI / Visual | Ambient Aurora Mesh Canvas & Subtle Micro-Grain Texture | **Phase 10** | **Medium** | Low | **Done & Verified** |
| **3-2** | Safety / Reflect | Alternate Filename UI on Original Conflict (e.g., `.edit.pdf`) | **Phase 11** | **High** | Med | **Done & Verified** |
| **3-3** | Safety / Backup | Backup Retention (Default 5, Max 10) & Prune UI | **Phase 11** | **High** | Med | **Done & Verified** |
| **4-3** | Safety / Sandbox | Warning / Consent Modal on Non-Landlock Kernels | **Phase 11** | **Medium** | Low | **Done & Verified** |
| **4-2** | Pipeline | Custom `allowed_read_paths` in Pipeline Presets | **Phase 12** | **High** | Med | **Done & Verified** |
| **4-1** | Pipeline | Input Modes (`batch`, `stdin`) & Typed Placeholders | **Phase 12** | **Medium** | Med | **Done & Verified** |
| **2-3** | UI / System | Desktop Environment Native Notification Abstraction | **Phase 12** | **Medium** | Med | **Done & Verified** |
| **UI-Font** | UI / Status | Dynamic Nerd Font Statusbar Detection & Active Indicator | **Phase 12** | **Medium** | Low | **Done & Verified** |
| **1-4** | Search / Extract | Structured Markdown / HTML Text Extraction | **Phase 13** | **Medium** | Med | **Done & Verified** |
| **1-3** | Search / Extract | Tier 3: Office Document Extraction (`.docx`, `.xlsx`) | **Phase 13** | **Medium** | High | **Done & Verified** |
| **1-2** | Search / Engine | Combined Morphological (IPADIC) + n-gram Search | **Phase 13** | **Medium** | High | **Done & Verified** |
| **1-1** | Search / Queue | Index Update / Repair Queue for Discrepancies | **Phase 13** | **Low** | High | **Done & Verified** |
| **1-5** | Search / Policy | Per-Rule Indexing Policy (Metadata-only / Filename-only) | **Phase 13** | **Low** | Med | **Done & Verified** |
| **3-1** | Safety Policy | Safety Confirmation Levels (Standard / Strict, Brief omitted) | **Phase 14** | **High** | Low | **Done & Verified** |
| **3-4** | Crash Recovery | Endorsement of SQLite WAL + Journal State Model | **Phase 14** | **High** | Low | **Done & Verified** |
| **5-1** | Configuration | Full Alignment between Settings Menu & config.toml | **Phase 14** | **High** | Low | **Done & Verified** |
| **6-1** | IPC Optimization | Validated Batch IPC + Virtual Scrolling Architecture | **Phase 14** | **Medium** | Low | **Done & Verified** |

---

## 3. Detailed Phase-by-Phase Implementation Plans

### Phase 10: Quick UX & Interaction Wins & Ultra-Rich Visual Polish [Completed & Verified by User]

Delivers immediate day-to-day workflow improvements (Part A) alongside a comprehensive sensory and visual upgrade to world-class design standards (Part B).

#### Part A: Interaction & Ergonomics

##### [Task 2-2] Pane Width & Layout Persistence across Sessions

* **Design Spec Reference:** Section 3.2 (Screen Structure & Layout)
* **Current Status:** `react-resizable-panels` initializes with static default ratios (52%, 20%, 68%, 50%), resetting whenever the app restarts.
* **Architecture & Strategy:**
  * Utilize `autoSaveId="cuddle-main-layout"` in `react-resizable-panels` or implement `onLayout={(sizes) => ...}` saving serialized pane ratios to `localStorage`.
  * Automatically restore pane proportions on window resize and startup.
* **Target Files:**
  * `src/App.tsx`
* **Verification:** Vitest test with mocked `localStorage`; manual pane resize and reload test.

##### [Task 2-1] Drag & Drop (D&D) between Browser & Workspace

* **Design Spec Reference:** Section 3.2 (Operation Flow)
* **Current Status:** Files can only be staged via toolbar buttons ("Stage Working Copy", "Stage All") and context menu.
* **Architecture & Strategy:**
  * Enable `draggable={true}` on `FileBrowserPanel.tsx` rows, writing `item_id` and `raw_base64` to `dataTransfer` on `dragstart`.
  * Implement `onDragOver` (`preventDefault()`) and `onDrop` in `WorkspacePanel.tsx`.
  * Trigger `handleStageFile(item)` upon drop to safely create working copies in `workspace-temp/`. Originals remain completely untouched.
* **Target Files:**
  * `src/components/FileBrowserPanel.tsx`
  * `src/components/WorkspacePanel.tsx`
  * `src/App.tsx`
* **Verification:** Vitest mock DragEvent tests (`fireEvent.dragStart`, `fireEvent.drop`).

##### [Task 2-4] Pipeline Log Display Filter Modes

* **Design Spec Reference:** Section 8.4 (Pipeline Logs and Audit Trails)
* **Current Status:** Flat linear string dump inside `BottomTabPanel.tsx`.
* **Architecture & Strategy:**
  * Refactor log model into structured entries or parse prefix tags: `LogEntry { id, timestamp, level, target_file, message }`.
  * Add toggle pills (`All`, `File Only` [file transformations only], `Hidden` [errors and major events only]) to the top bar of the log panel.
* **Target Files:**
  * `src/components/BottomTabPanel.tsx`
  * `src/locales/ja/translation.json`, `src/locales/en/translation.json`
* **Verification:** Vitest test verifying filtering behavior on log lines.

---

#### Part B: Ultra-Rich Visual Polish (6 Core Approaches)

##### [Task 2-5] Extension-Specific Rich Icons

* **Design Spec Reference:** Section 1.1 (Branding & Visuals), Section 3.3 (Information Density & Icons)
* **Current Status:** `VirtualFileList.tsx` renders all regular files with a generic, monochromatic `<FileText />` icon and directories with `<Folder />`.
* **Architecture & Strategy:**
  * Create `src/utils/fileIcons.tsx` mapping file extensions to dedicated colored Lucide icons.
  * Code/Config: Rust (orange), TS/JS (blue), Python (gold), HTML/CSS (indigo), TOML/JSON (amber). Documents: Markdown (teal), PDF (crimson), Text (slate). Media: Images (emerald), Audio (purple), Video (pink). Archives: ZIP/TAR (violet).
* **Target Files:**
  * `src/utils/fileIcons.tsx` (New)
  * `src/components/VirtualFileList.tsx`
  * `src/App.css`
* **Verification:** Vitest assertions for extension-to-icon mapping and CSS class assignment.

##### [Task 2-6] Lighting & Depth Layering (Inner Rim Light & Multi-layer Shadows)

* **Design Spec Reference:** Section 1.1, Section 3.1 (Glassmorphism & Cyber Dark Visual Concept)
* **Current Status:** Surfaces rely on flat single-layer borders (`1px solid rgba(...)`) and standard shadows.
* **Architecture & Strategy:**
  * Implement top inner specular rim lighting (`box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.12), inset 0 0 0 1px rgba(255, 255, 255, 0.04)`) across hero cards, feature cards, panel containers, and dialogs to simulate overhead illumination hitting glass and brushed aluminum edges.
  * Upgrade drop shadows into 3-tier composite elevations: sharp contact shadow + broad ambient dissipation + subtle accent bleed.
  * Refine floating surfaces (modals, header, context menus) with `backdrop-filter: blur(24px) saturate(180%)`.
* **Target Files:**
  * `src/App.css`
* **Verification:** Visual sanity across all 30 color themes; theme contrast verification.

##### [Task 2-7] Cursor Spotlight Glow & Tactile Spring Feedback

* **Design Spec Reference:** Section 3.2 (Interaction & Motion)
* **Current Status:** Static color tints or simple 1px vertical translation on hover.
* **Architecture & Strategy:**
  * Implement mouse tracking hook updating `--mouse-x` and `--mouse-y` CSS variables on cards and buttons, projecting a 120px radial gradient spotlight that follows the cursor.
  * Add tactile active-press feedback (`transform: scale(0.97)`) with physical spring ease curves on buttons and interactive cards.
  * Introduce smooth sliding highlight pill indicators behind active tabs in the header and bottom panel.
* **Target Files:**
  * `src/components/StartPage.tsx`
  * `src/components/Header.tsx`
  * `src/components/BottomTabPanel.tsx`
  * `src/App.css`
* **Verification:** Vitest tests for coordinate tracking and uninterrupted click dispatch.

##### [Task 2-8] Storage Capacity Gauges & Visual Permissions/Size Metrics

* **Design Spec Reference:** Section 3.3, Section 4.1, Section 5.1 (Pre-flight Capacity Checks)
* **Current Status:** File sizes, permissions, and disk space are displayed exclusively as text strings.
* **Architecture & Strategy:**
  * **Original Protection & Disk Usage Stacked Bar:** Multi-segmented graphical bar in the header or sidebar displaying Original Volume vs. Workspace Temp vs. Free Storage.
  * **Inline Size Heatmap Bar:** 32px micro-bar adjacent to file sizes indicating relative weight compared to the largest file in the active folder.
  * **Permission Color Segment Pill:** Segmented badge decomposing `rwxr-xr-x` into color-coded permissions (Read: Blue, Write: Orange, Exec: Green).
* **Target Files:**
  * `src/components/Header.tsx`
  * `src/components/VirtualFileList.tsx`
  * `src/components/DetailsSidebarPanel.tsx`
  * `src/App.css`
* **Verification:** Unit tests verifying ratio calculations and edge-case rendering (0-byte, massive files).

##### [Task 2-9] Tabular Precision Typography & Interactive Breadcrumb Trail

* **Design Spec Reference:** Section 3.2, Section 3.3 (Information Density & Path Ergonomics)
* **Current Status:** Path is plain text; numbers use proportional figures causing jagged column alignment.
* **Architecture & Strategy:**
  * Apply `font-variant-numeric: tabular-nums` / `font-feature-settings: "tnum"` to all size, date, inode, and permission columns for razor-sharp alignment.
  * Transform static path inputs in `Header.tsx` and `FileBrowserPanel.tsx` into clickable breadcrumb pills with subtle hover glows for instant ancestor directory navigation.
* **Target Files:**
  * `src/components/Header.tsx`
  * `src/components/FileBrowserPanel.tsx`
  * `src/App.css`
* **Verification:** Vitest tests for path segment parsing and ancestor navigation callbacks.

##### [Task 2-10] Ambient Aurora Mesh Canvas & Subtle Micro-Grain Texture (Theme Reactive & High Vibrancy)

* **Design Spec Reference:** Section 1.1 (Cyber Dark Atmosphere)
* **Implementation Details:**
  * **30 Color Themes Full Synchronization:** Eliminated hardcoded dark background (`#03070a`), rebinding `.ambient-aurora-mesh` to dynamic CSS radial gradients using `--bg-canvas` and `--bg-surface-base`. Switching among 30 color themes in Settings instantly and smoothly updates the background canvas.
  * **Theme-Reactive Glowing Orbs (High Vibrancy & Scaled):** Bound orb fill gradients to theme variables (`--accent-brand-bright`, `--accent-brand`, `--accent-safety`, `--border-glow`, `--text-secondary`). Expanded orb dimensions (660px-790px) and sharpened blur to 48px to intensify the core luminescence and physical depth, breathing with dynamic scale (0.88-1.34) and pulsating opacity (0.50-0.92).
  * **Harmonized Translucent Glassmorphism:** Applied `color-mix(in srgb, var(--bg-surface-card) 68%, transparent)` across workspace panels, launchpad cards, and headers so that background neon orb floating and breathing are clearly and dramatically visible through the frosted glass panels while preserving each theme's unique color identity.
  * **Hardware-Accelerated Micro-Grain:** Enhanced frontmost (`z-index: 9999`) inline SVG filter (`feTurbulence` + `feColorMatrix`, `opacity: 0.075`) with `mix-blend-mode: overlay` eliminating all color banding and delivering a consistent tactile film grain texture across diverse monitors and GPUs.
* **Target Files:**
  * `src/App.css`
  * `src/App.tsx`
  * `src/__tests__/Phase10Refinements.test.tsx`

---

### Phase 11: Safety, Conflict Resolution & Recovery Refinements [Implemented & Verified by Manual Testing]

Hardened original protection guarantees, conflict handling, backup retention, and workspace resumption. Verified and approved via real-world manual testing (conflict alternate save, retention prune, window close safety intercept, and resume from backup on Start Dashboard).

#### [Task 3-2] Alternate Filename UI on Original Conflict (e.g., `.edit.pdf`)

* **Design Spec Reference:** Section 5.3 (External Change & Conflict Interception)
* **Implementation Details:**
  * Implemented `reflect_staged_item_as_copy(item_id, custom_suffix)` (default: `.edit`), `get_alternate_copy_preview`, and `generate_alternate_copy_path` in `src-tauri/src/workspace.rs` and registered Tauri IPC commands.
  * Formats `filename.ext` into `filename.edit.ext` (or `filename.edit.1.ext` on subsequent collisions) safely without touching the original file.
  * In `BottomTabPanel.tsx`, rendered an alternate copy box on reflection conflict with customizable suffix, real-time path preview, and one-click copy saving.
* **Target Files:**
  * `src-tauri/src/workspace.rs`, `src-tauri/src/lib.rs`
  * `src/components/BottomTabPanel.tsx`
  * `src/locales/ja/translation.json`, `src/locales/en/translation.json`
* **Verification:** Cargo tests (`test_reflect_staged_item_as_copy_and_collision`) and Vitest (`Phase11SafetyRefinements.test.tsx`) pass with 100% test coverage.

#### [Task 3-3] Backup Retention Management & Pruning UI

* **Design Spec Reference:** Section 6.1 (Backup Retention & Disk Quota)
* **Implementation Details:**
  * Added `backup_retention_count: usize` (default: 5, range: 1..=10 with validation) to `WorkspaceSection` in `config.rs`.
  * Implemented `BackupArchiveInfoDto`, `list_backups()`, `delete_backup(filename)`, and `prune_old_backups(keep_count)` in `backup.rs`. Integrated `auto_prune_limit` into `create_workspace_backup`.
  * Added retention policy indicator, auto-prune checkbox, collapsible archive list, and per-backup deletion actions to `ExitBackupModal.tsx`.
  * Added backup retention slider (1–10 generations) and archive management card to `SettingsModal.tsx`.
* **Target Files:**
  * `src-tauri/src/backup.rs`, `src-tauri/src/config.rs`, `src-tauri/src/lib.rs`
  * `src/components/ExitBackupModal.tsx`, `src/components/SettingsModal.tsx`
* **Verification:** Cargo tests (`test_backup_list_delete_and_prune`) and Vitest (`ExitBackupModal.test.tsx`, `Phase11SafetyRefinements.test.tsx`) pass.

#### [Task 4-3] Warning / Consent Modal on Non-Landlock Kernels

* **Design Spec Reference:** Section 8.2 (Process Isolation & Runtime Security)
* **Implementation Details:**
  * Created `CriticalSecurityModal.tsx` providing a full-screen blocking modal when Landlock LSM is unsupported.
  * Detailed security risks regarding pipeline isolation and original protection reliance on internal mechanisms. Provided "Exit Application" and "I Understand the Risks & Unlock" buttons, along with a "Remember my choice" persistence checkbox.
  * Wired `App.tsx` to inspect `get_landlock_status` upon startup and lock the UI until consent is granted.
* **Target Files:**
  * `src/components/CriticalSecurityModal.tsx`
  * `src/App.tsx`, `src/App.css`
  * Bilingual locale translations in `src/locales/`.
* **Verification:** Vitest test (`Phase11SafetyRefinements.test.tsx`) verified startup blocking, exit handler, consent unlock, and choice persistence.

---

### Phase 12: Pipeline Flexibility & External Tool Integrations [Done & Verified]

Extended custom command pipelines with flexible execution arguments, input stream modes, Landlock sandbox read whitelists, and dynamic statusbar font status indicators. Fully tested and verified through manual verification scenarios and automated suites (208 Cargo tests passed, 19 Vitest files / 119 tests 100% passing).

#### [Refinement] Dynamic Nerd Font Statusbar Detection & Active Indicator

* **Issue:** When choosing a custom font or Nerd Font preset in Settings, the bottom-right statusbar permanently displayed "Nerd Font: Pre-required" / "Nerd Font: 導入推奨" as a static label.
* **Implementation:**
  * Added `isNerdFontConfigured(settings)`, `detectSystemNerdFont()`, and `getNerdFontStatus(settings)` to `src/utils/typography.ts`.
  * Emits `cuddle:typography-changed` custom window event upon `applyTypographySettings(settings)` for real-time reactivity without page reloads.
  * In `src/App.tsx`, rendered `statusbar-nerd-font-btn` with 3 dynamic states:
    * `active`: Emerald dot and "Nerd Font: Active" (`Nerd Font: 適用中`)
    * `detected`: Cyan dot and "Nerd Font: Detected" (`Nerd Font: 検出済`)
    * `recommended`: Slate dot and "Nerd Font: Recommended" (`Nerd Font: 導入推奨`)
  * Clicking the statusbar indicator immediately opens the Typography and Appearance preferences modal.
* **Impacted Files:**
  * `src/utils/typography.ts`, `src/App.tsx`, `src/App.css`
  * `src/locales/ja/translation.json`, `src/locales/en/translation.json`
  * `src/__tests__/Typography.test.ts`, `src/__tests__/App.test.tsx`
* **Verification:** Vitest test suite verifies reactive DOM updates on font setting changes.

#### [Task 4-2] Custom `allowed_read_paths` in Pipeline Presets

* **Design Spec Reference:** Section 8.2, Section 11 (Landlock Sandbox Whitelisting)
* **Implementation Details:**
  * Added `pub allowed_read_paths: Vec<String>` (`#[serde(default)]`) to `CustomPipelineDefDto`, supporting persistent definition via `config.toml` and UI editing.
  * Dynamically inspects specified paths during pipeline execution (`execute_custom_pipeline_with_progress`) and pre-execution Dry-Run (`dry_run_custom_pipeline`), appending valid paths to `LandlockSandbox::allow_read` while strictly keeping the write boundary isolated to working directories.
  * Added allowed read paths input field and explanatory tooltip in `SettingsModal.tsx` custom pipeline form, along with "Landlock Read" badges on registered cards.
  * Highlighted allowed read paths in the Step 1 Risk Specification card of `PipelineModal.tsx`.
* **Target Files:**
  * `src-tauri/src/config.rs`, `src-tauri/src/pipeline.rs`
  * `src/components/SettingsModal.tsx`, `src/components/PipelineModal.tsx`
  * `src/types/CustomPipelineDefDto.ts`
* **Verification:** Cargo tests (`test_custom_pipeline_def_dto_phase12_fields`, `test_custom_pipeline_dry_run_with_modes_and_allowed_reads`) and Vitest test (`Phase12PipelineAndNotifications.test.tsx`) pass.

#### [Task 4-1] Input Modes (`batch`, `stdin`) & Typed Placeholders

* **Design Spec Reference:** Section 8.1 (Pipeline Input Modes & Placeholders)
* **Implementation Details:**
  * Introduced `ExecutionInputMode` enum (`PerFile`, `BatchList`, `StandardInput`) and integrated it into `CustomPipelineDefDto`.
  * Implemented typed placeholder expansion function `expand_command_arguments`:
    * `{input}`: Individual file path (for `PerFile`).
    * `{output}`: Individual target output path (for `PerFile`).
    * `{InputList}`: Splatted into multiple distinct arguments (safely handling paths with whitespace) or space-delimited string.
    * `{WorkspaceDirectory}`: Base directory of the working copies.
  * Extended `SandboxedExecutor` with `options.stdin_input: Option<String>` to stream file path lists into child process `stdin` with automatic EOF closing.
  * Single-process invocation for `BatchList` and newline-delimited stream piping for `StandardInput`.
  * Added execution mode dropdown and placeholder quick-insert pills (`+ {input}`, `+ {output}`, `+ {InputList}`, `+ {WorkspaceDirectory}`) in `SettingsModal.tsx`.
* **Target Files:**
  * `src-tauri/src/config.rs`, `src-tauri/src/pipeline.rs`
  * `src/components/SettingsModal.tsx`, `src/components/PipelineModal.tsx`
  * `src/types/ExecutionInputMode.ts`, `src/types/CommandExecutionOptionsDto.ts`
* **Verification:** Cargo tests (`test_expand_command_arguments_typed_placeholders`, `test_sandboxed_executor_stdin_input`) and Vitest test (`Phase12PipelineAndNotifications.test.tsx`) pass.

#### [Task 2-3] Desktop Environment Native Notification Abstraction

* **Design Spec Reference:** Section 2.1, Section 13 (Notification Architecture)
* **Implementation Details:**
  * Created `notify.rs` module providing FreeDesktop Desktop Notifications Specification wrapper `notify_desktop(title, body, level)` and Tauri command `notify_user`.
  * Dispatches native notifications to COSMIC, GNOME, and KDE notification daemons via `notify-send` (`-a "Cuddle"`, `-u urgency`, `-i icon`), with graceful fallback when daemons are unavailable.
  * Automatically dispatches notifications upon preset and custom pipeline completion from both backend and frontend.
  * Added frontend abstraction helper `src/utils/notifications.ts` (`notifyUser`).
* **Target Files:**
  * `src-tauri/src/notify.rs` (new), `src-tauri/src/lib.rs`, `src-tauri/src/pipeline.rs`
  * `src/utils/notifications.ts` (new), `src/components/PipelineModal.tsx`
  * `src/types/NotificationLevel.ts`
* **Verification:** Cargo tests (`test_notify_desktop_does_not_panic`, `test_notification_level_defaults_and_conversion`) and Vitest test (`Phase12PipelineAndNotifications.test.tsx`) pass.

---

### Phase 13: Search, Extraction & Indexing Evolution [Done & Approved via Manual Testing]

Broadened document type extraction, hybrid morphological/n-gram search recall, background repair queue, and metadata-only indexing policies are fully implemented and verified.

#### [Task 1-4] Structured Markdown / HTML Text Extraction: **Done & Verified**

* **Design Spec Reference:** Section 7.2 (Tier 1 Text Extraction)
* **Architecture & Strategy:**
  * Ingest Markdown via `pulldown-cmark` (v0.12), cleanly stripping tokens and returning only headings and prose text.
  * Ingest HTML via regex stripper removing `<script>`, `<style>`, tags, and unescaping HTML entities.
  * Integrated into `extract_text_content` in `extractor.rs`.
* **Target Files:**
  * `src-tauri/Cargo.toml`, `src-tauri/src/extractor.rs`
* **Verification:** Verified by Cargo test `test_markdown_and_html_prose_extraction` and Vitest `Phase13SearchAndRepair.test.tsx`.

#### [Task 1-3] Tier 3: Office Document Extraction (`.docx`, `.xlsx`): **Done & Verified**

* **Design Spec Reference:** Section 7.2 (Tier 3 Office Extraction)
* **Architecture & Strategy:**
  * Implemented safe zero-dependency ZIP entry reader (`read_zip_file_entry`) parsing EOCD / Central Directory records and decompressing entries via `flate2::read::DeflateDecoder`.
  * Parsed `.docx` (`word/document.xml`) and `.xlsx` (`xl/sharedStrings.xml`) text elements via `quick-xml` (v0.41).
  * Enforced ZipBomb protection (100:1 ratio limit, 10 MiB decompressed limit, 256 MiB memory guardrails).
* **Target Files:**
  * `src-tauri/Cargo.toml`, `src-tauri/src/extractor.rs`
* **Verification:** Verified by Cargo test `test_tier3_office_docx_extraction`, corrupted zip tests, and Vitest suite.

#### [Task 1-2] Combined Morphological (IPADIC) + n-gram Search: **Done & Verified**

* **Design Spec Reference:** Section 2.1, Section 7.2
* **Architecture & Strategy:**
  * Added `body_ngram` field using Tantivy's built-in `NgramTokenizer::all_ngrams(2, 3)`.
  * Configured multi-field query boosting (`display_name: 3.0`, `body: 2.0`, `body_ngram: 0.8`).
  * Yields prioritized scoring for exact morpheme matches while capturing substring, out-of-vocabulary, and compound words.
* **Target Files:**
  * `src-tauri/src/search_fulltext.rs`
* **Verification:** Verified by Cargo test `test_ngram_and_morpheme_hybrid_search` and Vitest suite.

#### [Task 1-1] Index Update / Repair Queue for Discrepancies: **Done & Verified**

* **Design Spec Reference:** Section 1.1, Section 7.2, Section 14
* **Architecture & Strategy:**
  * Created SQLite WAL `index_repair_queue` table with schema migration.
  * Automatic queueing via `enqueue_repair_on_failure` on Tantivy lock or indexing errors.
  * Exposed Tauri commands `process_index_repair_queue` (with exponential retry and retry-count tracking) and `get_index_repair_queue_count`.
  * Added pending count badge and "Process Index Repair Queue" action to `SearchPanel` dropdown with bilingual notification banners.
* **Target Files:**
  * `src-tauri/src/history.rs`, `src-tauri/src/search_fulltext.rs`, `src-tauri/src/lib.rs`
  * `src/components/SearchPanel.tsx`, `src/types/IndexRepairResultDto.ts`
* **Verification:** Verified by Cargo test `test_index_repair_queue_workflow` and Vitest `Phase13SearchAndRepair.test.tsx`.

#### [Task 1-5] Per-Rule Indexing Policy (Metadata-only / Filename-only): **Done & Verified**

* **Design Spec Reference:** Section 7.1
* **Architecture & Strategy:**
  * Added `IndexingPolicy` (`Full`, `MetadataOnly`, `Exclude`) and `include_metadata_only_excluded` option.
  * For excluded paths (`.git`, `node_modules`), indexed document filename, size, and mtime with empty body text when policy is `MetadataOnly`.
* **Target Files:**
  * `src-tauri/src/extractor.rs`, `src-tauri/src/search_fulltext.rs`, `src/types/IndexingPolicy.ts`
* **Verification:** Verified by Cargo test `test_run_index_operations_workflow`.

---

### Phase 14: Architectural Decisions & Scale Verification [Done & Verified]

Evaluated robustness, scale, and performance characteristics, eliminated over-engineering, and established complete alignment between Settings UI and `config.toml` / `config.template.toml`.

#### [Task 3-1] Safety Confirmation Levels (Standard / Strict): **Done & Verified**

* **Architecture & Strategy:**
  * Strictly omitted "Brief" (no confirmation) to preserve Zero Data Loss and original protection.
  * Implemented 2-tier model: "Standard" (summary badges and disk space checks) and "Strict" (mandatory SHA-256 pre-verification of all items and diff review).
  * Added `ConfirmationLevel` in `src-tauri/src/config.rs` (`SecuritySection.confirmation_level`), `config.template.toml`, and `SettingsModal.tsx` (General Tab).
* **Target Files:**
  * `src-tauri/src/config.rs`, `config.template.toml`, `src/components/SettingsModal.tsx`
* **Verification:** Verified by Cargo test `test_default_config_creation_and_roundtrip` and Vitest `Phase14ArchitectureAndConfig.test.tsx`.

#### [Task 3-4] Endorsement of SQLite WAL + Journal State Model: **Done & Verified**

* **Architecture & Strategy:**
  * Re-evaluated the speculative 4-table model (`batches`, `batch_operations`, `recovery_checkpoints`, `batch_artifacts`).
  * Endorsed existing SQLite WAL `session_items`, `operation_history`, `index_repair_queue`, and `BatchJournalManager` (two-phase parking, atomic rollbacks, selective undo) as superior in reliability and simplicity with zero data loss on unexpected exits.
  * Formally solidified current schema as the master architectural specification.

#### [Task 5-1] Alignment between Settings Menu & config.toml: **Done & Verified**

* **Architecture & Strategy:**
  * Endorsed current 4-section model (`[app]`, `[workspace]`, `[security]`, `[[custom_pipelines]]`) over excessive 10-section partitioning for reliability, parse speed, and self-healing resilience.
  * Fully synchronized SettingsModal controls (language, theme, density, confirmation level, backup retention count, custom pipelines) with direct persistence to `config.toml`.
  * Updated `config.template.toml` and in-app template generator with complete section definitions and typed pipeline examples.
* **Target Files:**
  * `src-tauri/src/config.rs`, `config.template.toml`, `src/components/SettingsModal.tsx`
* **Verification:** Verified by Vitest `Phase14ArchitectureAndConfig.test.tsx`.

#### [Task 6-1] Validated Batch IPC + Virtual Scrolling Architecture: **Done & Verified**

* **Architecture & Strategy:**
  * Validated 10,000+ item directory transfer benchmarks under Tauri 2 IPC and `@tanstack/react-virtual`.
  * Preserved full batch transfer with virtualized rendering maintaining 60fps scrolling and sub-10MB memory usage, avoiding unnecessary pagination API state complexity.

---

## 4. Recommended Execution Order for Next Session

```mermaid
graph TD
    A[Phase 10: UX Ergonomics & Ultra-Rich Visual Polish] --> B[Phase 11: Safety & Conflict Refinements]
    B --> C[Phase 12: Pipeline Flexibility]
    C --> D[Phase 13: Search & Extraction Evolution]
    D --> E[Phase 14: Architectural Closure]

    subgraph "Phase 10 (First Recommended Sprint: UI/UX & Sensory Polish)"
        A1[Part A: Interaction & Ergonomics<br/>2-2 Pane Persistence / 2-1 D&D / 2-4 Logs]
        A2[Part B: 6 Core Visual Polish Approaches<br/>2-5 Icons / 2-6 Lighting / 2-7 Spotlight<br/>2-8 Gauges / 2-9 Typography / 2-10 Aurora Canvas]
    end

    subgraph "Phase 11 (Safety Hardening)"
        B1[3-2: Conflict Alternate Save .edit]
        B2[3-3: Backup Retention Prune UI]
        B3[4-3: Non-Landlock Warning Modal]
    end
```

1. **Sprint 1 (Phase 10):**
   * **Part A (Ergonomics):** Tasks 2-2 (Pane Persistence) → 2-1 (D&D) → 2-4 (Log Filter).
   * **Part B (Ultra-Rich Visuals):** Tasks 2-5 (Extension-Specific Rich Icons) → 2-6 (Inner Rim Light & Multi-layer Shadows) → 2-7 (Cursor Spotlight & Spring Feedback) → 2-8 (Storage Gauges & Size Heatbars) → 2-9 (Tabular Numbers & Breadcrumbs) → 2-10 (Ambient Aurora Canvas). Immediately elevates Cuddle's visual distinction to premier desktop software standards.
2. **Sprint 2 (Phase 11):** Tasks 3-2 (Conflict Save-As) → 3-3 (Backup Retention) → 4-3 (Non-Landlock Modal). Solidifies core safety and original protection.
3. **Sprint 3 (Phase 12):** Tasks 4-2 (`allowed_read_paths`) → 4-1 (Arguments & Stdin) → 2-3 (Native OS Notifications). Delivers full pipeline flexibility.
4. **Sprint 4 (Phase 13):** Tasks 1-4 (MD/HTML Extraction) → 1-3 (Office Extraction) → 1-2 (Morphological + n-gram) → 1-1 (Repair Queue) → 1-5 (Indexing Policy). [Done & Verified]
5. **Sprint 5 (Phase 14):** Architectural Decisions & Scale Verification: Task 3-1 (Confirmation Levels), Task 3-4 (4-Table Recovery evaluation), Task 5-1 (TOML sections & Settings alignment), Task 6-1 (IPC benchmark). [Done & Verified] All planned phases (Phase 1–14) complete.
