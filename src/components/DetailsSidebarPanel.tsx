import { Info, X, Shield, FileText, Binary, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { FileItem } from "../types/FileItem";
import type { ExtractedTextDto } from "../types/ExtractedTextDto";
import { formatBytes, formatTimestamp } from "../utils/format";

interface DetailsSidebarPanelProps {
  item: FileItem | null;
  onClose: () => void;
  onExtractText: (item: FileItem) => void;
  extractLoading: boolean;
  extractedInfo: ExtractedTextDto | null;
}

export function DetailsSidebarPanel({
  item,
  onClose,
  onExtractText,
  extractLoading,
  extractedInfo,
}: DetailsSidebarPanelProps) {
  const { t } = useTranslation();

  if (!item) {
    return (
      <div className="panel-container details-sidebar-container">
        <div className="panel-header">
          <div className="panel-title-group">
            <Info size={15} />
            <h3>{t("panels.details_title")}</h3>
          </div>
          <button type="button" className="panel-header-btn" onClick={onClose} title="Close">
            <X size={14} />
          </button>
        </div>
        <div className="details-empty-state">
          <p>{t("details.no_selection")}</p>
        </div>
      </div>
    );
  }

  const isNonUtf8 = item.display_name.includes("\uFFFD");

  return (
    <div className="panel-container details-sidebar-container">
      <div className="panel-header">
        <div className="panel-title-group">
          <Info size={15} />
          <h3>{t("panels.details_title")}</h3>
        </div>
        <button type="button" className="panel-header-btn" onClick={onClose} title="Close">
          <X size={14} />
        </button>
      </div>

      <div className="details-content-scroll">
        <div className="details-header-card">
          <div className="details-icon-wrap">
            {isNonUtf8 ? (
              <Binary size={24} className="details-non-utf8-icon" />
            ) : (
              <FileText size={24} className="details-file-icon" />
            )}
          </div>
          <div className="details-header-text">
            <h4 className="details-file-name">{item.display_name}</h4>
            <span className="details-type-badge">
              {item.is_dir
                ? t("details.file_type_dir")
                : item.is_symlink
                ? t("details.file_type_symlink")
                : t("details.file_type_file")}
            </span>
          </div>
        </div>

        <div className="details-meta-group">
          <div className="details-meta-row">
            <span className="details-meta-label">{t("browser.col_size")}</span>
            <span className="details-meta-value">{formatBytes(Number(item.size))} ({item.size.toString()} B)</span>
          </div>

          <div className="details-meta-row">
            <span className="details-meta-label">{t("browser.col_mtime")}</span>
            <span className="details-meta-value">{formatTimestamp(Number(item.mtime_ms))}</span>
          </div>

          <div className="details-meta-row">
            <span className="details-meta-label">inode</span>
            <span className="details-meta-value font-mono">#{item.inode}</span>
          </div>

          <div className="details-meta-row">
            <span className="details-meta-label">{t("details.device_id")}</span>
            <span className="details-meta-value font-mono">{item.dev}</span>
          </div>

          <div className="details-meta-row">
            <span className="details-meta-label">{t("details.permissions")}</span>
            <span className="details-meta-value">
              {item.readonly ? "Read-Only (0444)" : "Read-Write (0644)"}
            </span>
          </div>
        </div>

        <div className="details-section">
          <span className="section-label">{t("details.safe_status")}</span>
          <div className="safe-status-box">
            <Shield size={14} className="safe-icon" />
            <div>
              <p className="safe-title">{t("workspace.original_protected")}</p>
              <p className="safe-desc">Landlock LSM Kernel Sandbox Active</p>
            </div>
          </div>
        </div>

        <div className="details-section">
          <span className="section-label">{t("workspace.raw_base64_label")}</span>
          <code className="details-b64-box">{item.raw_base64}</code>
        </div>

        <div className="details-section">
          <span className="section-label">{t("details.actions")}</span>
          <div className="details-actions-list">
            {!item.is_dir && (
              <button
                type="button"
                className="details-action-btn primary"
                onClick={() => onExtractText(item)}
                disabled={extractLoading}
              >
                <Layers size={14} />
                <span>{t("extractor.title")}</span>
              </button>
            )}
          </div>
        </div>

        {extractedInfo && extractedInfo.item_id === item.item_id && (
          <div className="details-section extractor-details-box">
            <span className="section-label">{t("extractor.title")}</span>
            <div className="extractor-meta-line">
              <span>{t("extractor.encoding_label")}:</span>
              <span className="pill-tag secure">{extractedInfo.encoding_used}</span>
            </div>
            <div className="extractor-meta-line">
              <span>Tier:</span>
              <span className="pill-tag">{extractedInfo.tier_used}</span>
            </div>
            <div className="extractor-meta-line">
              <span>{t("extractor.chars_count", { count: extractedInfo.extracted_chars })}</span>
            </div>
            {extractedInfo.truncated && (
              <p className="details-warning-note">{t("extractor.truncated_warning")}</p>
            )}
            {extractedInfo.warning && (
              <p className="details-warning-note">{extractedInfo.warning}</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
