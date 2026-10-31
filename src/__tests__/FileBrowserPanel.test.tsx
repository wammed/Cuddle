import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { FileBrowserPanel } from "../components/FileBrowserPanel";
import { changeLanguage } from "../i18n";
import type { FileItem } from "../types/FileItem";

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: ({ count, estimateSize }: { count: number; estimateSize: () => number }) => ({
    getTotalSize: () => count * estimateSize(),
    getVirtualItems: () =>
      Array.from({ length: count }, (_, index) => ({
        index,
        start: index * estimateSize(),
        size: estimateSize(),
      })),
  }),
}));

describe("FileBrowserPanel (2-Row Header & Navigation)", () => {
  const sampleItems: FileItem[] = [
    {
      item_id: "item-1",
      raw_base64: "L2hvbWUvc3VzaWUvZG9jMS50eHQ=",
      display_name: "doc1.txt",
      parent_raw_base64: null,
      is_dir: false,
      is_symlink: false,
      size: 1024n,
      mtime_ms: 1729750000000n,
      readonly: false,
      inode: 12345n,
      dev: 66305n,
    },
    {
      item_id: "dir-1",
      raw_base64: "L2hvbWUvc3VzaWUvc3ViZGly",
      display_name: "subdir",
      parent_raw_base64: null,
      is_dir: true,
      is_symlink: false,
      size: 4096n,
      mtime_ms: 1729750000000n,
      readonly: false,
      inode: 12346n,
      dev: 66305n,
    },
  ];

  it("renders 2-row layout with separate top title/action row and bottom path toolbar", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const handleStage = vi.fn();
    const handleStageAll = vi.fn();
    const handleRefresh = vi.fn();
    const handleNavigateParent = vi.fn();

    const { container } = render(
      <FileBrowserPanel
        currentPath="/home/susie/documents"
        items={sampleItems}
        selectedItem={sampleItems[0]}
        onSelect={vi.fn()}
        onOpenDir={vi.fn()}
        onStage={handleStage}
        onStageAll={handleStageAll}
        onNavigateParent={handleNavigateParent}
        hasParent={true}
        isScanning={false}
        onRefresh={handleRefresh}
      />
    );

    // Verify 2-row layout structure
    expect(container.querySelector(".browser-header-top-row")).not.toBeNull();
    expect(container.querySelector(".browser-path-toolbar-row")).not.toBeNull();

    // Verify current path is visible
    expect(screen.getByText("/home/susie/documents")).toBeDefined();

    // Verify action buttons
    const stageSelectedBtn = screen.getByText("選択を作業領域へ");
    expect(stageSelectedBtn).toBeDefined();
    fireEvent.click(stageSelectedBtn);
    expect(handleStage).toHaveBeenCalledWith(sampleItems[0]);

    const stageAllBtn = screen.getByText("すべて作業領域へ");
    expect(stageAllBtn).toBeDefined();
    fireEvent.click(stageAllBtn);
    expect(handleStageAll).toHaveBeenCalledTimes(1);

    // Verify parent navigation button
    const navParentBtn = container.querySelector(".browser-nav-btn");
    expect(navParentBtn).not.toBeNull();
    if (navParentBtn) {
      fireEvent.click(navParentBtn);
      expect(handleNavigateParent).toHaveBeenCalledTimes(1);
    }
  });

  it("allows switching into inline path editing mode", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const handleNavigatePath = vi.fn();

    const { container } = render(
      <FileBrowserPanel
        currentPath="/home/susie/projects"
        items={sampleItems}
        selectedItem={null}
        onSelect={vi.fn()}
        onOpenDir={vi.fn()}
        onStage={vi.fn()}
        onStageAll={vi.fn()}
        onNavigatePath={handleNavigatePath}
        hasParent={false}
        isScanning={false}
        onRefresh={vi.fn()}
      />
    );

    const pathBadge = screen.getByText("/home/susie/projects");
    fireEvent.click(pathBadge);

    const input = container.querySelector(".browser-path-input") as HTMLInputElement;
    expect(input).not.toBeNull();
    expect(input.value).toBe("/home/susie/projects");

    fireEvent.change(input, { target: { value: "/home/susie/photos" } });
    const form = container.querySelector(".browser-path-form") as HTMLFormElement;
    fireEvent.submit(form);

    expect(handleNavigatePath).toHaveBeenCalledWith("/home/susie/photos");
  });

  it("renders dot file / directory toggle switch and responds to click", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const dotItems: FileItem[] = [
      {
        item_id: "dot-file-1",
        raw_base64: "L2hvbWUvc3VzaWUvLmJhc2hyYw==",
        display_name: ".bashrc",
        parent_raw_base64: null,
        is_dir: false,
        is_symlink: false,
        size: 512n,
        mtime_ms: 1729750000000n,
        readonly: false,
        inode: 9991n,
        dev: 66305n,
      },
      {
        item_id: "dot-dir-1",
        raw_base64: "L2hvbWUvc3VzaWUvLmNvbmZpZw==",
        display_name: ".config",
        parent_raw_base64: null,
        is_dir: true,
        is_symlink: false,
        size: 4096n,
        mtime_ms: 1729750000000n,
        readonly: false,
        inode: 9992n,
        dev: 66305n,
      },
    ];

    const handleToggleHidden = vi.fn();

    const { rerender, container } = render(
      <FileBrowserPanel
        currentPath="/home/susie"
        items={dotItems}
        selectedItem={null}
        onSelect={vi.fn()}
        onOpenDir={vi.fn()}
        onStage={vi.fn()}
        onStageAll={vi.fn()}
        showHidden={true}
        onToggleShowHidden={handleToggleHidden}
        isScanning={false}
        onRefresh={vi.fn()}
      />
    );

    // Verify toggle button exists and shows active state
    const toggleBtn = container.querySelector(".toggle-hidden-btn");
    expect(toggleBtn).not.toBeNull();
    expect(toggleBtn?.classList.contains("active")).toBe(true);
    expect(screen.getByText("隠し項目: 表示中")).toBeDefined();

    // Verify dot files & directories are displayed in the list with dot styling
    expect(screen.getByText(".bashrc")).toBeDefined();
    expect(screen.getByText(".config")).toBeDefined();
    expect(container.querySelector(".is-dot-name")).not.toBeNull();
    expect(container.querySelector(".is-dot-item")).not.toBeNull();

    // Click toggle button
    if (toggleBtn) {
      fireEvent.click(toggleBtn);
      expect(handleToggleHidden).toHaveBeenCalledTimes(1);
    }

    // Rerender with showHidden = false
    rerender(
      <FileBrowserPanel
        currentPath="/home/susie"
        items={[]}
        selectedItem={null}
        onSelect={vi.fn()}
        onOpenDir={vi.fn()}
        onStage={vi.fn()}
        onStageAll={vi.fn()}
        showHidden={false}
        onToggleShowHidden={handleToggleHidden}
        isScanning={false}
        onRefresh={vi.fn()}
      />
    );

    const inactiveToggleBtn = container.querySelector(".toggle-hidden-btn");
    expect(inactiveToggleBtn?.classList.contains("active")).toBe(false);
    expect(screen.getByText("隠し項目")).toBeDefined();
  });
});
