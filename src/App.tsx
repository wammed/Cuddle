import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { changeLanguage, SupportedLanguage } from "./i18n";
import type { FileItem } from "./types/FileItem";
import type { AppConfig } from "./types/AppConfig";
import type { ExtractedTextDto } from "./types/ExtractedTextDto";
import { Globe, ShieldCheck, HardDrive, Terminal, Lock, FileCode, Binary, Sliders, FileText, Layers } from "lucide-react";
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
  const [files, setFiles] = useState<FileItem[]>([]);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [extractedInfo, setExtractedInfo] = useState<ExtractedTextDto | null>(null);
  const [extractLoading, setExtractLoading] = useState(false);

  const currentLang = (i18n.language.startsWith("ja") ? "ja" : "en") as SupportedLanguage;

  const toggleLanguage = () => {
    const nextLang: SupportedLanguage = currentLang === "ja" ? "en" : "ja";
    changeLanguage(nextLang);
  };

  const handleExtractText = async (file: FileItem) => {
    try {
      setExtractLoading(true);
      const res = await invoke<ExtractedTextDto>("extract_file_text", {
        pathBase64: file.raw_base64,
        options: null,
      });
      setExtractedInfo(res);
    } catch (err) {
      console.error("Text extraction failed:", err);
    } finally {
      setExtractLoading(false);
    }
  };

  useEffect(() => {
    invoke<LandlockStatus>("get_landlock_status")
      .then((status) => setLandlock(status))
      .catch((err) => console.error("Failed to query landlock status:", err));

    invoke<FileItem[]>("get_demo_items")
      .then((items) => setFiles(items))
      .catch((err) => console.error("Failed to load demo files:", err));

    invoke<AppConfig>("get_app_config")
      .then((cfg) => setConfig(cfg))
      .catch((err) => console.error("Failed to load app config:", err));
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
          <img src="/Cuddle-Icon.svg" alt="Cuddle Icon" className="brand-logo-img" />
          <div>
            <div className="brand-title-row">
              <h1 className="brand-title">{t("app.title")}</h1>
              <span className="brand-badge">v0.1.0</span>
            </div>
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
            {landlock?.supported ? <ShieldCheck size={18} /> : <Lock size={18} />}
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
            <h2>{t("workspace.demo_items_title")}</h2>
          </div>
          <div className="file-list">
            {files.map((file) => (
              <div key={file.item_id} className="file-item-card">
                <div className="file-item-header">
                  {file.display_name.includes("\uFFFD") ? (
                    <Binary size={16} className="non-utf8-icon" />
                  ) : (
                    <FileCode size={16} className="utf8-icon" />
                  )}
                  <span className="file-item-name">{file.display_name}</span>
                  <span className="file-item-size">{file.size.toString()} B</span>
                  <button
                    type="button"
                    className="extract-action-btn"
                    onClick={() => handleExtractText(file)}
                    disabled={extractLoading}
                    title={t("extractor.title")}
                  >
                    <FileText size={12} />
                    <span>{t("extractor.title")}</span>
                  </button>
                </div>
                <div className="file-item-b64">
                  <span className="b64-label">{t("workspace.raw_base64_label")}:</span>
                  <code className="b64-value">{file.raw_base64}</code>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="card">
          <div className="card-header">
            <Layers size={18} />
            <h2>{t("extractor.title")}</h2>
          </div>
          {extractedInfo ? (
            <div className="extractor-result">
              <div className="config-item">
                <span className="config-label">{t("extractor.encoding_label")}:</span>
                <span className="config-val-pill secure">{extractedInfo.encoding_used}</span>
              </div>
              <div className="config-item">
                <span className="config-label">Tier:</span>
                <span className="config-val-pill">
                  {extractedInfo.tier_used === "tier1_direct"
                    ? t("extractor.tier1_label")
                    : extractedInfo.tier_used === "tier2_isolated_pdf"
                    ? t("extractor.tier2_label")
                    : t("extractor.skipped_label")}
                </span>
              </div>
              <div className="config-item">
                <span className="config-label">{t("extractor.chars_count", { count: extractedInfo.extracted_chars })}</span>
                {extractedInfo.truncated && (
                  <span className="status-badge warning">{t("extractor.truncated_warning")}</span>
                )}
              </div>
              {extractedInfo.warning && (
                <p className="card-meta warning">{extractedInfo.warning}</p>
              )}
              {extractedInfo.text && (
                <div className="extracted-text-preview">
                  <p className="b64-label">{t("search.snippet_label")}:</p>
                  <pre className="extracted-text-box">{extractedInfo.text.slice(0, 300)}</pre>
                </div>
              )}
            </div>
          ) : (
            <p className="card-desc">
              {t("extractor.tier1_label")} / {t("extractor.tier2_label")}
            </p>
          )}
        </div>

        <div className="card">
          <div className="card-header">
            <Sliders size={18} />
            <h2>{t("config.title")}</h2>
          </div>
          {config ? (
            <>
              <div className="config-item">
                <span className="config-label">{t("config.theme_label")}:</span>
                <span className="config-val-pill">{config.app.theme}</span>
              </div>
              <div className="config-item">
                <span className="config-label">{t("config.min_space_label")}:</span>
                <span className="config-val-pill secure">{config.workspace.min_free_space_multiplier}x</span>
              </div>
              <div className="config-item">
                <span className="config-label">{t("config.landlock_enforced")}:</span>
                <span className="config-val-pill secure">
                  {config.security.enforce_landlock ? "true" : "false"}
                </span>
              </div>
              <p className="card-meta">{t("config.guardrail_note")}</p>
            </>
          ) : (
            <p className="card-desc">Loading config.toml...</p>
          )}
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
        <span className="footer-brand">Cuddle: File Workbench - Phase 2.4 Text Extraction (Tier 1 Direct & Tier 2 Isolated Worker) Verified</span>
        <span className="footer-font-hint">Nerd Font: Pre-required (System Fallback)</span>
      </footer>
    </main>
  );
}

export default App;
