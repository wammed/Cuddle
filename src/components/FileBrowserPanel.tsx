import { useState, useEffect, useMemo } from "react";
import { FolderOpen, RefreshCw, CornerDownRight, ListPlus, ArrowUp, ChevronRight, Edit3 } from "lucide-react";
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
  onOpenTimestamp?: (item: FileItem) => void;
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
  onOpenTimestamp,
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

  // Interactive Breadcrumb Trail (Phase 10 / Task 2-9)
  const breadcrumbs = useMemo(() => {
    if (!currentPath || currentPath === "/") {
      return [{ name: "/", path: "/" }];
    }
    const parts = currentPath.split("/").filter(Boolean);
    const segments = [{ name: "/", path: "/" }];
    let accumulated = "";
    for (const part of parts) {
      accumulated += "/" + part;
      segments.push({
        name: part,
        path: accumulated,
      });
    }
    return segments;
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
            title={t("browser.refresh")}
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
          <div
            className="current-path-badge clickable font-mono"
            onClick={() => setIsEditingPath(true)}
            title={t("browser.click_edit_path")}
          >
            {/* Hidden fallback text for test compatibility & assistive tech */}
            <span className="sr-only">{currentPath}</span>

            {/* Interactive Breadcrumb Trail (Phase 10 / Task 2-9) */}
            <div className="breadcrumb-trail">
              {breadcrumbs.map((seg, idx) => (
                <span key={seg.path} className="breadcrumb-segment-wrapper">
                  {idx > 0 && <ChevronRight size={11} className="breadcrumb-separator" />}
                  <button
                    type="button"
                    className="breadcrumb-segment-btn"
                    onClick={(e) => {
                      e.stopPropagation();
                      if (onNavigatePath) {
                        onNavigatePath(seg.path);
                      }
                    }}
                    title={seg.path}
                  >
                    {seg.name}
                  </button>
                </span>
              ))}
              <Edit3 size={11} className="breadcrumb-edit-icon" />
            </div>
          </div>
        )}
      </div>

      <div className="browser-table-header">
        <div className="col-header col-header-icon" />
        <div className="col-header col-header-name">{t("browser.col_name_details")}</div>
        <div className="col-header col-header-actions">{t("browser.col_actions")}</div>
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
          onOpenTimestamp={onOpenTimestamp}
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
