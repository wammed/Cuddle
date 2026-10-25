import { useState, useEffect } from "react";
import { FolderOpen, RefreshCw, CornerDownRight, ListPlus, ArrowUp } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { FileItem } from "../types/FileItem";
import type { Density } from "../types/Density";
import { VirtualFileList } from "./VirtualFileList";

interface FileBrowserPanelProps {
  currentPath: string;
  items: FileItem[];
  selectedItem: FileItem | null;
  onSelect: (item: FileItem) => void;
  onOpenDir: (item: FileItem) => void;
  onStage: (item: FileItem) => void;
  onStageAll: () => void;
  onNavigateParent?: () => void;
  onNavigatePath?: (path: string) => void;
  onOpenPreview?: (item: FileItem) => void;
  onOpenChmod?: (item: FileItem) => void;
  hasParent?: boolean;
  isScanning: boolean;
  onRefresh: () => void;
  density?: Density;
}

export function FileBrowserPanel({
  currentPath,
  items,
  selectedItem,
  onSelect,
  onOpenDir,
  onStage,
  onStageAll,
  onNavigateParent,
  onNavigatePath,
  onOpenPreview,
  onOpenChmod,
  hasParent = false,
  isScanning,
  onRefresh,
  density = "normal",
}: FileBrowserPanelProps) {
  const { t } = useTranslation();
  const [isEditingPath, setIsEditingPath] = useState(false);
  const [pathInput, setPathInput] = useState(currentPath);

  useEffect(() => {
    setPathInput(currentPath);
    setIsEditingPath(false);
  }, [currentPath]);

  const handlePathSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (pathInput.trim() && onNavigatePath) {
      onNavigatePath(pathInput.trim());
      setIsEditingPath(false);
    }
  };

  const isFileSelected = selectedItem && !selectedItem.is_dir;

  return (
    <div className="panel-container file-browser-container">
      {/* Row 1: Title and Header Actions */}
      <div className="panel-header browser-header-top-row">
        <div className="panel-title-group browser-title-left">
          <FolderOpen size={16} />
          <h3>{t("panels.browser_title")}</h3>
        </div>

        <div className="panel-header-actions">
          {isFileSelected && (
            <button
              type="button"
              className="panel-header-btn-primary"
              onClick={() => onStage(selectedItem)}
              title={`${t("browser.stage_selected")}: ${selectedItem.display_name}`}
            >
              <CornerDownRight size={13} />
              <span>{t("browser.stage_selected")}</span>
            </button>
          )}

          <button
            type="button"
            className="panel-header-btn-subtle"
            onClick={onStageAll}
            disabled={items.length === 0}
            title={t("browser.stage_all")}
          >
            <ListPlus size={13} />
            <span>{t("browser.stage_all")}</span>
          </button>

          <button
            type="button"
            className={`panel-header-btn ${isScanning ? "spinning" : ""}`}
            onClick={onRefresh}
            title="Refresh"
          >
            <RefreshCw size={13} />
          </button>
        </div>
      </div>

      {/* Row 2: Path Bar & Navigation Toolbar (Full Width) */}
      <div className="browser-path-toolbar-row">
        {hasParent && onNavigateParent && (
          <button
            type="button"
            className="browser-nav-btn"
            onClick={onNavigateParent}
            title={t("browser.navigate_up")}
          >
            <ArrowUp size={13} />
          </button>
        )}

        {isEditingPath ? (
          <form onSubmit={handlePathSubmit} className="browser-path-form">
            <input
              type="text"
              className="browser-path-input font-mono"
              value={pathInput}
              onChange={(e) => setPathInput(e.target.value)}
              onBlur={() => setIsEditingPath(false)}
              autoFocus
              placeholder={t("browser.path_placeholder")}
            />
          </form>
        ) : (
          <span
            className="current-path-badge clickable font-mono"
            onClick={() => setIsEditingPath(true)}
            title="Click to edit path"
          >
            {currentPath}
          </span>
        )}
      </div>

      <div className="browser-table-header">
        <div className="col-header col-icon" />
        <div className="col-header col-name">{t("browser.col_name")}</div>
        <div className="col-header col-size">{t("browser.col_size")}</div>
        <div className="col-header col-mtime">{t("browser.col_mtime")}</div>
        <div className="col-header col-inode">{t("browser.col_inode")}</div>
        <div className="col-header col-actions" />
      </div>

      <div className="browser-table-body">
        <VirtualFileList
          items={items}
          selectedId={selectedItem?.item_id ?? null}
          onSelect={onSelect}
          onOpenDir={onOpenDir}
          onStage={onStage}
          onOpenPreview={onOpenPreview}
          onOpenChmod={onOpenChmod}
          emptyMessage={t("browser.empty_folder")}
          density={density}
        />
      </div>

      <div className="browser-footer-status">
        <span>{t("browser.items_count", { count: items.length })}</span>
        {selectedItem && <span>{t("browser.selected_count", { count: 1 })}</span>}
      </div>
    </div>
  );
}
