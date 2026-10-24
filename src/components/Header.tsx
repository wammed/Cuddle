import { Globe, ShieldCheck, Lock, PanelRight, PanelBottom, RotateCcw } from "lucide-react";
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
}: HeaderProps) {
  const { t } = useTranslation();

  return (
    <header className="main-app-header">
      <div className="header-brand-group">
        <img src="/Cuddle-Icon.svg" alt="Cuddle Icon" className="header-logo-img" />
        <div>
          <div className="header-title-row">
            <h1 className="header-title">{t("app.title")}</h1>
            <span className="header-badge">v0.1.0</span>
          </div>
          <p className="header-subtitle">{t("app.subtitle")}</p>
        </div>
      </div>

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

      <div className="header-controls">
        <button
          type="button"
          className="header-icon-btn"
          onClick={onResetLayout}
          title={t("panels.reset_layout")}
        >
          <RotateCcw size={15} />
        </button>

        <button
          type="button"
          className={`header-icon-btn ${bottomVisible ? "active" : ""}`}
          onClick={onToggleBottom}
          title={t("panels.toggle_bottom")}
        >
          <PanelBottom size={15} />
        </button>

        <button
          type="button"
          className={`header-icon-btn ${sidebarVisible ? "active" : ""}`}
          onClick={onToggleSidebar}
          title={t("panels.toggle_sidebar")}
        >
          <PanelRight size={15} />
        </button>

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
    </header>
  );
}
