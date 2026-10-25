import { useState } from "react";
import { Search, FileSearch, Sparkles, AlertCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { SearchResultDto } from "../types/SearchResultDto";
import type { FulltextHitDto } from "../types/FulltextHitDto";
import type { FileItem } from "../types/FileItem";

interface SearchPanelProps {
  onSelectItem: (item: Partial<FileItem>) => void;
  onLog: (msg: string) => void;
}

export function SearchPanel({ onSelectItem, onLog }: SearchPanelProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"filename" | "fulltext">("filename");
  const [query, setQuery] = useState("");
  const [isSearching, setIsSearching] = useState(false);
  const [filenameResult, setFilenameResult] = useState<SearchResultDto | null>(null);
  const [fulltextResult, setFulltextResult] = useState<FulltextHitDto[] | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const handleSearch = async () => {
    if (!query.trim()) return;
    setIsSearching(true);
    setErrorMsg(null);

    try {
      if (activeTab === "filename") {
        const res = await invoke<SearchResultDto>("search_files_by_name", {
          pattern: query.trim(),
          options: null,
        });
        setFilenameResult(res);
        onLog(`Filename search "${query}": found ${res.total_matches} matches (engine: ${res.engine_used})`);
      } else {
        const res = await invoke<FulltextHitDto[]>("search_fulltext", {
          query: query.trim(),
          options: null,
        });
        setFulltextResult(res);
        onLog(`Fulltext search "${query}": found ${res.length} matches via Tantivy+Lindera`);
      }
    } catch (err: unknown) {
      const msg = typeof err === "string" ? err : String(err);
      setErrorMsg(msg);
      onLog(`Search error: ${msg}`);
    } finally {
      setIsSearching(false);
    }
  };

  return (
    <div className="panel-container search-panel-container">
      <div className="panel-header">
        <div className="panel-title-group">
          <Search size={15} />
          <h3>{t("panels.search_title")}</h3>
        </div>

        <div className="search-tab-toggle">
          <button
            type="button"
            className={`tab-btn ${activeTab === "filename" ? "active" : ""}`}
            onClick={() => setActiveTab("filename")}
            title={t("search.filename_tab")}
          >
            <FileSearch size={12} />
            <span>{t("search.filename_tab").split(" ")[0]}</span>
          </button>
          <button
            type="button"
            className={`tab-btn ${activeTab === "fulltext" ? "active" : ""}`}
            onClick={() => setActiveTab("fulltext")}
            title={t("search.fulltext_tab")}
          >
            <Sparkles size={12} />
            <span>{t("search.fulltext_tab").split(" ")[0]}</span>
          </button>
        </div>
      </div>

      <div className="search-input-section">
        <form
          className="search-input-form"
          onSubmit={(e) => {
            e.preventDefault();
            handleSearch();
          }}
        >
          <input
            id="global-search-input"
            type="text"
            className="search-input-field"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={
              activeTab === "filename"
                ? t("search.placeholder_filename")
                : t("search.placeholder_fulltext")
            }
          />
          <button type="submit" className="search-submit-btn" disabled={isSearching}>
            {t("search.search_button")}
          </button>
        </form>
      </div>

      {errorMsg && (
        <div className="search-error-banner">
          <AlertCircle size={13} />
          <span>{errorMsg}</span>
        </div>
      )}

      <div className="search-results-list">
        {activeTab === "filename" && filenameResult && (
          <>
            <div className="search-result-meta">
              <span>{t("search.hits_count", { count: filenameResult.total_matches })}</span>
              <span className="backend-tag">{filenameResult.engine_used}</span>
            </div>
            <div className="search-items-scroll">
              {filenameResult.items.map((item) => (
                <div
                  key={item.item_id}
                  className="search-hit-item"
                  onClick={() => onSelectItem(item)}
                >
                  <span className="hit-name">{item.display_name}</span>
                  <span className="hit-b64-sub">#{item.inode}</span>
                </div>
              ))}
            </div>
          </>
        )}

        {activeTab === "fulltext" && fulltextResult && (
          <>
            <div className="search-result-meta">
              <span>{t("search.hits_count", { count: fulltextResult.length })}</span>
              <span className="backend-tag">Tantivy + Lindera</span>
            </div>
            <div className="search-items-scroll">
              {fulltextResult.map((hit) => (
                <div
                  key={hit.item_id}
                  className="search-hit-item fulltext-hit"
                  onClick={() =>
                    onSelectItem({
                      item_id: hit.item_id,
                      display_name: hit.display_name,
                      raw_base64: hit.raw_base64,
                      size: hit.size,
                      mtime_ms: hit.mtime_ms,
                    })
                  }
                >
                  <div className="hit-header">
                    <span className="hit-name">{hit.display_name}</span>
                    <span className="hit-score">score: {hit.score.toFixed(2)}</span>
                  </div>
                  {hit.snippet && (
                    <p
                      className="hit-snippet"
                      dangerouslySetInnerHTML={{ __html: hit.snippet }}
                    />
                  )}
                </div>
              ))}
            </div>
          </>
        )}

        {!filenameResult && !fulltextResult && !errorMsg && (
          <div className="search-empty-prompt">
            <p>{t("search.title")}</p>
          </div>
        )}
      </div>
    </div>
  );
}
