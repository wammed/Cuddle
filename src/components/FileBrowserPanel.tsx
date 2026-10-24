import { FolderOpen, RefreshCw, CornerDownRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { FileItem } from "../types/FileItem";
import { VirtualFileList } from "./VirtualFileList";

interface FileBrowserPanelProps {
  currentPath: string;
  items: FileItem[];
  selectedId: string | null;
  onSelect: (item: FileItem) => void;
  onOpenDir: (item: FileItem) => void;
  onStage: (item: FileItem) => void;
  onStageAll: () => void;
  isScanning: boolean;
  onRefresh: () => void;
}

export function FileBrowserPanel({
  currentPath,
  items,
  selectedId,
  onSelect,
  onOpenDir,
  onStage,
  onStageAll,
  isScanning,
  onRefresh,
}: FileBrowserPanelProps) {
  const { t } = useTranslation();

  return (
    <div className="panel-container file-browser-container">
      <div className="panel-header">
        <div className="panel-title-group">
          <FolderOpen size={16} />
          <h3>{t("panels.browser_title")}</h3>
          <span className="current-path-badge" title={currentPath}>
            {currentPath}
          </span>
        </div>

        <div className="panel-header-actions">
          <button
            type="button"
            className="panel-header-btn-subtle"
            onClick={onStageAll}
            disabled={items.length === 0}
            title={t("browser.stage_to_workspace")}
          >
            <CornerDownRight size={13} />
            <span>{t("browser.stage_to_workspace")}</span>
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
          selectedId={selectedId}
          onSelect={onSelect}
          onOpenDir={onOpenDir}
          onStage={onStage}
          emptyMessage={t("browser.empty_folder")}
        />
      </div>

      <div className="browser-footer-status">
        <span>{t("browser.items_count", { count: items.length })}</span>
        {selectedId && <span>{t("browser.selected_count", { count: 1 })}</span>}
      </div>
    </div>
  );
}
