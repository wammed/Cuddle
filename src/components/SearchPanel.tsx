import { useState, useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import {
  Search,
  FileSearch,
  Sparkles,
  AlertCircle,
  Database,
  RefreshCw,
  FolderPlus,
  RotateCcw,
  Trash2,
  ChevronDown,
  Loader2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { SearchResultDto } from "../types/SearchResultDto";
import type { FulltextHitDto } from "../types/FulltextHitDto";
import type { FileItem } from "../types/FileItem";
import type { IndexStatsDto } from "../types/IndexStatsDto";
import type { IndexOperationResultDto } from "../types/IndexOperationResultDto";
import type { IndexOperationMode } from "../types/IndexOperationMode";
import type { RunIndexOptions } from "../types/RunIndexOptions";
import { encodePathToBase64 } from "../utils/path";

interface SearchPanelProps {
  onSelectItem: (item: Partial<FileItem>) => void;
  onLog: (msg: string) => void;
  currentPath?: string;
}

export function SearchPanel({ onSelectItem, onLog, currentPath }: SearchPanelProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"filename" | "fulltext">("filename");
  const [query, setQuery] = useState("");
  const [isSearching, setIsSearching] = useState(false);
  const [filenameResult, setFilenameResult] = useState<SearchResultDto | null>(null);
  const [fulltextResult, setFulltextResult] = useState<FulltextHitDto[] | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Explicit indexing controls state
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const [isIndexing, setIsIndexing] = useState(false);
  const [indexStats, setIndexStats] = useState<IndexStatsDto | null>(null);
  const [indexStatusMsg, setIndexStatusMsg] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [menuPosition, setMenuPosition] = useState<{ top: number; left: number } | null>(null);

  const fetchStats = async () => {
    try {
      const stats = await invoke<IndexStatsDto>("get_index_stats");
      setIndexStats(stats);
    } catch {
      // Ignore background stat fetch failure
    }
  };

  const updateMenuPosition = () => {
    if (triggerRef.current) {
      const rect = triggerRef.current.getBoundingClientRect();
      const menuWidth = 290;
      const menuHeight = 280;
      let top = rect.bottom + 6;
      if (typeof window !== "undefined" && window.innerHeight > 0) {
        if (top + menuHeight > window.innerHeight - 8) {
          top = Math.max(8, rect.top - menuHeight - 6);
        }
      }
      let left = rect.right - menuWidth;
      if (left < 8) {
        left = Math.max(8, rect.left);
      }
      setMenuPosition({ top, left });
    }
  };

  const toggleMenu = () => {
    if (isMenuOpen) {
      setIsMenuOpen(false);
    } else {
      fetchStats();
      updateMenuPosition();
      setIsMenuOpen(true);
    }
  };

  useEffect(() => {
    fetchStats();
  }, []);

  // Close dropdown menu when clicking outside or pressing Escape, reposition on resize/scroll
  useEffect(() => {
    if (!isMenuOpen) return;

    const handleScrollOrResize = () => {
      updateMenuPosition();
    };

    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node;
      if (
        menuRef.current &&
        !menuRef.current.contains(target) &&
        triggerRef.current &&
        !triggerRef.current.contains(target)
      ) {
        setIsMenuOpen(false);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setIsMenuOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeyDown);
    window.addEventListener("resize", handleScrollOrResize);
    window.addEventListener("scroll", handleScrollOrResize, true);

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("resize", handleScrollOrResize);
      window.removeEventListener("scroll", handleScrollOrResize, true);
    };
  }, [isMenuOpen]);

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

  const handleIndexOperation = async (mode: IndexOperationMode) => {
    setIsMenuOpen(false);
    setIsIndexing(true);
    setIndexStatusMsg(t("search.index_running"));
    onLog(`[Index] Running operation: ${mode} on "${currentPath || "."}"...`);

    try {
      const targetB64 = currentPath && currentPath !== "." ? encodePathToBase64(currentPath) : null;
      const options: RunIndexOptions = {
        mode,
        target_raw_base64: targetB64,
        max_depth: null,
        include_hidden: false,
      };

      const result = await invoke<IndexOperationResultDto>("run_index_operation", { options });

      const summary = t("search.index_result_summary", {
        indexed: result.indexed_count,
        updated: result.updated_count,
        unchanged: result.unchanged_count,
        skipped: result.skipped_count,
        total: result.total_documents,
      });

      setIndexStatusMsg(summary);
      onLog(`[Index] ${result.message} (${result.elapsed_ms}ms)`);
      await fetchStats();

      // Clear summary message after 7 seconds
      setTimeout(() => {
        setIndexStatusMsg((prev) => (prev === summary ? null : prev));
      }, 7000);
    } catch (err: unknown) {
      const msg = typeof err === "string" ? err : String(err);
      setIndexStatusMsg(`Error: ${msg}`);
      onLog(`[Index] Error: ${msg}`);
    } finally {
      setIsIndexing(false);
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

          {/* Index Operation Dropdown Switch */}
          <div className="index-dropdown-container">
            <button
              type="button"
              id="index-management-dropdown-btn"
              ref={triggerRef}
              className={`index-dropdown-trigger-btn ${isIndexing ? "indexing" : ""}`}
              onClick={toggleMenu}
              title={t("search.index_button_title")}
              disabled={isIndexing}
            >
              {isIndexing ? (
                <Loader2 size={13} className="spin-animation" />
              ) : (
                <Database size={13} />
              )}
              <span className="index-btn-label">{t("search.index_button_label")}</span>
              <ChevronDown size={11} className={`chevron-icon ${isMenuOpen ? "open" : ""}`} />
            </button>

            {isMenuOpen &&
              createPortal(
                <div
                  ref={menuRef}
                  className="index-dropdown-menu"
                  style={{
                    position: "fixed",
                    top: `${menuPosition?.top ?? 70}px`,
                    left: `${menuPosition?.left ?? 20}px`,
                    zIndex: 99999,
                  }}
                >
                  <div className="index-dropdown-header">
                    <div className="index-header-title">
                      <Database size={12} />
                      <span>{t("search.index_mode_label")}</span>
                    </div>
                    {indexStats && (
                      <span className="index-stats-pill">
                        {t("search.index_stats", { count: indexStats.total_documents })}
                      </span>
                    )}
                  </div>

                  <div className="index-target-info">
                    <span className="target-label">{t("search.index_target_label")}:</span>
                    <span className="target-path" title={currentPath || "."}>
                      {currentPath
                        ? currentPath.length > 25
                          ? "..." + currentPath.slice(-22)
                          : currentPath
                        : "."}
                    </span>
                  </div>

                  <div className="index-dropdown-items">
                    <button
                      type="button"
                      className="index-dropdown-item"
                      onClick={() => handleIndexOperation("IncrementalUpdate")}
                    >
                      <div className="item-icon-col icon-incremental">
                        <RefreshCw size={13} />
                      </div>
                      <div className="item-text-col">
                        <span className="item-title">{t("search.index_incremental")}</span>
                        <span className="item-desc">{t("search.index_incremental_desc")}</span>
                      </div>
                    </button>

                    <button
                      type="button"
                      className="index-dropdown-item"
                      onClick={() => handleIndexOperation("IndexDirectory")}
                    >
                      <div className="item-icon-col icon-add">
                        <FolderPlus size={13} />
                      </div>
                      <div className="item-text-col">
                        <span className="item-title">{t("search.index_add_folder")}</span>
                        <span className="item-desc">{t("search.index_add_folder_desc")}</span>
                      </div>
                    </button>

                    <button
                      type="button"
                      className="index-dropdown-item"
                      onClick={() => handleIndexOperation("FullRebuild")}
                    >
                      <div className="item-icon-col icon-rebuild">
                        <RotateCcw size={13} />
                      </div>
                      <div className="item-text-col">
                        <span className="item-title">{t("search.index_rebuild")}</span>
                        <span className="item-desc">{t("search.index_rebuild_desc")}</span>
                      </div>
                    </button>

                    <div className="index-dropdown-divider" />

                    <button
                      type="button"
                      className="index-dropdown-item danger"
                      onClick={() => handleIndexOperation("ResetClear")}
                    >
                      <div className="item-icon-col icon-reset">
                        <Trash2 size={13} />
                      </div>
                      <div className="item-text-col">
                        <span className="item-title">{t("search.index_reset")}</span>
                        <span className="item-desc">{t("search.index_reset_desc")}</span>
                      </div>
                    </button>
                  </div>
                </div>,
                document.body
              )}
          </div>
        </form>

        {indexStatusMsg && (
          <div className="index-status-banner">
            {isIndexing && <Loader2 size={12} className="spin-animation" />}
            <span>{indexStatusMsg}</span>
          </div>
        )}
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
