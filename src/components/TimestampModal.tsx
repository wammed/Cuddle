// src/components/TimestampModal.tsx
// Cuddle - Phase 9.3: Batch Timestamp Change Modal (touch / relative shift / EXIF sync)

import React, { useState, useEffect, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  Clock,
  Calendar,
  Camera,
  FastForward,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  X,
  ArrowRight,
  Sparkles,
  File,
  Folder,
} from "lucide-react";

import type { StagedItemDto } from "../types/StagedItemDto";
import type { FileItem } from "../types/FileItem";
import type { TimestampTargetInputDto } from "../types/TimestampTargetInputDto";
import type { TimestampModeSpec } from "../types/TimestampModeSpec";
import type { TimestampPlanDto } from "../types/TimestampPlanDto";
import type { TimestampExecutionResultDto } from "../types/TimestampExecutionResultDto";
import type { PlannedTimestampItemDto } from "../types/PlannedTimestampItemDto";

interface TimestampModalProps {
  isOpen: boolean;
  onClose: () => void;
  items: (StagedItemDto | FileItem)[];
  onSuccess?: (result: TimestampExecutionResultDto) => void;
}

export const TimestampModal: React.FC<TimestampModalProps> = ({
  isOpen,
  onClose,
  items,
  onSuccess,
}) => {
  const { t } = useTranslation();

  const [activeTab, setActiveTab] = useState<"current" | "absolute" | "relative" | "exif">("current");
  const [absoluteValue, setAbsoluteValue] = useState<string>(() => {
    const now = new Date();
    const pad = (n: number) => n.toString().padStart(2, "0");
    return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`;
  });
  const [relativeSeconds, setRelativeSeconds] = useState<number>(3600);

  const [plan, setPlan] = useState<TimestampPlanDto | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [isExecuting, setIsExecuting] = useState<boolean>(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [feedbackMessage, setFeedbackMessage] = useState<string | null>(null);

  // Convert items to TimestampTargetInputDto
  const targetInputs: TimestampTargetInputDto[] = useMemo(() => {
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

  // Compute dry-run plan
  const computePlan = useCallback(async () => {
    if (targetInputs.length === 0) {
      setPlan(null);
      return;
    }

    setLoading(true);
    setErrorMessage(null);

    let modeSpec: TimestampModeSpec;
    switch (activeTab) {
      case "current":
        modeSpec = { type: "CurrentTime" };
        break;
      case "absolute":
        modeSpec = { type: "Absolute", value: absoluteValue.trim() };
        break;
      case "relative":
        modeSpec = { type: "RelativeOffset", value: BigInt(relativeSeconds) };
        break;
      case "exif":
        modeSpec = { type: "ExifSync" };
        break;
    }

    try {
      const generatedPlan = await invoke<TimestampPlanDto>("generate_timestamp_plan", {
        targetItems: targetInputs,
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
  }, [targetInputs, activeTab, absoluteValue, relativeSeconds]);

  // Recalculate plan on change
  useEffect(() => {
    if (isOpen) {
      computePlan();
    }
  }, [isOpen, computePlan]);

  // Execute timestamp changes
  const handleExecute = async () => {
    if (!plan || !plan.is_executable || isExecuting) return;

    setIsExecuting(true);
    setErrorMessage(null);
    setFeedbackMessage(null);

    try {
      const result = await invoke<TimestampExecutionResultDto>("execute_timestamp_plan", {
        plan,
      });

      if (result.success) {
        setFeedbackMessage(
          t("timestamp.execute_success", { count: result.succeeded_count })
        );
        if (onSuccess) {
          onSuccess(result);
        }
        setTimeout(() => {
          onClose();
        }, 1200);
      } else {
        setErrorMessage(result.error_message || t("timestamp.execute_failed"));
      }
    } catch (err: unknown) {
      setErrorMessage(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setIsExecuting(false);
    }
  };

  const renderItemStatusBadge = (item: PlannedTimestampItemDto) => {
    if (item.warning) {
      return (
        <span className="badge badge-warning" title={item.warning}>
          <AlertTriangle size={12} className="inline mr-1" />
          {item.warning}
        </span>
      );
    }
    if (!item.is_changed) {
      return (
        <span className="badge badge-muted">
          {t("timestamp.status_unchanged")}
        </span>
      );
    }
    return (
      <span className="badge badge-info">
        <CheckCircle2 size={12} className="inline mr-1" />
        {t("timestamp.status_ok")}
      </span>
    );
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-container timestamp-modal-dialog"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="timestamp-modal-title"
      >
        {/* Header */}
        <div className="dialog-header">
          <div className="header-title-group">
            <h3 id="timestamp-modal-title" className="dialog-title flex items-center gap-2">
              <Clock size={20} className="text-cyan-400" />
              <span>{t("timestamp.dialog_title")}</span>
            </h3>
            <p className="dialog-subtitle">{t("timestamp.dialog_subtitle")}</p>
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
          {/* Tabs */}
          <div className="timestamp-tab-bar">
            <button
              type="button"
              className={`timestamp-tab-btn ${activeTab === "current" ? "active" : ""}`}
              onClick={() => setActiveTab("current")}
            >
              <Clock size={13} />
              <span>{t("timestamp.tab_current")}</span>
            </button>
            <button
              type="button"
              className={`timestamp-tab-btn ${activeTab === "absolute" ? "active" : ""}`}
              onClick={() => setActiveTab("absolute")}
            >
              <Calendar size={13} />
              <span>{t("timestamp.tab_absolute")}</span>
            </button>
            <button
              type="button"
              className={`timestamp-tab-btn ${activeTab === "relative" ? "active" : ""}`}
              onClick={() => setActiveTab("relative")}
            >
              <FastForward size={13} />
              <span>{t("timestamp.tab_relative")}</span>
            </button>
            <button
              type="button"
              className={`timestamp-tab-btn ${activeTab === "exif" ? "active" : ""}`}
              onClick={() => setActiveTab("exif")}
            >
              <Camera size={13} />
              <span>{t("timestamp.tab_exif")}</span>
            </button>
          </div>

          {/* Configuration Area */}
          <div className="timestamp-config-section">
            {activeTab === "current" && (
              <div className="timestamp-info-banner">
                <Clock size={16} className="text-cyan-400 shrink-0" />
                <div className="text-sm text-slate-300">
                  {t("timestamp.touch_description")}
                </div>
              </div>
            )}

            {activeTab === "absolute" && (
              <div className="form-group mb-2">
                <label htmlFor="absolute-datetime-input" className="form-label">
                  {t("timestamp.absolute_input_label")}
                </label>
                <input
                  id="absolute-datetime-input"
                  type="text"
                  className="form-input font-mono"
                  value={absoluteValue}
                  placeholder={t("timestamp.absolute_placeholder")}
                  onChange={(e) => setAbsoluteValue(e.target.value)}
                />
              </div>
            )}

            {activeTab === "relative" && (
              <div className="timestamp-relative-group">
                <div className="form-group mb-3">
                  <label htmlFor="relative-seconds-input" className="form-label">
                    {t("timestamp.relative_input_label")}
                  </label>
                  <input
                    id="relative-seconds-input"
                    type="number"
                    className="form-input font-mono"
                    value={relativeSeconds}
                    placeholder={t("timestamp.relative_placeholder")}
                    onChange={(e) => setRelativeSeconds(Number(e.target.value))}
                  />
                </div>

                <div className="presets-container mb-2">
                  <div className="text-xs text-muted mb-2">{t("timestamp.quick_offsets_label")}</div>
                  <div className="timestamp-presets-grid">
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => setRelativeSeconds(3600)}
                    >
                      {t("timestamp.offset_preset_plus_1h")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => setRelativeSeconds(-3600)}
                    >
                      {t("timestamp.offset_preset_minus_1h")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => setRelativeSeconds(1800)}
                    >
                      {t("timestamp.offset_preset_plus_30m")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => setRelativeSeconds(-1800)}
                    >
                      {t("timestamp.offset_preset_minus_30m")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => setRelativeSeconds(86400)}
                    >
                      {t("timestamp.offset_preset_plus_1d")}
                    </button>
                    <button
                      type="button"
                      className="preset-btn"
                      onClick={() => setRelativeSeconds(-86400)}
                    >
                      {t("timestamp.offset_preset_minus_1d")}
                    </button>
                  </div>
                </div>
              </div>
            )}

            {activeTab === "exif" && (
              <div className="timestamp-info-banner">
                <Camera size={16} className="text-amber-400 shrink-0" />
                <div className="text-sm text-slate-300">
                  {t("timestamp.exif_description")}
                </div>
              </div>
            )}
          </div>

          {/* Error Message */}
          {errorMessage && (
            <div className="timestamp-error-banner">
              <XCircle size={16} className="text-rose-400 shrink-0" />
              <div className="text-sm text-rose-300">{errorMessage}</div>
            </div>
          )}

          {/* Feedback Message */}
          {feedbackMessage && (
            <div className="timestamp-feedback-banner">
              <CheckCircle2 size={16} className="text-emerald-400 shrink-0" />
              <div className="text-sm text-emerald-300">{feedbackMessage}</div>
            </div>
          )}

          {/* Dry-run Table Section */}
          <div className="timestamp-preview-section">
            <div className="timestamp-preview-header">
              <h5 className="text-sm font-semibold text-slate-300">
                {t("timestamp.preview_summary", {
                  total: plan?.total_count ?? items.length,
                  changed: plan?.changed_count ?? 0,
                })}
              </h5>
            </div>
            <div className="table-scroll-container">
              <table className="dry-run-table">
                <thead>
                  <tr>
                    <th>{t("timestamp.col_filename")}</th>
                    <th>{t("timestamp.col_current")}</th>
                    <th className="arrow-col"></th>
                    <th>{t("timestamp.col_target")}</th>
                    <th>{t("timestamp.col_status")}</th>
                  </tr>
                </thead>
                <tbody>
                  {plan?.items.map((item) => (
                    <tr
                      key={item.item_id}
                      className={
                        item.warning
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
                      <td className="font-mono text-muted text-xs">
                        {item.current_mtime_formatted}
                      </td>
                      <td className="arrow-col">
                        <ArrowRight size={14} className="text-muted" />
                      </td>
                      <td className="font-mono font-medium text-cyan-300 text-xs">
                        {item.target_mtime_formatted}
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
            {isExecuting ? t("timestamp.executing") : t("timestamp.apply_timestamp")}
          </button>
        </div>
      </div>
    </div>
  );
};
