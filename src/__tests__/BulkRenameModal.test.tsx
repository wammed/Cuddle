import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { BulkRenameModal } from "../components/BulkRenameModal";
import { changeLanguage } from "../i18n";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { BatchPlanDto } from "../types/BatchPlanDto";

const mockBatchPlanSuccess: BatchPlanDto = {
  batch_id: "batch-12345",
  operations: [
    {
      item_id: "stage-1",
      original_path: "/tmp/sample.txt",
      display_name: "sample.txt",
      raw_base64: "c2FtcGxlLnR4dA==",
      proposed_name: "SAMPLE.txt",
      target_path: "/tmp/SAMPLE.txt",
      status: "Ok",
      message: null,
    },
  ],
  total_count: 1,
  changed_count: 1,
  conflict_count: 0,
  error_count: 0,
  is_executable: true,
};

const mockBatchPlanConflict: BatchPlanDto = {
  batch_id: "batch-conflict",
  operations: [
    {
      item_id: "stage-1",
      original_path: "/tmp/a.txt",
      display_name: "a.txt",
      raw_base64: "YS50eHQ=",
      proposed_name: "duplicate.txt",
      target_path: "/tmp/duplicate.txt",
      status: "ConflictDuplicate",
      message: "Multiple items would be renamed to duplicate.txt",
    },
    {
      item_id: "stage-2",
      original_path: "/tmp/b.txt",
      display_name: "b.txt",
      raw_base64: "Yi50eHQ=",
      proposed_name: "duplicate.txt",
      target_path: "/tmp/duplicate.txt",
      status: "ConflictDuplicate",
      message: "Multiple items would be renamed to duplicate.txt",
    },
  ],
  total_count: 2,
  changed_count: 0,
  conflict_count: 2,
  error_count: 0,
  is_executable: false,
};

let currentPlan = mockBatchPlanSuccess;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "generate_bulk_rename_plan") {
      return Promise.resolve(currentPlan);
    }
    if (cmd === "execute_bulk_rename") {
      return Promise.resolve({
        batch_id: "batch-12345",
        success: true,
        total_count: 1,
        succeeded_count: 1,
        failed_count: 0,
        is_rolled_back: false,
        error_message: null,
        affected_items: currentPlan.operations,
      });
    }
    return Promise.resolve(null);
  }),
}));

describe("BulkRenameModal (Phase 5.1 & 5.2)", () => {
  const mockStagedItems: StagedItemDto[] = [
    {
      stage_id: "stage-1",
      original_item_id: "item-1",
      raw_base64: "c2FtcGxlLnR4dA==",
      staged_raw_base64: "c3RhZ2Vk",
      display_name: "sample.txt",
      staged_path: "/tmp/sample.txt",
      size: 100n,
      staged_at_ms: 1729750000000n,
      clone_method: "Reflink",
      status: "Staged",
      original_snapshot: {
        item_id: "item-1",
        raw_base64: "c2FtcGxlLnR4dA==",
        display_name: "sample.txt",
        canonical_path: "/home/susie/sample.txt",
        size: 100n,
        mtime_ms: 1729750000000n,
        sha256: "hash1",
        inode: 12345n,
        dev: 66305n,
      },
    },
  ];

  beforeEach(async () => {
    await changeLanguage("ja");
    currentPlan = mockBatchPlanSuccess;
  });

  it("renders modal header, summary bar, and dry-run table when open", async () => {
    render(
      <BulkRenameModal
        isOpen={true}
        onClose={vi.fn()}
        items={mockStagedItems}
      />
    );

    expect(screen.getByText("一括リネーム & 正規化 (Dry-run 検証)")).toBeDefined();
    expect(screen.getByText("Dry-run プレビュー検証")).toBeDefined();

    await waitFor(() => {
      expect(screen.getByText("SAMPLE.txt")).toBeDefined();
      expect(screen.getByText("正常 (変更あり)")).toBeDefined();
    });

    // Confirm button is enabled when plan is executable
    const confirmBtn = screen.getByRole("button", { name: /段階リネームを実行/i });
    expect((confirmBtn as HTMLButtonElement).disabled).toBe(false);
  });

  it("disables confirm button and displays conflict warning when collisions occur", async () => {
    currentPlan = mockBatchPlanConflict;

    render(
      <BulkRenameModal
        isOpen={true}
        onClose={vi.fn()}
        items={mockStagedItems}
      />
    );

    await waitFor(() => {
      expect(screen.getByText(/重複衝突または既存ファイルが存在するため/i)).toBeDefined();
      expect(screen.getAllByText("重複衝突 (同名)").length).toBeGreaterThan(0);
    });

    const confirmBtn = screen.getByRole("button", { name: /段階リネームを実行/i });
    expect((confirmBtn as HTMLButtonElement).disabled).toBe(true);
  });

  it("successfully invokes execute_bulk_rename and calls onExecuteSuccess", async () => {
    const handleSuccess = vi.fn();
    const handleClose = vi.fn();

    render(
      <BulkRenameModal
        isOpen={true}
        onClose={handleClose}
        items={mockStagedItems}
        onExecuteSuccess={handleSuccess}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("SAMPLE.txt")).toBeDefined();
    });

    const confirmBtn = screen.getByRole("button", { name: /段階リネームを実行/i });
    confirmBtn.click();

    await waitFor(() => {
      expect(handleSuccess).toHaveBeenCalledTimes(1);
      expect(handleClose).toHaveBeenCalledTimes(1);
    });
  });

  it("supports bilingual switching to English", async () => {
    await changeLanguage("en");

    render(
      <BulkRenameModal
        isOpen={true}
        onClose={vi.fn()}
        items={mockStagedItems}
      />
    );

    expect(screen.getByText("Bulk Rename & Normalize (Dry-run Verification)")).toBeDefined();
    expect(screen.getByText("Dry-run Preview Verification")).toBeDefined();
    expect(screen.getByRole("button", { name: /Execute Two-Phase Rename/i })).toBeDefined();
  });
});
