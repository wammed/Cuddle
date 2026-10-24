import { Folder, HardDrive, Home, FolderTree, RefreshCw, ArrowUp } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { FileItem } from "../types/FileItem";

interface FolderTreePanelProps {
  currentPath: string;
  onNavigatePath: (path: string) => void;
  onNavigateParent?: () => void;
  hasParent?: boolean;
  quickDirs: { name: string; path: string; icon: "home" | "root" | "folder" }[];
  subdirs: FileItem[];
  isScanning: boolean;
  onRefresh: () => void;
}

export function FolderTreePanel({
  currentPath,
  onNavigatePath,
  onNavigateParent,
  hasParent = false,
  quickDirs,
  subdirs,
  isScanning,
  onRefresh,
}: FolderTreePanelProps) {
  const { t } = useTranslation();

  return (
    <div className="panel-container folder-tree-container">
      <div className="panel-header">
        <div className="panel-title-group">
          <FolderTree size={15} />
          <h3>{t("panels.tree_title")}</h3>
        </div>

        <button
          type="button"
          className={`panel-header-btn ${isScanning ? "spinning" : ""}`}
          onClick={onRefresh}
          title="Refresh"
        >
          <RefreshCw size={13} />
        </button>
      </div>

      <div className="tree-quick-section">
        <span className="section-label">{t("tree.quick_access")}</span>
        <div className="quick-dirs-list">
          {quickDirs.map((dir) => (
            <button
              key={dir.path}
              type="button"
              className={`quick-dir-btn ${currentPath === dir.path ? "active" : ""}`}
              onClick={() => onNavigatePath(dir.path)}
            >
              {dir.icon === "home" ? (
                <Home size={14} className="quick-icon" />
              ) : dir.icon === "root" ? (
                <HardDrive size={14} className="quick-icon" />
              ) : (
                <Folder size={14} className="quick-icon" />
              )}
              <span className="quick-dir-name">{dir.name}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="tree-subdirs-section">
        <div className="tree-subdirs-header">
          <span className="section-label" title={currentPath}>
            {currentPath}
          </span>
          {hasParent && onNavigateParent && (
            <button
              type="button"
              className="tree-parent-btn"
              onClick={onNavigateParent}
              title={t("tree.parent_dir")}
            >
              <ArrowUp size={12} />
              <span>{t("tree.parent_dir")}</span>
            </button>
          )}
        </div>

        <div className="tree-subdirs-scroll">
          {subdirs.map((dir) => (
            <div
              key={dir.item_id}
              className="tree-node-item"
              onClick={() => onNavigatePath(dir.raw_base64)}
              title={dir.display_name}
            >
              <Folder size={14} className="node-folder-icon" />
              <span className="node-name">{dir.display_name}</span>
            </div>
          ))}
          {subdirs.length === 0 && !isScanning && (
            <p className="tree-empty-note">-</p>
          )}
        </div>
      </div>
    </div>
  );
}
