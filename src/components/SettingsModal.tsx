// src/components/SettingsModal.tsx
// Cuddle - Settings & 30-Theme Appearance Preference Modal

import React, { useState } from "react";
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
} from "lucide-react";
import { THEME_PRESETS, applyTheme, getSavedThemeId } from "../utils/themePresets";
import type { ThemeCategory, ThemePreset } from "../types/ThemePreset";
import type { Density } from "../types/Density";
import { applyDensity, getSavedDensity } from "../utils/density";
import { SupportedLanguage } from "../i18n";

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
  const [activeTab, setActiveTab] = useState<"appearance" | "general" | "workspace" | "shortcuts">("appearance");
  const [selectedCategory, setSelectedCategory] = useState<"all" | ThemeCategory>("all");
  const [currentThemeId, setCurrentThemeId] = useState<string>(() => getSavedThemeId());
  const [currentDensity, setCurrentDensity] = useState<Density>(() => density || getSavedDensity());

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

          {/* TAB 4: Shortcuts */}
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
