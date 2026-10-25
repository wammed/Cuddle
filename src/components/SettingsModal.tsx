import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import {
  Settings,
  X,
  Palette,
  Globe,
  HardDrive,
  ShieldCheck,
  Keyboard,
  Check,
  FileCode,
  Workflow,
  Plus,
  Trash2,
  Copy,
  CheckCircle2,
  Terminal,
  ShieldAlert,
  Sparkles,
  Edit2,
  Type,
  RotateCcw,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { THEME_PRESETS, applyTheme, getSavedThemeId } from "../utils/themePresets";
import type { ThemeCategory, ThemePreset } from "../types/ThemePreset";
import type { Density } from "../types/Density";
import type { AppConfig } from "../types/AppConfig";
import type { CustomPipelineDefDto } from "../types/CustomPipelineDefDto";
import { applyDensity, getSavedDensity } from "../utils/density";
import {
  TypographySettings,
  UI_FONT_PRESETS,
  MONO_FONT_PRESETS,
  FONT_SIZE_STEPS,
  applyTypographySettings,
  getSavedTypographySettings,
  resetTypographySettings,
} from "../utils/typography";
import { SupportedLanguage } from "../i18n";

const TOML_TEMPLATE_CODE = `# ==============================================================================
# [[custom_pipelines]] - ユーザー作成パイプライン設定 (設計書 8.3 必須安全フロー)
# ==============================================================================
# {input}  : 作業コピーの絶対パス
# {output} : 出力先ステージングパス (default_suffix / output_extension から自動生成)

[[custom_pipelines]]
id = "custom-webp-lossless"
name = "WebP 可逆圧縮カスタムコンバータ"
description = "cwebp を使用して PNG/JPEG を WebP に高品質変換 (品質 85)"
executable = "cwebp"
args = ["-q", "85", "{input}", "-o", "{output}"]
supported_extensions = ["png", "jpg", "jpeg"]
output_extension = "webp"
default_suffix = "_web"

[[custom_pipelines]]
id = "custom-strip-metadata"
name = "ExifTool メタデータ完全消去"
description = "exiftool を用いて全 Exif/GPS メタデータを消去"
executable = "exiftool"
args = ["-all=", "-overwrite_original", "{input}"]
supported_extensions = ["jpg", "jpeg", "png", "tiff", "webp"]
default_suffix = "_clean"
`;

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  currentLang: SupportedLanguage;
  onToggleLang: () => void;
  landlockSupported: boolean;
  landlockAbi: string | null;
  configPath?: string;
  density?: Density;
  onDensityChange?: (density: Density) => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  isOpen,
  onClose,
  currentLang,
  onToggleLang,
  landlockSupported,
  landlockAbi,
  configPath = "~/.config/Cuddle/config.toml",
  density,
  onDensityChange,
}) => {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"appearance" | "general" | "workspace" | "pipelines" | "shortcuts">("appearance");
  const [selectedCategory, setSelectedCategory] = useState<"all" | ThemeCategory>("all");
  const [currentThemeId, setCurrentThemeId] = useState<string>(() => getSavedThemeId());
  const [currentDensity, setCurrentDensity] = useState<Density>(() => density || getSavedDensity());
  const [typography, setTypography] = useState<TypographySettings>(() => getSavedTypographySettings());

  // AppConfig & Custom Pipelines State
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null);
  const [customPipelines, setCustomPipelines] = useState<CustomPipelineDefDto[]>([]);
  const [copiedTemplate, setCopiedTemplate] = useState<boolean>(false);
  const [isFormOpen, setIsFormOpen] = useState<boolean>(false);
  const [editingId, setEditingId] = useState<string | null>(null);

  // Form inputs
  const [formId, setFormId] = useState<string>("");
  const [formName, setFormName] = useState<string>("");
  const [formDesc, setFormDesc] = useState<string>("");
  const [formExecutable, setFormExecutable] = useState<string>("");
  const [formArgs, setFormArgs] = useState<string>("");
  const [formExts, setFormExts] = useState<string>("");
  const [formOutExt, setFormOutExt] = useState<string>("");
  const [formSuffix, setFormSuffix] = useState<string>("");

  useEffect(() => {
    if (!isOpen) return;
    setTypography(getSavedTypographySettings());
    invoke<AppConfig>("get_app_config")
      .then((cfg) => {
        setAppConfig(cfg);
        if (cfg.custom_pipelines) {
          setCustomPipelines(cfg.custom_pipelines);
        }
      })
      .catch((err) => console.error("Failed to load app config:", err));
  }, [isOpen]);

  const handleCopyTemplate = async () => {
    try {
      await navigator.clipboard.writeText(TOML_TEMPLATE_CODE);
      setCopiedTemplate(true);
      setTimeout(() => setCopiedTemplate(false), 2500);
    } catch (e) {
      console.error("Clipboard copy failed:", e);
    }
  };

  const handleOpenNewForm = () => {
    setEditingId(null);
    setFormId(`custom-tool-${Date.now().toString().slice(-4)}`);
    setFormName("");
    setFormDesc("");
    setFormExecutable("");
    setFormArgs("{input} -o {output}");
    setFormExts("png, jpg, jpeg");
    setFormOutExt("");
    setFormSuffix("_custom");
    setIsFormOpen(true);
  };

  const handleEditPipeline = (pipe: CustomPipelineDefDto) => {
    setEditingId(pipe.id);
    setFormId(pipe.id);
    setFormName(pipe.name);
    setFormDesc(pipe.description);
    setFormExecutable(pipe.executable);
    setFormArgs(pipe.args.join(" "));
    setFormExts(pipe.supported_extensions.join(", "));
    setFormOutExt(pipe.output_extension || "");
    setFormSuffix(pipe.default_suffix || "");
    setIsFormOpen(true);
  };

  const handleLoadPresetTemplate = (type: "webp" | "exif") => {
    if (type === "webp") {
      setFormId("custom-webp-lossless");
      setFormName("WebP 可逆圧縮カスタムコンバータ");
      setFormDesc("cwebp を使用して PNG/JPEG を WebP に高品質変換 (品質 85)");
      setFormExecutable("cwebp");
      setFormArgs("-q 85 {input} -o {output}");
      setFormExts("png, jpg, jpeg");
      setFormOutExt("webp");
      setFormSuffix("_web");
    } else {
      setFormId("custom-strip-metadata");
      setFormName("ExifTool メタデータ完全消去");
      setFormDesc("exiftool を用いて全 Exif/GPS メタデータを消去");
      setFormExecutable("exiftool");
      setFormArgs("-all= -overwrite_original {input}");
      setFormExts("jpg, jpeg, png, tiff, webp");
      setFormOutExt("");
      setFormSuffix("_clean");
    }
    setIsFormOpen(true);
  };

  const handleSavePipeline = async () => {
    if (!formId.trim() || !formName.trim() || !formExecutable.trim()) {
      alert("ID, 表示名, 実行バイナリは必須です / ID, Name, and Executable are required.");
      return;
    }

    const argsList = formArgs
      .trim()
      .split(/\s+/)
      .filter((a) => a.length > 0);

    const extList = formExts
      .split(/[,、\s]+/)
      .map((e) => e.replace(/^\./, "").trim())
      .filter((e) => e.length > 0);

    const newDef: CustomPipelineDefDto = {
      id: formId.trim(),
      name: formName.trim(),
      description: formDesc.trim(),
      executable: formExecutable.trim(),
      args: argsList,
      supported_extensions: extList,
      output_extension: formOutExt.trim() ? formOutExt.replace(/^\./, "").trim() : null,
      default_suffix: formSuffix.trim() ? formSuffix.trim() : "_custom",
    };

    let updatedList: CustomPipelineDefDto[];
    if (editingId) {
      updatedList = customPipelines.map((p) => (p.id === editingId ? newDef : p));
    } else {
      // If already exists with same ID, replace it; otherwise append
      const existsIndex = customPipelines.findIndex((p) => p.id === newDef.id);
      if (existsIndex >= 0) {
        updatedList = [...customPipelines];
        updatedList[existsIndex] = newDef;
      } else {
        updatedList = [...customPipelines, newDef];
      }
    }

    setCustomPipelines(updatedList);
    setIsFormOpen(false);
    setEditingId(null);

    // Save to AppConfig in backend
    if (appConfig) {
      const newConfig: AppConfig = {
        ...appConfig,
        custom_pipelines: updatedList,
      };
      try {
        await invoke("save_app_config", { config: newConfig });
        setAppConfig(newConfig);
      } catch (e) {
        console.error("Failed to save config:", e);
      }
    }
  };

  const handleDeletePipeline = async (id: string) => {
    if (!confirm(t("common.confirm") + " " + t("custom_pipelines.delete_btn") + "?")) {
      return;
    }
    const updatedList = customPipelines.filter((p) => p.id !== id);
    setCustomPipelines(updatedList);

    if (appConfig) {
      const newConfig: AppConfig = {
        ...appConfig,
        custom_pipelines: updatedList,
      };
      try {
        await invoke("save_app_config", { config: newConfig });
        setAppConfig(newConfig);
      } catch (e) {
        console.error("Failed to save config:", e);
      }
    }
  };

  if (!isOpen) return null;

  const handleSelectTheme = (preset: ThemePreset) => {
    setCurrentThemeId(preset.id);
    applyTheme(preset.id);
  };

  const handleSelectDensity = (newDensity: Density) => {
    setCurrentDensity(newDensity);
    applyDensity(newDensity);
    if (onDensityChange) {
      onDensityChange(newDensity);
    }
  };

  const handleUpdateTypography = (updates: Partial<TypographySettings>) => {
    setTypography((prev) => {
      const next = { ...prev, ...updates };
      applyTypographySettings(next);
      return next;
    });
  };

  const handleResetTypography = () => {
    const def = resetTypographySettings();
    setTypography(def);
  };

  const filteredPresets = THEME_PRESETS.filter((p) => {
    if (selectedCategory === "all") return true;
    return p.category === selectedCategory;
  });

  return (
    <div className="modal-backdrop">
      <div className="settings-modal-dialog">
        {/* Header */}
        <div className="dialog-header">
          <div className="dialog-title-group">
            <Settings size={18} className="text-teal-400" />
            <div>
              <h4 className="settings-main-title">{t("settings.dialog_title")}</h4>
              <p className="modal-subtitle">{t("settings.dialog_subtitle")}</p>
            </div>
          </div>
          <button
            type="button"
            className="panel-header-btn"
            onClick={onClose}
            title={t("common.close")}
          >
            <X size={15} />
          </button>
        </div>

        {/* Tab Navigation */}
        <div className="settings-tab-bar">
          <button
            type="button"
            className={`settings-tab-btn ${activeTab === "appearance" ? "active" : ""}`}
            onClick={() => setActiveTab("appearance")}
          >
            <Palette size={14} />
            <span>{t("settings.tab_appearance")}</span>
          </button>
          <button
            type="button"
            className={`settings-tab-btn ${activeTab === "general" ? "active" : ""}`}
            onClick={() => setActiveTab("general")}
          >
            <Globe size={14} />
            <span>{t("settings.tab_general")}</span>
          </button>
          <button
            type="button"
            className={`settings-tab-btn ${activeTab === "workspace" ? "active" : ""}`}
            onClick={() => setActiveTab("workspace")}
          >
            <HardDrive size={14} />
            <span>{t("settings.tab_workspace")}</span>
          </button>
          <button
            type="button"
            className={`settings-tab-btn ${activeTab === "pipelines" ? "active" : ""}`}
            onClick={() => setActiveTab("pipelines")}
          >
            <Workflow size={14} />
            <span>{t("settings.tab_pipelines")}</span>
          </button>
          <button
            type="button"
            className={`settings-tab-btn ${activeTab === "shortcuts" ? "active" : ""}`}
            onClick={() => setActiveTab("shortcuts")}
          >
            <Keyboard size={14} />
            <span>{t("settings.tab_shortcuts")}</span>
          </button>
        </div>

        {/* Modal Body */}
        <div className="dialog-body settings-body-scroll">
          {/* TAB 1: Appearance & 30 Theme Presets */}
          {activeTab === "appearance" && (
            <div className="settings-section-content">
              {/* Density Setting Section */}
              <div className="settings-card settings-density-card">
                <div className="density-meta-col">
                  <h4 className="settings-section-title">{t("density.title")}</h4>
                  <p className="settings-section-desc">{t("density.desc")}</p>
                </div>
                <div className="density-btn-group">
                  {(["compact", "normal", "spacious"] as Density[]).map((d) => (
                    <button
                      key={d}
                      type="button"
                      className={`density-pill-btn ${currentDensity === d ? "active" : ""}`}
                      onClick={() => handleSelectDensity(d)}
                    >
                      {t(`density.${d}`)}
                    </button>
                  ))}
                </div>
              </div>

              {/* Typography & Font Settings Section */}
              <div className="settings-card settings-typography-card">
                <div className="typo-card-header">
                  <div className="typo-header-title">
                    <Type size={18} className="text-teal-400" />
                    <div>
                      <h5>{t("typography.title")}</h5>
                      <p>{t("typography.desc")}</p>
                    </div>
                  </div>
                  <button
                    type="button"
                    className="panel-header-btn typo-reset-btn"
                    onClick={handleResetTypography}
                    title={t("typography.reset_btn")}
                  >
                    <RotateCcw size={13} />
                    <span style={{ fontSize: "11px", marginLeft: "4px" }}>{t("typography.reset_btn")}</span>
                  </button>
                </div>

                {/* 1. Base Font Size */}
                <div className="typo-section-block">
                  <div className="typo-section-heading">
                    <span className="typo-label">{t("typography.font_size_label")}</span>
                    <span className="typo-val-badge">
                      {typography.fontSize}px ({Math.round((typography.fontSize / 14) * 100)}%)
                    </span>
                  </div>
                  <div className="typo-size-slider-row">
                    <input
                      type="range"
                      className="typo-range-slider"
                      min={11}
                      max={18}
                      step={1}
                      value={typography.fontSize}
                      onChange={(e) => handleUpdateTypography({ fontSize: Number(e.target.value) })}
                    />
                    <div className="typo-size-pills">
                      {FONT_SIZE_STEPS.map((s) => (
                        <button
                          key={s}
                          type="button"
                          className={`typo-size-pill-btn ${typography.fontSize === s ? "active" : ""}`}
                          onClick={() => handleUpdateTypography({ fontSize: s })}
                        >
                          {s}px
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                {/* 2. UI Font Family (Sans-serif) */}
                <div className="typo-section-block">
                  <div className="typo-section-heading">
                    <span className="typo-label">{t("typography.ui_font_label")}</span>
                  </div>
                  <div className="typo-font-pills">
                    {UI_FONT_PRESETS.map((p) => (
                      <button
                        key={p.id}
                        type="button"
                        className={`typo-font-pill-btn ${typography.uiFontPreset === p.id ? "active" : ""}`}
                        onClick={() => handleUpdateTypography({ uiFontPreset: p.id })}
                      >
                        {t(p.labelKey, p.fallbackName)}
                      </button>
                    ))}
                  </div>
                  {typography.uiFontPreset === "custom" && (
                    <input
                      type="text"
                      className="typo-custom-input"
                      placeholder={t("typography.custom_ui_placeholder")}
                      value={typography.customUiFont}
                      onChange={(e) => handleUpdateTypography({ customUiFont: e.target.value })}
                    />
                  )}
                </div>

                {/* 3. Monospace Code Font Family */}
                <div className="typo-section-block">
                  <div className="typo-section-heading">
                    <span className="typo-label">{t("typography.mono_font_label")}</span>
                  </div>
                  <div className="typo-font-pills">
                    {MONO_FONT_PRESETS.map((p) => (
                      <button
                        key={p.id}
                        type="button"
                        className={`typo-font-pill-btn ${typography.monoFontPreset === p.id ? "active" : ""}`}
                        onClick={() => handleUpdateTypography({ monoFontPreset: p.id })}
                      >
                        {t(p.labelKey, p.fallbackName)}
                      </button>
                    ))}
                  </div>
                  {typography.monoFontPreset === "custom" && (
                    <input
                      type="text"
                      className="typo-custom-input"
                      placeholder={t("typography.custom_mono_placeholder")}
                      value={typography.customMonoFont}
                      onChange={(e) => handleUpdateTypography({ customMonoFont: e.target.value })}
                    />
                  )}
                </div>

                {/* 4. Live Preview Box */}
                <div className="typo-preview-card">
                  <span className="typo-preview-label">{t("typography.preview_label")}</span>
                  <div className="typo-preview-ui-sample">
                    {t("typography.preview_ui_sample")}
                  </div>
                  <div className="typo-preview-mono-sample">
                    {t("typography.preview_mono_sample")}
                  </div>
                </div>
              </div>

              {/* Theme Categories Title */}
              <div className="settings-theme-header-row">
                <h4 className="settings-section-title">{t("settings.tab_appearance")}</h4>
                <p className="settings-section-desc">{t("settings.active_theme_label")}: <span className="text-teal-400 font-mono font-bold">{currentThemeId}</span></p>
              </div>

              {/* Category Pills */}
              <div className="theme-category-pills">
                {[
                  { id: "all", label: t("pipeline.all_categories") },
                  { id: "cyber", label: t("settings.theme_cat_cyber") },
                  { id: "neon", label: t("settings.theme_cat_neon") },
                  { id: "modern", label: t("settings.theme_cat_modern") },
                  { id: "classic", label: t("settings.theme_cat_classic") },
                ].map((cat) => (
                  <button
                    key={cat.id}
                    type="button"
                    className={`cat-pill-btn ${selectedCategory === cat.id ? "active" : ""}`}
                    onClick={() => setSelectedCategory(cat.id as any)}
                  >
                    {cat.label}
                  </button>
                ))}
              </div>

              {/* Theme Grid */}
              <div className="theme-preset-grid">
                {filteredPresets.map((preset) => {
                  const isSelected = preset.id === currentThemeId;
                  return (
                    <div
                      key={preset.id}
                      className={`theme-card ${isSelected ? "selected" : ""}`}
                      onClick={() => handleSelectTheme(preset)}
                      style={{
                        backgroundColor: preset.colors.bgSurfaceCard,
                        borderColor: isSelected ? preset.colors.accentBrandBright : preset.colors.borderSubtle,
                      }}
                    >
                      <div className="theme-card-header">
                        <div className="theme-title-wrap">
                          <span className="theme-name" style={{ color: preset.colors.textPrimary }}>
                            {preset.fallbackName}
                          </span>
                          <span
                            className="theme-category-tag"
                            style={{
                              backgroundColor: preset.colors.bgSurfaceElevated,
                              color: preset.colors.accentBrandBright,
                              borderColor: preset.colors.borderMedium,
                            }}
                          >
                            {preset.category.toUpperCase()}
                          </span>
                        </div>
                        {isSelected && (
                          <span
                            className="theme-check-badge"
                            style={{ backgroundColor: preset.colors.accentBrand, color: "#fff" }}
                          >
                            <Check size={11} />
                          </span>
                        )}
                      </div>

                      <p className="theme-desc" style={{ color: preset.colors.textSecondary }}>
                        {preset.fallbackDesc}
                      </p>

                      {/* Swatch palette preview */}
                      <div className="theme-swatch-row">
                        <span className="swatch-dot" style={{ backgroundColor: preset.colors.bgCanvas }} title="Canvas" />
                        <span className="swatch-dot" style={{ backgroundColor: preset.colors.bgSurfaceBase }} title="Surface" />
                        <span className="swatch-dot" style={{ backgroundColor: preset.colors.accentBrand }} title="Brand Accent" />
                        <span className="swatch-dot" style={{ backgroundColor: preset.colors.accentBrandBright }} title="Bright Accent" />
                        <span className="swatch-dot" style={{ backgroundColor: preset.colors.accentSafety }} title="Safety Emerald" />
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* TAB 2: General & Language */}
          {activeTab === "general" && (
            <div className="settings-section-content">
              <div className="settings-card">
                <div className="settings-card-header">
                  <Globe size={16} className="text-teal-400" />
                  <h5>{t("settings.lang_label")}</h5>
                </div>
                <div className="settings-lang-options">
                  <button
                    type="button"
                    className={`lang-option-card ${currentLang === "ja" ? "selected" : ""}`}
                    onClick={() => {
                      if (currentLang !== "ja") onToggleLang();
                    }}
                  >
                    <span className="lang-name">{t("settings.lang_ja")}</span>
                    {currentLang === "ja" && <Check size={14} className="text-teal-400" />}
                  </button>
                  <button
                    type="button"
                    className={`lang-option-card ${currentLang === "en" ? "selected" : ""}`}
                    onClick={() => {
                      if (currentLang !== "en") onToggleLang();
                    }}
                  >
                    <span className="lang-name">{t("settings.lang_en")}</span>
                    {currentLang === "en" && <Check size={14} className="text-teal-400" />}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* TAB 3: Workspace & Security */}
          {activeTab === "workspace" && (
            <div className="settings-section-content">
              {/* Workspace Directory */}
              <div className="settings-card">
                <div className="settings-card-header">
                  <HardDrive size={16} className="text-teal-400" />
                  <h5>{t("settings.workspace_dir_label")}</h5>
                </div>
                <p className="settings-card-desc">{t("settings.workspace_auto_desc")}</p>
                <code className="settings-path-badge font-mono">workspace-temp/ (.fileworkbench-temp)</code>
              </div>

              {/* Free Space Guardrail */}
              <div className="settings-card">
                <div className="settings-card-header">
                  <ShieldCheck size={16} className="text-emerald-400" />
                  <h5>{t("settings.disk_guardrail_label")}</h5>
                </div>
                <p className="settings-card-desc">{t("settings.disk_guardrail_desc")}</p>
                <div className="settings-guardrail-value font-mono">
                  <span>1.5x (Safety Floor)</span>
                </div>
              </div>

              {/* Landlock LSM Status */}
              <div className="settings-card">
                <div className="settings-card-header">
                  <ShieldCheck size={16} className="text-teal-400" />
                  <h5>{t("settings.landlock_label")}</h5>
                </div>
                <p className="settings-card-desc">{t("settings.landlock_active_desc")}</p>
                <div className="settings-status-row">
                  <span className={`security-pill ${landlockSupported ? "secure" : "warning"}`}>
                    <ShieldCheck size={13} />
                    <span>{landlockSupported ? `Active (ABI ${landlockAbi || "V5"})` : "Unsupported"}</span>
                  </span>
                </div>
              </div>

              {/* Config File Path */}
              <div className="settings-card">
                <div className="settings-card-header">
                  <FileCode size={16} className="text-teal-400" />
                  <h5>{t("settings.config_file_label")}</h5>
                </div>
                <p className="settings-card-desc">{t("settings.config_file_hint")}</p>
                <code className="settings-path-badge font-mono">{configPath}</code>
              </div>
            </div>
          )}

          {/* TAB 4: Custom Pipelines & Templates (Design 8.3 Safe Flow) */}
          {activeTab === "pipelines" && (
            <div className="settings-section-content">
              {/* Safety Flow Explainer Banner */}
              <div className="settings-card settings-safety-guide-card">
                <div className="safety-guide-header">
                  <ShieldAlert size={18} className="text-amber-400" />
                  <div>
                    <h5 className="safety-guide-title">{t("pipeline.safety_flow_badge")}</h5>
                    <p className="safety-guide-desc">{t("custom_pipelines.desc")}</p>
                  </div>
                </div>
                <div className="safety-flow-steps-pill">
                  <span className="step-pill">1. {t("pipeline.step_risk_title")}</span>
                  <span className="step-arrow">→</span>
                  <span className="step-pill">2. {t("pipeline.step_dry_run_title")}</span>
                  <span className="step-arrow">→</span>
                  <span className="step-pill">3. {t("pipeline.step_review_title")}</span>
                  <span className="step-arrow">→</span>
                  <span className="step-pill active">4. {t("pipeline.step_exec_title")}</span>
                </div>
              </div>

              {/* TOML Template Section */}
              <div className="settings-card">
                <div className="settings-card-header flex justify-between items-center">
                  <div className="flex items-center gap-2">
                    <Terminal size={16} className="text-teal-400" />
                    <h5>{t("custom_pipelines.template_title")}</h5>
                  </div>
                  <button
                    type="button"
                    className="dialog-btn secondary small"
                    onClick={handleCopyTemplate}
                  >
                    {copiedTemplate ? (
                      <>
                        <CheckCircle2 size={13} className="text-emerald-400" />
                        <span>{t("custom_pipelines.template_copied")}</span>
                      </>
                    ) : (
                      <>
                        <Copy size={13} />
                        <span>{t("custom_pipelines.template_copy_btn")}</span>
                      </>
                    )}
                  </button>
                </div>
                <p className="settings-card-desc">{t("custom_pipelines.template_desc")}</p>
                <div className="toml-snippet-block">
                  <pre className="font-mono text-xs">{TOML_TEMPLATE_CODE}</pre>
                </div>
              </div>

              {/* Registered Pipelines Header & Actions */}
              <div className="custom-pipelines-header-row">
                <div>
                  <h4 className="settings-section-title">
                    {t("custom_pipelines.registered_list", { count: customPipelines.length })}
                  </h4>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    className="dialog-btn secondary small"
                    onClick={() => handleLoadPresetTemplate("webp")}
                    title={t("custom_pipelines.load_template_webp")}
                  >
                    <Sparkles size={12} className="text-cyan-400" />
                    <span>{t("custom_pipelines.load_template_webp")}</span>
                  </button>
                  <button
                    type="button"
                    className="dialog-btn secondary small"
                    onClick={() => handleLoadPresetTemplate("exif")}
                    title={t("custom_pipelines.load_template_exif")}
                  >
                    <Sparkles size={12} className="text-amber-400" />
                    <span>{t("custom_pipelines.load_template_exif")}</span>
                  </button>
                  <button
                    type="button"
                    className="dialog-btn primary small"
                    onClick={handleOpenNewForm}
                  >
                    <Plus size={13} />
                    <span>{t("custom_pipelines.add_new_btn")}</span>
                  </button>
                </div>
              </div>

              {/* Form Card (When Opened) */}
              {isFormOpen && (
                <div className="settings-card custom-pipeline-form-card">
                  <div className="settings-card-header">
                    <Workflow size={16} className="text-teal-400" />
                    <h5>{editingId ? t("custom_pipelines.edit_btn") : t("custom_pipelines.add_new_btn")}</h5>
                  </div>
                  <div className="form-grid-2col">
                    <div className="form-field-group">
                      <label>{t("custom_pipelines.field_id")} *</label>
                      <input
                        type="text"
                        className="form-text-input font-mono"
                        placeholder="custom-my-tool"
                        value={formId}
                        onChange={(e) => setFormId(e.target.value)}
                        disabled={!!editingId}
                      />
                    </div>
                    <div className="form-field-group">
                      <label>{t("custom_pipelines.field_name")} *</label>
                      <input
                        type="text"
                        className="form-text-input"
                        placeholder="ツール表示名"
                        value={formName}
                        onChange={(e) => setFormName(e.target.value)}
                      />
                    </div>
                  </div>

                  <div className="form-field-group">
                    <label>{t("custom_pipelines.field_desc")}</label>
                    <input
                      type="text"
                      className="form-text-input"
                      placeholder="処理内容の説明"
                      value={formDesc}
                      onChange={(e) => setFormDesc(e.target.value)}
                    />
                  </div>

                  <div className="form-grid-2col">
                    <div className="form-field-group">
                      <label>{t("custom_pipelines.field_executable")} *</label>
                      <input
                        type="text"
                        className="form-text-input font-mono"
                        placeholder="cwebp / ffmpeg / exiftool / /usr/local/bin/mytool"
                        value={formExecutable}
                        onChange={(e) => setFormExecutable(e.target.value)}
                      />
                    </div>
                    <div className="form-field-group">
                      <label>{t("custom_pipelines.field_args")} *</label>
                      <input
                        type="text"
                        className="form-text-input font-mono"
                        placeholder="-q 80 {input} -o {output}"
                        value={formArgs}
                        onChange={(e) => setFormArgs(e.target.value)}
                      />
                    </div>
                  </div>

                  <div className="form-grid-3col">
                    <div className="form-field-group">
                      <label>{t("custom_pipelines.field_exts")}</label>
                      <input
                        type="text"
                        className="form-text-input font-mono"
                        placeholder="png, jpg, jpeg"
                        value={formExts}
                        onChange={(e) => setFormExts(e.target.value)}
                      />
                    </div>
                    <div className="form-field-group">
                      <label>{t("custom_pipelines.field_out_ext")}</label>
                      <input
                        type="text"
                        className="form-text-input font-mono"
                        placeholder="webp (空欄なら同一拡張子)"
                        value={formOutExt}
                        onChange={(e) => setFormOutExt(e.target.value)}
                      />
                    </div>
                    <div className="form-field-group">
                      <label>{t("custom_pipelines.field_suffix")}</label>
                      <input
                        type="text"
                        className="form-text-input font-mono"
                        placeholder="_clean / _web"
                        value={formSuffix}
                        onChange={(e) => setFormSuffix(e.target.value)}
                      />
                    </div>
                  </div>

                  <div className="form-actions-row">
                    <button
                      type="button"
                      className="dialog-btn secondary small"
                      onClick={() => setIsFormOpen(false)}
                    >
                      {t("common.cancel")}
                    </button>
                    <button
                      type="button"
                      className="dialog-btn primary small"
                      onClick={handleSavePipeline}
                    >
                      <Check size={13} />
                      <span>{t("custom_pipelines.save_btn")}</span>
                    </button>
                  </div>
                </div>
              )}

              {/* Registered Pipelines List */}
              {customPipelines.length === 0 ? (
                <div className="empty-state-notice">
                  <p>{t("custom_pipelines.empty")}</p>
                </div>
              ) : (
                <div className="custom-pipelines-grid">
                  {customPipelines.map((pipe) => (
                    <div key={pipe.id} className="custom-pipeline-item-card">
                      <div className="pipe-card-header">
                        <div className="pipe-title-wrap">
                          <span className="pipe-name">{pipe.name}</span>
                          <code className="pipe-id-badge font-mono">{pipe.id}</code>
                        </div>
                        <div className="pipe-card-actions">
                          <button
                            type="button"
                            className="pipe-action-btn edit"
                            onClick={() => handleEditPipeline(pipe)}
                            title={t("custom_pipelines.edit_btn")}
                          >
                            <Edit2 size={13} />
                          </button>
                          <button
                            type="button"
                            className="pipe-action-btn delete"
                            onClick={() => handleDeletePipeline(pipe.id)}
                            title={t("custom_pipelines.delete_btn")}
                          >
                            <Trash2 size={13} />
                          </button>
                        </div>
                      </div>

                      <p className="pipe-desc">{pipe.description}</p>

                      <div className="pipe-details-row">
                        <div className="detail-item">
                          <span className="label">Binary:</span>
                          <code className="val font-mono">{pipe.executable}</code>
                        </div>
                        <div className="detail-item">
                          <span className="label">Args:</span>
                          <code className="val font-mono">{pipe.args.join(" ")}</code>
                        </div>
                        <div className="detail-item">
                          <span className="label">Exts:</span>
                          <span className="val font-mono">
                            {pipe.supported_extensions.length > 0
                              ? pipe.supported_extensions.join(", ")
                              : "* (All)"}
                          </span>
                        </div>
                        {pipe.output_extension && (
                          <div className="detail-item">
                            <span className="label">Out:</span>
                            <span className="val font-mono">.{pipe.output_extension}</span>
                          </div>
                        )}
                        {pipe.default_suffix && (
                          <div className="detail-item">
                            <span className="label">Suffix:</span>
                            <span className="val font-mono">{pipe.default_suffix}</span>
                          </div>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* TAB 5: Shortcuts */}
          {activeTab === "shortcuts" && (
            <div className="settings-section-content">
              <div className="shortcuts-table-container">
                <table className="shortcuts-table">
                  <thead>
                    <tr>
                      <th>ショートカット / Shortcut</th>
                      <th>操作 / Action</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr>
                      <td><kbd className="font-mono">Ctrl + F</kbd> / <kbd className="font-mono">Cmd + F</kbd></td>
                      <td>検索入力にフォーカス & テキスト全選択 (Focus Search Field)</td>
                    </tr>
                    <tr>
                      <td><kbd className="font-mono">Ctrl + R</kbd> / <kbd className="font-mono">Cmd + R</kbd></td>
                      <td>一括リネームモーダルを開く (Open Bulk Rename Modal)</td>
                    </tr>
                    <tr>
                      <td><kbd className="font-mono">Ctrl + P</kbd> / <kbd className="font-mono">Cmd + P</kbd></td>
                      <td>外部ツールパイプラインを開く (Open Pipeline Runner)</td>
                    </tr>
                    <tr>
                      <td><kbd className="font-mono">Ctrl + Z</kbd> / <kbd className="font-mono">Cmd + Z</kbd></td>
                      <td>直前のリネーム操作を取り消す (Undo Last Bulk Rename)</td>
                    </tr>
                    <tr>
                      <td><kbd className="font-mono">Escape</kbd></td>
                      <td>開いているモーダルを閉じる (Close Active Modal)</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>

        {/* Fixed Bottom Action Bar */}
        <div className="dialog-footer">
          <button type="button" className="dialog-btn primary" onClick={onClose}>
            <Check size={14} />
            <span>{t("common.close")}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
