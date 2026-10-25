import { useMemo } from "react";
import { Briefcase, ShieldCheck, Trash2, Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { FileItem } from "../types/FileItem";
import type { StagedItemDto } from "../types/StagedItemDto";
import { VirtualFileList } from "./VirtualFileList";

interface WorkspacePanelProps {
  stagedFiles: FileItem[];
  stagedItems?: StagedItemDto[];
  selectedId: string | null;
  onSelect: (item: FileItem) => void;
  onClearWorkspace: () => void;
  onUnstageItem?: (stageId: string) => void;
  onOpenBulkRename?: () => void;
}

export function WorkspacePanel({
  stagedFiles,
  stagedItems = [],
  selectedId,
  onSelect,
  onClearWorkspace,
  onUnstageItem,
  onOpenBulkRename,
}: WorkspacePanelProps) {
  const { t } = useTranslation();

  // Map item identifiers to clone method (Reflink or StandardCopy)
  const cloneMethodMap = useMemo(() => {
    const map: Record<string, "Reflink" | "StandardCopy"> = {};
    for (const s of stagedItems) {
      map[s.stage_id] = s.clone_method;
      map[s.original_item_id] = s.clone_method;
      map[s.raw_base64] = s.clone_method;
      map[s.staged_raw_base64] = s.clone_method;
    }
    return map;
  }, [stagedItems]);

  const handleUnstage = (item: FileItem) => {
    if (!onUnstageItem) return;
    const match = stagedItems.find(
      (s) =>
        s.stage_id === item.item_id ||
        s.original_item_id === item.item_id ||
        s.raw_base64 === item.raw_base64 ||
        s.staged_raw_base64 === item.raw_base64 ||
        (item.inode > 0n &&
          s.original_snapshot.inode === item.inode &&
          s.original_snapshot.dev === item.dev) ||
        s.display_name === item.display_name
    );
    const targetStageId = match ? match.stage_id : item.item_id;
    onUnstageItem(targetStageId);
  };

  return (
    <div className="panel-container workspace-panel-container">
      <div className="panel-header">
        <div className="panel-title-group">
          <Briefcase size={16} />
          <h3>{t("panels.workspace_title")}</h3>
          <span className="workspace-badge-protected">
            <ShieldCheck size={13} />
            <span>workspace-temp/</span>
          </span>
        </div>

        <div className="panel-header-actions">
          {stagedFiles.length > 0 && onOpenBulkRename && (
            <button
              type="button"
              className="panel-header-btn-primary"
              onClick={onOpenBulkRename}
              title={t("workspace.bulk_rename_btn")}
            >
              <Sparkles size={13} />
              <span>{t("workspace.bulk_rename_btn")}</span>
            </button>
          )}

          {stagedFiles.length > 0 && (
            <button
              type="button"
              className="panel-header-btn-danger"
              onClick={onClearWorkspace}
              title={t("workspace.clear_all_btn")}
            >
              <Trash2 size={13} />
              <span>{t("workspace.clear_all_btn")}</span>
            </button>
          )}
        </div>
      </div>

      <div className="workspace-security-banner">
        <ShieldCheck size={14} className="banner-icon" />
        <p>{t("workspace.original_protected")}</p>
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
          items={stagedFiles}
          selectedId={selectedId}
          onSelect={onSelect}
          onUnstage={handleUnstage}
          cloneMethodMap={cloneMethodMap}
          isWorkspace
          emptyMessage={t("workspace.empty_state")}
        />
      </div>

      <div className="browser-footer-status">
        <span>{t("workspace.staged_files_count", { count: stagedFiles.length })}</span>
        <span className="reflink-note">{t("workspace.reflink_enabled")}</span>
      </div>
    </div>
  );
}
