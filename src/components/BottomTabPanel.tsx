import { useState, useEffect, useMemo, useCallback } from "react";
import { Eye, History, Shield, Terminal, X, RotateCcw, GitCompare, AlertCircle, CheckCircle2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import type { FileItem } from "../types/FileItem";
import type { ExtractedTextDto } from "../types/ExtractedTextDto";
import type { AppConfig } from "../types/AppConfig";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { FileDiffResultDto } from "../types/FileDiffResultDto";

interface BottomTabPanelProps {
  selectedItem: FileItem | null;
  extractedInfo: ExtractedTextDto | null;
  appConfig: AppConfig | null;
  logs: string[];
  history: string[];
  stagedItems?: StagedItemDto[];
  onResetStagedItem?: (stageId: string) => void;
  onClose: () => void;
}

export function BottomTabPanel({
  selectedItem,
  extractedInfo,
  appConfig,
  logs,
  history,
  stagedItems = [],
  onResetStagedItem,
  onClose,
}: BottomTabPanelProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"preview" | "history" | "security" | "logs">("preview");
  const [diffResult, setDiffResult] = useState<FileDiffResultDto | null>(null);
  const [isDiffLoading, setIsDiffLoading] = useState(false);

  const matchedStaged = useMemo(() => {
    if (!selectedItem) return null;
    return (
      stagedItems.find(
        (s) => s.original_item_id === selectedItem.item_id || s.stage_id === selectedItem.item_id
      ) ?? null
    );
  }, [selectedItem, stagedItems]);

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
  }, [matchedStaged, loadDiff]);

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

  return (
    <div className="panel-container bottom-tab-container">
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
                    <span className="diff-file-tag">{matchedStaged.display_name}</span>
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
                    <button
                      type="button"
                      className="diff-reset-btn"
                      onClick={handleReset}
                      title={t("diff.reset_to_original")}
                      disabled={isDiffLoading}
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
            {history.length > 0 ? (
              <ul className="history-list">
                {history.map((entry, idx) => (
                  <li key={idx} className="history-entry">
                    <span className="history-bullet">•</span>
                    <span className="history-text">{entry}</span>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="tab-empty-text">No operations performed in this session.</p>
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
