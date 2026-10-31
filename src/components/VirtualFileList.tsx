import { useRef, useState, useMemo } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { FileItem } from "../types/FileItem";
import type { Density } from "../types/Density";
import { DENSITY_CONFIGS } from "../utils/density";
import { formatBytes, formatTimestamp } from "../utils/format";
import { FileIcon } from "../utils/fileIcons";
import {
  Shield,
  CornerDownRight,
  Trash2,
  AlertTriangle,
  Eye,
  Copy,
  FileCode,
  Clock,
  FileEdit,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { ContextMenu, type ContextMenuItem } from "./ContextMenu";

interface VirtualFileListProps {
  items: FileItem[];
  selectedId: string | null;
  onSelect: (item: FileItem) => void;
  onOpenDir?: (item: FileItem) => void;
  onStage?: (item: FileItem) => void;
  onUnstage?: (item: FileItem) => void;
  onOpenPreview?: (item: FileItem) => void;
  onOpenEditor?: (item: FileItem) => void;
  onOpenChmod?: (item: FileItem) => void;
  onOpenTimestamp?: (item: FileItem) => void;
  cloneMethodMap?: Record<string, "Reflink" | "StandardCopy">;
  statusMap?: Record<string, string>;
  emptyMessage?: string;
  isWorkspace?: boolean;
  density?: Density;
}

export function VirtualFileList({
  items,
  selectedId,
  onSelect,
  onOpenDir,
  onStage,
  onUnstage,
  onOpenPreview,
  onOpenEditor,
  onOpenChmod,
  onOpenTimestamp,
  cloneMethodMap,
  statusMap,
  emptyMessage,
  isWorkspace = false,
  density = "normal",
}: VirtualFileListProps) {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);

  const densityConfig = DENSITY_CONFIGS[density] || DENSITY_CONFIGS.normal;
  const rowHeight = densityConfig.rowHeight;

  // Maximum file size in current list for inline size heat-bar (Phase 10 / Task 2-8)
  const maxFileSize = useMemo(() => {
    let max = 0n;
    for (const it of items) {
      if (!it.is_dir && it.size > max) {
        max = it.size;
      }
    }
    return Number(max);
  }, [items]);

  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => rowHeight,
    overscan: 10,
  });

  // Context Menu State
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    item: FileItem;
  } | null>(null);

  const handleContextMenu = (e: React.MouseEvent, item: FileItem) => {
    e.preventDefault();
    e.stopPropagation();
    onSelect(item);
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      item,
    });
  };

  const getContextMenuItems = (item: FileItem): ContextMenuItem[] => {
    const menuItems: ContextMenuItem[] = [];

    if (!item.is_dir && onOpenPreview) {
      menuItems.push({
        label: t("context_menu.open_preview"),
        icon: <Eye size={14} className="text-teal-400" />,
        onClick: () => onOpenPreview(item),
      });
    }

    if (isWorkspace && !item.is_dir && onOpenEditor) {
      menuItems.push({
        label: t("editor.edit_workspace_copy"),
        icon: <FileEdit size={14} className="text-teal-400" />,
        onClick: () => onOpenEditor(item),
      });
    }

    if (!item.is_dir && onStage && !isWorkspace) {
      menuItems.push({
        label: t("context_menu.stage_item"),
        icon: <CornerDownRight size={14} className="text-cyan-400" />,
        onClick: () => onStage(item),
      });
    }

    if (onOpenChmod) {
      menuItems.push({
        label: t("chmod.action_open_modal"),
        icon: <FileCode size={14} className="text-amber-400" />,
        onClick: () => onOpenChmod(item),
      });
    }

    if (onOpenTimestamp) {
      menuItems.push({
        label: t("timestamp.action_open_modal"),
        icon: <Clock size={14} className="text-emerald-400" />,
        onClick: () => onOpenTimestamp(item),
      });
    }

    if (isWorkspace && onUnstage) {
      menuItems.push({
        label: t("context_menu.unstage_item"),
        icon: <Trash2 size={14} className="text-rose-400" />,
        danger: true,
        onClick: () => onUnstage(item),
      });
    }

    menuItems.push({
      label: t("context_menu.copy_path"),
      icon: <Copy size={14} />,
      onClick: () => {
        navigator.clipboard.writeText(item.display_name);
      },
    });

    return menuItems;
  };

  if (items.length === 0) {
    return (
      <div className="virtual-empty-state">
        <p className="empty-text">{emptyMessage || t("browser.empty_folder")}</p>
      </div>
    );
  }

  return (
    <div ref={parentRef} className={`virtual-list-scroll-container density-${density}`}>
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
          const isDotItem = item.display_name.startsWith(".");

          return (
            <div
              key={item.item_id}
              className={`virtual-row-item density-${density} ${isSelected ? "selected" : ""} ${isNonUtf8 ? "has-warning" : ""} ${isDotItem ? "is-dot-item" : ""} ${!isWorkspace && !item.is_dir ? "is-draggable" : ""}`}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
              draggable={!isWorkspace && !item.is_dir}
              onDragStart={(e) => {
                if (!isWorkspace && !item.is_dir) {
                  const serialized = JSON.stringify(item, (_, v) =>
                    typeof v === "bigint" ? v.toString() : v
                  );
                  e.dataTransfer.setData("application/cuddle-file-item", serialized);
                  e.dataTransfer.setData("application/json", serialized);
                  e.dataTransfer.effectAllowed = "copy";
                }
              }}
              onClick={() => onSelect(item)}
              onDoubleClick={() => {
                if (item.is_dir && onOpenDir) {
                  onOpenDir(item);
                } else if (!item.is_dir && onOpenPreview) {
                  onOpenPreview(item);
                }
              }}
              onContextMenu={(e) => handleContextMenu(e, item)}
            >
              <div className="row-col-icon">
                <FileIcon item={item} size={densityConfig.iconSize} />
              </div>

              <div className="row-main-content">
                <div className="row-line-primary">
                  <span className={`file-name-text ${isDotItem ? "is-dot-name" : ""}`} title={item.display_name}>
                    {item.display_name}
                  </span>
                  {isNonUtf8 && <span className="warning-pill-sm">Non-UTF-8</span>}
                  {isWorkspace && statusMap?.[item.item_id] === "Conflict" && (
                    <span
                      className="conflict-pill-sm"
                      title={t("workspace.status_conflict")}
                    >
                      <AlertTriangle size={10} />
                      <span>{t("workspace.status_conflict")}</span>
                    </span>
                  )}
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

                <div className="row-line-secondary">
                  <span className="file-meta-item file-meta-size">
                    {item.is_dir ? (
                      t("browser.dir_type")
                    ) : (
                      <span className="size-with-heat-bar">
                        <span>{formatBytes(Number(item.size))}</span>
                        {maxFileSize > 0 && Number(item.size) > 0 && (
                          <span
                            className="size-heat-bar"
                            title={`Relative size: ${((Number(item.size) / maxFileSize) * 100).toFixed(1)}%`}
                          >
                            <span
                              className="size-heat-bar-fill"
                              style={{
                                width: `${Math.max(4, Math.min(100, Math.round((Number(item.size) / maxFileSize) * 100)))}%`,
                              }}
                            />
                          </span>
                        )}
                      </span>
                    )}
                  </span>
                  <span className="file-meta-separator">•</span>
                  <span className="file-meta-item file-meta-mtime">
                    {formatTimestamp(Number(item.mtime_ms))}
                  </span>
                  <span className="file-meta-separator">•</span>
                  <span className="file-meta-item file-meta-inode" title={`inode: ${item.inode}`}>
                    #{item.inode}
                  </span>
                </div>
              </div>

              <div className="row-col-actions">
                {onOpenPreview && !item.is_dir && (
                  <button
                    type="button"
                    className="row-action-btn preview-action-btn"
                    onClick={(e) => {
                      e.stopPropagation();
                      onOpenPreview(item);
                    }}
                    title={t("context_menu.open_preview")}
                  >
                    <Eye size={density === "compact" ? 11 : density === "spacious" ? 14 : 13} />
                  </button>
                )}

                {isWorkspace && onOpenEditor && !item.is_dir && (
                  <button
                    type="button"
                    className="row-action-btn editor-action-btn"
                    onClick={(e) => {
                      e.stopPropagation();
                      onOpenEditor(item);
                    }}
                    title={t("editor.edit_tooltip")}
                  >
                    <FileEdit size={density === "compact" ? 11 : density === "spacious" ? 14 : 13} className="text-teal-400" />
                  </button>
                )}

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
                    <CornerDownRight size={density === "compact" ? 11 : density === "spacious" ? 14 : 13} />
                  </button>
                )}

                {onUnstage && (
                  <button
                    type="button"
                    className="row-action-btn unstage-btn"
                    onClick={(e) => {
                      e.stopPropagation();
                      onUnstage(item);
                    }}
                    title={t("workspace.unstage_btn")}
                  >
                    <Trash2 size={density === "compact" ? 11 : density === "spacious" ? 14 : 13} />
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Right Click Custom Context Menu */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          isOpen={true}
          onClose={() => setContextMenu(null)}
          items={getContextMenuItems(contextMenu.item)}
        />
      )}
    </div>
  );
}
