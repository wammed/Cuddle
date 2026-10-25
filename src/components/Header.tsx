import {
  Globe,
  ShieldCheck,
  Lock,
  PanelRight,
  PanelBottom,
  RotateCcw,
  Settings,
  Home,
  Briefcase,
  Folder,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { SupportedLanguage } from "../i18n";

interface HeaderProps {
  currentLang: SupportedLanguage;
  onToggleLang: () => void;
  landlockSupported: boolean;
  landlockAbi: string | null;
  sidebarVisible: boolean;
  bottomVisible: boolean;
  onToggleSidebar: () => void;
  onToggleBottom: () => void;
  onResetLayout: () => void;
  onOpenSettings?: () => void;
  activeView?: "start" | "workbench";
  onToggleView?: () => void;
  currentPath?: string;
  stagedCount?: number;
}

export function Header({
  currentLang,
  onToggleLang,
  landlockSupported,
  landlockAbi,
  sidebarVisible,
  bottomVisible,
  onToggleSidebar,
  onToggleBottom,
  onResetLayout,
  onOpenSettings,
  activeView = "workbench",
  onToggleView,
  currentPath,
  stagedCount = 0,
}: HeaderProps) {
  const { t } = useTranslation();

  return (
    <header className="main-app-header-multiline">
      {/* Row 1: Brand, Navigation Mode, Security Badge, Settings & Language */}
      <div className="header-row header-primary-row">
        <div className="header-brand-group">
          <img src="/Cuddle-Icon.svg" alt="Cuddle Icon" className="header-logo-img" />
          <div className="header-brand-text">
            <div className="header-title-row">
              <h1 className="header-title">{t("app.title")}</h1>
              <span className="header-badge">v0.1.0</span>
            </div>
            <p className="header-subtitle">{t("app.subtitle")}</p>
          </div>
        </div>

        {/* View Switcher: Start Dashboard <-> Workbench */}
        {onToggleView && (
          <div className="header-view-toggle-wrap">
            <button
              type="button"
              className={`header-mode-tab ${activeView === "start" ? "active" : ""}`}
              onClick={() => {
                if (activeView !== "start") onToggleView();
              }}
              title="Start Dashboard"
            >
              <Home size={14} />
              <span>{t("start.btn_open_home")}</span>
            </button>
            <button
              type="button"
              className={`header-mode-tab ${activeView === "workbench" ? "active" : ""}`}
              onClick={() => {
                if (activeView !== "workbench") onToggleView();
              }}
              title="File Workbench"
            >
              <Briefcase size={14} />
              <span>{t("panels.workspace_title")}</span>
              {stagedCount > 0 && <span className="mode-tab-count font-mono">{stagedCount}</span>}
            </button>
          </div>
        )}

        {/* Security & Global Actions */}
        <div className="header-top-right-group">
          <div className="header-status-badge">
            {landlockSupported ? (
              <span className="security-pill secure" title={t("security.landlock_active")}>
                <ShieldCheck size={14} />
                <span>Landlock {landlockAbi || "LSM"}</span>
              </span>
            ) : (
              <span className="security-pill warning" title={t("security.landlock_unsupported")}>
                <Lock size={14} />
                <span>No Landlock</span>
              </span>
            )}
          </div>

          {/* Settings Button */}
          {onOpenSettings && (
            <button
              type="button"
              className="header-icon-btn header-settings-btn"
              onClick={onOpenSettings}
              title={t("settings.dialog_title")}
            >
              <Settings size={15} />
              <span className="settings-btn-label">{t("start.btn_settings")}</span>
            </button>
          )}

          {/* Language Toggle */}
          <button
            type="button"
            className="header-lang-btn"
            onClick={onToggleLang}
            title={t("common.switch_language")}
          >
            <Globe size={14} />
            <span>{currentLang === "ja" ? "English" : "日本語"}</span>
          </button>
        </div>
      </div>

      {/* Row 2: Secondary Tool Bar (Path Info & Panel Layout Toggles) */}
      <div className="header-row header-secondary-row">
        <div className="header-location-summary">
          {currentPath && (
            <div className="header-current-dir-chip" title={currentPath}>
              <Folder size={12} className="text-teal-400" />
              <span className="location-label font-mono">{currentPath}</span>
            </div>
          )}
        </div>

        <div className="header-layout-controls">
          <span className="layout-controls-hint">{t("panels.toggle_sidebar")}:</span>
          <button
            type="button"
            className={`header-sub-btn ${sidebarVisible ? "active" : ""}`}
            onClick={onToggleSidebar}
            title={t("panels.toggle_sidebar")}
          >
            <PanelRight size={13} />
            <span>{t("panels.details_title")}</span>
          </button>

          <button
            type="button"
            className={`header-sub-btn ${bottomVisible ? "active" : ""}`}
            onClick={onToggleBottom}
            title={t("panels.toggle_bottom")}
          >
            <PanelBottom size={13} />
            <span>{t("panels.bottom_title")}</span>
          </button>

          <button
            type="button"
            className="header-sub-btn"
            onClick={onResetLayout}
            title={t("panels.reset_layout")}
          >
            <RotateCcw size={13} />
            <span>{t("panels.reset_layout")}</span>
          </button>
        </div>
      </div>
    </header>
  );
}
