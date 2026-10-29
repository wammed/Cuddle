import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  Archive,
  X,
  CheckCircle2,
  AlertTriangle,
  FolderArchive,
  FileCheck,
  Layers,
  CheckSquare,
} from "lucide-react";
import type { FileItem } from "../types/FileItem";
import type { TarExportFormat } from "../types/TarExportFormat";
import type { TarExportScope } from "../types/TarExportScope";
import type { TarExportOptionsDto } from "../types/TarExportOptionsDto";
import type { TarExportResultDto } from "../types/TarExportResultDto";

interface TarExportModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess?: (result: TarExportResultDto) => void;
  stagedItems: FileItem[];
  selectedIds?: string[];
}

export const TarExportModal: React.FC<TarExportModalProps> = ({
  isOpen,
  onClose,
  onSuccess,
  stagedItems,
  selectedIds = [],
}) => {
  const { t } = useTranslation();

  const [scope, setScope] = useState<TarExportScope>("ModifiedOnly");
  const [format, setFormat] = useState<TarExportFormat>("TarGz");
  const [destPath, setDestPath] = useState<string>("");
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [result, setResult] = useState<TarExportResultDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  // 初期ファイル名生成 (YYYYMMDD_HHMMSS)
  const generateDefaultPath = (fmt: TarExportFormat) => {
    const now = new Date();
    const pad = (n: number) => String(n).padStart(2, "0");
    const timestamp = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(
      now.getDate()
    )}_${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
    const ext = fmt === "TarGz" ? ".tar.gz" : ".tar";
    return `/tmp/cuddle_workspace_${timestamp}${ext}`;
  };

  useEffect(() => {
    if (isOpen) {
      setResult(null);
      setError(null);
      setDestPath(generateDefaultPath(format));
    }
  }, [isOpen]);

  // フォーマット切り替え時に拡張子を追従
  const handleFormatChange = (newFormat: TarExportFormat) => {
    setFormat(newFormat);
    if (!destPath) {
      setDestPath(generateDefaultPath(newFormat));
      return;
    }
    if (newFormat === "TarGz" && destPath.endsWith(".tar")) {
      setDestPath(destPath + ".gz");
    } else if (newFormat === "Tar" && destPath.endsWith(".tar.gz")) {
      setDestPath(destPath.slice(0, -3));
    }
  };

  const handleExport = async () => {
    if (!destPath.trim()) return;

    setIsExporting(true);
    setError(null);
    setResult(null);

    const options: TarExportOptionsDto = {
      destination_path: destPath.trim(),
      format,
      scope,
      selected_ids: selectedIds,
    };

    try {
      const res = await invoke<TarExportResultDto>("export_workspace_tar", {
        options,
      });
      setResult(res);
      if (res.success) {
        if (onSuccess) {
          onSuccess(res);
        }
      } else {
        setError(res.error_message || t("tar_export.export_failed"));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setIsExporting(false);
    }
  };

  const formatBytes = (bytes: bigint | number): string => {
    const num = Number(bytes);
    if (num < 1024) return `${num} B`;
    if (num < 1024 * 1024) return `${(num / 1024).toFixed(1)} KB`;
    if (num < 1024 * 1024 * 1024) return `${(num / (1024 * 1024)).toFixed(1)} MB`;
    return `${(num / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  if (!isOpen) return null;

  const totalStagedCount = stagedItems.length;
  const selectedCount = selectedIds.length;

  return (
    <div className="modal-backdrop" onClick={onClose} role="dialog" aria-modal="true">
      <div
        className="tar-export-modal-dialog"
        onClick={(e) => e.stopPropagation()}
        style={{ maxWidth: "680px", width: "90%" }}
      >
        {/* ヘッダー */}
        <div className="modal-header">
          <div className="modal-title-wrap">
            <FolderArchive className="modal-title-icon" size={22} />
            <div>
              <h2 className="modal-title">{t("tar_export.dialog_title")}</h2>
              <p className="modal-subtitle">{t("tar_export.dialog_subtitle")}</p>
            </div>
          </div>
          <button
            className="modal-close-btn"
            onClick={onClose}
            aria-label={t("bulk_rename.cancel")}
          >
            <X size={18} />
          </button>
        </div>

        {/* 本文 */}
        <div className="modal-body" style={{ maxHeight: "70vh", overflowY: "auto", padding: "16px 20px" }}>
          {/* 成功通知 */}
          {result && result.success && (
            <div className="tar-export-success-banner">
              <CheckCircle2 size={24} className="text-accent" />
              <div>
                <div style={{ fontWeight: 600, fontSize: "14px", marginBottom: "4px" }}>
                  {t("tar_export.export_success", {
                    count: result.file_count,
                    path: result.archive_path,
                  })}
                </div>
                <div style={{ fontSize: "12px", opacity: 0.85 }}>
                  {t("tar_export.size_summary", {
                    archiveSize: formatBytes(result.archive_bytes),
                    origSize: formatBytes(result.total_uncompressed_bytes),
                  })}
                </div>
              </div>
            </div>
          )}

          {/* エラー通知 */}
          {error && (
            <div className="tar-export-error-banner">
              <AlertTriangle size={20} />
              <span>{error}</span>
            </div>
          )}

          {/* 1. スコープ選択 */}
          <div style={{ marginBottom: "20px" }}>
            <label className="tar-section-label">{t("tar_export.scope_label")}</label>
            <div className="tar-options-grid">
              <div
                className={`tar-option-card ${scope === "ModifiedOnly" ? "active" : ""}`}
                onClick={() => setScope("ModifiedOnly")}
                role="button"
                tabIndex={0}
              >
                <div className="tar-option-header">
                  <div className="tar-option-radio">
                    <span className={`radio-dot ${scope === "ModifiedOnly" ? "selected" : ""}`} />
                  </div>
                  <FileCheck size={16} />
                  <span className="tar-option-title">{t("tar_export.scope_modified")}</span>
                </div>
                <p className="tar-option-desc">
                  {t("tar_export.scope_modified_desc")}
                </p>
              </div>

              <div
                className={`tar-option-card ${scope === "AllStaged" ? "active" : ""}`}
                onClick={() => setScope("AllStaged")}
                role="button"
                tabIndex={0}
              >
                <div className="tar-option-header">
                  <div className="tar-option-radio">
                    <span className={`radio-dot ${scope === "AllStaged" ? "selected" : ""}`} />
                  </div>
                  <Layers size={16} />
                  <span className="tar-option-title">{t("tar_export.scope_all")}</span>
                  <span className="tar-count-badge">{totalStagedCount}</span>
                </div>
                <p className="tar-option-desc">
                  {t("tar_export.scope_all_desc")}
                </p>
              </div>

              <div
                className={`tar-option-card ${scope === "SelectedOnly" ? "active" : ""} ${
                  selectedCount === 0 ? "disabled" : ""
                }`}
                onClick={() => {
                  if (selectedCount > 0) setScope("SelectedOnly");
                }}
                role="button"
                tabIndex={0}
              >
                <div className="tar-option-header">
                  <div className="tar-option-radio">
                    <span className={`radio-dot ${scope === "SelectedOnly" ? "selected" : ""}`} />
                  </div>
                  <CheckSquare size={16} />
                  <span className="tar-option-title">
                    {t("tar_export.scope_selected", { count: selectedCount })}
                  </span>
                </div>
                <p className="tar-option-desc">
                  {t("tar_export.scope_selected_desc")}
                </p>
              </div>
            </div>
          </div>

          {/* 2. フォーマット選択 */}
          <div style={{ marginBottom: "20px" }}>
            <label className="tar-section-label">{t("tar_export.format_label")}</label>
            <div className="tar-format-grid">
              <div
                className={`tar-format-card ${format === "TarGz" ? "active" : ""}`}
                onClick={() => handleFormatChange("TarGz")}
                role="button"
                tabIndex={0}
              >
                <div className="tar-format-header">
                  <span className={`radio-dot ${format === "TarGz" ? "selected" : ""}`} />
                  <Archive size={16} />
                  <span style={{ fontWeight: 600 }}>{t("tar_export.format_targz")}</span>
                  <span className="tar-rec-badge">{t("tar_export.format_rec_badge")}</span>
                </div>
                <div style={{ fontSize: "11px", opacity: 0.7, marginTop: "4px" }}>
                  {t("tar_export.format_targz_desc")}
                </div>
              </div>

              <div
                className={`tar-format-card ${format === "Tar" ? "active" : ""}`}
                onClick={() => handleFormatChange("Tar")}
                role="button"
                tabIndex={0}
              >
                <div className="tar-format-header">
                  <span className={`radio-dot ${format === "Tar" ? "selected" : ""}`} />
                  <Archive size={16} />
                  <span style={{ fontWeight: 600 }}>{t("tar_export.format_tar")}</span>
                </div>
                <div style={{ fontSize: "11px", opacity: 0.7, marginTop: "4px" }}>
                  {t("tar_export.format_tar_desc")}
                </div>
              </div>
            </div>
          </div>

          {/* 3. 出力先ファイルパス */}
          <div style={{ marginBottom: "16px" }}>
            <label className="tar-section-label">{t("tar_export.destination_label")}</label>
            <div className="tar-dest-input-wrap">
              <input
                type="text"
                className="input-field tar-dest-input font-mono"
                value={destPath}
                onChange={(e) => setDestPath(e.target.value)}
                placeholder={t("tar_export.destination_placeholder")}
              />
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => setDestPath(generateDefaultPath(format))}
                title={t("tar_export.reset_path_title")}
                style={{ fontSize: "12px", whiteSpace: "nowrap" }}
              >
                {t("tar_export.regenerate_btn")}
              </button>
            </div>
            <p style={{ fontSize: "11px", opacity: 0.6, marginTop: "4px" }}>
              {t("tar_export.dest_hint")}
            </p>
          </div>
        </div>

        {/* フッターアクションバー */}
        <div className="modal-footer" style={{ display: "flex", justifyContent: "flex-end", gap: "10px" }}>
          <button
            type="button"
            className="btn btn-secondary"
            onClick={onClose}
            disabled={isExporting}
          >
            {t("bulk_rename.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleExport}
            disabled={isExporting || !destPath.trim()}
          >
            <Archive size={16} />
            <span>{isExporting ? t("tar_export.exporting") : t("tar_export.export_btn")}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
