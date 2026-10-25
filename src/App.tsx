import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { Panel, Group, Separator } from "react-resizable-panels";
import { changeLanguage, SupportedLanguage } from "./i18n";
import type { FileItem } from "./types/FileItem";
import type { AppConfig } from "./types/AppConfig";
import type { ExtractedTextDto } from "./types/ExtractedTextDto";
import type { StagedItemDto } from "./types/StagedItemDto";
import type { SystemPathsDto } from "./types/SystemPathsDto";
import type { ResolvedPathDto } from "./types/ResolvedPathDto";
import { Header } from "./components/Header";
import { SearchPanel } from "./components/SearchPanel";
import { FolderTreePanel } from "./components/FolderTreePanel";
import { FileBrowserPanel } from "./components/FileBrowserPanel";
import { WorkspacePanel } from "./components/WorkspacePanel";
import { DetailsSidebarPanel } from "./components/DetailsSidebarPanel";
import { BottomTabPanel } from "./components/BottomTabPanel";
import { BulkRenameModal } from "./components/BulkRenameModal";
import "./App.css";

interface LandlockStatus {
  supported: boolean;
  abi_version: string | null;
}

export function App() {
  const { t, i18n } = useTranslation();
  const [currentPath, setCurrentPath] = useState<string>(".");
  const [parentPath, setParentPath] = useState<string | null>(null);
  const [systemPaths, setSystemPaths] = useState<SystemPathsDto | null>(null);

  const [browserItems, setBrowserItems] = useState<FileItem[]>([]);
  const [stagedFiles, setStagedFiles] = useState<FileItem[]>([]);
  const [stagedItems, setStagedItems] = useState<StagedItemDto[]>([]);
  const [selectedItem, setSelectedItem] = useState<FileItem | null>(null);
  const [bulkRenameOpen, setBulkRenameOpen] = useState(false);

  const [landlock, setLandlock] = useState<LandlockStatus | null>(null);
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null);
  const [isScanning, setIsScanning] = useState(false);

  const [sidebarVisible, setSidebarVisible] = useState(true);
  const [bottomVisible, setBottomVisible] = useState(true);

  const [extractedInfo, setExtractedInfo] = useState<ExtractedTextDto | null>(null);
  const [extractLoading, setExtractLoading] = useState(false);

  const [logs, setLogs] = useState<string[]>([]);
  const [history, setHistory] = useState<string[]>([]);

  const [layoutKey, setLayoutKey] = useState(0);

  const currentLang = (i18n.language.startsWith("ja") ? "ja" : "en") as SupportedLanguage;

  const addLog = useCallback((msg: string) => {
    const timestamp = new Date().toLocaleTimeString();
    setLogs((prev) => [`[${timestamp}] ${msg}`, ...prev.slice(0, 99)]);
  }, []);

  const addHistory = useCallback((entry: string) => {
    const timestamp = new Date().toLocaleTimeString();
    setHistory((prev) => [`[${timestamp}] ${entry}`, ...prev]);
  }, []);

  const toggleLanguage = () => {
    const nextLang: SupportedLanguage = currentLang === "ja" ? "en" : "ja";
    changeLanguage(nextLang);
    addLog(`Language switched to ${nextLang}`);
  };

  // Scans a directory via Tauri IPC
  const scanDirectory = useCallback(
    async (target: string) => {
      setIsScanning(true);
      try {
        // Resolve target to canonical path, raw_base64 and parent
        const resolved = await invoke<ResolvedPathDto>("resolve_path", { input: target });
        const items = await invoke<FileItem[]>("scan_dir", {
          pathBase64: resolved.raw_base64,
          options: {
            max_depth: 1,
            include_hidden: false,
            follow_symlinks: false,
          },
        });
        setBrowserItems(items);
        setCurrentPath(resolved.path_str);
        setParentPath(resolved.parent_raw_base64 ?? resolved.parent_path_str ?? null);
        addLog(`Scanned directory "${resolved.path_str}": found ${items.length} items`);
      } catch (err: unknown) {
        addLog(`scan_dir failed for "${target}", falling back: ${String(err)}`);
        try {
          const demo = await invoke<FileItem[]>("get_demo_items");
          setBrowserItems(demo);
        } catch {
          // ignore
        }
      } finally {
        setIsScanning(false);
      }
    },
    [addLog]
  );

  const handleNavigateParent = useCallback(() => {
    if (parentPath) {
      scanDirectory(parentPath);
    }
  }, [parentPath, scanDirectory]);

  useEffect(() => {
    invoke<LandlockStatus>("get_landlock_status")
      .then((status) => {
        setLandlock(status);
        addLog(`Landlock LSM status: supported=${status.supported}, ABI=${status.abi_version}`);
      })
      .catch((err) => console.error(err));

    invoke<AppConfig>("get_app_config")
      .then((cfg) => setAppConfig(cfg))
      .catch((err) => console.error(err));

    // Get system paths and start scanning from Home
    invoke<SystemPathsDto>("get_system_paths")
      .then((paths) => {
        setSystemPaths(paths);
        scanDirectory(paths.home_raw_base64 || paths.home_dir);
      })
      .catch(() => {
        scanDirectory(".");
      });
  }, [scanDirectory, addLog]);

  const handleStageFile = async (item: FileItem) => {
    try {
      const staged = await invoke<StagedItemDto>("stage_file", { pathBase64: item.raw_base64 });
      setStagedItems((prev) => [...prev.filter((s) => s.stage_id !== staged.stage_id), staged]);

      const stagedAsFileItem: FileItem = {
        ...item,
        item_id: staged.stage_id, // Map directly to stage_id for consistent unstage
        raw_base64: staged.staged_raw_base64,
      };

      setStagedFiles((prev) => {
        const filtered = prev.filter(
          (f) =>
            f.item_id !== staged.stage_id &&
            f.item_id !== item.item_id &&
            f.display_name !== staged.display_name
        );
        return [...filtered, stagedAsFileItem];
      });

      setSelectedItem(stagedAsFileItem);
      addHistory(`Staged "${item.display_name}" via ${staged.clone_method}`);
      addLog(`Staged file "${item.display_name}" (${staged.clone_method}) -> ${staged.staged_path}`);
    } catch (err) {
      addLog(`Failed to stage "${item.display_name}": ${String(err)}`);
    }
  };

  const handleStageAll = async () => {
    const candidates = browserItems.filter(
      (b) => !b.is_dir && !stagedItems.some((s) => s.original_item_id === b.item_id)
    );
    for (const item of candidates) {
      await handleStageFile(item);
    }
  };

  const handleUnstageItem = async (stageId: string) => {
    try {
      await invoke("unstage_file", { stageId });
      const target = stagedItems.find(
        (s) =>
          s.stage_id === stageId ||
          s.original_item_id === stageId ||
          s.display_name === stageId
      );
      const effectiveStageId = target ? target.stage_id : stageId;

      setStagedItems((prev) => prev.filter((s) => s.stage_id !== effectiveStageId));
      setStagedFiles((prev) =>
        prev.filter(
          (f) =>
            f.item_id !== effectiveStageId &&
            f.item_id !== stageId &&
            (!target || (f.display_name !== target.display_name && f.raw_base64 !== target.staged_raw_base64))
        )
      );

      if (selectedItem?.item_id === stageId || selectedItem?.item_id === effectiveStageId) {
        setSelectedItem(null);
      }

      const displayName = target?.display_name ?? stageId;
      addHistory(`Unstaged "${displayName}"`);
      addLog(`Unstaged file "${displayName}" from staging area`);
    } catch (err) {
      addLog(`Failed to unstage item ${stageId}: ${String(err)}`);
    }
  };

  const handleClearWorkspace = async () => {
    try {
      await invoke("clear_staged_items");
      setStagedFiles([]);
      setStagedItems([]);
      addHistory("Cleared all items from workspace-temp/");
      addLog("Cleared workspace-temp/ staging area");
    } catch (err) {
      addLog(`Failed to clear workspace: ${String(err)}`);
    }
  };

  const handleResetStagedItem = async (stageId: string) => {
    try {
      const resetItem = await invoke<StagedItemDto>("reset_staged_file", { stageId });
      setStagedItems((prev) => prev.map((s) => (s.stage_id === stageId ? resetItem : s)));
      addHistory(`Reset "${resetItem.display_name}" in workspace-temp to original state`);
      addLog(`Reset staged file "${resetItem.display_name}" back to original`);
    } catch (err) {
      addLog(`Failed to reset staged item ${stageId}: ${String(err)}`);
    }
  };

  const handleReflectStagedItem = async (stageId: string) => {
    try {
      const updated = await invoke<StagedItemDto>("reflect_staged_file", { stageId });
      setStagedItems((prev) => prev.map((s) => (s.stage_id === stageId ? updated : s)));
      addHistory(`Reflected changes of "${updated.display_name}" back to original safely`);
      addLog(`Reflected staged item "${updated.display_name}" to original: ${updated.original_snapshot.canonical_path}`);
      // Refresh browser items to update size/mtime
      await scanDirectory(currentPath);
    } catch (err) {
      addLog(`Failed to reflect staged item ${stageId}: ${String(err)}`);
      throw err;
    }
  };

  const handleExtractText = async (item: FileItem) => {
    try {
      setExtractLoading(true);
      addLog(`Starting text extraction for "${item.display_name}"...`);
      const res = await invoke<ExtractedTextDto>("extract_file_text", {
        pathBase64: item.raw_base64,
        options: null,
      });
      setExtractedInfo(res);
      addHistory(`Extracted text from "${item.display_name}" (${res.extracted_chars} chars, ${res.encoding_used})`);
      addLog(`Extraction completed: ${res.tier_used}, encoding: ${res.encoding_used}, chars: ${res.extracted_chars}`);
    } catch (err: unknown) {
      const msg = typeof err === "string" ? err : String(err);
      addLog(`Extraction error for "${item.display_name}": ${msg}`);
    } finally {
      setExtractLoading(false);
    }
  };

  const handleResetLayout = () => {
    setSidebarVisible(true);
    setBottomVisible(true);
    setLayoutKey((prev) => prev + 1);
    addLog("Reset 6-region pane layout");
  };

  const quickDirs = [
    {
      name: t("tree.home"),
      path: systemPaths?.home_raw_base64 || systemPaths?.home_dir || "~",
      icon: "home" as const,
    },
    {
      name: t("tree.current"),
      path: systemPaths?.current_raw_base64 || systemPaths?.current_dir || ".",
      icon: "folder" as const,
    },
    {
      name: t("tree.root"),
      path: "/",
      icon: "root" as const,
    },
  ];

  const subdirs = browserItems.filter((i) => i.is_dir);

  return (
    <div className="cuddle-root-app">
      <Header
        currentLang={currentLang}
        onToggleLang={toggleLanguage}
        landlockSupported={landlock?.supported ?? false}
        landlockAbi={landlock?.abi_version ?? null}
        sidebarVisible={sidebarVisible}
        bottomVisible={bottomVisible}
        onToggleSidebar={() => setSidebarVisible((v) => !v)}
        onToggleBottom={() => setBottomVisible((v) => !v)}
        onResetLayout={handleResetLayout}
      />

      <main className="workbench-workspace-layout">
        <Group
          key={layoutKey}
          orientation="horizontal"
          id="cuddle-main-h-panels"
          className="main-panel-group"
        >
          {/* LEFT COLUMN: Top: Search Results, Bottom: Folder Tree */}
          <Panel defaultSize={22} minSize={14} className="panel-left-col">
            <Group orientation="vertical" id="cuddle-left-v-panels">
              <Panel defaultSize={48} minSize={20} className="subpanel-wrap">
                <SearchPanel
                  onSelectItem={(partialItem) => {
                    const found = browserItems.find((i) => i.item_id === partialItem.item_id);
                    if (found) {
                      setSelectedItem(found);
                    } else if (partialItem.item_id && partialItem.display_name && partialItem.raw_base64) {
                      const pseudo: FileItem = {
                        item_id: partialItem.item_id,
                        raw_base64: partialItem.raw_base64,
                        display_name: partialItem.display_name,
                        parent_raw_base64: null,
                        is_dir: false,
                        is_symlink: false,
                        size: BigInt(partialItem.size ?? 0),
                        mtime_ms: BigInt(partialItem.mtime_ms ?? 0),
                        readonly: false,
                        inode: 0n,
                        dev: 0n,
                      };
                      setSelectedItem(pseudo);
                    }
                  }}
                  onLog={addLog}
                />
              </Panel>

              <Separator className="cuddle-resize-handle-v" />

              <Panel defaultSize={52} minSize={20} className="subpanel-wrap">
                <FolderTreePanel
                  currentPath={currentPath}
                  onNavigatePath={(p) => scanDirectory(p)}
                  onNavigateParent={handleNavigateParent}
                  hasParent={!!parentPath}
                  quickDirs={quickDirs}
                  subdirs={subdirs}
                  isScanning={isScanning}
                  onRefresh={() => scanDirectory(currentPath)}
                />
              </Panel>
            </Group>
          </Panel>

          <Separator className="cuddle-resize-handle-h" />

          {/* CENTER & BOTTOM COLUMN */}
          <Panel defaultSize={sidebarVisible ? 56 : 78} minSize={30} className="panel-center-col">
            <Group orientation="vertical" id="cuddle-center-v-panels">
              {/* TOP: Center-Left Browser & Center-Right Workspace */}
              <Panel defaultSize={bottomVisible ? 68 : 100} minSize={30} className="subpanel-wrap">
                <Group orientation="horizontal" id="cuddle-center-h-panels">
                  {/* Central-Left: File Browser */}
                  <Panel defaultSize={50} minSize={25} className="subpanel-wrap">
                    <FileBrowserPanel
                      currentPath={currentPath}
                      items={browserItems}
                      selectedItem={selectedItem}
                      onSelect={(item) => setSelectedItem(item)}
                      onOpenDir={(dir) => scanDirectory(dir.raw_base64)}
                      onStage={handleStageFile}
                      onStageAll={handleStageAll}
                      onNavigateParent={handleNavigateParent}
                      onNavigatePath={(p) => scanDirectory(p)}
                      hasParent={!!parentPath}
                      isScanning={isScanning}
                      onRefresh={() => scanDirectory(currentPath)}
                    />
                  </Panel>

                  <Separator className="cuddle-resize-handle-h" />

                  {/* Central-Right: Workspace-temp */}
                  <Panel defaultSize={50} minSize={25} className="subpanel-wrap">
                    <WorkspacePanel
                      stagedFiles={stagedFiles}
                      stagedItems={stagedItems}
                      selectedId={selectedItem?.item_id ?? null}
                      onSelect={(item) => setSelectedItem(item)}
                      onClearWorkspace={handleClearWorkspace}
                      onUnstageItem={handleUnstageItem}
                      onOpenBulkRename={() => setBulkRenameOpen(true)}
                    />
                  </Panel>
                </Group>
              </Panel>

              {bottomVisible && (
                <>
                  <Separator className="cuddle-resize-handle-v" />
                  <Panel defaultSize={32} minSize={15} className="subpanel-wrap">
                    <BottomTabPanel
                      selectedItem={selectedItem}
                      extractedInfo={extractedInfo}
                      appConfig={appConfig}
                      logs={logs}
                      history={history}
                      stagedItems={stagedItems}
                      onResetStagedItem={handleResetStagedItem}
                      onReflectStagedItem={handleReflectStagedItem}
                      onClose={() => setBottomVisible(false)}
                    />
                  </Panel>
                </>
              )}
            </Group>
          </Panel>

          {/* RIGHT COLUMN: Details Sidebar */}
          {sidebarVisible && (
            <>
              <Separator className="cuddle-resize-handle-h" />
              <Panel defaultSize={22} minSize={15} className="panel-right-col subpanel-wrap">
                <DetailsSidebarPanel
                  item={selectedItem}
                  onClose={() => setSidebarVisible(false)}
                  onExtractText={handleExtractText}
                  onStageFile={handleStageFile}
                  isStaged={stagedItems.some((s) => s.original_item_id === selectedItem?.item_id)}
                  extractLoading={extractLoading}
                  extractedInfo={extractedInfo}
                />
              </Panel>
            </>
          )}
        </Group>
      </main>

      {/* Bulk Rename & Normalization Modal (Phase 5.1) */}
      <BulkRenameModal
        isOpen={bulkRenameOpen}
        onClose={() => setBulkRenameOpen(false)}
        items={stagedItems.length > 0 ? stagedItems : stagedFiles}
        onConfirmPlan={(plan) => {
          addLog(`Bulk rename dry-run plan confirmed: ${plan.batch_id} (${plan.changed_count} changed)`);
          setBulkRenameOpen(false);
        }}
      />

      <footer className="cuddle-global-statusbar">
        <div className="statusbar-left">
          <span className="status-label">Cuddle</span>
          <span className="status-sep">|</span>
          <span>Phase 5.1 Bulk Normalization &amp; Dry-run Engine Active</span>
        </div>
        <div className="statusbar-right">
          <span>Staged: {stagedFiles.length} files</span>
          <span className="status-sep">|</span>
          <span>Nerd Font: Pre-required</span>
        </div>
      </footer>
    </div>
  );
}

export default App;
