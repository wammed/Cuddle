import React, { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  Plus,
  Trash2,
  Edit2,
  RotateCcw,
  Search,
  Check,
  AlertTriangle,
  Code2,
  Terminal,
  Save,
  X,
  FileCode,
} from "lucide-react";
import type { EditorSnippetDto } from "../types/EditorSnippetDto";
import {
  BASE_SNIPPET_LANGUAGES,
  KNOWN_LANGUAGES_MAP,
  DEFAULT_EDITOR_SNIPPETS,
  SnippetLanguageKey,
  getAvailableSnippetLanguages,
  normalizeSnippetLanguageKey,
} from "../utils/snippets";

export interface SnippetManagerViewProps {
  snippets: EditorSnippetDto[];
  onUpdateSnippets: (newSnippets: EditorSnippetDto[]) => Promise<void> | void;
  initialLanguage?: SnippetLanguageKey | "all";
  onSnippetSelect?: (snippet: EditorSnippetDto) => void;
}

export function SnippetManagerView({
  snippets,
  onUpdateSnippets,
  initialLanguage,
  onSnippetSelect,
}: SnippetManagerViewProps) {
  const { t } = useTranslation();

  const [selectedFilterLang, setSelectedFilterLang] = useState<string>(
    initialLanguage || "all"
  );
  const [searchQuery, setSearchQuery] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [isAddingNew, setIsAddingNew] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [isCustomLangInput, setIsCustomLangInput] = useState(false);
  const [customLangText, setCustomLangText] = useState("");
  const [statusMessage, setStatusMessage] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);

  // Available languages dynamically computed from base + snippets
  const availableFilterLanguages = useMemo(() => {
    return getAvailableSnippetLanguages(snippets);
  }, [snippets]);

  // Form options: all known languages (Python, Rust, etc.) + custom
  const selectableFormLanguages = useMemo(() => {
    const seenKeys = new Set<string>();
    const seenLabels = new Set<string>();
    const list: { key: string; label: string }[] = [];

    for (const b of BASE_SNIPPET_LANGUAGES) {
      const normKey = normalizeSnippetLanguageKey(b.key);
      seenKeys.add(normKey);
      seenLabels.add(b.label.toLowerCase());
      list.push({ key: normKey, label: b.label });
    }

    for (const [k, v] of Object.entries(KNOWN_LANGUAGES_MAP)) {
      const normKey = normalizeSnippetLanguageKey(k);
      if (!seenKeys.has(normKey) && !seenLabels.has(v.label.toLowerCase())) {
        seenKeys.add(normKey);
        seenLabels.add(v.label.toLowerCase());
        list.push({ key: normKey, label: v.label });
      }
    }

    for (const s of snippets) {
      const normKey = normalizeSnippetLanguageKey(s.language);
      if (normKey && !seenKeys.has(normKey)) {
        const label =
          KNOWN_LANGUAGES_MAP[normKey]?.label ||
          normKey.charAt(0).toUpperCase() + normKey.slice(1);
        if (!seenLabels.has(label.toLowerCase())) {
          seenKeys.add(normKey);
          seenLabels.add(label.toLowerCase());
          list.push({
            key: normKey,
            label,
          });
        }
      }
    }

    return list;
  }, [snippets]);

  // Form State for Adding / Editing
  const [formData, setFormData] = useState<{
    id: string;
    language: SnippetLanguageKey;
    label: string;
    snippet: string;
    description: string;
  }>({
    id: "",
    language: (initialLanguage && initialLanguage !== "all"
      ? normalizeSnippetLanguageKey(initialLanguage)
      : "bash") as SnippetLanguageKey,
    label: "",
    snippet: "",
    description: "",
  });

  const showStatus = (type: "success" | "error", text: string) => {
    setStatusMessage({ type, text });
    setTimeout(() => {
      setStatusMessage(null);
    }, 3000);
  };

  // Filter snippets by language and search query
  const filteredSnippets = useMemo(() => {
    return snippets.filter((s) => {
      const sLang = normalizeSnippetLanguageKey(s.language);
      const matchesLang =
        selectedFilterLang === "all" || sLang === selectedFilterLang;
      if (!matchesLang) return false;

      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        s.label.toLowerCase().includes(q) ||
        s.snippet.toLowerCase().includes(q) ||
        (s.description && s.description.toLowerCase().includes(q))
      );
    });
  }, [snippets, selectedFilterLang, searchQuery]);

  // Open Form to Add New Snippet
  const handleStartAdd = () => {
    const defaultLang =
      selectedFilterLang !== "all"
        ? (normalizeSnippetLanguageKey(selectedFilterLang) as SnippetLanguageKey)
        : "bash";
    setFormData({
      id: `custom-${Date.now()}`,
      language: defaultLang,
      label: "",
      snippet: "",
      description: "",
    });
    setIsCustomLangInput(false);
    setCustomLangText("");
    setEditingId(null);
    setIsAddingNew(true);
  };

  // Open Form to Edit Existing Snippet
  const handleStartEdit = (s: EditorSnippetDto) => {
    const normLang = normalizeSnippetLanguageKey(s.language);
    const isKnown = selectableFormLanguages.some((l) => l.key === normLang);
    setFormData({
      id: s.id,
      language: (normLang as SnippetLanguageKey) || "bash",
      label: s.label,
      snippet: s.snippet,
      description: s.description || "",
    });
    if (!isKnown) {
      setIsCustomLangInput(true);
      setCustomLangText(s.language);
    } else {
      setIsCustomLangInput(false);
      setCustomLangText("");
    }
    setIsAddingNew(false);
    setEditingId(s.id);
  };

  // Cancel Form
  const handleCancelForm = () => {
    setIsAddingNew(false);
    setEditingId(null);
    setIsCustomLangInput(false);
    setCustomLangText("");
  };

  // Save Add/Edit
  const handleSaveForm = async (e: React.FormEvent) => {
    e.preventDefault();
    const rawLang = isCustomLangInput ? customLangText : formData.language;
    const finalLang = normalizeSnippetLanguageKey(rawLang) as SnippetLanguageKey;

    if (!finalLang) {
      showStatus("error", t("snippets.validation_lang_required") || "言語/形式を入力してください");
      return;
    }

    if (!formData.label.trim() || !formData.snippet.trim()) {
      showStatus("error", t("snippets.validation_required") || "ラベルとスニペット内容は必須です");
      return;
    }

    try {
      let updated: EditorSnippetDto[];
      if (isAddingNew) {
        const newSnippet: EditorSnippetDto = {
          id: formData.id || `custom-${Date.now()}`,
          language: finalLang,
          label: formData.label.trim(),
          snippet: formData.snippet,
          description: formData.description.trim() || null,
        };
        updated = [...snippets, newSnippet];
        showStatus("success", t("snippets.saved_new") || "スニペットを追加しました");
      } else {
        updated = snippets.map((s) =>
          s.id === editingId
            ? {
                ...s,
                language: finalLang,
                label: formData.label.trim(),
                snippet: formData.snippet,
                description: formData.description.trim() || null,
              }
            : s
        );
        showStatus("success", t("snippets.saved_updated") || "スニペットを更新しました");
      }

      await onUpdateSnippets(updated);
      setIsAddingNew(false);
      setEditingId(null);
      setIsCustomLangInput(false);
      setCustomLangText("");
    } catch (err) {
      console.error("Failed to save snippet:", err);
      showStatus("error", t("snippets.save_failed") || "保存に失敗しました");
    }
  };

  // Delete Snippet
  const handleDelete = async (id: string) => {
    try {
      const updated = snippets.filter((s) => s.id !== id);
      await onUpdateSnippets(updated);
      setDeletingId(null);
      showStatus("success", t("snippets.deleted") || "スニペットを削除しました");
    } catch (err) {
      console.error("Failed to delete snippet:", err);
      showStatus("error", t("snippets.delete_failed") || "削除に失敗しました");
    }
  };

  // Reset to default presets
  const handleResetDefaults = async () => {
    if (
      window.confirm(
        t("snippets.reset_confirm") ||
          "すべてのスニペットを初期プリセット（各形式5個、計40個）にリセットしますか？"
      )
    ) {
      try {
        await onUpdateSnippets(DEFAULT_EDITOR_SNIPPETS);
        showStatus("success", t("snippets.reset_success") || "初期プリセットにリセットしました");
      } catch (err) {
        console.error("Failed to reset snippets:", err);
        showStatus("error", t("snippets.reset_failed") || "リセットに失敗しました");
      }
    }
  };

  return (
    <div className="snippet-manager-container">
      {/* Top Controls Bar: Language Filter, Search, Action Buttons */}
      <div className="snippet-manager-controls">
        <div className="snippet-filter-group">
          <label className="text-xs font-semibold text-muted flex items-center gap-1.5">
            <Terminal size={14} className="text-teal-400" />
            <span>{t("snippets.filter_lang") || "対象形式:"}</span>
          </label>
          <select
            className="snippet-lang-select font-mono text-xs"
            value={selectedFilterLang}
            onChange={(e) => setSelectedFilterLang(e.target.value)}
          >
            <option value="all">{t("snippets.all_languages") || "全言語 / 全形式 (All)"}</option>
            {availableFilterLanguages.map((lang) => {
              const count = snippets.filter(
                (s) => normalizeSnippetLanguageKey(s.language) === lang.key
              ).length;
              return (
                <option key={lang.key} value={lang.key}>
                  {lang.label} ({count})
                </option>
              );
            })}
          </select>
        </div>

        <div className="snippet-search-wrap">
          <Search size={14} className="text-muted shrink-0" />
          <input
            type="text"
            className="snippet-search-input font-mono text-xs"
            placeholder={t("snippets.search_placeholder") || "スニペットを検索..."}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>

        <div className="snippet-actions-wrap">
          <button
            type="button"
            className="btn-editor-primary text-xs"
            onClick={handleStartAdd}
          >
            <Plus size={13} />
            <span>{t("snippets.add_btn") || "新規追加"}</span>
          </button>
          <button
            type="button"
            className="btn-editor-secondary text-xs"
            onClick={handleResetDefaults}
            title={t("snippets.reset_tooltip") || "初期プリセット（40種）に戻す"}
          >
            <RotateCcw size={13} />
            <span>{t("snippets.reset_btn") || "初期化"}</span>
          </button>
        </div>
      </div>

      {/* Status Alert Banner */}
      {statusMessage && (
        <div
          className={`snippet-alert ${statusMessage.type === "success" ? "success" : "error"}`}
        >
          {statusMessage.type === "success" ? (
            <Check size={14} className="shrink-0" />
          ) : (
            <AlertTriangle size={14} className="shrink-0" />
          )}
          <span>{statusMessage.text}</span>
        </div>
      )}

      {/* Add / Edit Form Modal / Inline Card */}
      {(isAddingNew || editingId) && (
        <form className="snippet-form-card" onSubmit={handleSaveForm}>
          <div className="snippet-form-header">
            <h4 className="text-sm font-semibold text-teal-300 flex items-center gap-2">
              <Code2 size={16} />
              {isAddingNew
                ? t("snippets.add_title") || "新規スニペットの作成"
                : t("snippets.edit_title") || "スニペットの編集"}
            </h4>
            <button
              type="button"
              className="panel-header-btn"
              onClick={handleCancelForm}
            >
              <X size={15} />
            </button>
          </div>

          <div className="snippet-form-grid">
            <div className="snippet-form-field">
              <label className="text-xs text-muted font-medium">
                {t("snippets.field_lang") || "言語 / 形式"}
              </label>
              {!isCustomLangInput ? (
                <div className="flex gap-1.5 items-center w-full min-w-0">
                  <select
                    className="snippet-input font-mono text-xs flex-1 min-w-0"
                    value={formData.language}
                    onChange={(e) => {
                      if (e.target.value === "__custom__") {
                        setIsCustomLangInput(true);
                        setCustomLangText("");
                      } else {
                        setFormData({
                          ...formData,
                          language: e.target.value as SnippetLanguageKey,
                        });
                      }
                    }}
                  >
                    {selectableFormLanguages.map((lang) => (
                      <option key={lang.key} value={lang.key}>
                        {lang.label} ({lang.key})
                      </option>
                    ))}
                    <option value="__custom__">
                      {t("snippets.custom_lang_option") || "＋ その他の言語を入力..."}
                    </option>
                  </select>
                </div>
              ) : (
                <div className="flex gap-1.5 items-center w-full min-w-0">
                  <input
                    type="text"
                    className="snippet-input font-mono text-xs flex-1 min-w-0"
                    placeholder="例: python, rust, nix"
                    value={customLangText}
                    onChange={(e) => setCustomLangText(e.target.value)}
                    autoFocus
                    required
                  />
                  <button
                    type="button"
                    className="btn-editor-secondary text-xs py-1 px-2"
                    onClick={() => {
                      setIsCustomLangInput(false);
                      setCustomLangText("");
                    }}
                    title={t("snippets.cancel_custom_lang") || "プリセット一覧に戻す"}
                  >
                    <X size={12} />
                  </button>
                </div>
              )}
            </div>

            <div className="snippet-form-field">
              <label className="text-xs text-muted font-medium">
                {t("snippets.field_label") || "表示ラベル (例: + PATH 追加)"}
              </label>
              <input
                type="text"
                className="snippet-input font-mono text-xs w-full min-w-0"
                value={formData.label}
                onChange={(e) =>
                  setFormData({ ...formData, label: e.target.value })
                }
                placeholder="+ PATH 追加"
                required
              />
            </div>
          </div>

          <div className="snippet-form-field mb-2">
            <label className="text-xs text-muted font-medium">
              {t("snippets.field_desc") || "説明・補足 (任意)"}
            </label>
            <input
              type="text"
              className="snippet-input text-xs"
              value={formData.description}
              onChange={(e) =>
                setFormData({ ...formData, description: e.target.value })
              }
              placeholder="~/.local/bin を PATH に追加"
            />
          </div>

          <div className="snippet-form-field mb-3">
            <label className="text-xs text-muted font-medium">
              {t("snippets.field_code") || "挿入テキスト / スニペット内容 (複数行可)"}
            </label>
            <textarea
              className="snippet-textarea font-mono text-xs"
              value={formData.snippet}
              onChange={(e) =>
                setFormData({ ...formData, snippet: e.target.value })
              }
              placeholder='export PATH="$HOME/.local/bin:$PATH"'
              rows={4}
              required
            />
          </div>

          <div className="snippet-form-actions">
            <button
              type="button"
              className="btn-editor-secondary text-xs"
              onClick={handleCancelForm}
            >
              {t("common.cancel") || "キャンセル"}
            </button>
            <button type="submit" className="btn-editor-primary text-xs">
              <Save size={13} />
              <span>{t("common.save") || "保存"}</span>
            </button>
          </div>
        </form>
      )}

      {/* Snippet List */}
      <div className="snippet-list-scroll">
        {filteredSnippets.length === 0 ? (
          <div className="snippet-empty-state">
            <FileCode size={32} className="text-muted mb-2" />
            <p className="text-sm font-medium text-slate-300">
              {t("snippets.empty_message") || "該当するスニペットがありません"}
            </p>
            <span className="text-xs text-muted">
              {t("snippets.empty_hint") || "上の「＋ 新規追加」からお好みのスニペットを登録できます"}
            </span>
          </div>
        ) : (
          <div className="snippet-grid">
            {filteredSnippets.map((s) => {
              const normLang = normalizeSnippetLanguageKey(s.language);
              const langConfig = KNOWN_LANGUAGES_MAP[normLang];
              const badge = langConfig?.badge || normLang.slice(0, 4).toUpperCase();
              return (
                <div key={s.id} className="snippet-card">
                  <div className="snippet-card-header">
                    <div className="flex items-center gap-1.5 min-w-0">
                      <span className="snippet-badge uppercase font-mono">
                        {badge}
                      </span>
                      <span
                        className="snippet-card-label font-mono truncate"
                        title={s.label}
                      >
                        {s.label}
                      </span>
                    </div>

                    <div className="snippet-card-btns">
                      {onSnippetSelect && (
                        <button
                          type="button"
                          className="snippet-mini-btn insert"
                          onClick={() => onSnippetSelect(s)}
                          title={t("snippets.insert_tooltip") || "このスニペットを挿入"}
                        >
                          <Plus size={12} />
                        </button>
                      )}
                      <button
                        type="button"
                        className="snippet-mini-btn edit"
                        onClick={() => handleStartEdit(s)}
                        title={t("snippets.edit_tooltip") || "編集"}
                      >
                        <Edit2 size={12} />
                      </button>
                      <button
                        type="button"
                        className="snippet-mini-btn delete"
                        onClick={() => setDeletingId(s.id)}
                        title={t("snippets.delete_tooltip") || "削除"}
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  </div>

                  {s.description && (
                    <p className="snippet-card-desc text-xs text-muted truncate" title={s.description}>
                      {s.description}
                    </p>
                  )}

                  <pre className="snippet-card-code font-mono text-xs">
                    <code>{s.snippet}</code>
                  </pre>

                  {/* Delete Confirmation Overlay */}
                  {deletingId === s.id && (
                    <div className="snippet-delete-confirm-overlay">
                      <p className="text-xs text-rose-300 font-semibold mb-2">
                        {t("snippets.delete_confirm_text") || "このスニペットを削除しますか？"}
                      </p>
                      <div className="flex gap-2">
                        <button
                          type="button"
                          className="btn-editor-secondary text-xs py-1 px-2"
                          onClick={() => setDeletingId(null)}
                        >
                          {t("common.cancel") || "キャンセル"}
                        </button>
                        <button
                          type="button"
                          className="btn-editor-danger text-xs py-1 px-2"
                          onClick={() => handleDelete(s.id)}
                        >
                          {t("common.delete") || "削除"}
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Footer config file note */}
      <div className="snippet-manager-footer-note font-mono text-xs text-muted">
        <span>
          {t("snippets.config_sync_note") ||
            "※ スニペット設定は $XDG_CONFIG_HOME/Cuddle/config.toml ([[snippets]]) に保存されます。"}
        </span>
      </div>
    </div>
  );
}

