import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { changeLanguage, SupportedLanguage } from "./i18n";
import { Globe, ShieldCheck, HardDrive, Terminal, Lock } from "lucide-react";
import "./App.css";

interface LandlockStatus {
  supported: boolean;
  abi_version: string | null;
}

function App() {
  const { t, i18n } = useTranslation();
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [landlock, setLandlock] = useState<LandlockStatus | null>(null);

  const currentLang = (i18n.language.startsWith("ja") ? "ja" : "en") as SupportedLanguage;

  const toggleLanguage = () => {
    const nextLang: SupportedLanguage = currentLang === "ja" ? "en" : "ja";
    changeLanguage(nextLang);
  };

  useEffect(() => {
    invoke<LandlockStatus>("get_landlock_status")
      .then((status) => setLandlock(status))
      .catch((err) => console.error("Failed to query landlock status:", err));
  }, []);

  async function greet() {
    try {
      const res = await invoke<string>("greet", { name });
      setGreetMsg(res);
    } catch (err) {
      console.error(err);
    }
  }

  return (
    <main className="app-container">
      <header className="app-header">
        <div className="brand-group">
          <ShieldCheck className="brand-icon" size={28} />
          <div>
            <h1 className="brand-title">{t("app.title")}</h1>
            <p className="brand-subtitle">{t("app.subtitle")}</p>
          </div>
        </div>

        <button
          type="button"
          className="lang-toggle-button"
          onClick={toggleLanguage}
          title={t("common.switch_language")}
        >
          <Globe size={16} />
          <span>{currentLang === "ja" ? "English" : "日本語"}</span>
        </button>
      </header>

      <section className="dashboard-grid">
        <div className="card">
          <div className="card-header">
            <Lock size={18} />
            <h2>Landlock LSM</h2>
          </div>
          {landlock?.supported ? (
            <>
              <p className="status-badge secure">
                {t("security.landlock_active")}
              </p>
              <p className="card-desc">
                {t("security.landlock_abi", { abi: landlock.abi_version || "Unknown" })}
              </p>
              <p className="card-meta">{t("workspace.original_protected")}</p>
            </>
          ) : (
            <>
              <p className="status-badge warning">
                {t("security.landlock_unsupported")}
              </p>
              <p className="card-desc">{t("workspace.original_protected")}</p>
            </>
          )}
        </div>

        <div className="card">
          <div className="card-header">
            <HardDrive size={18} />
            <h2>{t("nav.workspace")}</h2>
          </div>
          <p className="status-badge ready">{t("common.status_ready")}</p>
          <p className="card-desc">{t("workspace.empty_state")}</p>
          <p className="card-meta">{t("workspace.reflink_enabled")}</p>
        </div>

        <div className="card">
          <div className="card-header">
            <Terminal size={18} />
            <h2>{t("common.greet")} (IPC)</h2>
          </div>
          <form
            className="input-row"
            onSubmit={(e) => {
              e.preventDefault();
              greet();
            }}
          >
            <input
              id="greet-input"
              className="text-input"
              value={name}
              onChange={(e) => setName(e.currentTarget.value)}
              placeholder={t("common.placeholder_name")}
            />
            <button type="submit" className="primary-button">
              {t("common.greet")}
            </button>
          </form>
          {greetMsg && <p className="greet-result">{greetMsg}</p>}
        </div>
      </section>

      <footer className="app-footer">
        <span>FileWorkbench Security PoC - Landlock Kernel Isolation Active</span>
      </footer>
    </main>
  );
}

export default App;
