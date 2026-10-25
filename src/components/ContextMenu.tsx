import React, { useEffect, useRef } from "react";
import { createPortal } from "react-dom";

export interface ContextMenuItem {
  label: string;
  icon?: React.ReactNode;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
}

interface ContextMenuProps {
  x: number;
  y: number;
  isOpen: boolean;
  onClose: () => void;
  items: ContextMenuItem[];
}

export const ContextMenu: React.FC<ContextMenuProps> = ({
  x,
  y,
  isOpen,
  onClose,
  items,
}) => {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  // Window boundary adjustment & smart flip to prevent clipping
  const menuWidth = 220;
  const menuHeight = items.length * 36 + 18;

  // If there's not enough room below, open upward from the cursor
  let adjustedY = y;
  if (y + menuHeight > window.innerHeight - 8) {
    adjustedY = Math.max(8, y - menuHeight);
  } else {
    adjustedY = Math.max(8, y);
  }

  // If there's not enough room on the right, open to the left of the cursor
  let adjustedX = x;
  if (x + menuWidth > window.innerWidth - 8) {
    adjustedX = Math.max(8, x - menuWidth);
  } else {
    adjustedX = Math.max(8, x);
  }

  const menuContent = (
    <div
      ref={menuRef}
      className="custom-context-menu"
      style={{
        left: `${adjustedX}px`,
        top: `${adjustedY}px`,
        zIndex: 99999,
      }}
    >
      {items.map((item, idx) => (
        <button
          key={idx}
          type="button"
          className={`context-menu-item ${item.danger ? "danger" : ""}`}
          disabled={item.disabled}
          onClick={() => {
            if (!item.disabled) {
              item.onClick();
              onClose();
            }
          }}
        >
          {item.icon && <span className="menu-item-icon">{item.icon}</span>}
          <span className="menu-item-label">{item.label}</span>
        </button>
      ))}
    </div>
  );

  return createPortal(menuContent, document.body);
};
