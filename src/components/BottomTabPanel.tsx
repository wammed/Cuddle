import { useState, useEffect, useMemo, useCallback } from "react";
import {
  Eye,
  History,
  Shield,
  Terminal,
  X,
  RotateCcw,
  GitCompare,
  AlertCircle,
  CheckCircle2,
  Upload,
  AlertTriangle,
  GitBranch,
  GitCommit,
  RefreshCw,
  FolderGit2,
  FileText,
  Sparkles,
  Database,
  Check,
  MinusCircle,
  Trash2,
  Wrench,
  Loader2,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import type { FileItem } from "../types/FileItem";
import type { ExtractedTextDto } from "../types/ExtractedTextDto";
import type { AppConfig } from "../types/AppConfig";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { FileDiffResultDto } from "../types/FileDiffResultDto";
import type { OperationHistoryDto } from "../types/OperationHistoryDto";
import type { ExternalToolStatusDto } from "../types/ExternalToolStatusDto";
import type { GitStatusDto } from "../types/GitStatusDto";
import type { ReflectionMode } from "../types/ReflectionMode";
import { RichPreview } from "./RichPreview";

export type BottomTabId = "preview" | "diff" | "history" | "git" | "security" | "logs";

interface BottomTabPanelProps {
  selectedItem: FileItem | null;
  extractedInfo: ExtractedTextDto | null;
  appConfig: AppConfig | null;
  logs: string[];
  history: string[];
  historyItems?: OperationHistoryDto[];
  onClearHistory?: () => void;
  stagedItems?: StagedItemDto[];
  onResetStagedItem?: (stageId: string) => void;
  onReflectStagedItem?: (stageId: string, mode?: ReflectionMode) => Promise<void> | void;
  onUndoHistoryItem?: (operationId: string) => Promise<void> | void;
  onClose: () => void;
  currentPath?: string;
  activeTab?: BottomTabId;
  onTabChange?: (tab: BottomTabId) => void;
  onTriggerPreview?: (item: FileItem) => void;
}

export function BottomTabPanel({
  selectedItem,
  extractedInfo,
  appConfig,
  logs,
  history,
  historyItems,
  onClearHistory,
  stagedItems = [],
  onResetStagedItem,
  onReflectStagedItem,
  onUndoHistoryItem,
  onClose,
  currentPath,
  activeTab: controlledActiveTab,
  onTabChange,
  onTriggerPreview,
}: BottomTabPanelProps) {
  const { t } = useTranslation();
  const [internalActiveTab, setInternalActiveTab] = useState<BottomTabId>("preview");
  const activeTab = controlledActiveTab !== undefined ? controlledActiveTab : internalActiveTab;

  const handleSetActiveTab = (tab: BottomTabId) => {
    setInternalActiveTab(tab);
    if (onTabChange) {
      onTabChange(tab);
    }
  };

  const [diffResult, setDiffResult] = useState<FileDiffResultDto | null>(null);
  const [isDiffLoading, setIsDiffLoading] = useState(false);
  const [showReflectModal, setShowReflectModal] = useState(false);
  const [isReflecting, setIsReflecting] = useState(false);
  const [reflectError, setReflectError] = useState<string | null>(null);
  const [reflectionMode, setReflectionMode] = useState<ReflectionMode>("BackupOrigin");
  const [backupPreviewPath, setBackupPreviewPath] = useState<string | null>(null);
  const [isUndoingId, setIsUndoingId] = useState<string | null>(null);
  const [selectedStageId, setSelectedStageId] = useState<string | null>(null);
  const [externalTools, setExternalTools] = useState<ExternalToolStatusDto[]>([]);
  const [isToolsLoading, setIsToolsLoading] = useState(false);

  // Git Status State
  const [gitStatus, setGitStatus] = useState<GitStatusDto | null>(null);
  const [isGitLoading, setIsGitLoading] = useState(false);


  const loadTools = useCallback(async () => {
    try {
      setIsToolsLoading(true);
      const tools = await invoke<ExternalToolStatusDto[]>("probe_external_tools");
      setExternalTools(tools);
    } catch (err) {
      console.error("Failed to probe external tools:", err);
    } finally {
      setIsToolsLoading(false);
    }
  }, []);

  const loadGitStatus = useCallback(async () => {
    try {
      setIsGitLoading(true);
      const targetPath = selectedItem?.raw_base64 || currentPath || ".";
      const res = await invoke<GitStatusDto>("get_git_status", { pathBase64: targetPath });
      setGitStatus(res);
    } catch (err) {
      console.error("Failed to load git status:", err);
      setGitStatus(null);
    } finally {
      setIsGitLoading(false);
    }
  }, [selectedItem, currentPath]);

  useEffect(() => {
    if (activeTab === "security") {
      loadTools();
    } else if (activeTab === "git") {
      loadGitStatus();
    }
  }, [activeTab, loadTools, loadGitStatus]);

  const matchedStaged = useMemo(() => {
    // 1. If explicit selectedStageId exists in current stagedItems
    if (selectedStageId) {
      const found = stagedItems.find((s) => s.stage_id === selectedStageId);
      if (found) return found;
    }

    // 2. If selectedItem matches a staged item through any identity
    if (selectedItem) {
      const found = stagedItems.find(
        (s) =>
          s.stage_id === selectedItem.item_id ||
          s.original_item_id === selectedItem.item_id ||
          s.raw_base64 === selectedItem.raw_base64 ||
          s.staged_raw_base64 === selectedItem.raw_base64 ||
          (selectedItem.inode > 0n &&
            s.original_snapshot.inode === selectedItem.inode &&
            s.original_snapshot.dev === selectedItem.dev) ||
          s.display_name === selectedItem.display_name
      );
      if (found) return found;
    }

    // 3. Fallback to the most recent staged item if any exist
    return stagedItems.length > 0 ? stagedItems[stagedItems.length - 1] : null;
  }, [selectedItem, stagedItems, selectedStageId]);

  // Load preview backup path whenever reflection modal opens (Design 5.2 Mode B)
  useEffect(() => {
    if (showReflectModal && matchedStaged) {
      invoke<string>("get_origin_backup_preview", {
        path: matchedStaged.original_snapshot.canonical_path,
      })
        .then((preview) => setBackupPreviewPath(preview))
        .catch((err) => console.error("Failed to load origin backup preview:", err));
    }
  }, [showReflectModal, matchedStaged]);

  const loadDiff = useCallback(async (stageId: string) => {
    try {
      setIsDiffLoading(true);
      const res = await invoke<FileDiffResultDto>("get_file_diff", { stageId });
      setDiffResult(res);
    } catch (err) {
      console.error("Failed to load diff:", err);
      setDiffResult(null);
    } finally {
      setIsDiffLoading(false);
    }
  }, []);

  useEffect(() => {
    if (matchedStaged) {
      loadDiff(matchedStaged.stage_id);
    } else {
      setDiffResult(null);
    }
  }, [matchedStaged, loadDiff, activeTab]);

  const handleReset = async () => {
    if (!matchedStaged) return;
    if (onResetStagedItem) {
      onResetStagedItem(matchedStaged.stage_id);
      loadDiff(matchedStaged.stage_id);
    } else {
      try {
        await invoke("reset_staged_file", { stageId: matchedStaged.stage_id });
        loadDiff(matchedStaged.stage_id);
      } catch (err) {
        console.error("Reset failed:", err);
      }
    }
  };

  const handleExecuteReflect = async () => {
    if (!matchedStaged) return;
    try {
      setIsReflecting(true);
      setReflectError(null);
      if (onReflectStagedItem) {
        await onReflectStagedItem(matchedStaged.stage_id, reflectionMode);
      } else {
        await invoke("reflect_staged_file", {
          stageId: matchedStaged.stage_id,
          mode: reflectionMode,
        });
      }
      setShowReflectModal(false);
      await loadDiff(matchedStaged.stage_id);
    } catch (err) {
      console.error("Reflection failed:", err);
      const errMsg = typeof err === "string" ? err : String(err);
      setReflectError(errMsg);
    } finally {
      setIsReflecting(false);
    }
  };

  const handleUndoOperation = async (item: OperationHistoryDto) => {
    if (!item.can_undo) return;
    const confirmed = window.confirm(
      t("history.undo_confirm", { name: item.display_name })
    );
    if (!confirmed) return;

    try {
      setIsUndoingId(item.operation_id);
      if (onUndoHistoryItem) {
        await onUndoHistoryItem(item.operation_id);
      } else {
        await invoke("undo_history_operation", { operationId: item.operation_id });
      }
    } catch (err) {
      console.error("Selective undo failed:", err);
      const errStr = typeof err === "string" ? err : String(err);
      alert(`${t("history.undo_failed")}: ${errStr}`);
    } finally {
      setIsUndoingId(null);
    }
  };

  const getOpBadgeClass = (opType: string) => {
    switch (opType) {
      case "BulkRename":
        return "badge-op badge-rename";
      case "UndoBulkRename":
        return "badge-op badge-undo";
      case "chmod":
        return "badge-op badge-chmod";
      case "UndoChmod":
        return "badge-op badge-undo";
      case "touch_mtime":
        return "badge-op badge-time";
      case "UndoTouchMtime":
        return "badge-op badge-undo";
      case "Reflect":
        return "badge-op badge-reflect";
      case "Stage":
        return "badge-op badge-stage";
      case "Unstage":
        return "badge-op badge-unstage";
      case "Reset":
        return "badge-op badge-reset";
      case "ClearWorkspace":
        return "badge-op badge-clear";
      default:
        return "badge-op";
    }
  };

  const getOpLabel = (opType: string) => {
    switch (opType) {
      case "BulkRename":
        return t("history.type_bulk_rename");
      case "UndoBulkRename":
        return t("history.type_undo_rename");
      case "chmod":
        return t("history.type_chmod");
      case "UndoChmod":
        return t("history.type_undo_chmod");
      case "touch_mtime":
        return t("history.type_touch_mtime");
      case "UndoTouchMtime":
        return t("history.type_undo_touch_mtime");
      case "Reflect":
        return t("history.type_reflect");
      case "Stage":
        return t("history.type_stage");
      case "Unstage":
        return t("history.type_unstage");
      case "Reset":
        return t("history.type_reset");
      case "ClearWorkspace":
        return t("history.type_clear");
      default:
        return opType;
    }
  };

  const getStatusBadge = (status: string) => {
    if (status === "Success") {
      return (
        <span className="history-status-badge success" title="Success">
          <CheckCircle2 size={12} />
          <span>{t("history.status_success")}</span>
        </span>
      );
    }
    if (status === "Undone") {
      return (
        <span className="history-status-badge undone" title="Undone">
          <RotateCcw size={12} />
          <span>{t("history.status_undone")}</span>
        </span>
      );
    }
    return (
      <span className="history-status-badge failed" title={status}>
        <AlertCircle size={12} />
        <span>{status}</span>
      </span>
    );
  };

  return (
    <div className="panel-container bottom-tab-container">
      {/* Reflection Confirmation Modal */}
      {showReflectModal && matchedStaged && (
        <div className="modal-backdrop">
          <div className="reflect-confirm-dialog">
            <div className="dialog-header">
              <div className="dialog-title-group">
                <AlertTriangle size={18} className={reflectError ? "text-red-400" : "text-amber-400"} />
                <h4 className={reflectError ? "text-red-400" : ""}>{t("reflection.reflect_title")}</h4>
              </div>
              <button
                type="button"
                className="panel-header-btn"
                onClick={() => {
                  setShowReflectModal(false);
                  setReflectError(null);
                }}
                title={t("reflection.cancel_btn")}
              >
                <X size={14} />
              </button>
            </div>

            <div className="dialog-body">
              {reflectError ? (
                <div className="dialog-error-banner">
                  <div className="error-banner-header">
                    <AlertTriangle size={16} className="text-red-400" />
                    <h5>{t("reflection.reflect_failed")}</h5>
                  </div>
                  <p className="dialog-error-text">{reflectError}</p>
                  {(reflectError.includes("Conflict") || reflectError.includes("conflict")) && (
                    <p className="dialog-conflict-note">{t("reflection.conflict_detected_detail")}</p>
                  )}
                </div>
              ) : (
                <p className="dialog-warning-text">{t("reflection.reflect_warning")}</p>
              )}

              <div className="dialog-meta-card">
                <div className="dialog-meta-row">
                  <span className="dialog-label">{t("reflection.original_path_label")}:</span>
                  <span className="dialog-value font-mono">{matchedStaged.original_snapshot.canonical_path}</span>
                </div>
                <div className="dialog-meta-row">
                  <span className="dialog-label">{t("reflection.staged_path_label")}:</span>
                  <span className="dialog-value font-mono">{matchedStaged.staged_path}</span>
                </div>
                {diffResult && (
                  <div className="dialog-meta-row">
                    <span className="dialog-label">{t("diff.title")}:</span>
                    <span className="dialog-value font-mono text-cyan-400">
                      +{diffResult.additions} / -{diffResult.deletions} lines
                    </span>
                  </div>
                )}
                {reflectionMode === "BackupOrigin" && backupPreviewPath && (
                  <div className="dialog-meta-row backup-preview-row">
                    <span className="dialog-label text-teal-400">{t("reflection.backup_preview_label")}:</span>
                    <span className="dialog-value font-mono text-teal-300 font-bold">{backupPreviewPath}</span>
                  </div>
                )}
              </div>

              {/* Reflection Mode Selection (Design 5.2) */}
              <div className="dialog-mode-selection">
                <label className="dialog-mode-group-label">{t("reflection.mode_label")}</label>
                <div className="dialog-mode-cards">
                  <div
                    className={`dialog-mode-card ${reflectionMode === "BackupOrigin" ? "selected" : ""}`}
                    onClick={() => setReflectionMode("BackupOrigin")}
                  >
                    <div className="mode-card-radio">
                      <span className={`custom-radio ${reflectionMode === "BackupOrigin" ? "checked" : ""}`} />
                    </div>
                    <div className="mode-card-content">
                      <span className="mode-card-title">{t("reflection.mode_backup_origin")}</span>
                      <p className="mode-card-desc">{t("reflection.mode_backup_desc")}</p>
                    </div>
                  </div>

                  <div
                    className={`dialog-mode-card ${reflectionMode === "Overwrite" ? "selected" : ""}`}
                    onClick={() => setReflectionMode("Overwrite")}
                  >
                    <div className="mode-card-radio">
                      <span className={`custom-radio ${reflectionMode === "Overwrite" ? "checked" : ""}`} />
                    </div>
                    <div className="mode-card-content">
                      <span className="mode-card-title">{t("reflection.mode_overwrite")}</span>
                      <p className="mode-card-desc">{t("reflection.mode_overwrite_desc")}</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className="dialog-footer">
              <button
                type="button"
                className="dialog-btn secondary"
                onClick={() => {
                  setShowReflectModal(false);
                  setReflectError(null);
                }}
                disabled={isReflecting}
              >
                {reflectError ? t("reflection.close_btn") : t("reflection.cancel_btn")}
              </button>

              {reflectError ? (
                <button
                  type="button"
                  className="dialog-btn secondary text-amber-400"
                  onClick={async () => {
                    await handleReset();
                    setShowReflectModal(false);
                    setReflectError(null);
                  }}
                  title={t("diff.reset_to_original")}
                >
                  <RotateCcw size={14} />
                  <span>{t("diff.reset_to_original")}</span>
                </button>
              ) : (
                <button
                  type="button"
                  className="dialog-btn primary reflect-action-btn"
                  onClick={handleExecuteReflect}
                  disabled={isReflecting}
                >
                  <Upload size={14} />
                  <span>{isReflecting ? "Reflecting..." : t("reflection.confirm_reflect_btn")}</span>
                </button>
              )}
            </div>
          </div>
        </div>
      )}

      <div className="bottom-tabs-header">
        <div className="tab-buttons-group">
          {/* 1. Preview Tab */}
          <button
            type="button"
            className={`bottom-tab-btn ${activeTab === "preview" ? "active" : ""}`}
            onClick={() => handleSetActiveTab("preview")}
          >
            <Eye size={13} />
            <span>{t("panels.tab_preview")}</span>
          </button>

          {/* 2. Diff Tab */}
          <button
            type="button"
            className={`bottom-tab-btn ${activeTab === "diff" ? "active" : ""}`}
            onClick={() => handleSetActiveTab("diff")}
          >
            <GitCompare size={13} />
            <span>{t("panels.tab_diff")}</span>
            {diffResult && diffResult.has_changes && (
              <span className="tab-badge-accent">+{diffResult.additions} -{diffResult.deletions}</span>
            )}
          </button>

          {/* 3. History Tab */}
          <button
            type="button"
            className={`bottom-tab-btn ${activeTab === "history" ? "active" : ""}`}
            onClick={() => handleSetActiveTab("history")}
          >
            <History size={13} />
            <span>{t("panels.tab_history")}</span>
            {historyItems !== undefined && historyItems.length > 0 && (
              <span className="tab-badge-accent">{historyItems.length}</span>
            )}
          </button>

          {/* 4. Git Info Tab */}
          <button
            type="button"
            className={`bottom-tab-btn ${activeTab === "git" ? "active" : ""}`}
            onClick={() => handleSetActiveTab("git")}
          >
            <GitBranch size={13} />
            <span>{t("panels.tab_git")}</span>
            {gitStatus?.is_repo && (
              <span className={`tab-badge-accent ${gitStatus.is_clean ? "clean" : "dirty"}`}>
                {gitStatus.branch || "HEAD"}
                {!gitStatus.is_clean && "*"}
              </span>
            )}
          </button>

          {/* 5. Security & Tools Diagnostic Tab */}
          <button
            type="button"
            className={`bottom-tab-btn ${activeTab === "security" ? "active" : ""}`}
            onClick={() => handleSetActiveTab("security")}
          >
            <Shield size={13} />
            <span>{t("panels.tab_security")}</span>
          </button>

          {/* 6. Logs Tab */}
          <button
            type="button"
            className={`bottom-tab-btn ${activeTab === "logs" ? "active" : ""}`}
            onClick={() => handleSetActiveTab("logs")}
          >
            <Terminal size={13} />
            <span>{t("panels.tab_logs")} ({logs.length})</span>
          </button>
        </div>

        <button type="button" className="panel-header-btn" onClick={onClose} title="Close">
          <X size={14} />
        </button>
      </div>

      <div className="bottom-tab-content">
        {activeTab === "preview" && (
          <div className="tab-preview-content">
            {extractedInfo ? (
              <RichPreview
                filename={extractedInfo.display_name}
                text={extractedInfo.text}
                encoding={extractedInfo.encoding_used}
                extractedChars={extractedInfo.extracted_chars}
              />
            ) : selectedItem ? (
              <div className="preview-explicit-prompt">
                <div className="preview-prompt-card">
                  <FileText size={28} className="text-teal-400 mb-2" />
                  <h4>{selectedItem.display_name}</h4>
                  <p className="preview-guidance-text">
                    {t("rich_preview.no_selection_hint")}
                  </p>
                  {onTriggerPreview && (
                    <button
                      type="button"
                      className="panel-header-btn-primary preview-load-btn"
                      onClick={() => onTriggerPreview(selectedItem)}
                    >
                      <Sparkles size={13} />
                      <span>{t("context_menu.open_preview")}</span>
                    </button>
                  )}
                </div>
              </div>
            ) : (
              <div className="preview-explicit-prompt">
                <p className="preview-empty-text">{t("rich_preview.no_selection_hint")}</p>
              </div>
            )}
          </div>
        )}

        {activeTab === "diff" && (
          <div className="tab-preview-content">
            {matchedStaged ? (
              <div className="preview-diff-container">
                <div className="diff-header-bar">
                  <div className="diff-header-left">
                    <GitCompare size={14} className="text-cyan-400" />
                    <span className="diff-title">{t("diff.title")}</span>
                    {stagedItems.length > 1 ? (
                      <select
                        className="diff-file-select font-mono"
                        value={matchedStaged.stage_id}
                        onChange={(e) => setSelectedStageId(e.target.value)}
                        title="Select staged file to preview"
                      >
                        {stagedItems.map((s) => (
                          <option key={s.stage_id} value={s.stage_id}>
                            {s.display_name} ({s.clone_method === "Reflink" ? "CoW" : "Copy"})
                          </option>
                        ))}
                      </select>
                    ) : (
                      <span className="diff-file-tag">{matchedStaged.display_name}</span>
                    )}
                    {diffResult && (
                      <span className="diff-stat-tag">
                        {t("diff.additions_deletions", {
                          additions: diffResult.additions,
                          deletions: diffResult.deletions,
                        })}
                      </span>
                    )}
                  </div>

                  <div className="diff-header-right">
                    {diffResult?.has_changes && (
                      <button
                        type="button"
                        className="diff-reflect-btn"
                        onClick={() => setShowReflectModal(true)}
                        title={t("reflection.reflect_btn")}
                        disabled={isDiffLoading || isReflecting}
                      >
                        <Upload size={12} />
                        <span>{t("reflection.reflect_btn")}</span>
                      </button>
                    )}
                    <button
                      type="button"
                      className="diff-reset-btn"
                      onClick={handleReset}
                      title={t("diff.reset_to_original")}
                      disabled={isDiffLoading || isReflecting}
                    >
                      <RotateCcw size={12} />
                      <span>{t("diff.reset_to_original")}</span>
                    </button>
                  </div>
                </div>

                {isDiffLoading ? (
                  <p className="diff-status-text">{t("diff.loading_diff")}</p>
                ) : diffResult?.is_binary ? (
                  <div className="diff-empty-banner binary">
                    <AlertCircle size={15} />
                    <span>{t("diff.binary_file")}</span>
                  </div>
                ) : diffResult && !diffResult.has_changes ? (
                  <div className="diff-empty-banner identical">
                    <CheckCircle2 size={15} />
                    <span>{t("diff.no_diff")}</span>
                  </div>
                ) : diffResult ? (
                  <div className="diff-table-scroll">
                    <table className="diff-table">
                      <tbody>
                        {diffResult.lines.map((line, idx) => (
                          <tr
                            key={idx}
                            className={`diff-row ${
                              line.change_type === "Insert"
                                ? "diff-insert"
                                : line.change_type === "Delete"
                                ? "diff-delete"
                                : "diff-equal"
                            }`}
                          >
                            <td className="diff-ln old-ln">
                              {line.old_line_number ?? ""}
                            </td>
                            <td className="diff-ln new-ln">
                              {line.new_line_number ?? ""}
                            </td>
                            <td className="diff-marker">
                              {line.change_type === "Insert"
                                ? "+"
                                : line.change_type === "Delete"
                                ? "-"
                                : " "}
                            </td>
                            <td className="diff-line-content">
                              <code>{line.content || " "}</code>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                ) : (
                  <p className="diff-status-text">{t("diff.no_diff")}</p>
                )}
              </div>
            ) : (
              <div className="diff-empty-banner">
                <GitCompare size={16} className="text-muted" />
                <span>{t("diff.no_staged_items")}</span>
              </div>
            )}
          </div>
        )}

        {activeTab === "history" && (
          <div className="tab-history-content">
            <div className="history-header-bar">
              <div className="history-header-left">
                <Database size={14} className="text-cyan-400" />
                <span className="history-title">{t("history.title")}</span>
                <span className="history-stat-tag">
                  {t("history.record_count", {
                    count: historyItems !== undefined ? historyItems.length : history.length,
                  })}
                </span>
              </div>
              {onClearHistory && (
                <div className="history-header-right">
                  <button
                    type="button"
                    className="history-clear-btn"
                    onClick={onClearHistory}
                    title={t("history.clear_btn")}
                  >
                    <Trash2 size={12} />
                    <span>{t("history.clear_btn")}</span>
                  </button>
                </div>
              )}
            </div>

            {historyItems !== undefined ? (
              historyItems.length > 0 ? (
                <div className="history-table-scroll">
                  <table className="history-table">
                    <thead>
                      <tr>
                        <th className="th-time">{t("history.col_time")}</th>
                        <th className="th-type">{t("history.col_type")}</th>
                        <th className="th-target">{t("history.col_target")}</th>
                        <th className="th-status">{t("history.col_status")}</th>
                        <th className="th-details">{t("history.col_details")}</th>
                        <th className="th-actions">{t("history.col_actions")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {historyItems.map((item) => {
                        const timeStr = new Date(Number(item.timestamp_ms)).toLocaleTimeString([], {
                          hour: "2-digit",
                          minute: "2-digit",
                          second: "2-digit",
                        });
                        return (
                          <tr key={item.id.toString()} className="history-row">
                            <td className="history-col-time font-mono">{timeStr}</td>
                            <td className="history-col-type">
                              <span className={getOpBadgeClass(item.operation_type)}>
                                {getOpLabel(item.operation_type)}
                              </span>
                            </td>
                            <td className="history-col-target">
                              <span className="history-target-name font-mono">{item.display_name}</span>
                              {item.original_path && (
                                <span className="history-target-sub font-mono">{item.original_path}</span>
                              )}
                            </td>
                            <td className="history-col-status">{getStatusBadge(item.status)}</td>
                            <td className="history-col-details font-mono text-muted">
                              {item.details || "-"}
                            </td>
                            <td className="history-col-actions">
                              {item.can_undo && (
                                <button
                                  type="button"
                                  className="history-undo-btn"
                                  onClick={() => handleUndoOperation(item)}
                                  disabled={isUndoingId === item.operation_id}
                                  title={t("history.undo_btn")}
                                >
                                  {isUndoingId === item.operation_id ? (
                                    <Loader2 size={12} className="animate-spin" />
                                  ) : (
                                    <RotateCcw size={12} />
                                  )}
                                  <span>{t("history.undo_btn")}</span>
                                </button>
                              )}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              ) : (
                <p className="tab-empty-text">{t("history.empty")}</p>
              )
            ) : history.length > 0 ? (
              <ul className="history-list">
                {history.map((entry, idx) => (
                  <li key={idx} className="history-entry">
                    <span className="history-bullet">•</span>
                    <span className="history-text">{entry}</span>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="tab-empty-text">{t("history.empty")}</p>
            )}
          </div>
        )}

        {activeTab === "git" && (
          <div className="tab-git-content">
            {isGitLoading ? (
              <div className="git-loading-state">
                <RefreshCw size={18} className="spinning text-teal-400" />
                <span>{t("common.loading")}</span>
              </div>
            ) : !gitStatus?.is_repo ? (
              <div className="git-not-repo-state">
                <FolderGit2 size={32} className="text-slate-500 mb-2" />
                <h4>{t("git.not_repo")}</h4>
                <p className="text-muted text-xs font-mono">
                  {currentPath || "~"}
                </p>
              </div>
            ) : (
              <div className="git-dashboard">
                {/* 1. Branch & Sync Status Banner */}
                <div className="git-summary-banner">
                  <div className="git-branch-info">
                    <span className="git-branch-badge">
                      <GitBranch size={14} />
                      <span className="font-mono">{gitStatus.branch || "HEAD"}</span>
                    </span>
                    {gitStatus.upstream && (
                      <span className="git-upstream-label font-mono text-xs">
                        &rarr; {gitStatus.upstream}
                      </span>
                    )}
                    {gitStatus.ahead > 0 && (
                      <span className="git-ahead-badge">
                        &uarr; {t("git.ahead", { count: gitStatus.ahead })}
                      </span>
                    )}
                    {gitStatus.behind > 0 && (
                      <span className="git-behind-badge">
                        &darr; {t("git.behind", { count: gitStatus.behind })}
                      </span>
                    )}
                    <span className={`git-clean-pill ${gitStatus.is_clean ? "clean" : "dirty"}`}>
                      {gitStatus.is_clean ? t("git.clean") : "Changes Present"}
                    </span>
                  </div>

                  <div className="git-summary-actions">
                    {gitStatus.latest_commit_hash && (
                      <div className="git-latest-commit font-mono text-xs" title={gitStatus.latest_commit_message || ""}>
                        <GitCommit size={13} className="text-slate-400" />
                        <span className="commit-hash text-teal-400 font-bold">{gitStatus.latest_commit_hash}</span>
                        <span className="commit-msg">{gitStatus.latest_commit_message}</span>
                        <span className="commit-meta text-muted">({gitStatus.latest_commit_author}, {gitStatus.latest_commit_date})</span>
                      </div>
                    )}
                    <button
                      type="button"
                      className="panel-header-btn-subtle"
                      onClick={loadGitStatus}
                      title={t("git.refresh")}
                    >
                      <RefreshCw size={12} />
                      <span>{t("git.refresh")}</span>
                    </button>
                  </div>
                </div>

                {/* 2. Structured Change Categories Grid */}
                <div className="git-sections-grid">
                  {/* Category 1: Staged Changes */}
                  <div className="git-section-card staged-card">
                    <div className="git-section-header">
                      <h5>{t("git.staged_changes")}</h5>
                      <span className="git-count-badge staged font-mono">{gitStatus.staged_files.length}</span>
                    </div>
                    <div className="git-file-list">
                      {gitStatus.staged_files.length === 0 ? (
                        <p className="git-empty-hint">-</p>
                      ) : (
                        gitStatus.staged_files.map((file, i) => (
                          <div key={i} className="git-file-row">
                            <span className={`git-code-pill code-${file.status_code}`}>{file.status_code}</span>
                            <span className="git-file-path font-mono" title={file.path}>{file.path}</span>
                            {file.original_path && (
                              <span className="git-orig-path text-muted font-mono">&larr; {file.original_path}</span>
                            )}
                          </div>
                        ))
                      )}
                    </div>
                  </div>

                  {/* Category 2: Unstaged Working Tree Changes */}
                  <div className="git-section-card unstaged-card">
                    <div className="git-section-header">
                      <h5>{t("git.unstaged_changes")}</h5>
                      <span className="git-count-badge unstaged font-mono">{gitStatus.unstaged_files.length}</span>
                    </div>
                    <div className="git-file-list">
                      {gitStatus.unstaged_files.length === 0 ? (
                        <p className="git-empty-hint">-</p>
                      ) : (
                        gitStatus.unstaged_files.map((file, i) => (
                          <div key={i} className="git-file-row">
                            <span className={`git-code-pill code-${file.status_code}`}>{file.status_code}</span>
                            <span className="git-file-path font-mono" title={file.path}>{file.path}</span>
                          </div>
                        ))
                      )}
                    </div>
                  </div>

                  {/* Category 3: Untracked Files */}
                  <div className="git-section-card untracked-card">
                    <div className="git-section-header">
                      <h5>{t("git.untracked_files")}</h5>
                      <span className="git-count-badge untracked font-mono">{gitStatus.untracked_files.length}</span>
                    </div>
                    <div className="git-file-list">
                      {gitStatus.untracked_files.length === 0 ? (
                        <p className="git-empty-hint">-</p>
                      ) : (
                        gitStatus.untracked_files.map((file, i) => (
                          <div key={i} className="git-file-row">
                            <span className="git-code-pill code-untracked">?</span>
                            <span className="git-file-path font-mono" title={file.path}>{file.path}</span>
                          </div>
                        ))
                      )}
                    </div>
                  </div>

                  {/* Category 4: Merge Conflicts */}
                  {gitStatus.conflict_files.length > 0 && (
                    <div className="git-section-card conflict-card">
                      <div className="git-section-header">
                        <h5>{t("git.conflicts")}</h5>
                        <span className="git-count-badge conflict font-mono">{gitStatus.conflict_files.length}</span>
                      </div>
                      <div className="git-file-list">
                        {gitStatus.conflict_files.map((file, i) => (
                          <div key={i} className="git-file-row text-rose-400">
                            <span className="git-code-pill code-conflict">!</span>
                            <span className="git-file-path font-mono" title={file.path}>{file.path}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === "security" && (
          <div className="tab-security-content">
            <div className="security-summary-card">
              <h4>{t("security.landlock_active")}</h4>
              <p>{t("config.guardrail_note")}</p>
              {appConfig && (
                <div className="config-grid-mini">
                  <div className="config-row">
                    <span>{t("config.min_space_label")}:</span>
                    <span className="font-mono secure">{appConfig.workspace.min_free_space_multiplier}x</span>
                  </div>
                  <div className="config-row">
                    <span>{t("config.landlock_enforced")}:</span>
                    <span className="font-mono secure">
                      {appConfig.security.enforce_landlock ? "true" : "false"}
                    </span>
                  </div>
                  <div className="config-row">
                    <span>{t("config.theme_label")}:</span>
                    <span className="font-mono">{appConfig.app.theme}</span>
                  </div>
                </div>
              )}
            </div>

            {/* Phase 7.1: External Tools Ecosystem Status */}
            <div className="security-summary-card tools-ecosystem-card">
              <div className="tools-card-header">
                <div className="tools-title-group">
                  <Wrench size={15} className="text-cyan-400" />
                  <h4>{t("pipeline.tools_title")}</h4>
                </div>
                <button
                  type="button"
                  className="rescan-tools-btn"
                  onClick={loadTools}
                  disabled={isToolsLoading}
                  title={t("pipeline.refresh_tools")}
                >
                  <RefreshCw size={11} className={isToolsLoading ? "animate-spin" : ""} />
                  <span>{t("pipeline.refresh_tools")}</span>
                </button>
              </div>
              <p className="tools-desc-text text-muted">{t("pipeline.tools_desc")}</p>

              <div className="tools-status-grid">
                {externalTools.map((tool) => (
                  <div key={tool.tool_id} className={`tool-status-item ${tool.is_available ? "available" : "missing"}`}>
                    <div className="tool-status-top">
                      <span className="tool-name">{tool.display_name}</span>
                      <span className={`tool-badge ${tool.is_available ? "badge-available" : "badge-missing"}`}>
                        {tool.is_available ? (
                          <>
                            <Check size={10} />
                            <span>{t("pipeline.tool_available")}</span>
                          </>
                        ) : (
                          <>
                            <MinusCircle size={10} />
                            <span>{t("pipeline.tool_missing")}</span>
                          </>
                        )}
                      </span>
                    </div>
                    {tool.is_available ? (
                      <div className="tool-details-meta">
                        {tool.version && (
                          <div className="tool-meta-line font-mono">
                            <span className="meta-key">{t("pipeline.version_label")}:</span> {tool.version}
                          </div>
                        )}
                        {tool.executable_path && (
                          <div className="tool-meta-line font-mono text-muted">
                            <span className="meta-key">{t("pipeline.path_label")}:</span> {tool.executable_path}
                          </div>
                        )}
                      </div>
                    ) : (
                      <div className="tool-missing-note text-muted font-mono">
                        {tool.tool_id}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {activeTab === "logs" && (
          <div className="tab-logs-content">
            {logs.length > 0 ? (
              <div className="logs-terminal">
                {logs.map((log, idx) => (
                  <div key={idx} className="log-line">
                    <span className="log-prefix">&gt;</span>
                    <span className="log-text">{log}</span>
                  </div>
                ))}
              </div>
            ) : (
              <p className="tab-empty-text">{t("logs.empty_logs")}</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
