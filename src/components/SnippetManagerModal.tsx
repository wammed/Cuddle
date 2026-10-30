import { useTranslation } from "react-i18next";
import { Terminal, X } from "lucide-react";
import type { EditorSnippetDto } from "../types/EditorSnippetDto";
import { SnippetManagerView } from "./SnippetManagerView";
import { SnippetLanguageKey } from "../utils/snippets";

export interface SnippetManagerModalProps {
  isOpen: boolean;
  initialLanguage?: SnippetLanguageKey;
  onClose: () => void;
  snippets: EditorSnippetDto[];
  onUpdateSnippets: (newSnippets: EditorSnippetDto[]) => Promise<void> | void;
  onSnippetSelect?: (snippet: EditorSnippetDto) => void;
}

export function SnippetManagerModal({
  isOpen,
  initialLanguage,
  onClose,
  snippets,
  onUpdateSnippets,
  onSnippetSelect,
}: SnippetManagerModalProps) {
  const { t } = useTranslation();

  if (!isOpen) return null;

  return (
    <div className="modal-backdrop snippet-manager-backdrop" role="dialog" aria-modal="true">
      <div className="snippet-manager-dialog">
        <div className="dialog-header snippet-manager-header">
          <div className="snippet-manager-title-group">
            <div className="quick-editor-icon-badge">
              <Terminal size={18} className="text-teal-400" />
            </div>
            <div>
              <h3 className="snippet-manager-title">
                {t("snippets.modal_title") || "クイックスニペット管理・カスタマイズ"}
              </h3>
              <p className="snippet-manager-subtitle">
                {t("snippets.modal_subtitle") ||
                  "言語別の挿入スニペットを追加・編集・削除できます。config.toml に自動同期されます。"}
              </p>
            </div>
          </div>
          <button
            type="button"
            className="panel-header-btn"
            onClick={onClose}
            title={t("common.close") || "閉じる"}
          >
            <X size={16} />
          </button>
        </div>

        <div className="snippet-manager-modal-body">
          <SnippetManagerView
            snippets={snippets}
            onUpdateSnippets={onUpdateSnippets}
            initialLanguage={initialLanguage}
            onSnippetSelect={onSnippetSelect}
          />
        </div>
      </div>
    </div>
  );
}

