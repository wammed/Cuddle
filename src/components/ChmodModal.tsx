// src/components/ChmodModal.tsx
// Cuddle - Phase 9.2: Batch Attribute / Permission Change (chmod) with Real-time Dry-run

import React, { useState, useEffect, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  AlertTriangle,
  CheckCircle2,
  XCircle,
  X,
  ArrowRight,
  ShieldAlert,
  Sparkles,
  FileCode,
  Folder,
  File,
} from "lucide-react";

import type { StagedItemDto } from "../types/StagedItemDto";
import type { FileItem } from "../types/FileItem";
import type { ChmodTargetInputDto } from "../types/ChmodTargetInputDto";
import type { ChmodModeSpec } from "../types/ChmodModeSpec";
import type { ChmodPlanDto } from "../types/ChmodPlanDto";
import type { ChmodExecutionResultDto } from "../types/ChmodExecutionResultDto";
import type { PlannedChmodItemDto } from "../types/PlannedChmodItemDto";

interface ChmodModalProps {
  isOpen: boolean;
  onClose: () => void;
  items: (StagedItemDto | FileItem)[];
  onSuccess?: (result: ChmodExecutionResultDto) => void;
}

export const ChmodModal: React.FC<ChmodModalProps> = ({
  isOpen,
  onClose,
  items,
  onSuccess,
}) => {
  const { t } = useTranslation();

  const [modeType, setModeType] = useState<"octal" | "symbolic">("octal");
  const [octalValue, setOctalValue] = useState<string>("755");
  const [symbolicValue, setSymbolicValue] = useState<string>("+x");

  // Permission matrix state (user, group, others x r, w, x)
  const [matrixState, setMatrixState] = useState({
    uR: true, uW: true, uX: true,
    gR: true, gW: false, gX: true,
    oR: true, oW: false, oX: true,
  });

  const [plan, setPlan] = useState<ChmodPlanDto | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [isExecuting, setIsExecuting] = useState<boolean>(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [feedbackMessage, setFeedbackMessage] = useState<string | null>(null);

  // Convert inputs to ChmodTargetInputDto
  const targetInputs: ChmodTargetInputDto[] = useMemo(() => {
    return items.map((it) => {
      if ("stage_id" in it) {
        return {
          item_id: it.stage_id,
          path_or_base64: it.staged_path,
          display_name: it.display_name,
        };
      } else {
        return {
          item_id: it.item_id,
          path_or_base64: it.raw_base64,
          display_name: it.display_name,
        };
      }
    });
  }, [items]);

  // Synchronize matrix checkboxes with an octal value
  const applyOctalToMatrix = (val: string) => {
    const trimmed = val.trim();
    if (/^[0-7]{3,4}$/.test(trimmed)) {
      const octalPart = trimmed.slice(-3);
      const u = parseInt(octalPart[0], 10);
      const g = parseInt(octalPart[1], 10);
      const o = parseInt(octalPart[2], 10);
      setMatrixState({
        uR: (u & 4) !== 0, uW: (u & 2) !== 0, uX: (u & 1) !== 0,
        gR: (g & 4) !== 0, gW: (g & 2) !== 0, gX: (g & 1) !== 0,
        oR: (o & 4) !== 0, oW: (o & 2) !== 0, oX: (o & 1) !== 0,
      });
    }
  };

  // Convert matrix to octal string
  const updateMatrixAndSyncOctal = (updater: (prev: typeof matrixState) => typeof matrixState) => {
    const next = updater(matrixState);
    setMatrixState(next);
    const u = (next.uR ? 4 : 0) + (next.uW ? 2 : 0) + (next.uX ? 1 : 0);
    const g = (next.gR ? 4 : 0) + (next.gW ? 2 : 0) + (next.gX ? 1 : 0);
    const o = (next.oR ? 4 : 0) + (next.oW ? 2 : 0) + (next.oX ? 1 : 0);
    const newOctal = `${u}${g}${o}`;
    setOctalValue(newOctal);
  };

  // Dry-run plan computation
  const computePlan = useCallback(async () => {
    if (targetInputs.length === 0) {
      setPlan(null);
      return;
    }

    setLoading(true);
    setErrorMessage(null);

    const modeSpec: ChmodModeSpec =
      modeType === "octal"
        ? { type: "Octal", value: octalValue.trim() }
        : { type: "Symbolic", value: symbolicValue.trim() };

    try {
      const generatedPlan = await invoke<ChmodPlanDto>("generate_chmod_plan", {
        targets: targetInputs,
        mode: modeSpec,
      });
      setPlan(generatedPlan);
      if (generatedPlan.error_message) {
        setErrorMessage(generatedPlan.error_message);
      }
    } catch (err: unknown) {
      setPlan(null);
      setErrorMessage(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setLoading(false);
    }
  }, [targetInputs, modeType, octalValue, symbolicValue]);

  // Recalculate dry-run on input changes
  useEffect(() => {
    if (isOpen) {
      computePlan();
    }
  }, [isOpen, computePlan]);

  // Apply preset octal value
  const handleSelectOctalPreset = (val: string) => {
    setOctalValue(val);
    applyOctalToMatrix(val);
  };

  // Apply preset symbolic value
  const handleSelectSymbolicPreset = (val: string) => {
    setSymbolicValue(val);
  };

  // Execute chmod plan
  const handleExecute = async () => {
    if (!plan || !plan.is_executable || isExecuting) return;

    setIsExecuting(true);
    setErrorMessage(null);
    setFeedbackMessage(null);

    try {
      const result = await invoke<ChmodExecutionResultDto>("execute_chmod_plan", {
        plan,
      });

      if (result.success) {
        setFeedbackMessage(
          t("chmod.execute_success", { count: result.succeeded_count })
        );
        if (onSuccess) {
          onSuccess(result);
        }
        setTimeout(() => {
          onClose();
        }, 1200);
      } else {
        setErrorMessage(
          result.error_message || t("chmod.execute_failed")
        );
      }
    } catch (err: unknown) {
      setErrorMessage(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setIsExecuting(false);
    }
  };

  // Render status badge for an item
  const renderItemStatusBadge = (item: PlannedChmodItemDto) => {
    if (item.warning) {
      return (
        <span className="badge badge-warning" title={item.warning}>
          <AlertTriangle size={12} className="inline mr-1" />
          {item.warning}
        </span>
      );
    }
    if (item.has_special_bits) {
      return (
        <span className="badge badge-warning" title={t("chmod.status_special")}>
          <AlertTriangle size={12} className="inline mr-1" />
          {t("chmod.status_special")}
        </span>
      );
    }
    if (!item.is_changed) {
      return (
        <span className="badge badge-muted">
          {t("chmod.status_unchanged")}
        </span>
      );
    }
    return (
      <span className="badge badge-info">
        <CheckCircle2 size={12} className="inline mr-1" />
        {t("chmod.status_ok")}
      </span>
    );
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-container chmod-modal-dialog"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="chmod-modal-title"
      >
        {/* Header */}
        <div className="dialog-header">
          <div className="header-title-group">
            <h3 id="chmod-modal-title" className="dialog-title flex items-center gap-2">
              <FileCode size={20} className="text-cyan-400" />
              <span>{t("chmod.dialog_title")}</span>
            </h3>
            <p className="dialog-subtitle">{t("chmod.dialog_subtitle")}</p>
          </div>
          <button
            type="button"
            className="dialog-close-btn"
            onClick={onClose}
            aria-label={t("common.close")}
          >
            <X size={18} />
          </button>
        </div>

        {/* Modal Body */}
        <div className="dialog-body">
          {/* Mode Tabs */}
          <div className="chmod-tab-bar">
            <button
              type="button"
              className={`chmod-tab-btn ${modeType === "octal" ? "active" : ""}`}
              onClick={() => setModeType("octal")}
            >
              {t("chmod.tab_octal")}
            </button>
            <button
              type="button"
              className={`chmod-tab-btn ${modeType === "symbolic" ? "active" : ""}`}
              onClick={() => setModeType("symbolic")}
            >
              {t("chmod.tab_symbolic")}
            </button>
          </div>

          {/* Configuration Area */}
          <div className="chmod-config-section">
            {modeType === "octal" ? (
              <div className="chmod-octal-group">
                <div className="form-group mb-3">
                  <label htmlFor="octal-input" className="form-label">
                    {t("chmod.octal_input_label")}
                  </label>
                  <input
                    id="octal-input"
                    type="text"
                    className="form-input font-mono"
                    value={octalValue}
                    placeholder={t("chmod.octal_placeholder")}
                    onChange={(e) => {
                      setOctalValue(e.target.value);
                      applyOctalToMatrix(e.target.value);
                    }}
                    maxLength={4}
                  />
                </div>

                {/* Octal Presets */}
                <div className="presets-container mb-4">
                  <div className="text-xs text-muted mb-2">{t("chmod.quick_symbolic_label")}</div>
                  <div className="chmod-presets-grid">
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => handleSelectOctalPreset("644")}
                    >
                      {t("chmod.preset_644")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => handleSelectOctalPreset("755")}
                    >
                      {t("chmod.preset_755")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => handleSelectOctalPreset("600")}
                    >
                      {t("chmod.preset_600")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => handleSelectOctalPreset("700")}
                    >
                      {t("chmod.preset_700")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => handleSelectOctalPreset("664")}
                    >
                      {t("chmod.preset_664")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => handleSelectOctalPreset("775")}
                    >
                      {t("chmod.preset_775")}
                    </button>
                  </div>
                </div>

                {/* Visual Permission Matrix */}
                <div className="chmod-matrix-card">
                  <div className="matrix-title text-xs font-semibold text-muted mb-2">
                    パーミッション・マトリクス (r / w / x)
                  </div>
                  <div className="matrix-table-wrapper">
                    <table className="chmod-matrix-table">
                      <thead>
                        <tr>
                          <th>対象</th>
                          <th>{t("chmod.perm_read")}</th>
                          <th>{t("chmod.perm_write")}</th>
                          <th>{t("chmod.perm_exec")}</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr>
                          <td className="matrix-target-cell">{t("chmod.matrix_owner")}</td>
                          <td>
                            <input
                              type="checkbox"
                              checked={matrixState.uR}
                              onChange={(e) => updateMatrixAndSyncOctal(prev => ({ ...prev, uR: e.target.checked }))}
                            />
                          </td>
                          <td>
                            <input
                              type="checkbox"
                              checked={matrixState.uW}
                              onChange={(e) => updateMatrixAndSyncOctal(prev => ({ ...prev, uW: e.target.checked }))}
                            />
                          </td>
                          <td>
                            <input
                              type="checkbox"
                              checked={matrixState.uX}
                              onChange={(e) => updateMatrixAndSyncOctal(prev => ({ ...prev, uX: e.target.checked }))}
                            />
                          </td>
                        </tr>
                        <tr>
                          <td className="matrix-target-cell">{t("chmod.matrix_group")}</td>
                          <td>
                            <input
                              type="checkbox"
                              checked={matrixState.gR}
                              onChange={(e) => updateMatrixAndSyncOctal(prev => ({ ...prev, gR: e.target.checked }))}
                            />
                          </td>
                          <td>
                            <input
                              type="checkbox"
                              checked={matrixState.gW}
                              onChange={(e) => updateMatrixAndSyncOctal(prev => ({ ...prev, gW: e.target.checked }))}
                            />
                          </td>
                          <td>
                            <input
                              type="checkbox"
                              checked={matrixState.gX}
                              onChange={(e) => updateMatrixAndSyncOctal(prev => ({ ...prev, gX: e.target.checked }))}
                            />
                          </td>
                        </tr>
                        <tr>
                          <td className="matrix-target-cell">{t("chmod.matrix_others")}</td>
                          <td>
                            <input
                              type="checkbox"
                              checked={matrixState.oR}
                              onChange={(e) => updateMatrixAndSyncOctal(prev => ({ ...prev, oR: e.target.checked }))}
                            />
                          </td>
                          <td>
                            <input
                              type="checkbox"
                              checked={matrixState.oW}
                              onChange={(e) => updateMatrixAndSyncOctal(prev => ({ ...prev, oW: e.target.checked }))}
                            />
                          </td>
                          <td>
                            <input
                              type="checkbox"
                              checked={matrixState.oX}
                              onChange={(e) => updateMatrixAndSyncOctal(prev => ({ ...prev, oX: e.target.checked }))}
                            />
                          </td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>
            ) : (
              <div className="chmod-symbolic-group">
                <div className="form-group mb-3">
                  <label htmlFor="symbolic-input" className="form-label">
                    {t("chmod.symbolic_input_label")}
                  </label>
                  <input
                    id="symbolic-input"
                    type="text"
                    className="form-input font-mono"
                    value={symbolicValue}
                    placeholder={t("chmod.symbolic_placeholder")}
                    onChange={(e) => setSymbolicValue(e.target.value)}
                  />
                </div>

                {/* Symbolic Presets */}
                <div className="presets-container mb-3">
                  <div className="text-xs text-muted mb-2">{t("chmod.quick_symbolic_label")}</div>
                  <div className="chmod-presets-grid">
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => handleSelectSymbolicPreset("+x")}
                    >
                      {t("chmod.sym_add_x")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => handleSelectSymbolicPreset("-x")}
                    >
                      {t("chmod.sym_remove_x")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => handleSelectSymbolicPreset("u+x")}
                    >
                      {t("chmod.sym_user_x")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => handleSelectSymbolicPreset("go-w")}
                    >
                      {t("chmod.sym_gow_remove")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => handleSelectSymbolicPreset("a+r")}
                    >
                      {t("chmod.sym_all_read")}
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Special Bits Warning Banner */}
          {plan?.has_special_bits && (
            <div className="chmod-special-warning-banner">
              <ShieldAlert size={18} className="text-amber-400 shrink-0" />
              <div className="text-sm text-amber-200">
                {t("chmod.special_bits_warning")}
              </div>
            </div>
          )}

          {/* Error Message */}
          {errorMessage && (
            <div className="chmod-error-banner">
              <XCircle size={16} className="text-rose-400 shrink-0" />
              <div className="text-sm text-rose-300">{errorMessage}</div>
            </div>
          )}

          {/* Feedback Message */}
          {feedbackMessage && (
            <div className="chmod-feedback-banner">
              <CheckCircle2 size={16} className="text-emerald-400 shrink-0" />
              <div className="text-sm text-emerald-300">{feedbackMessage}</div>
            </div>
          )}

          {/* Dry-run Table Section */}
          <div className="chmod-preview-section">
            <div className="chmod-preview-header">
              <h5 className="text-sm font-semibold text-slate-300">
                Dry-run 事前プレビュー ({plan?.total_count ?? items.length} 件中 {plan?.changed_count ?? 0} 件変更予定)
              </h5>
            </div>
            <div className="table-scroll-container">
              <table className="dry-run-table">
                <thead>
                  <tr>
                    <th>{t("chmod.col_filename")}</th>
                    <th>{t("chmod.col_current")}</th>
                    <th className="arrow-col"></th>
                    <th>{t("chmod.col_target")}</th>
                    <th>{t("chmod.col_status")}</th>
                  </tr>
                </thead>
                <tbody>
                  {plan?.items.map((item) => (
                    <tr
                      key={item.item_id}
                      className={
                        item.has_special_bits
                          ? "row-warning"
                          : !item.is_changed
                          ? "row-muted"
                          : ""
                      }
                    >
                      <td className="file-info-cell">
                        <div className="flex items-center gap-2">
                          {item.is_dir ? (
                            <Folder size={14} className="text-amber-400 shrink-0" />
                          ) : (
                            <File size={14} className="text-cyan-400 shrink-0" />
                          )}
                          <span className="font-mono text-slate-200" title={item.path}>
                            {item.display_name}
                          </span>
                        </div>
                      </td>
                      <td className="font-mono text-muted">
                        <span className="text-xs bg-slate-800/80 px-1.5 py-0.5 rounded mr-1.5 text-slate-400">
                          {item.current_octal}
                        </span>
                        {item.current_mode_str}
                      </td>
                      <td className="arrow-col">
                        <ArrowRight size={14} className="text-muted" />
                      </td>
                      <td className="font-mono font-medium text-cyan-300">
                        <span className="text-xs bg-cyan-950/80 border border-cyan-800/50 px-1.5 py-0.5 rounded mr-1.5 text-cyan-300">
                          {item.target_octal}
                        </span>
                        {item.target_mode_str}
                      </td>
                      <td>{renderItemStatusBadge(item)}</td>
                    </tr>
                  ))}
                  {(!plan || plan.items.length === 0) && (
                    <tr>
                      <td colSpan={5} className="empty-table-cell">
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
          <button
            type="button"
            className="dialog-btn secondary"
            onClick={onClose}
            disabled={isExecuting}
          >
            {t("common.cancel")}
          </button>
          <button
            type="button"
            className="dialog-btn primary"
            disabled={!plan?.is_executable || loading || isExecuting || (plan?.changed_count ?? 0) === 0}
            onClick={handleExecute}
          >
            <Sparkles size={14} className={isExecuting ? "animate-spin" : ""} />
            {isExecuting ? t("chmod.executing") : t("chmod.apply_chmod")}
          </button>
        </div>
      </div>
    </div>
  );
};
