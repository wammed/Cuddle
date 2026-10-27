import React, { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  FileText,
  Code2,
  Eye,
  Copy,
  Check,
} from "lucide-react";
import {
  detectLanguage,
  highlightCode,
  renderMarkdown,
  type SupportedLanguage,
} from "../utils/syntaxHighlight";

interface RichPreviewProps {
  filename: string;
  text: string;
  encoding?: string;
  extractedChars?: number;
}

export const RichPreview: React.FC<RichPreviewProps> = ({
  filename,
  text,
  encoding = "UTF-8",
  extractedChars,
}) => {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const lang = useMemo<SupportedLanguage>(() => detectLanguage(filename), [filename]);

  // Mode state: for Markdown/HTML, toggle between 'rendered' and 'source'
  const [viewMode, setViewMode] = useState<"rendered" | "source">(
    lang === "markdown" || lang === "html" ? "rendered" : "source"
  );

  const handleCopy = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const highlightedHtml = useMemo(() => {
    return highlightCode(text, lang);
  }, [text, lang]);

  const renderedMarkdownHtml = useMemo(() => {
    if (lang === "markdown") {
      return renderMarkdown(text);
    }
    return "";
  }, [text, lang]);

  return (
    <div className="rich-preview-container">
      {/* Top Header Bar */}
      <div className="rich-preview-bar">
        <div className="rich-preview-meta">
          <FileText size={14} className="text-teal-400" />
          <span className="preview-filename font-mono">{filename}</span>
          <span className="preview-pill lang-pill">{lang.toUpperCase()}</span>
          {encoding && <span className="preview-pill enc-pill">{encoding}</span>}
          {extractedChars !== undefined && (
            <span className="preview-pill chars-pill">{extractedChars} chars</span>
          )}
        </div>

        <div className="rich-preview-actions">
          {/* View Mode Toggle for Markdown and HTML */}
          {(lang === "markdown" || lang === "html") && (
            <div className="preview-mode-toggle">
              <button
                type="button"
                className={`mode-toggle-btn ${viewMode === "rendered" ? "active" : ""}`}
                onClick={() => setViewMode("rendered")}
                title={lang === "markdown" ? t("rich_preview.markdown_view") : t("rich_preview.html_view")}
              >
                <Eye size={12} />
                <span>{lang === "markdown" ? t("rich_preview.markdown_view") : t("rich_preview.html_view")}</span>
              </button>
              <button
                type="button"
                className={`mode-toggle-btn ${viewMode === "source" ? "active" : ""}`}
                onClick={() => setViewMode("source")}
                title={t("rich_preview.code_view")}
              >
                <Code2 size={12} />
                <span>{t("rich_preview.code_view")}</span>
              </button>
            </div>
          )}

          {/* Copy Button */}
          <button
            type="button"
            className="panel-header-btn-subtle preview-copy-btn"
            onClick={handleCopy}
            title={t("rich_preview.copy_text")}
          >
            {copied ? <Check size={12} /> : <Copy size={12} />}
            <span>{copied ? t("rich_preview.copied") : t("rich_preview.copy")}</span>
          </button>
        </div>
      </div>

      {/* Main Preview Content Body */}
      <div className="rich-preview-body">
        {viewMode === "rendered" && lang === "markdown" ? (
          <div
            className="markdown-rendered-view"
            dangerouslySetInnerHTML={{ __html: renderedMarkdownHtml }}
          />
        ) : viewMode === "rendered" && lang === "html" ? (
          <div className="html-rendered-wrap">
            <iframe
              title="HTML Preview"
              srcDoc={text}
              sandbox="allow-same-origin"
              className="html-preview-frame"
            />
          </div>
        ) : (
          <div
            className="code-highlighted-view"
            dangerouslySetInnerHTML={{ __html: highlightedHtml }}
          />
        )}
      </div>
    </div>
  );
};
