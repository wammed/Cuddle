import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { FolderTreePanel } from "../components/FolderTreePanel";
import { changeLanguage } from "../i18n";
import type { FileItem } from "../types/FileItem";

describe("FolderTreePanel (Directory Tree & Hidden Switch)", () => {
  const quickDirs = [
    { name: "ホーム (~)", path: "/home/susie", icon: "home" as const },
    { name: "ルート (/)", path: "/", icon: "root" as const },
  ];

  const subdirs: FileItem[] = [
    {
      item_id: "dir-norm",
      raw_base64: "L2hvbWUvc3VzaWUvZG9jdW1lbnRz",
      display_name: "documents",
      parent_raw_base64: null,
      is_dir: true,
      is_symlink: false,
      size: 4096n,
      mtime_ms: 1729750000000n,
      readonly: false,
      inode: 101n,
      dev: 66305n,
    },
    {
      item_id: "dir-dot",
      raw_base64: "L2hvbWUvc3VzaWUvLmNvbmZpZw==",
      display_name: ".config",
      parent_raw_base64: null,
      is_dir: true,
      is_symlink: false,
      size: 4096n,
      mtime_ms: 1729750000000n,
      readonly: false,
      inode: 102n,
      dev: 66305n,
    },
  ];

  it("renders folder tree and supports dot directory display and hidden toggle", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const handleNavigate = vi.fn();
    const handleRefresh = vi.fn();
    const handleToggle = vi.fn();

    const { rerender, container } = render(
      <FolderTreePanel
        currentPath="/home/susie"
        onNavigatePath={handleNavigate}
        quickDirs={quickDirs}
        subdirs={subdirs}
        isScanning={false}
        onRefresh={handleRefresh}
        showHidden={true}
        onToggleShowHidden={handleToggle}
      />
    );

    // Verify header title
    expect(screen.getByText("ディレクトリツリー")).toBeDefined();

    // Verify toggle button exists and is active
    const toggleBtn = container.querySelector(".toggle-hidden-btn");
    expect(toggleBtn).not.toBeNull();
    expect(toggleBtn?.classList.contains("active")).toBe(true);

    // Click toggle button
    if (toggleBtn) {
      fireEvent.click(toggleBtn);
      expect(handleToggle).toHaveBeenCalledTimes(1);
    }

    // Verify subdirs rendered including dot directory with is-dot-dir class
    expect(screen.getByText("documents")).toBeDefined();
    expect(screen.getByText(".config")).toBeDefined();
    const dotDirElem = container.querySelector(".is-dot-dir");
    expect(dotDirElem).not.toBeNull();
    expect(dotDirElem?.textContent).toContain(".config");

    // Click on .config dir
    if (dotDirElem) {
      fireEvent.click(dotDirElem);
      expect(handleNavigate).toHaveBeenCalledWith(subdirs[1].raw_base64);
    }

    // Rerender with showHidden = false
    rerender(
      <FolderTreePanel
        currentPath="/home/susie"
        onNavigatePath={handleNavigate}
        quickDirs={quickDirs}
        subdirs={[subdirs[0]]}
        isScanning={false}
        onRefresh={handleRefresh}
        showHidden={false}
        onToggleShowHidden={handleToggle}
      />
    );

    const inactiveToggleBtn = container.querySelector(".toggle-hidden-btn");
    expect(inactiveToggleBtn?.classList.contains("active")).toBe(false);
  });
});

