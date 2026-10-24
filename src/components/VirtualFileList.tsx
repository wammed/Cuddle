import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { FileItem } from "../types/FileItem";
import { formatBytes, formatTimestamp } from "../utils/format";
import { Folder, FileText, Binary, Shield, CornerDownRight, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

interface VirtualFileListProps {
  items: FileItem[];
  selectedId: string | null;
  onSelect: (item: FileItem) => void;
  onOpenDir?: (item: FileItem) => void;
  onStage?: (item: FileItem) => void;
  onUnstage?: (item: FileItem) => void;
  cloneMethodMap?: Record<string, "Reflink" | "StandardCopy">;
  emptyMessage?: string;
  isWorkspace?: boolean;
}

const ROW_HEIGHT = 42; // Standard information density (42px)

export function VirtualFileList({
  items,
  selectedId,
  onSelect,
  onOpenDir,
  onStage,
  onUnstage,
  cloneMethodMap,
  emptyMessage,
  isWorkspace = false,
}: VirtualFileListProps) {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
  });

  if (items.length === 0) {
    return (
      <div className="virtual-empty-state">
        <p className="empty-text">{emptyMessage || t("browser.empty_folder")}</p>
      </div>
    );
  }

  return (
    <div ref={parentRef} className="virtual-list-scroll-container">
      <div
        className="virtual-list-track"
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const item = items[virtualRow.index];
          const isSelected = item.item_id === selectedId;
          const isNonUtf8 = item.display_name.includes("\uFFFD");

          return (
            <div
              key={item.item_id}
              className={`virtual-row-item ${isSelected ? "selected" : ""} ${isNonUtf8 ? "has-warning" : ""}`}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
              onClick={() => onSelect(item)}
              onDoubleClick={() => {
                if (item.is_dir && onOpenDir) {
                  onOpenDir(item);
                }
              }}
            >
              <div className="row-col-icon">
                {item.is_dir ? (
                  <Folder size={16} className="item-icon-folder" />
                ) : isNonUtf8 ? (
                  <span title="Non-UTF-8 path">
                    <Binary size={16} className="item-icon-non-utf8" />
                  </span>
                ) : (
                  <FileText size={16} className="item-icon-file" />
                )}
              </div>

              <div className="row-col-name" title={item.display_name}>
                <span className="file-name-text">{item.display_name}</span>
                {isNonUtf8 && <span className="warning-pill-sm">Non-UTF-8</span>}
                {isWorkspace && (
                  <span
                    className={`reflink-pill-sm ${cloneMethodMap?.[item.item_id] === "Reflink" ? "is-reflink" : ""}`}
                    title={
                      cloneMethodMap?.[item.item_id] === "Reflink"
                        ? t("workspace.clone_method_reflink")
                        : t("workspace.clone_method_standard")
                    }
                  >
                    <Shield size={10} />
                    <span>{cloneMethodMap?.[item.item_id] === "Reflink" ? "Reflink" : "Copy"}</span>
                  </span>
                )}
              </div>

              <div className="row-col-size">
                {item.is_dir ? "-" : formatBytes(Number(item.size))}
              </div>

              <div className="row-col-mtime">
                {formatTimestamp(Number(item.mtime_ms))}
              </div>

              <div className="row-col-inode" title={`inode: ${item.inode}`}>
                #{item.inode}
              </div>

              <div className="row-col-actions">
                {onStage && !item.is_dir && (
                  <button
                    type="button"
                    className="row-action-btn"
                    onClick={(e) => {
                      e.stopPropagation();
                      onStage(item);
                    }}
                    title={t("browser.stage_to_workspace")}
                  >
                    <CornerDownRight size={12} />
                  </button>
                )}
                {isWorkspace && onUnstage && (
                  <button
                    type="button"
                    className="row-action-btn danger"
                    onClick={(e) => {
                      e.stopPropagation();
                      onUnstage(item);
                    }}
                    title={t("workspace.unstage_btn")}
                  >
                    <Trash2 size={12} />
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
