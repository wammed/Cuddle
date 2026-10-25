// src/components/BulkRenameModal.tsx
// Cuddle - Phase 5.1: Bulk Rename & Normalization Modal with Real-time Dry-run

import React, { useState, useEffect, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Plus,
  Trash2,
  X,
  ShieldCheck,
  ArrowRight,
  Sparkles,
  Sliders,
  FileEdit,
  ExternalLink,
  RefreshCw,
} from "lucide-react";

import type { StagedItemDto } from "../types/StagedItemDto";
import type { FileItem } from "../types/FileItem";
import type { BatchPlanDto } from "../types/BatchPlanDto";
import type { BatchExecutionResultDto } from "../types/BatchExecutionResultDto";
import type { BulkRenameOptions } from "../types/BulkRenameOptions";
import type { RenameRule } from "../types/RenameRule";
import type { RenameTargetInputDto } from "../types/RenameTargetInputDto";

interface BulkRenameModalProps {
  isOpen: boolean;
  onClose: () => void;
  items: (StagedItemDto | FileItem)[];
  onConfirmPlan?: (plan: BatchPlanDto) => void;
  onExecuteSuccess?: (result: BatchExecutionResultDto) => void;
}

export const BulkRenameModal: React.FC<BulkRenameModalProps> = ({
  isOpen,
  onClose,
  items,
  onConfirmPlan,
  onExecuteSuccess,
}) => {
  const { t } = useTranslation();

  // Active rules list
  const [rules, setRules] = useState<RenameRule[]>([
    {
      type: "ConvertCharacter",
      half_width_alnum: true,
      half_width_kana: false,
      space_to_underscore: false,
      underscore_to_space: false,
      to_lowercase: false,
      to_uppercase: false,
    },
  ]);

  // UI state for adding a new rule
  const [selectedRuleType, setSelectedRuleType] = useState<string>("regex");

  // Rule-specific parameters for additions
  const [regexPattern, setRegexPattern] = useState("");
  const [regexReplacement, setRegexReplacement] = useState("");
  const [normForm, setNormForm] = useState<"Nfc" | "Nfd" | "Nfkc" | "Nfkd">("Nfc");
  const [prefix, setPrefix] = useState("");
  const [suffix, setSuffix] = useState("");
  const [preserveExt, setPreserveExt] = useState(true);
  const [seqStart, setSeqStart] = useState(1);
  const [seqStep, setSeqStep] = useState(1);
  const [seqPadding, setSeqPadding] = useState(3);
  const [seqPosition, setSeqPosition] = useState<"Prefix" | "SuffixBeforeExtension">("SuffixBeforeExtension");
  const [dtFormat, setDtFormat] = useState("%Y%m%d_%H%M%S");
  const [dtSource, setDtSource] = useState<"CurrentTime" | "FileModifiedTime">("CurrentTime");

  // Mode toggle (rules vs vidir editor)
  const [mode, setMode] = useState<"rules" | "vidir">("rules");
  const [vidirContent, setVidirContent] = useState<string>("");
  const [customEditor, setCustomEditor] = useState<string>("");
  const [isLaunchingEditor, setIsLaunchingEditor] = useState<boolean>(false);

  // Dry-run results
  const [plan, setPlan] = useState<BatchPlanDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Convert inputs to RenameTargetInputDto
  const targetInputs: RenameTargetInputDto[] = useMemo(() => {
    return items.map((it) => {
      if ("stage_id" in it) {
        // StagedItemDto
        return {
          item_id: it.stage_id,
          path_or_base64: it.staged_path,
          display_name: it.display_name,
          mtime_ms: it.staged_at_ms ?? null,
        };
      } else {
        // FileItem
        return {
          item_id: it.item_id,
          path_or_base64: it.raw_base64,
          display_name: it.display_name,
          mtime_ms: it.mtime_ms ?? null,
        };
      }
    });
  }, [items]);

  // Compute dry-run plan for rules
  const computeDryRun = useCallback(
    async (currentRules: RenameRule[]) => {
      if (targetInputs.length === 0) {
        setPlan(null);
        return;
      }

      setLoading(true);
      setError(null);

      try {
        const options: BulkRenameOptions = {
          target_items: targetInputs,
          rules: currentRules,
        };

        const result = await invoke<BatchPlanDto>("generate_bulk_rename_plan", {
          options,
        });
        setPlan(result);
      } catch (err) {
        console.error("Dry-run computation failed:", err);
        setError(typeof err === "string" ? err : String(err));
      } finally {
        setLoading(false);
      }
    },
    [targetInputs]
  );

  // Load vidir buffer text from backend
  const loadVidirBuffer = useCallback(async () => {
    if (targetInputs.length === 0) return;
    try {
      setLoading(true);
      setError(null);
      const text = await invoke<string>("generate_vidir_text", {
        targetItems: targetInputs,
      });
      setVidirContent(text);
      const initialPlan = await invoke<BatchPlanDto>("parse_vidir_text", {
        targetItems: targetInputs,
        content: text,
      });
      setPlan(initialPlan);
    } catch (err) {
      console.error("Failed to generate vidir buffer:", err);
      setError(typeof err === "string" ? err : String(err));
    } finally {
      setLoading(false);
    }
  }, [targetInputs]);

  // Handle in-app vidir textarea edits
  const handleVidirChange = async (newText: string) => {
    setVidirContent(newText);
    try {
      const newPlan = await invoke<BatchPlanDto>("parse_vidir_text", {
        targetItems: targetInputs,
        content: newText,
      });
      setPlan(newPlan);
      setError(null);
    } catch (err) {
      console.error("Failed to parse vidir buffer:", err);
      setError(typeof err === "string" ? err : String(err));
    }
  };

  // Launch external editor ($VISUAL, $EDITOR, code, etc.)
  const handleLaunchExternalEditor = async () => {
    if (isLaunchingEditor || targetInputs.length === 0) return;
    setIsLaunchingEditor(true);
    setError(null);

    try {
      const newPlan = await invoke<BatchPlanDto>("launch_external_editor_vidir", {
        targetItems: targetInputs,
        customEditor: customEditor.trim() ? customEditor.trim() : null,
      });
      setPlan(newPlan);

      // Re-sync textarea with updated plan values
      const text = await invoke<string>("generate_vidir_text", {
        targetItems: targetInputs,
      });
      const lines = text.split("\n").map((line) => {
        if (line.startsWith("#") || !line.trim()) return line;
        const tabPos = line.indexOf("\t");
        if (tabPos >= 0) {
          const id = line.slice(0, tabPos);
          const op = newPlan.operations.find((o) => o.item_id === id);
          if (op) {
            return `${id}\t${op.proposed_name}`;
          }
        }
        return line;
      });
      setVidirContent(lines.join("\n"));
    } catch (err) {
      console.error("External editor launch failed:", err);
      setError(typeof err === "string" ? err : String(err));
    } finally {
      setIsLaunchingEditor(false);
    }
  };

  // Switch mode handler
  const handleSwitchMode = (newMode: "rules" | "vidir") => {
    setMode(newMode);
    if (newMode === "vidir") {
      loadVidirBuffer();
    } else {
      computeDryRun(rules);
    }
  };

  const handleExecute = async () => {
    if (!plan || !plan.is_executable || isExecuting) return;

    setIsExecuting(true);
    setError(null);

    try {
      const result = await invoke<BatchExecutionResultDto>("execute_bulk_rename", {
        plan,
      });

      if (result.success) {
        if (onExecuteSuccess) {
          onExecuteSuccess(result);
        } else if (onConfirmPlan) {
          onConfirmPlan(plan);
        }
        onClose();
      } else {
        setError(result.error_message || t("bulk_rename.execute_failed"));
      }
    } catch (err) {
      console.error("Execute bulk rename failed:", err);
      const errMsg = typeof err === "string" ? err : String(err);
      setError(`${errMsg} - ${t("bulk_rename.execute_failed")}`);
    } finally {
      setIsExecuting(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      if (mode === "rules") {
        computeDryRun(rules);
      } else {
        loadVidirBuffer();
      }
    }
  }, [isOpen, rules, mode, computeDryRun, loadVidirBuffer]);

  if (!isOpen) return null;

  // Add rule handler
  const handleAddRule = () => {
    let newRule: RenameRule;
    switch (selectedRuleType) {
      case "regex":
        if (!regexPattern) return;
        newRule = {
          type: "RegexReplace",
          pattern: regexPattern,
          replacement: regexReplacement,
        };
        break;
      case "normalize":
        newRule = {
          type: "UnicodeNormalize",
          form: normForm,
        };
        break;
      case "prefix_suffix":
        newRule = {
          type: "AddPrefixSuffix",
          prefix,
          suffix,
          preserve_extension: preserveExt,
        };
        break;
      case "sequence":
        newRule = {
          type: "AddSequence",
          start: Number(seqStart) || 1,
          step: Number(seqStep) || 1,
          padding: Number(seqPadding) || 3,
          position: seqPosition,
          preserve_extension: preserveExt,
        };
        break;
      case "datetime":
        newRule = {
          type: "AddDateTime",
          format: dtFormat,
          source: dtSource,
          position: "SuffixBeforeExtension",
          preserve_extension: preserveExt,
        };
        break;
      case "convert_char":
        newRule = {
          type: "ConvertCharacter",
          half_width_alnum: true,
          half_width_kana: false,
          space_to_underscore: false,
          underscore_to_space: false,
          to_lowercase: false,
          to_uppercase: false,
        };
        break;
      default:
        return;
    }

    const updated = [...rules, newRule];
    setRules(updated);
  };

  const handleRemoveRule = (index: number) => {
    const updated = rules.filter((_, i) => i !== index);
    setRules(updated);
  };

  const renderStatusBadge = (opStatus: string, msg: string | null) => {
    switch (opStatus) {
      case "Ok":
        return (
          <span className="status-badge ok" title={msg || undefined}>
            <CheckCircle2 size={12} />
            {t("bulk_rename.status_ok")}
          </span>
        );
      case "Unchanged":
        return (
          <span className="status-badge unchanged" title={msg || undefined}>
            {t("bulk_rename.status_unchanged")}
          </span>
        );
      case "ConflictDuplicate":
        return (
          <span className="status-badge conflict" title={msg || undefined}>
            <AlertTriangle size={12} />
            {t("bulk_rename.status_conflict_duplicate")}
          </span>
        );
      case "ConflictExistsOnDisk":
        return (
          <span className="status-badge conflict" title={msg || undefined}>
            <AlertTriangle size={12} />
            {t("bulk_rename.status_conflict_disk")}
          </span>
        );
      case "InvalidFilename":
      case "Error":
        return (
          <span className="status-badge error" title={msg || undefined}>
            <XCircle size={12} />
            {t("bulk_rename.status_invalid")}
          </span>
        );
      default:
        return <span>{opStatus}</span>;
    }
  };

  return (
    <div className="modal-backdrop">
      <div className="bulk-rename-modal">
        {/* Header */}
        <div className="dialog-header">
          <div className="dialog-title-group">
            <Sparkles size={18} className="text-amber-400" />
            <div>
              <h4>{t("bulk_rename.title")}</h4>
              <p className="modal-subtitle">{t("bulk_rename.subtitle")}</p>
            </div>
          </div>
          <button type="button" className="panel-header-btn" onClick={onClose} title={t("common.close")}>
            <X size={16} />
          </button>
        </div>

        {/* Modal Body */}
        <div className="bulk-rename-body">
          {/* Mode Switch Tabs */}
          <div className="bulk-rename-tab-bar">
            <button
              type="button"
              className={`bulk-rename-tab-btn ${mode === "rules" ? "active" : ""}`}
              onClick={() => handleSwitchMode("rules")}
            >
              <Sliders size={13} />
              <span>{t("bulk_rename.tab_rules")}</span>
            </button>
            <button
              type="button"
              className={`bulk-rename-tab-btn ${mode === "vidir" ? "active" : ""}`}
              onClick={() => handleSwitchMode("vidir")}
            >
              <FileEdit size={13} />
              <span>{t("bulk_rename.tab_vidir")}</span>
            </button>
          </div>

          {/* Top: Summary Badges */}
          {plan && (
            <div className="dry-run-summary-bar">
              <div className="summary-stat">
                <span className="stat-label">{t("bulk_rename.total_items")}:</span>
                <span className="stat-val">{plan.total_count}</span>
              </div>
              <div className="summary-stat">
                <span className="stat-label">{t("bulk_rename.changed_items")}:</span>
                <span className="stat-val text-cyan-400">{plan.changed_count}</span>
              </div>
              <div className="summary-stat">
                <span className="stat-label">{t("bulk_rename.conflict_items")}:</span>
                <span className={`stat-val ${plan.conflict_count > 0 ? "text-amber-400 font-bold" : ""}`}>
                  {plan.conflict_count}
                </span>
              </div>
              <div className="summary-stat">
                <span className="stat-label">{t("bulk_rename.error_items")}:</span>
                <span className={`stat-val ${plan.error_count > 0 ? "text-red-400 font-bold" : ""}`}>
                  {plan.error_count}
                </span>
              </div>

              <div className="summary-validation-status">
                {plan.is_executable ? (
                  <span className="badge-pass">
                    <ShieldCheck size={14} />
                    {t("bulk_rename.dry_run_success")}
                  </span>
                ) : (
                  <span className="badge-warning">
                    <AlertTriangle size={14} />
                    {t("bulk_rename.conflict_warning")}
                  </span>
                )}
              </div>
            </div>
          )}

          {error && (
            <div className="dialog-error-banner">
              <div className="error-banner-header">
                <AlertTriangle size={14} className="text-red-400" />
                <h5>{t("common.error")}</h5>
              </div>
              <p className="dialog-error-text">{error}</p>
            </div>
          )}

          {/* Mode 1: Rules Configuration Section */}
          {mode === "rules" && (
            <div className="rules-section">
            <div className="section-title-row">
              <h5>{t("bulk_rename.rules_header")}</h5>
              <div className="add-rule-controls">
                <select
                  value={selectedRuleType}
                  onChange={(e) => setSelectedRuleType(e.target.value)}
                  className="rule-type-select"
                >
                  <option value="regex">{t("bulk_rename.regex")}</option>
                  <option value="normalize">{t("bulk_rename.normalize")}</option>
                  <option value="convert_char">{t("bulk_rename.convert_char")}</option>
                  <option value="prefix_suffix">{t("bulk_rename.prefix_suffix")}</option>
                  <option value="sequence">{t("bulk_rename.sequence")}</option>
                  <option value="datetime">{t("bulk_rename.datetime")}</option>
                </select>
                <button type="button" className="btn-add-rule" onClick={handleAddRule}>
                  <Plus size={14} />
                  {t("bulk_rename.add_rule")}
                </button>
              </div>
            </div>

            {/* Rule input parameters preview row */}
            <div className="rule-inputs-builder">
              {selectedRuleType === "regex" && (
                <div className="rule-builder-group">
                  <input
                    type="text"
                    placeholder={t("bulk_rename.regex_pattern")}
                    value={regexPattern}
                    onChange={(e) => setRegexPattern(e.target.value)}
                    className="rule-input"
                  />
                  <input
                    type="text"
                    placeholder={t("bulk_rename.regex_replacement")}
                    value={regexReplacement}
                    onChange={(e) => setRegexReplacement(e.target.value)}
                    className="rule-input"
                  />
                </div>
              )}

              {selectedRuleType === "normalize" && (
                <div className="rule-builder-group">
                  <select
                    value={normForm}
                    onChange={(e) => setNormForm(e.target.value as any)}
                    className="rule-input"
                  >
                    <option value="Nfc">{t("bulk_rename.normalize_nfc")}</option>
                    <option value="Nfd">{t("bulk_rename.normalize_nfd")}</option>
                    <option value="Nfkc">{t("bulk_rename.normalize_nfkc")}</option>
                    <option value="Nfkd">{t("bulk_rename.normalize_nfkd")}</option>
                  </select>
                </div>
              )}

              {selectedRuleType === "prefix_suffix" && (
                <div className="rule-builder-group">
                  <input
                    type="text"
                    placeholder={t("bulk_rename.prefix")}
                    value={prefix}
                    onChange={(e) => setPrefix(e.target.value)}
                    className="rule-input"
                  />
                  <input
                    type="text"
                    placeholder={t("bulk_rename.suffix")}
                    value={suffix}
                    onChange={(e) => setSuffix(e.target.value)}
                    className="rule-input"
                  />
                  <label className="checkbox-label">
                    <input
                      type="checkbox"
                      checked={preserveExt}
                      onChange={(e) => setPreserveExt(e.target.checked)}
                    />
                    {t("bulk_rename.preserve_ext")}
                  </label>
                </div>
              )}

              {selectedRuleType === "sequence" && (
                <div className="rule-builder-group">
                  <div className="input-with-label">
                    <span>{t("bulk_rename.seq_start")}:</span>
                    <input
                      type="number"
                      value={seqStart}
                      onChange={(e) => setSeqStart(Number(e.target.value))}
                      className="rule-input-sm"
                    />
                  </div>
                  <div className="input-with-label">
                    <span>{t("bulk_rename.seq_step")}:</span>
                    <input
                      type="number"
                      value={seqStep}
                      onChange={(e) => setSeqStep(Number(e.target.value))}
                      className="rule-input-sm"
                    />
                  </div>
                  <div className="input-with-label">
                    <span>{t("bulk_rename.seq_padding")}:</span>
                    <input
                      type="number"
                      value={seqPadding}
                      onChange={(e) => setSeqPadding(Number(e.target.value))}
                      className="rule-input-sm"
                    />
                  </div>
                  <select
                    value={seqPosition}
                    onChange={(e) => setSeqPosition(e.target.value as any)}
                    className="rule-input"
                  >
                    <option value="SuffixBeforeExtension">{t("bulk_rename.seq_pos_suffix")}</option>
                    <option value="Prefix">{t("bulk_rename.seq_pos_prefix")}</option>
                  </select>
                </div>
              )}

              {selectedRuleType === "datetime" && (
                <div className="rule-builder-group">
                  <input
                    type="text"
                    value={dtFormat}
                    onChange={(e) => setDtFormat(e.target.value)}
                    placeholder={t("bulk_rename.datetime_format")}
                    className="rule-input"
                  />
                  <select
                    value={dtSource}
                    onChange={(e) => setDtSource(e.target.value as any)}
                    className="rule-input"
                  >
                    <option value="CurrentTime">{t("bulk_rename.datetime_now")}</option>
                    <option value="FileModifiedTime">{t("bulk_rename.datetime_mtime")}</option>
                  </select>
                </div>
              )}
            </div>

            {/* Active Rules List */}
            <div className="active-rules-list">
              {rules.map((rule, idx) => (
                <div key={idx} className="active-rule-card">
                  <span className="rule-index">#{idx + 1}</span>
                  <div className="rule-desc">
                    {rule.type === "RegexReplace" && (
                      <span>
                        <strong>{t("bulk_rename.regex")}:</strong> <code>/{rule.pattern}/</code> →{" "}
                        <code>"{rule.replacement}"</code>
                      </span>
                    )}
                    {rule.type === "UnicodeNormalize" && (
                      <span>
                        <strong>{t("bulk_rename.normalize")}:</strong> {rule.form}
                      </span>
                    )}
                    {rule.type === "ConvertCharacter" && (
                      <span>
                        <strong>{t("bulk_rename.convert_char")}:</strong>{" "}
                        {rule.half_width_alnum && t("bulk_rename.half_width_alnum")}
                      </span>
                    )}
                    {rule.type === "AddPrefixSuffix" && (
                      <span>
                        <strong>{t("bulk_rename.prefix_suffix")}:</strong>{" "}
                        {rule.prefix && `+${rule.prefix} `}
                        {rule.suffix && `+${rule.suffix}`}
                      </span>
                    )}
                    {rule.type === "AddSequence" && (
                      <span>
                        <strong>{t("bulk_rename.sequence")}:</strong> {rule.start}.. (
                        {rule.padding} digits)
                      </span>
                    )}
                    {rule.type === "AddDateTime" && (
                      <span>
                        <strong>{t("bulk_rename.datetime")}:</strong> {rule.format}
                      </span>
                    )}
                  </div>
                  <button
                    type="button"
                    className="rule-remove-btn"
                    onClick={() => handleRemoveRule(idx)}
                    title={t("bulk_rename.remove_rule")}
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              ))}
            </div>
          </div>
          )}

          {/* Mode 2: vidir Editor Section */}
          {mode === "vidir" && (
            <div className="vidir-editor-section">
              <div className="vidir-toolbar">
                <div className="vidir-actions-left">
                  <button
                    type="button"
                    className="btn-vidir-ext"
                    onClick={handleLaunchExternalEditor}
                    disabled={isLaunchingEditor}
                    title={t("bulk_rename.launch_external_editor")}
                  >
                    <ExternalLink size={13} />
                    <span>
                      {isLaunchingEditor
                        ? t("bulk_rename.vidir_launching")
                        : t("bulk_rename.launch_external_editor")}
                    </span>
                  </button>
                  <input
                    type="text"
                    className="vidir-custom-cmd-input"
                    placeholder={t("bulk_rename.external_editor_placeholder")}
                    value={customEditor}
                    onChange={(e) => setCustomEditor(e.target.value)}
                  />
                  <button
                    type="button"
                    className="panel-header-btn"
                    onClick={loadVidirBuffer}
                    title={t("bulk_rename.reload_vidir")}
                  >
                    <RefreshCw size={13} />
                  </button>
                </div>
                <span className="vidir-safe-badge">
                  <ShieldCheck size={13} />
                  <span>{t("bulk_rename.vidir_safe_notice")}</span>
                </span>
              </div>

              <div className="vidir-textarea-wrap">
                <textarea
                  className="vidir-textarea"
                  value={vidirContent}
                  onChange={(e) => handleVidirChange(e.target.value)}
                  spellCheck={false}
                />
              </div>
              <span className="vidir-hint-text">{t("bulk_rename.vidir_hint")}</span>
            </div>
          )}

          {/* Dry-run Table Section */}
          <div className="preview-table-section">
            <h5>{t("bulk_rename.preview_header")}</h5>
            <div className="table-scroll-container">
              <table className="dry-run-table">
                <thead>
                  <tr>
                    <th>{t("bulk_rename.col_original")}</th>
                    <th className="arrow-col"></th>
                    <th>{t("bulk_rename.col_proposed")}</th>
                    <th>{t("bulk_rename.col_status")}</th>
                  </tr>
                </thead>
                <tbody>
                  {plan?.operations.map((op) => (
                    <tr
                      key={op.item_id}
                      className={
                        op.status === "ConflictDuplicate" || op.status === "ConflictExistsOnDisk"
                          ? "row-conflict"
                          : op.status === "InvalidFilename" || op.status === "Error"
                          ? "row-error"
                          : ""
                      }
                    >
                      <td className="font-mono text-muted">{op.display_name}</td>
                      <td className="arrow-col">
                        <ArrowRight size={14} className="text-muted" />
                      </td>
                      <td className="font-mono font-medium text-cyan-300">{op.proposed_name}</td>
                      <td>{renderStatusBadge(op.status, op.message)}</td>
                    </tr>
                  ))}
                  {(!plan || plan.operations.length === 0) && (
                    <tr>
                      <td colSpan={4} className="empty-table-cell">
                        {loading ? t("common.status_running") : t("workspace.empty_state")}
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="dialog-footer">
          <button type="button" className="dialog-btn secondary" onClick={onClose} disabled={isExecuting}>
            {t("bulk_rename.cancel")}
          </button>
          <button
            type="button"
            className="dialog-btn primary"
            disabled={!plan?.is_executable || loading || isExecuting}
            onClick={handleExecute}
          >
            <Sparkles size={14} className={isExecuting ? "animate-spin" : ""} />
            {isExecuting ? t("bulk_rename.executing") : t("bulk_rename.apply_plan")}
          </button>
        </div>
      </div>
    </div>
  );
};
