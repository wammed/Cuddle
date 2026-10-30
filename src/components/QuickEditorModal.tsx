import React, { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import {
  FileEdit,
  Save,
  X,
  AlertTriangle,
  Check,
  Code2,
  Terminal,
  ShieldCheck,
  Loader2,
  FolderOpen,
  Settings2,
} from "lucide-react";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { EditorSnippetDto } from "../types/EditorSnippetDto";
import { detectLanguage, highlightCode } from "../utils/syntaxHighlight";
import { formatBytes } from "../utils/format";
import {
  DEFAULT_EDITOR_SNIPPETS,
  SnippetLanguageKey,
  detectInitialSnippetLanguage,
  loadSnippets,
  saveSnippets,
  getAvailableSnippetLanguages,
  getCommentPrefixForLanguage,
} from "../utils/snippets";
import { SnippetManagerModal } from "./SnippetManagerModal";

export interface QuickEditorModalProps {
  isOpen: boolean;
  stagedItem: StagedItemDto | null;
  onClose: () => void;
  onSaveSuccess: (stageId: string) => void;
}

export function QuickEditorModal({
  isOpen,
  stagedItem,
  onClose,
  onSaveSuccess,
}: QuickEditorModalProps) {
  const { t } = useTranslation();

  const [content, setContent] = useState<string>("");
  const [initialContent, setInitialContent] = useState<string>("");
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [isSaving, setIsSaving] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [saveSuccessMsg, setSaveSuccessMsg] = useState<string | null>(null);
  const [cursorPos, setCursorPos] = useState<{ line: number; col: number }>({ line: 1, col: 1 });
  const [showUnsavedPrompt, setShowUnsavedPrompt] = useState<boolean>(false);

  // Snippets & Language Selector
  const [snippets, setSnippets] = useState<EditorSnippetDto[]>(DEFAULT_EDITOR_SNIPPETS);
  const [selectedSnippetLang, setSelectedSnippetLang] = useState<SnippetLanguageKey>("bash");
  const [showSnippetManager, setShowSnippetManager] = useState<boolean>(false);

  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const highlightLayerRef = useRef<HTMLPreElement | null>(null);
  const gutterRef = useRef<HTMLDivElement | null>(null);

  const isDirty = content !== initialContent;

  const language = useMemo(() => {
    if (!stagedItem) return "plaintext";
    return detectLanguage(stagedItem.display_name);
  }, [stagedItem]);

  const currentSnippets = useMemo(() => {
    return snippets.filter((s) => s.language === selectedSnippetLang);
  }, [snippets, selectedSnippetLang]);

  const availableLanguages = useMemo(() => {
    return getAvailableSnippetLanguages(snippets);
  }, [snippets]);

  // Load configured snippets
  useEffect(() => {
    loadSnippets().then((loaded) => {
      if (loaded && loaded.length > 0) {
        setSnippets(loaded);
      }
    });
  }, []);

  // Load content when modal opens with a valid stagedItem
  useEffect(() => {
    if (!isOpen || !stagedItem) {
      setContent("");
      setInitialContent("");
      setIsLoading(false);
      setError(null);
      setSaveSuccessMsg(null);
      setShowUnsavedPrompt(false);
      return;
    }

    let isCancelled = false;
    setIsLoading(true);
    setError(null);
    setSaveSuccessMsg(null);
    setShowUnsavedPrompt(false);

    // Set initial selectable snippet language
    setSelectedSnippetLang(detectInitialSnippetLanguage(stagedItem.display_name));

    invoke<string>("read_staged_file_text", { stageId: stagedItem.stage_id })
      .then((data) => {
        if (isCancelled) return;
        setContent(data);
        setInitialContent(data);
        setIsLoading(false);
        setCursorPos({ line: 1, col: 1 });
      })
      .catch((err) => {
        if (isCancelled) return;
        console.error("Failed to read staged text:", err);
        setError(typeof err === "string" ? err : t("editor.load_error"));
        setIsLoading(false);
      });

    return () => {
      isCancelled = true;
    };
  }, [isOpen, stagedItem, t]);

  // Update cursor line and column position
  const updateCursorPosition = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    const pos = el.selectionStart || 0;
    const textUpToCursor = el.value.substring(0, pos);
    const lines = textUpToCursor.split("\n");
    const currentLine = lines.length;
    const currentCol = lines[lines.length - 1].length + 1;
    setCursorPos({ line: currentLine, col: currentCol });
  }, []);

  // Synchronize scroll between textarea, syntax highlight pre, and line numbers gutter
  const handleScroll = (e: React.UIEvent<HTMLTextAreaElement>) => {
    const target = e.currentTarget;
    if (highlightLayerRef.current) {
      highlightLayerRef.current.scrollTop = target.scrollTop;
      highlightLayerRef.current.scrollLeft = target.scrollLeft;
    }
    if (gutterRef.current) {
      gutterRef.current.scrollTop = target.scrollTop;
    }
  };

  // Syntax highlighted HTML computation
  const highlightedHtml = useMemo(() => {
    let html = highlightCode(content, language);
    // Ensure trailing newline is accounted for in pre tag
    if (content.endsWith("\n")) {
      html += '<span class="token-newline"> </span>';
    }
    return html;
  }, [content, language]);

  const lines = useMemo(() => {
    return content.split("\n");
  }, [content]);

  // Save handler
  const handleSave = useCallback(async () => {
    if (!stagedItem || isSaving) return;
    setIsSaving(true);
    setError(null);

    try {
      await invoke("write_staged_file_text", {
        stageId: stagedItem.stage_id,
        content,
      });
      setInitialContent(content);
      setSaveSuccessMsg(t("editor.saved"));
      onSaveSuccess(stagedItem.stage_id);
      setTimeout(() => {
        setSaveSuccessMsg(null);
      }, 2500);
    } catch (err) {
      console.error("Failed to write staged text:", err);
      setError(typeof err === "string" ? err : t("editor.save_error"));
    } finally {
      setIsSaving(false);
    }
  }, [stagedItem, content, isSaving, t, onSaveSuccess]);

  // Close handler with dirty checking
  const handleRequestClose = useCallback(() => {
    if (isDirty) {
      setShowUnsavedPrompt(true);
    } else {
      onClose();
    }
  }, [isDirty, onClose]);

  // Insert snippet at cursor or append
  const insertSnippet = useCallback((snippetText: string) => {
    const el = textareaRef.current;
    if (!el) {
      setContent((prev) => (prev.endsWith("\n") || prev.length === 0 ? prev + snippetText + "\n" : prev + "\n" + snippetText + "\n"));
      return;
    }
    const start = el.selectionStart;
    const end = el.selectionEnd;
    const prev = el.value;

    const prefix = prev.substring(0, start);
    const suffix = prev.substring(end);
    const needLeadingNewline = prefix.length > 0 && !prefix.endsWith("\n");
    const toInsert = (needLeadingNewline ? "\n" : "") + snippetText;

    const newContent = prefix + toInsert + suffix;
    setContent(newContent);

    setTimeout(() => {
      el.focus();
      const newCursor = start + toInsert.length;
      el.setSelectionRange(newCursor, newCursor);
      updateCursorPosition();
    }, 0);
  }, [updateCursorPosition]);

  // Toggle comments on current line or selection (# or -- or //)
  const toggleComment = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    const start = el.selectionStart;
    const end = el.selectionEnd;
    const fullText = el.value;

    const lineStart = fullText.lastIndexOf("\n", start - 1) + 1;
    let lineEnd = fullText.indexOf("\n", end);
    if (lineEnd === -1) lineEnd = fullText.length;

    const targetChunk = fullText.substring(lineStart, lineEnd);
    const targetLines = targetChunk.split("\n");

    const { prefix, check, regex } = getCommentPrefixForLanguage(selectedSnippetLang);

    const allCommented = targetLines.every((l) =>
      l.trimStart().startsWith(check)
    );
    const modifiedLines = targetLines.map((l) => {
      if (allCommented) {
        return l.replace(regex, "$1");
      } else {
        return l.replace(/^(\s*)/, `$1${prefix}`);
      }
    });

    const newChunk = modifiedLines.join("\n");
    const newContent =
      fullText.substring(0, lineStart) + newChunk + fullText.substring(lineEnd);
    setContent(newContent);

    setTimeout(() => {
      el.focus();
      el.setSelectionRange(lineStart, lineStart + newChunk.length);
      updateCursorPosition();
    }, 0);
  }, [selectedSnippetLang, updateCursorPosition]);

  // Keyboard shortcut handler
  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Save (Ctrl+S / Cmd+S)
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      handleSave();
      return;
    }

    // Toggle Comment (Ctrl+/ / Cmd+/)
    if ((e.ctrlKey || e.metaKey) && e.key === "/") {
      e.preventDefault();
      toggleComment();
      return;
    }

    // Escape to close
    if (e.key === "Escape") {
      e.preventDefault();
      handleRequestClose();
      return;
    }

    // Tab key indentation
    if (e.key === "Tab") {
      e.preventDefault();
      const el = e.currentTarget;
      const start = el.selectionStart;
      const end = el.selectionEnd;
      const val = el.value;

      if (!e.shiftKey) {
        // Insert 2 spaces
        const nextVal = val.substring(0, start) + "  " + val.substring(end);
        setContent(nextVal);
        setTimeout(() => {
          el.setSelectionRange(start + 2, start + 2);
          updateCursorPosition();
        }, 0);
      } else {
        // Shift+Tab: Remove up to 2 leading spaces
        const lineStart = val.lastIndexOf("\n", start - 1) + 1;
        const currentLine = val.substring(lineStart, start);
        if (currentLine.startsWith("  ")) {
          const nextVal = val.substring(0, lineStart) + currentLine.substring(2) + val.substring(start);
          setContent(nextVal);
          setTimeout(() => {
            el.setSelectionRange(Math.max(lineStart, start - 2), Math.max(lineStart, start - 2));
            updateCursorPosition();
          }, 0);
        } else if (currentLine.startsWith(" ")) {
          const nextVal = val.substring(0, lineStart) + currentLine.substring(1) + val.substring(start);
          setContent(nextVal);
          setTimeout(() => {
            el.setSelectionRange(Math.max(lineStart, start - 1), Math.max(lineStart, start - 1));
            updateCursorPosition();
          }, 0);
        }
      }
    }
  };

  if (!isOpen || !stagedItem) return null;

  return (
    <div className="modal-backdrop quick-editor-backdrop" role="dialog" aria-modal="true">
      <div className="quick-editor-dialog">
        {/* Header */}
        <div className="dialog-header quick-editor-header">
          <div className="quick-editor-title-group">
            <div className="quick-editor-icon-badge">
              <FileEdit size={18} className="text-teal-400" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="quick-editor-title">{t("editor.title")}</h3>
                <span className="quick-editor-filename font-mono">{stagedItem.display_name}</span>
                <span className="quick-editor-lang-badge uppercase">{language}</span>
                {isDirty && (
                  <span className="quick-editor-dirty-indicator" title={t("editor.modified_badge")}>
                    ● {t("editor.modified_badge")}
                  </span>
                )}
              </div>
              <p className="quick-editor-subtitle">{t("editor.subtitle")}</p>
            </div>
          </div>
          <button
            type="button"
            className="panel-header-btn"
            onClick={handleRequestClose}
            title={t("editor.close_btn")}
          >
            <X size={16} />
          </button>
        </div>

        {/* Path Info Banner */}
        <div className="quick-editor-paths-bar">
          <div className="path-item">
            <ShieldCheck size={14} className="text-emerald-400 shrink-0" />
            <span className="path-label">{t("editor.original_path")}</span>
            <span className="path-value font-mono" title={stagedItem.original_snapshot.canonical_path}>
              {stagedItem.original_snapshot.canonical_path}
            </span>
          </div>
          <div className="path-item">
            <FolderOpen size={14} className="text-teal-400 shrink-0" />
            <span className="path-label">{t("editor.staged_path")}</span>
            <span className="path-value font-mono text-teal-300" title={stagedItem.staged_path}>
              {stagedItem.staged_path}
            </span>
          </div>
        </div>

        {/* Snippets / Quick Toolbar */}
        <div className="quick-editor-toolbar">
          <div className="snippets-group">
            {/* Language Selector Dropdown */}
            <div className="snippet-lang-select-wrap">
              <Terminal size={13} className="text-teal-400 shrink-0" />
              <select
                className="snippet-lang-select font-mono text-xs"
                value={selectedSnippetLang}
                onChange={(e) =>
                  setSelectedSnippetLang(e.target.value as SnippetLanguageKey)
                }
                title={t("snippets.lang_select_title") || "スニペット形式を選択"}
              >
                {availableLanguages.map((lang) => (
                  <option key={lang.key} value={lang.key}>
                    {lang.label}
                  </option>
                ))}
              </select>
            </div>

            {/* Dynamic Snippet Chips for selected language */}
            {currentSnippets.map((s) => (
              <button
                key={s.id}
                type="button"
                className="snippet-chip-btn"
                onClick={() => insertSnippet(s.snippet)}
                title={s.description || s.snippet}
              >
                {s.label}
              </button>
            ))}

            {/* Comment Toggle */}
            <button
              type="button"
              className="snippet-chip-btn"
              onClick={toggleComment}
              title="コメント切替 (Ctrl+/)"
            >
              <Code2 size={12} />
              {t("editor.snippet_comment")}
            </button>

            {/* Open Snippet Manager Modal */}
            <button
              type="button"
              className="snippet-manage-btn"
              onClick={() => setShowSnippetManager(true)}
              title={t("snippets.manage_tooltip") || "スニペットを追加・編集・カスタマイズ"}
            >
              <Settings2 size={12} />
              <span>{t("snippets.manage_btn") || "カスタマイズ"}</span>
            </button>
          </div>

          <div className="quick-editor-hints">
            <span className="text-muted text-xs">{t("editor.read_only_hint")}</span>
          </div>
        </div>

        {/* Feedback / Alert Banners */}
        {error && (
          <div className="quick-editor-alert error">
            <AlertTriangle size={15} className="shrink-0" />
            <span>{error}</span>
          </div>
        )}
        {saveSuccessMsg && (
          <div className="quick-editor-alert success">
            <Check size={15} className="shrink-0" />
            <span>{saveSuccessMsg}</span>
          </div>
        )}

        {/* Main Code Editor Body */}
        <div className="quick-editor-workspace">
          {isLoading ? (
            <div className="quick-editor-loading">
              <Loader2 size={24} className="animate-spin text-teal-400 mb-2" />
              <span>{t("editor.loading")}</span>
            </div>
          ) : (
            <div className="quick-editor-container">
              {/* Line Numbers Gutter */}
              <div className="quick-editor-gutter" ref={gutterRef} aria-hidden="true">
                {lines.map((_, i) => (
                  <div key={i} className="gutter-line-number font-mono">
                    {i + 1}
                  </div>
                ))}
              </div>

              {/* Code Scroll Area (Highlight Layer + Transparent Textarea) */}
              <div className="quick-editor-scroll-area">
                <pre
                  ref={highlightLayerRef}
                  className="quick-editor-highlight-layer font-mono"
                  aria-hidden="true"
                  dangerouslySetInnerHTML={{ __html: highlightedHtml }}
                />
                <textarea
                  ref={textareaRef}
                  className="quick-editor-textarea font-mono"
                  value={content}
                  onChange={(e) => {
                    setContent(e.target.value);
                    updateCursorPosition();
                  }}
                  onSelect={updateCursorPosition}
                  onClick={updateCursorPosition}
                  onKeyUp={updateCursorPosition}
                  onScroll={handleScroll}
                  onKeyDown={handleKeyDown}
                  spellCheck={false}
                  autoCapitalize="off"
                  autoComplete="off"
                  autoCorrect="off"
                />
              </div>
            </div>
          )}
        </div>

        {/* Footer / Status Bar & Action Buttons */}
        <div className="quick-editor-footer">
          <div className="quick-editor-status-info font-mono">
            <span>{t("editor.cursor_pos", { line: cursorPos.line, col: cursorPos.col })}</span>
            <span className="divider">•</span>
            <span>{t("editor.lines_count", { count: lines.length })}</span>
            <span className="divider">•</span>
            <span>{formatBytes(new TextEncoder().encode(content).length)}</span>
          </div>

          <div className="quick-editor-footer-actions">
            <button
              type="button"
              className="btn-editor-secondary"
              onClick={handleRequestClose}
            >
              {t("editor.close_btn")}
            </button>
            <button
              type="button"
              className="btn-editor-primary"
              onClick={handleSave}
              disabled={isSaving || !isDirty}
            >
              {isSaving ? (
                <>
                  <Loader2 size={14} className="animate-spin" />
                  <span>{t("editor.saving")}</span>
                </>
              ) : (
                <>
                  <Save size={14} />
                  <span>{t("editor.save_btn")}</span>
                </>
              )}
            </button>
          </div>
        </div>

        {/* Unsaved Changes Confirmation Modal */}
        {showUnsavedPrompt && (
          <div className="unsaved-prompt-backdrop">
            <div className="unsaved-prompt-card">
              <div className="flex items-center gap-2 mb-2 text-amber-400">
                <AlertTriangle size={18} />
                <h4 className="font-semibold">{t("editor.modified_badge")}</h4>
              </div>
              <p className="text-sm text-slate-300 mb-4">{t("editor.unsaved_confirm")}</p>
              <div className="flex justify-end gap-2">
                <button
                  type="button"
                  className="btn-editor-secondary"
                  onClick={() => setShowUnsavedPrompt(false)}
                >
                  {t("common.cancel")}
                </button>
                <button
                  type="button"
                  className="btn-editor-danger"
                  onClick={() => {
                    setShowUnsavedPrompt(false);
                    onClose();
                  }}
                >
                  {t("editor.close_btn")}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Snippet Manager Modal */}
        {showSnippetManager && (
          <SnippetManagerModal
            isOpen={showSnippetManager}
            initialLanguage={selectedSnippetLang}
            onClose={() => setShowSnippetManager(false)}
            snippets={snippets}
            onUpdateSnippets={async (newSnippets) => {
              setSnippets(newSnippets);
              await saveSnippets(newSnippets);
            }}
            onSnippetSelect={(s) => {
              insertSnippet(s.snippet);
              setShowSnippetManager(false);
            }}
          />
        )}
      </div>
    </div>
  );
}

