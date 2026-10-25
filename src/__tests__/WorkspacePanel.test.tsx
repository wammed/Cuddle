import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { WorkspacePanel } from "../components/WorkspacePanel";
import { changeLanguage } from "../i18n";
import type { FileItem } from "../types/FileItem";
import type { StagedItemDto } from "../types/StagedItemDto";

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

describe("WorkspacePanel & Staging Operations (Phase 4.1)", () => {
  const sampleStagedFiles: FileItem[] = [
    {
      item_id: "orig-1",
      raw_base64: "L2hvbWUvc3VzaWUvZG9jMS50eHQ=",
      display_name: "doc1.txt",
      parent_raw_base64: null,
      is_dir: false,
      is_symlink: false,
      size: 4096n,
      mtime_ms: 1729750000000n,
      readonly: false,
      inode: 11111n,
      dev: 66305n,
    },
    {
      item_id: "orig-2",
      raw_base64: "L2hvbWUvc3VzaWUvZG9jMi50eHQ=",
      display_name: "doc2.txt",
      parent_raw_base64: null,
      is_dir: false,
      is_symlink: false,
      size: 8192n,
      mtime_ms: 1729750000000n,
      readonly: false,
      inode: 22222n,
      dev: 66305n,
    },
  ];

  const sampleStagedItems: StagedItemDto[] = [
    {
      stage_id: "stage-1",
      original_item_id: "orig-1",
      raw_base64: "L2hvbWUvc3VzaWUvZG9jMS50eHQ=",
      display_name: "doc1.txt",
      staged_raw_base64: "L2hvbWUvc3VzaWUvLmN1ZGRsZS10ZW1wL3N0YWdlLTFfZG9jMS50eHQ=",
      staged_path: "/home/susie/.cuddle-temp/stage-1_doc1.txt",
      size: 4096n,
      clone_method: "Reflink",
      status: "Staged",
      staged_at_ms: 1729750100000n,
      original_snapshot: {
        item_id: "orig-1",
        raw_base64: "L2hvbWUvc3VzaWUvZG9jMS50eHQ=",
        display_name: "doc1.txt",
        canonical_path: "/home/susie/doc1.txt",
        inode: 11111n,
        dev: 66305n,
        size: 4096n,
        mtime_ms: 1729750000000n,
        sha256: "abc1",
      },
    },
    {
      stage_id: "stage-2",
      original_item_id: "orig-2",
      raw_base64: "L2hvbWUvc3VzaWUvZG9jMi50eHQ=",
      display_name: "doc2.txt",
      staged_raw_base64: "L2NhY2hlL0N1ZGRsZS9zdGFnaW5nL3N0YWdlLTJfZG9jMi50eHQ=",
      staged_path: "/cache/Cuddle/staging/stage-2_doc2.txt",
      size: 8192n,
      clone_method: "StandardCopy",
      status: "Staged",
      staged_at_ms: 1729750100000n,
      original_snapshot: {
        item_id: "orig-2",
        raw_base64: "L2hvbWUvc3VzaWUvZG9jMi50eHQ=",
        display_name: "doc2.txt",
        canonical_path: "/home/susie/doc2.txt",
        inode: 22222n,
        dev: 66305n,
        size: 8192n,
        mtime_ms: 1729750000000n,
        sha256: "abc2",
      },
    },
  ];

  it("renders staged items with clone method badges in Japanese", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const handleSelect = vi.fn();
    const handleClear = vi.fn();
    const handleUnstage = vi.fn();

    render(
      <WorkspacePanel
        stagedFiles={sampleStagedFiles}
        stagedItems={sampleStagedItems}
        selectedId={null}
        onSelect={handleSelect}
        onClearWorkspace={handleClear}
        onUnstageItem={handleUnstage}
      />
    );

    expect(screen.getByText("doc1.txt")).toBeDefined();
    expect(screen.getByText("doc2.txt")).toBeDefined();
    expect(screen.getByText("2 個の作業コピー")).toBeDefined();

    // Check Reflink badge
    expect(screen.getByText("Reflink")).toBeDefined();
    expect(screen.getByText("Copy")).toBeDefined();

    // Click clear all
    const clearBtn = screen.getByText("すべて解除");
    fireEvent.click(clearBtn);
    expect(handleClear).toHaveBeenCalledTimes(1);
  });

  it("renders empty state in English when no files are staged", async () => {
    await act(async () => {
      await changeLanguage("en");
    });

    render(
      <WorkspacePanel
        stagedFiles={[]}
        stagedItems={[]}
        selectedId={null}
        onSelect={vi.fn()}
        onClearWorkspace={vi.fn()}
      />
    );

    expect(screen.getByText("No files staged in workspace-temp")).toBeDefined();
    expect(screen.getByText("0 staged copy(ies)")).toBeDefined();
  });

  it("renders undo button when canUndo is true and invokes callback on click", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const handleUndo = vi.fn();

    const { rerender } = render(
      <WorkspacePanel
        stagedFiles={sampleStagedFiles}
        stagedItems={sampleStagedItems}
        selectedId={null}
        canUndo={false}
        onSelect={vi.fn()}
        onClearWorkspace={vi.fn()}
        onUndo={handleUndo}
      />
    );

    expect(screen.queryByText("直前のリネームを元に戻す")).toBeNull();

    rerender(
      <WorkspacePanel
        stagedFiles={sampleStagedFiles}
        stagedItems={sampleStagedItems}
        selectedId={null}
        canUndo={true}
        onSelect={vi.fn()}
        onClearWorkspace={vi.fn()}
        onUndo={handleUndo}
      />
    );

    const undoBtn = screen.getByText("直前のリネームを元に戻す");
    expect(undoBtn).toBeDefined();

    fireEvent.click(undoBtn);
    expect(handleUndo).toHaveBeenCalledTimes(1);
  });
});

