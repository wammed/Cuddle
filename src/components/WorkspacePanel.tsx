import { useMemo } from "react";
import { Briefcase, ShieldCheck, Trash2, Sparkles, RotateCcw, Archive, Workflow, FileCode, Clock, FolderArchive } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { FileItem } from "../types/FileItem";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { Density } from "../types/Density";
import { VirtualFileList } from "./VirtualFileList";

interface WorkspacePanelProps {
  stagedFiles: FileItem[];
  stagedItems?: StagedItemDto[];
  selectedId: string | null;
  canUndo?: boolean;
  onSelect: (item: FileItem) => void;
  onClearWorkspace: () => void;
  onUnstageItem?: (stageId: string) => void;
  onOpenBulkRename?: () => void;
  onOpenChmod?: (items?: FileItem[]) => void;
  onOpenTimestamp?: (items?: FileItem[]) => void;
  onOpenTarExport?: () => void;
  onUndo?: () => void;
  onOpenBackup?: () => void;
  onOpenPipeline?: () => void;
  onOpenPreview?: (item: FileItem) => void;
  density?: Density;
}

export function WorkspacePanel({
  stagedFiles,
  stagedItems = [],
  selectedId,
  canUndo = false,
  onSelect,
  onClearWorkspace,
  onUnstageItem,
  onOpenBulkRename,
  onOpenChmod,
  onOpenTimestamp,
  onOpenTarExport,
  onUndo,
  onOpenBackup,
  onOpenPipeline,
  onOpenPreview,
  density = "normal",
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

  // Map item identifiers to current stage status (Staged, Modified, Reflected, Conflict)
  const statusMap = useMemo(() => {
    const map: Record<string, string> = {};
    for (const s of stagedItems) {
      map[s.stage_id] = s.status;
      map[s.original_item_id] = s.status;
      map[s.raw_base64] = s.status;
      map[s.staged_raw_base64] = s.status;
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
      {/* Row 1: Workspace Title, Protection Badge, and Clear All */}
      <div className="panel-header workspace-header-top-row">
        <div className="panel-title-group">
          <Briefcase size={16} />
          <h3>{t("panels.workspace_title")}</h3>
          <span className="workspace-badge-protected">
            <ShieldCheck size={13} />
            <span>workspace-temp/</span>
          </span>
        </div>

        <div className="panel-header-actions">
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

      {/* Row 2: Workspace Operations Action Toolbar (Pipeline, Rename, Undo, Backup) */}
      {stagedFiles.length > 0 && (
        <div className="workspace-action-toolbar-row">
          <div className="workspace-toolbar-actions">
            {onOpenPipeline && (
              <button
                type="button"
                className="panel-header-btn-pipeline"
                onClick={onOpenPipeline}
                title={t("pipeline.action_open_modal")}
              >
                <Workflow size={13} />
                <span>{t("pipeline.action_open_modal")}</span>
              </button>
            )}

            {onOpenBulkRename && (
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

            {onOpenChmod && (
              <button
                type="button"
                className="panel-header-btn-secondary"
                onClick={() => onOpenChmod()}
                title={t("chmod.action_open_modal")}
              >
                <FileCode size={13} />
                <span>{t("chmod.action_open_modal")}</span>
              </button>
            )}

            {onOpenTimestamp && (
              <button
                type="button"
                className="panel-header-btn-secondary"
                onClick={() => onOpenTimestamp()}
                title={t("timestamp.action_open_modal")}
              >
                <Clock size={13} />
                <span>{t("timestamp.action_open_modal")}</span>
              </button>
            )}

            {canUndo && onUndo && (
              <button
                type="button"
                className="panel-header-btn-undo"
                onClick={onUndo}
                title={t("workspace.undo_rename_btn")}
              >
                <RotateCcw size={13} />
                <span>{t("workspace.undo_rename_btn")}</span>
              </button>
            )}

            {onOpenTarExport && (
              <button
                type="button"
                className="panel-header-btn-secondary"
                onClick={onOpenTarExport}
                title={t("tar_export.action_open_modal")}
              >
                <FolderArchive size={13} />
                <span>{t("tar_export.action_open_modal")}</span>
              </button>
            )}

            {onOpenBackup && (
              <button
                type="button"
                className="panel-header-btn-secondary"
                onClick={onOpenBackup}
                title={t("backup.action_open_modal")}
              >
                <Archive size={13} />
                <span>{t("backup.action_open_modal")}</span>
              </button>
            )}
          </div>
        </div>
      )}

      <div className="workspace-security-banner">
        <ShieldCheck size={14} className="banner-icon" />
        <p>{t("workspace.original_protected")}</p>
      </div>

      <div className="browser-table-header">
        <div className="col-header col-header-icon" />
        <div className="col-header col-header-name">{t("browser.col_name_details")}</div>
        <div className="col-header col-header-actions">{t("browser.col_actions")}</div>
      </div>

      <div className="browser-table-body">
        <VirtualFileList
          items={stagedFiles}
          selectedId={selectedId}
          onSelect={onSelect}
          onUnstage={handleUnstage}
          onOpenPreview={onOpenPreview}
          onOpenChmod={(item) => onOpenChmod ? onOpenChmod([item]) : undefined}
          onOpenTimestamp={(item) => onOpenTimestamp ? onOpenTimestamp([item]) : undefined}
          cloneMethodMap={cloneMethodMap}
          statusMap={statusMap}
          isWorkspace
          emptyMessage={t("workspace.empty_state")}
          density={density}
        />
      </div>

      <div className="browser-footer-status">
        <span>{t("workspace.staged_files_count", { count: stagedFiles.length })}</span>
        <span className="reflink-note">{t("workspace.reflink_enabled")}</span>
      </div>
    </div>
  );
}
