import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import {
  Workflow,
  X,
  Search,
  CheckCircle2,
  AlertCircle,
  AlertTriangle,
  Play,
  Cpu,
  Wrench,
  FileCheck,
} from "lucide-react";
import type { PipelinePresetDto } from "../types/PipelinePresetDto";
import type { ExternalToolStatusDto } from "../types/ExternalToolStatusDto";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { ExecutePipelineRequestDto } from "../types/ExecutePipelineRequestDto";
import type { PipelineExecutionResultDto } from "../types/PipelineExecutionResultDto";
import type { PipelineCategory } from "../types/PipelineCategory";

interface PipelineModalProps {
  stagedFiles: StagedItemDto[];
  onClose: () => void;
  onExecuteSuccess: () => void;
}

export function PipelineModal({
  stagedFiles,
  onClose,
  onExecuteSuccess,
}: PipelineModalProps) {
  const { t } = useTranslation();

  const [presets, setPresets] = useState<PipelinePresetDto[]>([]);
  const [tools, setTools] = useState<ExternalToolStatusDto[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [selectedPresetId, setSelectedPresetId] = useState<string>("");
  const [parameters, setParameters] = useState<Record<string, string>>({});
  const [isExecuting, setIsExecuting] = useState<boolean>(false);
  const [executionResult, setExecutionResult] = useState<PipelineExecutionResultDto | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Load presets and external tools status on mount
  useEffect(() => {
    let mounted = true;
    Promise.all([
      invoke<PipelinePresetDto[]>("get_pipeline_presets").catch(() => []),
      invoke<ExternalToolStatusDto[]>("probe_external_tools").catch(() => []),
    ]).then(([loadedPresets, loadedTools]) => {
      if (mounted) {
        if (Array.isArray(loadedPresets) && loadedPresets.length > 0) {
          setPresets(loadedPresets);
          setSelectedPresetId(loadedPresets[0].id);
        }
        if (Array.isArray(loadedTools)) {
          setTools(loadedTools);
        }
      }
    });

    return () => {
      mounted = false;
    };
  }, []);

  // Set default parameters when preset changes
  useEffect(() => {
    if (!selectedPresetId) return;
    const current = presets.find((p) => p.id === selectedPresetId);
    if (current) {
      const defaultMap: Record<string, string> = {};
      current.parameters.forEach((param) => {
        defaultMap[param.key] = param.default_value;
      });
      setParameters(defaultMap);
      setExecutionResult(null);
      setErrorMessage(null);
    }
  }, [selectedPresetId, presets]);

  // Filter presets
  const filteredPresets = useMemo(() => {
    return presets.filter((preset) => {
      // Category filter
      if (selectedCategory !== "all") {
        const catMap: Record<string, PipelineCategory> = {
          image: "Image",
          media: "Media",
          text: "Text",
          document: "Document",
          file: "File",
          dev: "Dev",
        };
        if (preset.category !== catMap[selectedCategory]) {
          return false;
        }
      }

      // Search filter
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase();
        const name = t(preset.name_key).toLowerCase();
        const desc = t(preset.desc_key).toLowerCase();
        return name.includes(query) || desc.includes(query) || preset.id.toLowerCase().includes(query);
      }

      return true;
    });
  }, [presets, selectedCategory, searchQuery, t]);

  // When filteredPresets changes and current selectedPresetId is not in it, auto-select first item
  useEffect(() => {
    if (filteredPresets.length > 0) {
      const exists = filteredPresets.some((p) => p.id === selectedPresetId);
      if (!exists) {
        setSelectedPresetId(filteredPresets[0].id);
      }
    }
  }, [filteredPresets, selectedPresetId]);

  const selectedPreset = presets.find((p) => p.id === selectedPresetId);

  // Check tool availability for selected preset
  const toolAvailability = useMemo(() => {
    if (!selectedPreset || selectedPreset.is_builtin) {
      return { isAvailable: true, missingTools: [] };
    }
    const missing: string[] = [];
    selectedPreset.required_tools.forEach((toolId) => {
      const found = tools.find((t) => t.tool_id === toolId);
      if (!found || !found.is_available) {
        missing.push(found?.display_name ?? toolId);
      }
    });
    return {
      isAvailable: missing.length === 0,
      missingTools: missing,
    };
  }, [selectedPreset, tools]);

  const handleExecute = async () => {
    if (!selectedPreset) return;
    if (stagedFiles.length === 0) {
      setErrorMessage(t("pipeline.no_files_staged"));
      return;
    }

    setIsExecuting(true);
    setErrorMessage(null);
    setExecutionResult(null);

    const req: ExecutePipelineRequestDto = {
      preset_id: selectedPreset.id,
      stage_ids: stagedFiles.map((f) => f.stage_id),
      custom_parameters: parameters,
    };

    try {
      const res = await invoke<PipelineExecutionResultDto>("execute_pipeline_preset", {
        request: req,
      });
      setExecutionResult(res);
      onExecuteSuccess();
    } catch (err: unknown) {
      const msg = typeof err === "object" && err !== null && "message" in err
        ? String((err as { message: unknown }).message)
        : String(err);
      setErrorMessage(msg);
    } finally {
      setIsExecuting(false);
    }
  };

  const handleParamChange = (key: string, val: string) => {
    setParameters((prev) => ({
      ...prev,
      [key]: val,
    }));
  };

  return (
    <div className="modal-backdrop">
      <div className="pipeline-modal-dialog">
        {/* Header */}
        <div className="dialog-header">
          <div className="dialog-title-group">
            <Workflow size={18} className="text-cyan-400" />
            <div>
              <h4>{t("pipeline.modal_title")}</h4>
              <p className="dialog-subtitle">{t("pipeline.modal_desc")}</p>
            </div>
          </div>
          <button
            type="button"
            className="panel-header-btn"
            onClick={onClose}
            disabled={isExecuting}
            title={t("bulk_rename.cancel")}
          >
            <X size={15} />
          </button>
        </div>

        {/* Modal Body */}
        <div className="pipeline-modal-body">
          {/* Left Panel: Preset Browser */}
          <div className="pipeline-presets-panel">
            {/* Search Input */}
            <div className="pipeline-search-box">
              <Search size={14} className="search-icon" />
              <input
                type="text"
                className="pipeline-search-input"
                placeholder={t("pipeline.search_placeholder")}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>

            {/* Category Pills */}
            <div className="pipeline-category-pills">
              {[
                { id: "all", label: t("pipeline.all_categories") },
                { id: "image", label: t("pipeline.cat_image") },
                { id: "media", label: t("pipeline.cat_media") },
                { id: "text", label: t("pipeline.cat_text") },
                { id: "document", label: t("pipeline.cat_document") },
                { id: "file", label: t("pipeline.cat_file") },
                { id: "dev", label: t("pipeline.cat_dev") },
              ].map((cat) => (
                <button
                  key={cat.id}
                  type="button"
                  className={`cat-pill-btn ${selectedCategory === cat.id ? "active" : ""}`}
                  onClick={() => setSelectedCategory(cat.id)}
                >
                  {cat.label}
                </button>
              ))}
            </div>

            {/* Presets List */}
            <div className="pipeline-preset-list">
              {filteredPresets.map((preset) => {
                const isSelected = preset.id === selectedPresetId;
                return (
                  <button
                    key={preset.id}
                    type="button"
                    className={`preset-card-item ${isSelected ? "selected" : ""}`}
                    onClick={() => setSelectedPresetId(preset.id)}
                  >
                    <div className="preset-card-top">
                      <span className="preset-card-title">{t(preset.name_key)}</span>
                      {preset.is_builtin ? (
                        <span className="preset-engine-badge builtin">
                          <Cpu size={10} />
                          <span>{t("pipeline.pure_rust_tag")}</span>
                        </span>
                      ) : (
                        <span className="preset-engine-badge external">
                          <Wrench size={10} />
                          <span>{t("pipeline.external_tool_tag")}</span>
                        </span>
                      )}
                    </div>
                    <p className="preset-card-desc">{t(preset.desc_key)}</p>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Right Panel: Preset Details & Execution */}
          <div className="pipeline-details-panel">
            {selectedPreset ? (
              <div className="pipeline-config-scroll">
                <div className="config-header-card">
                  <div className="config-title-row">
                    <h3>{t(selectedPreset.name_key)}</h3>
                    {selectedPreset.is_builtin ? (
                      <span className="preset-engine-badge builtin">
                        <Cpu size={11} />
                        <span>{t("pipeline.pure_rust_badge")}</span>
                      </span>
                    ) : (
                      <span className="preset-engine-badge external">
                        <Wrench size={11} />
                        <span>{selectedPreset.required_tools.join(", ")}</span>
                      </span>
                    )}
                  </div>
                  <p className="config-desc-text">{t(selectedPreset.desc_key)}</p>
                </div>

                {/* External Tool Missing Warning */}
                {!toolAvailability.isAvailable && (
                  <div className="tool-warning-banner">
                    <AlertTriangle size={16} className="text-amber-400" />
                    <div>
                      <h5>{t("pipeline.tool_missing")}</h5>
                      <p>
                        {t("pipeline.tool_missing_alert", {
                          tool: toolAvailability.missingTools.join(", "),
                        })}
                      </p>
                    </div>
                  </div>
                )}

                {/* Target Staged Files Summary */}
                <div className="target-summary-box">
                  <div className="summary-title-row">
                    <FileCheck size={14} className="text-cyan-400" />
                    <span>{t("pipeline.target_files_count", { count: stagedFiles.length })}</span>
                  </div>
                  <div className="target-files-tags">
                    {stagedFiles.slice(0, 8).map((f) => (
                      <span key={f.stage_id} className="target-file-pill font-mono">
                        {f.display_name}
                      </span>
                    ))}
                    {stagedFiles.length > 8 && (
                      <span className="target-file-pill font-mono more">
                        +{stagedFiles.length - 8} ...
                      </span>
                    )}
                  </div>
                </div>

                {/* Parameters Form */}
                {selectedPreset.parameters.length > 0 && (
                  <div className="pipeline-params-section">
                    <h4>{t("bulk_rename.rules_header")}</h4>
                    <div className="params-grid">
                      {selectedPreset.parameters.map((param) => (
                        <div key={param.key} className="param-field-group">
                          <label className="param-label">{t(param.label_key)}</label>
                          {param.param_type === "number" && (
                            <div className="param-slider-row">
                              <input
                                type="range"
                                min={param.min ?? 0}
                                max={param.max ?? 100}
                                step={param.step ?? 1}
                                value={parameters[param.key] ?? param.default_value}
                                onChange={(e) => handleParamChange(param.key, e.target.value)}
                                className="param-range-slider"
                              />
                              <input
                                type="number"
                                min={param.min ?? 0}
                                max={param.max ?? 100}
                                step={param.step ?? 1}
                                value={parameters[param.key] ?? param.default_value}
                                onChange={(e) => handleParamChange(param.key, e.target.value)}
                                className="param-num-input font-mono"
                              />
                            </div>
                          )}

                          {param.param_type === "select" && param.options && (
                            <select
                              value={parameters[param.key] ?? param.default_value}
                              onChange={(e) => handleParamChange(param.key, e.target.value)}
                              className="param-select-box"
                            >
                              {param.options.map((opt) => (
                                <option key={opt} value={opt}>
                                  {opt}
                                </option>
                              ))}
                            </select>
                          )}

                          {param.param_type === "string" && (
                            <input
                              type="text"
                              value={parameters[param.key] ?? param.default_value}
                              onChange={(e) => handleParamChange(param.key, e.target.value)}
                              className="param-text-input font-mono"
                            />
                          )}

                          {param.param_type === "boolean" && (
                            <label className="param-checkbox-label">
                              <input
                                type="checkbox"
                                checked={parameters[param.key] === "true"}
                                onChange={(e) =>
                                  handleParamChange(param.key, e.target.checked ? "true" : "false")
                                }
                              />
                              <span>{t(param.label_key)}</span>
                            </label>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Error Banner */}
                {errorMessage && (
                  <div className="dialog-error-banner">
                    <AlertCircle size={15} className="text-red-400" />
                    <span>{errorMessage}</span>
                  </div>
                )}

                {/* Success Banner */}
                {executionResult && (
                  <div className="dialog-success-banner">
                    <CheckCircle2 size={15} className="text-emerald-400" />
                    <span>
                      {t("pipeline.exec_success", {
                        success: executionResult.success_count,
                        failed: executionResult.failure_count,
                      })}
                    </span>
                  </div>
                )}
              </div>
            ) : (
              <div className="empty-details-hint">
                <Workflow size={32} className="text-muted" />
                <p>{t("pipeline.select_preset_hint")}</p>
              </div>
            )}

            {/* Bottom Actions */}
            <div className="dialog-actions-bar">
              <button
                type="button"
                className="dialog-btn secondary"
                onClick={onClose}
                disabled={isExecuting}
              >
                {t("bulk_rename.cancel")}
              </button>
              <button
                type="button"
                className="dialog-btn primary pipeline-run-action-btn"
                onClick={handleExecute}
                disabled={
                  isExecuting ||
                  !selectedPreset ||
                  !toolAvailability.isAvailable ||
                  stagedFiles.length === 0
                }
              >
                <Play size={13} className={isExecuting ? "animate-spin" : ""} />
                <span>{isExecuting ? t("pipeline.running") : t("pipeline.run_pipeline_btn")}</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
