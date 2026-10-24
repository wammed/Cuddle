import { useState } from "react";
import { Eye, History, Shield, Terminal, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { FileItem } from "../types/FileItem";
import type { ExtractedTextDto } from "../types/ExtractedTextDto";
import type { AppConfig } from "../types/AppConfig";

interface BottomTabPanelProps {
  selectedItem: FileItem | null;
  extractedInfo: ExtractedTextDto | null;
  appConfig: AppConfig | null;
  logs: string[];
  history: string[];
  onClose: () => void;
}

export function BottomTabPanel({
  selectedItem,
  extractedInfo,
  appConfig,
  logs,
  history,
  onClose,
}: BottomTabPanelProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"preview" | "history" | "security" | "logs">("preview");

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
            {extractedInfo ? (
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
