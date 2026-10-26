import { useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { FileItem } from "../types/FileItem";
import type { Density } from "../types/Density";
import { DENSITY_CONFIGS } from "../utils/density";
import { formatBytes, formatTimestamp } from "../utils/format";
import {
  Folder,
  FileText,
  Binary,
  Shield,
  CornerDownRight,
  Trash2,
  AlertTriangle,
  Eye,
  Copy,
  FileCode,
  Clock,
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

          return (
            <div
              key={item.item_id}
              className={`virtual-row-item density-${density} ${isSelected ? "selected" : ""} ${isNonUtf8 ? "has-warning" : ""}`}
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
                } else if (!item.is_dir && onOpenPreview) {
                  onOpenPreview(item);
                }
              }}
              onContextMenu={(e) => handleContextMenu(e, item)}
            >
              <div className="row-col-icon">
                {item.is_dir ? (
                  <Folder size={densityConfig.iconSize} className="item-icon-folder" />
                ) : isNonUtf8 ? (
                  <span title="Non-UTF-8 path">
                    <Binary size={densityConfig.iconSize} className="item-icon-non-utf8" />
                  </span>
                ) : (
                  <FileText size={densityConfig.iconSize} className="item-icon-file" />
                )}
              </div>

              <div className="row-main-content">
                <div className="row-line-primary">
                  <span className="file-name-text" title={item.display_name}>
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
                    {item.is_dir ? t("browser.dir_type") : formatBytes(Number(item.size))}
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
