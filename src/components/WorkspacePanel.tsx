import { Briefcase, ShieldCheck, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { FileItem } from "../types/FileItem";
import { VirtualFileList } from "./VirtualFileList";

interface WorkspacePanelProps {
  stagedFiles: FileItem[];
  selectedId: string | null;
  onSelect: (item: FileItem) => void;
  onClearWorkspace: () => void;
}

export function WorkspacePanel({
  stagedFiles,
  selectedId,
  onSelect,
  onClearWorkspace,
}: WorkspacePanelProps) {
  const { t } = useTranslation();

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
          {stagedFiles.length > 0 && (
            <button
              type="button"
              className="panel-header-btn-danger"
              onClick={onClearWorkspace}
              title="Clear Workspace"
            >
              <Trash2 size={13} />
              <span>Clear</span>
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
          isWorkspace
          emptyMessage={t("workspace.empty_state")}
        />
      </div>

      <div className="browser-footer-status">
        <span>{t("browser.items_count", { count: stagedFiles.length })}</span>
        <span className="reflink-note">{t("workspace.reflink_enabled")}</span>
      </div>
    </div>
  );
}
