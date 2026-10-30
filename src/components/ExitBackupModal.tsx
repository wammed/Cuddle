import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { Archive, X, Trash2, AlertTriangle, RefreshCw, FolderArchive, Layers } from "lucide-react";
import type { BackupResultDto } from "../types/BackupResultDto";
import type { BackupArchiveInfoDto } from "../types/BackupArchiveInfoDto";
import type { AppConfig } from "../types/AppConfig";
import { formatBytes, formatTimestamp } from "../utils/format";

interface ExitBackupModalProps {
  isOpen: boolean;
  stagedCount: number;
  onClose: () => void;
  onDiscard: () => void;
  onBackupSuccess: (result: BackupResultDto) => void;
  onBackupAndContinue?: (result: BackupResultDto) => void;
}

export function ExitBackupModal({
  isOpen,
  stagedCount,
  onClose,
  onDiscard,
  onBackupSuccess,
  onBackupAndContinue,
}: ExitBackupModalProps) {
  const { t } = useTranslation();
  const [destPath, setDestPath] = useState<string>("");
  const [defaultPath, setDefaultPath] = useState<string>("");
  const [isArchiving, setIsArchiving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [existingBackups, setExistingBackups] = useState<BackupArchiveInfoDto[]>([]);
  const [autoPrune, setAutoPrune] = useState(true);
  const [retentionLimit, setRetentionLimit] = useState(5);
  const [showArchiveList, setShowArchiveList] = useState(false);
  const [deletingName, setDeletingName] = useState<string | null>(null);

  const loadExistingBackups = useCallback(async () => {
    try {
      const list = await invoke<BackupArchiveInfoDto[]>("list_workspace_backups");
      setExistingBackups(Array.isArray(list) ? list : []);
    } catch (err) {
      console.error("Failed to list backups:", err);
      setExistingBackups([]);
    }
  }, []);

  useEffect(() => {
    if (isOpen) {
      setError(null);
      loadExistingBackups();
      invoke<AppConfig>("get_app_config")
        .then((cfg) => {
          if (cfg?.workspace?.backup_retention_count) {
            setRetentionLimit(cfg.workspace.backup_retention_count);
          }
        })
        .catch(() => {});
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
  }, [isOpen, loadExistingBackups]);

  if (!isOpen) return null;

  const handleExecuteBackup = async (shouldExit: boolean = true) => {
    try {
      setIsArchiving(true);
      setError(null);
      const res = await invoke<BackupResultDto>("create_workspace_backup", {
        destinationPath: destPath.trim() ? destPath.trim() : null,
        autoPruneLimit: autoPrune ? retentionLimit : 999,
      });
      if (shouldExit) {
        onBackupSuccess(res);
      } else if (onBackupAndContinue) {
        onBackupAndContinue(res);
      } else {
        onBackupSuccess(res);
      }
      onClose();
    } catch (err) {
      console.error("Backup failed:", err);
      setError(typeof err === "string" ? err : String(err));
    } finally {
      setIsArchiving(false);
    }
  };

  const handleDeleteBackup = async (filename: string) => {
    if (!window.confirm(t("backup.delete_confirm", { name: filename }))) return;
    try {
      setDeletingName(filename);
      await invoke("delete_workspace_backup", { filename });
      await loadExistingBackups();
    } catch (err) {
      console.error("Failed to delete backup:", err);
      setError(typeof err === "string" ? err : String(err));
    } finally {
      setDeletingName(null);
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

          {/* Retention & Archive Management Card (Phase 11 / Task 3-3) */}
          <div className="backup-retention-card p-3 rounded bg-black/40 border border-white/10 mb-3">
            <div className="flex items-center justify-between mb-1.5">
              <span className="text-xs font-semibold text-slate-200 flex items-center gap-1.5">
                <Layers size={13} className="text-teal-400" />
                {t("backup.retention_label")} ({(existingBackups?.length ?? 0)} / {retentionLimit})
              </span>
              <button
                type="button"
                className="text-xs text-teal-400 hover:text-teal-300 underline"
                onClick={() => setShowArchiveList(!showArchiveList)}
              >
                {showArchiveList ? t("common.close") : `${t("backup.archive_list_title")} (${(existingBackups?.length ?? 0)})`}
              </button>
            </div>
            <p className="text-[11px] text-slate-400 mb-2">
              {t("backup.retention_desc", { count: retentionLimit })}
            </p>
            <label className="flex items-center gap-2 text-xs text-slate-300 cursor-pointer select-none">
              <input
                type="checkbox"
                checked={autoPrune}
                onChange={(e) => setAutoPrune(e.target.checked)}
                className="rounded accent-teal-500"
              />
              <span>{t("backup.auto_prune_checkbox")}</span>
            </label>

            {showArchiveList && (
              <div className="existing-backups-list mt-3 pt-2 border-t border-white/10 max-h-36 overflow-y-auto space-y-1.5">
                {existingBackups.length === 0 ? (
                  <p className="text-xs text-slate-500 py-1">{t("backup.no_archives")}</p>
                ) : (
                  existingBackups.map((bkp) => (
                    <div
                      key={bkp.filename}
                      className="flex items-center justify-between text-xs p-1.5 rounded bg-black/30 border border-white/5"
                    >
                      <div className="flex flex-col min-w-0 mr-2">
                        <span className="font-mono text-slate-200 truncate">{bkp.filename}</span>
                        <span className="text-[10px] text-slate-400">
                          {formatTimestamp(Number(bkp.created_at_ms))} • {formatBytes(Number(bkp.size_bytes))}
                          {bkp.file_count != null ? ` • ${t("backup.files_count", { count: bkp.file_count })}` : ""}
                        </span>
                      </div>
                      <button
                        type="button"
                        className="text-rose-400 hover:text-rose-300 p-1 shrink-0"
                        title={t("backup.delete_archive_btn")}
                        onClick={() => handleDeleteBackup(bkp.filename)}
                        disabled={deletingName === bkp.filename}
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  ))
                )}
              </div>
            )}
          </div>

          {error && (
            <div className="dialog-error-banner">
              <AlertTriangle size={15} className="text-red-400" />
              <span className="error-text">{error}</span>
            </div>
          )}
        </div>

        <div className="dialog-footer exit-backup-footer-stacked">
          {/* Row 1: Continue working without exiting */}
          <div className="exit-backup-footer-row">
            <button
              type="button"
              className="dialog-btn secondary"
              onClick={onClose}
              disabled={isArchiving}
            >
              <X size={13} />
              <span>{t("backup.cancel_btn")}</span>
            </button>

            {onBackupAndContinue && (
              <button
                type="button"
                className="dialog-btn backup-continue-btn"
                onClick={() => handleExecuteBackup(false)}
                disabled={isArchiving}
                title={t("backup.backup_and_continue_btn")}
              >
                <Archive size={14} />
                <span>{isArchiving ? t("backup.archiving") : t("backup.backup_and_continue_btn")}</span>
              </button>
            )}
          </div>

          {/* Row 2: Exit application actions */}
          <div className="exit-backup-footer-row exit-actions">
            <button
              type="button"
              className="dialog-btn discard-action-btn"
              onClick={onDiscard}
              disabled={isArchiving}
              title={t("backup.discard_btn")}
            >
              <Trash2 size={14} />
              <span>{t("backup.discard_btn")}</span>
            </button>

            <button
              type="button"
              className="dialog-btn primary backup-action-btn"
              onClick={() => handleExecuteBackup(true)}
              disabled={isArchiving}
            >
              <Archive size={14} />
              <span>{isArchiving ? t("backup.archiving") : t("backup.backup_btn")}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
