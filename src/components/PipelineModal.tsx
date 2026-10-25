import { useState, useEffect, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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
  Loader2,
  StopCircle,
  ShieldAlert,
  ShieldCheck,
  Terminal,
  ArrowRight,
  RotateCcw,
} from "lucide-react";
import type { PipelinePresetDto } from "../types/PipelinePresetDto";
import type { ExternalToolStatusDto } from "../types/ExternalToolStatusDto";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { ExecutePipelineRequestDto } from "../types/ExecutePipelineRequestDto";
import type { PipelineExecutionResultDto } from "../types/PipelineExecutionResultDto";
import type { PipelineCategory } from "../types/PipelineCategory";
import type { PipelineProgressEvent } from "../types/PipelineProgressEvent";
import type { CustomPipelineDefDto } from "../types/CustomPipelineDefDto";
import type { CustomPipelineDryRunResultDto } from "../types/CustomPipelineDryRunResultDto";
import type { AppConfig } from "../types/AppConfig";

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
  const [customPipelines, setCustomPipelines] = useState<CustomPipelineDefDto[]>([]);
  const [tools, setTools] = useState<ExternalToolStatusDto[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [selectedPresetId, setSelectedPresetId] = useState<string>("");
  const [parameters, setParameters] = useState<Record<string, string>>({});
  const [isExecuting, setIsExecuting] = useState<boolean>(false);
  const [isCancelling, setIsCancelling] = useState<boolean>(false);
  const [executionResult, setExecutionResult] = useState<PipelineExecutionResultDto | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Safety Flow (Design 8.3) State
  const [safetyStep, setSafetyStep] = useState<"risk" | "dry_running" | "reviewed" | "executing" | "done">("risk");
  const [dryRunResult, setDryRunResult] = useState<CustomPipelineDryRunResultDto | null>(null);
  const [isDryRunning, setIsDryRunning] = useState<boolean>(false);

  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const activeJobIdRef = useRef<string | null>(null);
  activeJobIdRef.current = activeJobId;

  const [progress, setProgress] = useState<{
    currentIndex: number;
    totalItems: number;
    currentFilename: string;
    percent: number;
    status: string;
  } | null>(null);

  // Listen to pipeline-progress events from backend
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let isMounted = true;

    listen<PipelineProgressEvent>("pipeline-progress", (event) => {
      if (!isMounted) return;
      const payload = event.payload;
      if (activeJobIdRef.current && payload.job_id !== activeJobIdRef.current) {
        return;
      }
      setProgress({
        currentIndex: payload.current_index,
        totalItems: payload.total_items,
        currentFilename: payload.current_filename,
        percent: payload.percent,
        status: payload.status,
      });
    }).then((fn) => {
      if (isMounted) {
        unlisten = fn;
      } else {
        fn();
      }
    });

    return () => {
      isMounted = false;
      if (unlisten) unlisten();
    };
  }, []);

  // Load presets, custom pipelines, and external tools status on mount
  useEffect(() => {
    let mounted = true;
    Promise.all([
      invoke<PipelinePresetDto[]>("get_pipeline_presets").catch(() => []),
      invoke<AppConfig>("get_app_config").catch(() => null),
      invoke<ExternalToolStatusDto[]>("probe_external_tools").catch(() => []),
    ]).then(([loadedPresets, loadedConfig, loadedTools]) => {
      if (!mounted) return;

      const customDefs: CustomPipelineDefDto[] = loadedConfig?.custom_pipelines || [];
      setCustomPipelines(customDefs);

      // Convert custom pipelines into preset representations
      const customAsPresets: PipelinePresetDto[] = customDefs.map((cp) => ({
        id: cp.id,
        name_key: cp.name,
        desc_key: cp.description,
        category: "Custom",
        is_builtin: false,
        required_tools: [cp.executable],
        default_suffix: cp.default_suffix || "_custom",
        parameters: [],
        supported_extensions: cp.supported_extensions,
      }));

      const allPresets = [...(Array.isArray(loadedPresets) ? loadedPresets : []), ...customAsPresets];
      setPresets(allPresets);

      if (allPresets.length > 0) {
        setSelectedPresetId(allPresets[0].id);
      }
      if (Array.isArray(loadedTools)) {
        setTools(loadedTools);
      }
    });

    return () => {
      mounted = false;
    };
  }, []);

  // Set default parameters & reset safety flow when preset changes
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
      setSafetyStep("risk");
      setDryRunResult(null);
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
          custom: "Custom",
        };
        if (preset.category !== catMap[selectedCategory]) {
          return false;
        }
      }

      // Search filter
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase();
        const name = (preset.category === "Custom" ? preset.name_key : t(preset.name_key)).toLowerCase();
        const desc = (preset.category === "Custom" ? preset.desc_key : t(preset.desc_key)).toLowerCase();
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
  const isCustom = selectedPreset?.category === "Custom";
  const currentCustomDef = customPipelines.find((c) => c.id === selectedPresetId);

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

  const handleRunDryRun = async () => {
    if (!currentCustomDef) return;
    if (stagedFiles.length === 0) {
      setErrorMessage(t("pipeline.no_files_staged"));
      return;
    }
    setIsDryRunning(true);
    setSafetyStep("dry_running");
    setErrorMessage(null);
    try {
      const res = await invoke<CustomPipelineDryRunResultDto>("validate_custom_pipeline_dry_run", {
        pipeline: currentCustomDef,
        stageIds: stagedFiles.map((f) => f.stage_id),
      });
      setDryRunResult(res);
      setSafetyStep("reviewed");
    } catch (err: unknown) {
      const msg = typeof err === "object" && err !== null && "message" in err
        ? String((err as { message: unknown }).message)
        : String(err);
      setErrorMessage(msg);
      setSafetyStep("risk");
    } finally {
      setIsDryRunning(false);
    }
  };

  const handleExecuteCustomPipeline = async () => {
    if (!currentCustomDef || !dryRunResult || !dryRunResult.can_proceed_to_execution) return;
    if (stagedFiles.length === 0) {
      setErrorMessage(t("pipeline.no_files_staged"));
      return;
    }

    const jobId = typeof crypto !== "undefined" && crypto.randomUUID
      ? crypto.randomUUID()
      : `custom-job-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;

    setActiveJobId(jobId);
    setIsExecuting(true);
    setIsCancelling(false);
    setErrorMessage(null);
    setExecutionResult(null);
    setSafetyStep("executing");
    setProgress({
      currentIndex: 0,
      totalItems: stagedFiles.length,
      currentFilename: stagedFiles[0]?.display_name ?? "",
      percent: 0,
      status: "Starting Custom Pipeline",
    });

    try {
      const res = await invoke<PipelineExecutionResultDto>("execute_custom_pipeline", {
        pipeline: currentCustomDef,
        stageIds: stagedFiles.map((f) => f.stage_id),
        jobId,
      });
      setExecutionResult(res);
      setSafetyStep("done");
      onExecuteSuccess();
    } catch (err: unknown) {
      const msg = typeof err === "object" && err !== null && "message" in err
        ? String((err as { message: unknown }).message)
        : String(err);
      setErrorMessage(msg);
      setSafetyStep("reviewed");
    } finally {
      setIsExecuting(false);
      setIsCancelling(false);
      setActiveJobId(null);
    }
  };

  const handleResetSafetyFlow = () => {
    setSafetyStep("risk");
    setDryRunResult(null);
    setErrorMessage(null);
  };

  const handleExecute = async () => {
    if (!selectedPreset) return;
    if (stagedFiles.length === 0) {
      setErrorMessage(t("pipeline.no_files_staged"));
      return;
    }

    const jobId = typeof crypto !== "undefined" && crypto.randomUUID
      ? crypto.randomUUID()
      : `job-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;

    setActiveJobId(jobId);
    setIsExecuting(true);
    setIsCancelling(false);
    setErrorMessage(null);
    setExecutionResult(null);
    setProgress({
      currentIndex: 0,
      totalItems: stagedFiles.length,
      currentFilename: stagedFiles[0]?.display_name ?? "",
      percent: 0,
      status: "Starting",
    });

    const req: ExecutePipelineRequestDto = {
      job_id: jobId,
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
      setIsCancelling(false);
      setActiveJobId(null);
    }
  };

  const handleCancel = async () => {
    if (!activeJobId || isCancelling) return;
    setIsCancelling(true);
    try {
      await invoke("cancel_pipeline", { jobId: activeJobId });
    } catch (err) {
      console.error("Failed to cancel pipeline:", err);
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
                { id: "custom", label: t("pipeline.cat_custom") },
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
                      <span className="preset-card-title">
                        {preset.category === "Custom" ? preset.name_key : t(preset.name_key)}
                      </span>
                      {preset.category === "Custom" ? (
                        <span className="preset-engine-badge custom">
                          <ShieldAlert size={10} className="text-amber-400" />
                          <span>{t("pipeline.custom_tag")}</span>
                        </span>
                      ) : preset.is_builtin ? (
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
                    <p className="preset-card-desc">
                      {preset.category === "Custom" ? preset.desc_key : t(preset.desc_key)}
                    </p>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Right Panel: Preset Details & Execution */}
          <div className="pipeline-details-panel">
            {isCustom && currentCustomDef ? (
              /* ==============================================================================
                 Design 8.3 Mandatory Safety Flow for User-Created Pipelines
                 [Risk Explanation] -> [Confirm 1] -> [Dry-Run] -> [Review] -> [Confirm 2] -> [Execute]
                 ============================================================================== */
              <div className="pipeline-config-scroll custom-safety-flow-scroll">
                {/* Safety Badge Banner */}
                <div className="safety-flow-header-banner">
                  <div className="flex items-center gap-2">
                    <ShieldAlert size={20} className="text-amber-400 flex-shrink-0" />
                    <div>
                      <div className="flex items-center gap-2">
                        <h3 className="custom-pipeline-title">{currentCustomDef.name}</h3>
                        <span className="safety-flow-badge font-mono">{t("pipeline.safety_flow_badge")}</span>
                      </div>
                      <p className="dialog-subtitle">{currentCustomDef.description}</p>
                    </div>
                  </div>
                </div>

                {/* 4-Step Tracker */}
                <div className="safety-steps-tracker">
                  <div className={`step-node ${safetyStep === "risk" ? "active" : "completed"}`}>
                    <span className="step-num">1</span>
                    <span className="step-label">{t("pipeline.step_risk_title")}</span>
                  </div>
                  <div className="step-divider" />
                  <div className={`step-node ${safetyStep === "dry_running" ? "active" : dryRunResult ? "completed" : ""}`}>
                    <span className="step-num">2</span>
                    <span className="step-label">{t("pipeline.step_dry_run_title")}</span>
                  </div>
                  <div className="step-divider" />
                  <div className={`step-node ${safetyStep === "reviewed" ? "active" : (safetyStep === "executing" || safetyStep === "done") ? "completed" : ""}`}>
                    <span className="step-num">3</span>
                    <span className="step-label">{t("pipeline.step_review_title")}</span>
                  </div>
                  <div className="step-divider" />
                  <div className={`step-node ${safetyStep === "executing" ? "active" : safetyStep === "done" ? "completed" : ""}`}>
                    <span className="step-num">4</span>
                    <span className="step-label">{t("pipeline.step_exec_title")}</span>
                  </div>
                </div>

                {/* STEP 1: [Risk Explanation] */}
                {safetyStep === "risk" && (
                  <div className="safety-step-content-card">
                    <h5 className="step-section-heading">
                      <Terminal size={15} className="text-teal-400" />
                      <span>{t("pipeline.risk_summary")}</span>
                    </h5>

                    <div className="risk-spec-box font-mono text-xs">
                      <div className="spec-row">
                        <span className="spec-label">Executable:</span>
                        <span className="spec-val text-cyan-300 font-bold">{currentCustomDef.executable}</span>
                      </div>
                      <div className="spec-row">
                        <span className="spec-label">Args Template:</span>
                        <span className="spec-val text-amber-300">{currentCustomDef.args.join(" ")}</span>
                      </div>
                      <div className="spec-row">
                        <span className="spec-label">Supported Exts:</span>
                        <span className="spec-val">
                          {currentCustomDef.supported_extensions.length > 0
                            ? currentCustomDef.supported_extensions.join(", ")
                            : "* (All Staged Files)"}
                        </span>
                      </div>
                      <div className="spec-row">
                        <span className="spec-label">Output Extension:</span>
                        <span className="spec-val text-muted">
                          {currentCustomDef.output_extension ? `.${currentCustomDef.output_extension}` : "(Keep Original)"}
                        </span>
                      </div>
                      <div className="spec-row">
                        <span className="spec-label">Target Files:</span>
                        <span className="spec-val text-teal-400 font-bold">{stagedFiles.length} files</span>
                      </div>
                    </div>

                    <div className="sandbox-security-notice">
                      <ShieldCheck size={18} className="text-emerald-400 flex-shrink-0" />
                      <div>
                        <p className="font-semibold text-emerald-300">Linux Landlock LSM 物理隔離 & 原本保護</p>
                        <p className="text-xs text-muted leading-relaxed">{t("pipeline.risk_warning_sandbox")}</p>
                      </div>
                    </div>

                    {errorMessage && (
                      <div className="dialog-error-banner mt-3">
                        <AlertCircle size={15} className="text-red-400 flex-shrink-0" />
                        <span>{errorMessage}</span>
                      </div>
                    )}

                    <div className="step-action-row mt-4">
                      <button
                        type="button"
                        className="dialog-btn primary w-full safety-action-btn"
                        onClick={handleRunDryRun}
                        disabled={stagedFiles.length === 0 || isDryRunning}
                      >
                        <ArrowRight size={14} />
                        <span>{t("pipeline.confirm_1_btn")}</span>
                      </button>
                    </div>
                  </div>
                )}

                {/* STEP 2: [Dry-Running] */}
                {safetyStep === "dry_running" && (
                  <div className="safety-step-content-card text-center py-8">
                    <Loader2 size={36} className="animate-spin text-cyan-400 mx-auto mb-3" />
                    <p className="text-sm font-semibold">{t("pipeline.dry_run_running")}</p>
                    <p className="text-xs text-muted mt-1">バイナリ検証・引数展開・名前衝突チェックを実行しています...</p>
                  </div>
                )}

                {/* STEP 3: [Review Results & Re-present Risks] */}
                {safetyStep === "reviewed" && dryRunResult && (
                  <div className="safety-step-content-card">
                    <div className="flex justify-between items-center mb-3">
                      <h5 className="step-section-heading">
                        <CheckCircle2 size={15} className="text-teal-400" />
                        <span>{t("pipeline.step_review_title")}</span>
                      </h5>
                      <button
                        type="button"
                        className="dialog-btn secondary small"
                        onClick={handleResetSafetyFlow}
                      >
                        <RotateCcw size={12} />
                        <span>再設定・最初へ</span>
                      </button>
                    </div>

                    {/* Overall Dry-Run Status Banner */}
                    {dryRunResult.can_proceed_to_execution ? (
                      <div className="dry-run-status-banner success">
                        <CheckCircle2 size={16} className="text-emerald-400 flex-shrink-0" />
                        <div>
                          <p className="font-semibold text-emerald-300">{t("pipeline.dry_run_success")}</p>
                          <p className="text-xs text-muted">
                            {dryRunResult.valid_items} / {dryRunResult.total_items} 件の対象ファイルについて安全に実行可能です。
                          </p>
                        </div>
                      </div>
                    ) : (
                      <div className="dry-run-status-banner failure">
                        <AlertCircle size={16} className="text-red-400 flex-shrink-0" />
                        <div>
                          <p className="font-semibold text-red-300">{t("pipeline.dry_run_failed")}</p>
                          {!dryRunResult.executable_found && (
                            <p className="text-xs text-red-200 font-mono mt-1">
                              {t("pipeline.executable_not_found_alert")} ({currentCustomDef.executable})
                            </p>
                          )}
                          {dryRunResult.has_conflicts && (
                            <p className="text-xs text-amber-200 mt-1">
                              出力先ファイル名の衝突が検出されました。設定でサフィックスを変更してください。
                            </p>
                          )}
                        </div>
                      </div>
                    )}

                    {/* Pre-flight Info Meta */}
                    <div className="verification-meta-grid font-mono text-xs my-3">
                      <div className="meta-item">
                        <span className="text-muted">{t("pipeline.resolved_path")}: </span>
                        <span className={dryRunResult.executable_found ? "text-emerald-400 font-bold" : "text-red-400 font-bold"}>
                          {dryRunResult.executable_path || "NOT FOUND"}
                        </span>
                      </div>
                      <div className="meta-item">
                        <span className="text-muted">空き容量ガードレール: </span>
                        <span className={dryRunResult.has_disk_space ? "text-emerald-400" : "text-amber-400 font-bold"}>
                          {dryRunResult.disk_space_message || "OK (>= 1.5x)"}
                        </span>
                      </div>
                    </div>

                    {/* Dry-run item-by-item Table */}
                    <div className="dry-run-table-wrapper">
                      <table className="dry-run-table font-mono text-xs">
                        <thead>
                          <tr>
                            <th>原本対象ファイル</th>
                            <th>展開後実行コマンド</th>
                            <th>予測出力ファイル名</th>
                            <th>状態</th>
                          </tr>
                        </thead>
                        <tbody>
                          {dryRunResult.items.map((item) => (
                            <tr key={item.stage_id} className={item.valid ? "" : "invalid-row"}>
                              <td className="file-cell" title={item.original_name}>
                                {item.original_name}
                              </td>
                              <td className="cmd-cell" title={item.expanded_command.join(" ")}>
                                <code>{item.expanded_command.join(" ")}</code>
                              </td>
                              <td className="out-cell" title={item.proposed_output_name}>
                                {item.proposed_output_name}
                              </td>
                              <td className="status-cell">
                                {item.conflict_detected ? (
                                  <span className="badge-danger">衝突</span>
                                ) : item.valid ? (
                                  <span className="badge-success">OK</span>
                                ) : (
                                  <span className="badge-warning">Skip</span>
                                )}
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>

                    {/* Risk Re-presentation Warning */}
                    <div className="sandbox-security-notice mt-3">
                      <ShieldCheck size={18} className="text-emerald-400 flex-shrink-0" />
                      <div>
                        <p className="font-semibold text-emerald-300">安全制約の再提示（設計書 8.3）</p>
                        <p className="text-xs text-muted leading-relaxed">
                          上記テーブルに表示されたコマンドおよびパスのみが、Landlock LSM による原本保護隔離環境下で実行されます。原本ファイルに対する変更・破壊は発生しません。
                        </p>
                      </div>
                    </div>

                    {errorMessage && (
                      <div className="dialog-error-banner mt-3">
                        <AlertCircle size={15} className="text-red-400 flex-shrink-0" />
                        <span>{errorMessage}</span>
                      </div>
                    )}

                    {/* STEP 3 Confirmation Action: Confirm 2 */}
                    <div className="step-action-row mt-4">
                      <button
                        type="button"
                        className="dialog-btn primary w-full safety-action-btn"
                        onClick={handleExecuteCustomPipeline}
                        disabled={!dryRunResult.can_proceed_to_execution || isExecuting}
                      >
                        <Play size={14} />
                        <span>{t("pipeline.confirm_2_btn")}</span>
                      </button>
                    </div>
                  </div>
                )}

                {/* STEP 4: [Execution & Results] */}
                {(safetyStep === "executing" || safetyStep === "done") && (
                  <div className="safety-step-content-card">
                    <h5 className="step-section-heading mb-3">
                      <Workflow size={15} className="text-teal-400" />
                      <span>{t("pipeline.step_exec_title")}</span>
                    </h5>

                    {/* Real-time Progress Bar */}
                    {isExecuting && progress && (
                      <div className="pipeline-progress-container mb-4">
                        <div className="progress-info-row">
                          <span className="progress-label">
                            {t("pipeline.progress_running", {
                              current: progress.currentIndex,
                              total: progress.totalItems,
                            })}
                          </span>
                          <span className="progress-percent font-mono">
                            {Math.round(progress.percent)}%
                          </span>
                        </div>
                        <div className="pipeline-progress-track">
                          <div
                            className="pipeline-progress-fill"
                            style={{
                              width: `${Math.min(100, Math.max(0, progress.percent))}%`,
                            }}
                          />
                        </div>
                        {progress.currentFilename && (
                          <div className="progress-current-filename font-mono">
                            <Loader2 size={12} className="animate-spin text-cyan-400" />
                            <span>{t("pipeline.processing_file")}: {progress.currentFilename}</span>
                          </div>
                        )}
                      </div>
                    )}

                    {/* Success Banner */}
                    {executionResult && !executionResult.is_cancelled && (
                      <div className="dialog-success-banner mb-3">
                        <CheckCircle2 size={16} className="text-emerald-400 flex-shrink-0" />
                        <div>
                          <p className="font-semibold text-emerald-300">
                            {t("pipeline.exec_success", {
                              success: executionResult.success_count,
                              failed: executionResult.failure_count,
                            })}
                          </p>
                          <p className="text-xs text-muted">成果物は作業領域にステージングされました。</p>
                        </div>
                      </div>
                    )}

                    {/* Cancelled Banner */}
                    {executionResult && executionResult.is_cancelled && (
                      <div className="dialog-warning-banner mb-3">
                        <AlertTriangle size={16} className="text-amber-400 flex-shrink-0" />
                        <span>
                          {t("pipeline.exec_cancelled", {
                            success: executionResult.success_count,
                            total: executionResult.total_count,
                          })}
                        </span>
                      </div>
                    )}

                    {errorMessage && (
                      <div className="dialog-error-banner mb-3">
                        <AlertCircle size={15} className="text-red-400 flex-shrink-0" />
                        <span>{errorMessage}</span>
                      </div>
                    )}

                    {isExecuting ? (
                      <div className="step-action-row">
                        <button
                          type="button"
                          className="dialog-btn danger w-full pipeline-cancel-btn"
                          onClick={handleCancel}
                          disabled={isCancelling}
                        >
                          <StopCircle size={14} className={isCancelling ? "animate-spin" : ""} />
                          <span>{isCancelling ? t("pipeline.cancelling") : t("pipeline.cancel_btn")}</span>
                        </button>
                      </div>
                    ) : (
                      <div className="step-action-row flex gap-2">
                        <button
                          type="button"
                          className="dialog-btn secondary w-full"
                          onClick={handleResetSafetyFlow}
                        >
                          <RotateCcw size={13} />
                          <span>最初から再実行</span>
                        </button>
                        <button
                          type="button"
                          className="dialog-btn primary w-full"
                          onClick={onClose}
                        >
                          <CheckCircle2 size={13} />
                          <span>{t("common.close")}</span>
                        </button>
                      </div>
                    )}
                  </div>
                )}
              </div>
            ) : selectedPreset ? (
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

                {/* Real-time Progress Bar */}
                {isExecuting && progress && (
                  <div className="pipeline-progress-container">
                    <div className="progress-info-row">
                      <span className="progress-label">
                        {t("pipeline.progress_running", {
                          current: progress.currentIndex,
                          total: progress.totalItems,
                        })}
                      </span>
                      <span className="progress-percent font-mono">
                        {Math.round(progress.percent)}%
                      </span>
                    </div>
                    <div className="pipeline-progress-track">
                      <div
                        className="pipeline-progress-fill"
                        style={{
                          width: `${Math.min(100, Math.max(0, progress.percent))}%`,
                        }}
                      />
                    </div>
                    {progress.currentFilename && (
                      <div className="progress-current-filename font-mono">
                        <Loader2 size={12} className="animate-spin text-cyan-400" />
                        <span>{t("pipeline.processing_file")}: {progress.currentFilename}</span>
                      </div>
                    )}
                  </div>
                )}

                {/* Error Banner */}
                {errorMessage && (
                  <div className="dialog-error-banner">
                    <AlertCircle size={15} className="text-red-400" />
                    <span>{errorMessage}</span>
                  </div>
                )}

                {/* Cancelled Banner */}
                {executionResult && executionResult.is_cancelled && (
                  <div className="dialog-warning-banner">
                    <AlertTriangle size={15} className="text-amber-400" />
                    <span>
                      {t("pipeline.exec_cancelled", {
                        success: executionResult.success_count,
                        total: executionResult.total_count,
                      })}
                    </span>
                  </div>
                )}

                {/* Success Banner */}
                {executionResult && !executionResult.is_cancelled && (
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
              {isCustom ? (
                /* For custom pipelines, execution is strictly handled inside the Design 8.3 wizard */
                null
              ) : isExecuting ? (
                <button
                  type="button"
                  className="dialog-btn danger pipeline-cancel-btn"
                  onClick={handleCancel}
                  disabled={isCancelling}
                >
                  <StopCircle size={13} className={isCancelling ? "animate-spin" : ""} />
                  <span>{isCancelling ? t("pipeline.cancelling") : t("pipeline.cancel_btn")}</span>
                </button>
              ) : (
                <button
                  type="button"
                  className="dialog-btn primary pipeline-run-action-btn"
                  onClick={handleExecute}
                  disabled={
                    !selectedPreset ||
                    !toolAvailability.isAvailable ||
                    stagedFiles.length === 0
                  }
                >
                  <Play size={13} />
                  <span>{t("pipeline.run_pipeline_btn")}</span>
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
