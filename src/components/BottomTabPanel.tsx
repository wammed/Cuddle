import { useState, useEffect, useMemo, useCallback } from "react";
import { Eye, History, Shield, Terminal, X, RotateCcw, GitCompare, AlertCircle, CheckCircle2, Upload, AlertTriangle, Trash2, Database } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import type { FileItem } from "../types/FileItem";
import type { ExtractedTextDto } from "../types/ExtractedTextDto";
import type { AppConfig } from "../types/AppConfig";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { FileDiffResultDto } from "../types/FileDiffResultDto";
import type { OperationHistoryDto } from "../types/OperationHistoryDto";

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
  onReflectStagedItem?: (stageId: string) => Promise<void> | void;
  onClose: () => void;
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
  onClose,
}: BottomTabPanelProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"preview" | "history" | "security" | "logs">("preview");
  const [diffResult, setDiffResult] = useState<FileDiffResultDto | null>(null);
  const [isDiffLoading, setIsDiffLoading] = useState(false);
  const [showReflectModal, setShowReflectModal] = useState(false);
  const [isReflecting, setIsReflecting] = useState(false);
  const [reflectError, setReflectError] = useState<string | null>(null);
  const [selectedStageId, setSelectedStageId] = useState<string | null>(null);

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
        await onReflectStagedItem(matchedStaged.stage_id);
      } else {
        await invoke("reflect_staged_file", { stageId: matchedStaged.stage_id });
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

  const getOpBadgeClass = (opType: string) => {
    switch (opType) {
      case "BulkRename":
        return "badge-op badge-rename";
      case "UndoBulkRename":
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
          <button
            type="button"
            className={`bottom-tab-btn ${activeTab === "preview" ? "active" : ""}`}
            onClick={() => setActiveTab("preview")}
          >
            <Eye size={13} />
            <span>{t("panels.tab_preview")}</span>
            {diffResult && diffResult.has_changes && (
              <span className="tab-badge-accent">+{diffResult.additions} -{diffResult.deletions}</span>
            )}
          </button>

          <button
            type="button"
            className={`bottom-tab-btn ${activeTab === "history" ? "active" : ""}`}
            onClick={() => setActiveTab("history")}
          >
            <History size={13} />
            <span>{t("panels.tab_history")}</span>
            {historyItems !== undefined && historyItems.length > 0 && (
              <span className="tab-badge-accent">{historyItems.length}</span>
            )}
          </button>

          <button
            type="button"
            className={`bottom-tab-btn ${activeTab === "security" ? "active" : ""}`}
            onClick={() => setActiveTab("security")}
          >
            <Shield size={13} />
            <span>{t("panels.tab_security")}</span>
          </button>

          <button
            type="button"
            className={`bottom-tab-btn ${activeTab === "logs" ? "active" : ""}`}
            onClick={() => setActiveTab("logs")}
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
            {/* 1. Staged Diff Preview */}
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
                  <p className="diff-status-text">Loading diff...</p>
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
            ) : extractedInfo ? (
              <div className="preview-extracted">
                <div className="preview-meta-bar">
                  <span>{extractedInfo.display_name}</span>
                  <span className="pill-tag secure">{extractedInfo.encoding_used}</span>
                  <span>{extractedInfo.extracted_chars} chars</span>
                </div>
                <pre className="preview-code-block">{extractedInfo.text || "(Empty content)"}</pre>
              </div>
            ) : selectedItem ? (
              <div className="preview-item-info">
                <p className="preview-placeholder-text">
                  {selectedItem.display_name} ({selectedItem.size.toString()} bytes)
                </p>
                <p className="preview-hint-text">{t("workspace.original_protected")}</p>
              </div>
            ) : (
              <p className="preview-empty-text">{t("details.no_selection")}</p>
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
              <p className="tab-empty-text">Logs are empty.</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
