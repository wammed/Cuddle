import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { Archive, X, Trash2, AlertTriangle, RefreshCw, FolderArchive } from "lucide-react";
import type { BackupResultDto } from "../types/BackupResultDto";

interface ExitBackupModalProps {
  isOpen: boolean;
  stagedCount: number;
  onClose: () => void;
  onDiscard: () => void;
  onBackupSuccess: (result: BackupResultDto) => void;
}

export function ExitBackupModal({
  isOpen,
  stagedCount,
  onClose,
  onDiscard,
  onBackupSuccess,
}: ExitBackupModalProps) {
  const { t } = useTranslation();
  const [destPath, setDestPath] = useState<string>("");
  const [defaultPath, setDefaultPath] = useState<string>("");
  const [isArchiving, setIsArchiving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      setError(null);
      invoke<string>("get_default_backup_path")
        .then((path) => {
          setDefaultPath(path);
          setDestPath(path);
        })
        .catch((err) => {
          console.error("Failed to get default backup path:", err);
          setDefaultPath("");
        });
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleExecuteBackup = async () => {
    try {
      setIsArchiving(true);
      setError(null);
      const res = await invoke<BackupResultDto>("create_workspace_backup", {
        destinationPath: destPath.trim() ? destPath.trim() : null,
      });
      onBackupSuccess(res);
      onClose();
    } catch (err) {
      console.error("Backup failed:", err);
      setError(typeof err === "string" ? err : String(err));
    } finally {
      setIsArchiving(false);
    }
  };

  const handleResetToDefault = () => {
    setDestPath(defaultPath);
  };

  return (
    <div className="modal-backdrop">
      <div className="exit-backup-dialog">
        <div className="dialog-header">
          <div className="dialog-title-group">
            <Archive size={18} className="text-cyan-400" />
            <h4>{t("backup.dialog_title")}</h4>
          </div>
          <button
            type="button"
            className="panel-header-btn"
            onClick={onClose}
            title={t("common.close")}
            disabled={isArchiving}
          >
            <X size={14} />
          </button>
        </div>

        <div className="dialog-body">
          <div className="dialog-warning-banner">
            <AlertTriangle size={18} className="text-amber-400 shrink-0" />
            <p className="dialog-warning-text">
              {t("backup.dialog_desc", { count: stagedCount })}
            </p>
          </div>

          <div className="backup-destination-card">
            <div className="dest-label-row">
              <label htmlFor="backup-dest-input" className="dest-label font-medium">
                {t("backup.dest_path_label")}
              </label>
              {defaultPath && destPath !== defaultPath && (
                <button
                  type="button"
                  className="reset-path-btn"
                  onClick={handleResetToDefault}
                  title={t("backup.reset_path")}
                >
                  <RefreshCw size={11} />
                  <span>{t("backup.reset_path")}</span>
                </button>
              )}
            </div>

            <div className="dest-input-wrap">
              <FolderArchive size={15} className="input-icon text-muted" />
              <input
                id="backup-dest-input"
                type="text"
                className="dest-input font-mono"
                value={destPath}
                onChange={(e) => setDestPath(e.target.value)}
                placeholder="/path/to/backup.tar.gz"
                disabled={isArchiving}
              />
            </div>
            <p className="dest-hint-text text-muted">{t("backup.dest_hint")}</p>
          </div>

          {error && (
            <div className="dialog-error-banner">
              <AlertTriangle size={15} className="text-red-400" />
              <span className="error-text">{error}</span>
            </div>
          )}
        </div>

        <div className="dialog-footer">
          <button
            type="button"
            className="dialog-btn secondary"
            onClick={onClose}
            disabled={isArchiving}
          >
            {t("backup.cancel_btn")}
          </button>

          <button
            type="button"
            className="dialog-btn secondary text-rose-400"
            onClick={onDiscard}
            disabled={isArchiving}
            title={t("backup.discard_btn")}
          >
            <Trash2 size={13} />
            <span>{t("backup.discard_btn")}</span>
          </button>

          <button
            type="button"
            className="dialog-btn primary backup-action-btn"
            onClick={handleExecuteBackup}
            disabled={isArchiving}
          >
            <Archive size={14} />
            <span>{isArchiving ? t("backup.archiving") : t("backup.backup_btn")}</span>
          </button>
        </div>
      </div>
    </div>
  );
}
