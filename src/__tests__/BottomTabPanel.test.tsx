import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { BottomTabPanel } from "../components/BottomTabPanel";
import { changeLanguage } from "../i18n";
import type { FileItem } from "../types/FileItem";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { FileDiffResultDto } from "../types/FileDiffResultDto";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "get_file_diff") {
      if (args?.stageId === "stage-identical") {
        const identicalDiff: FileDiffResultDto = {
          stage_id: "stage-identical",
          is_binary: false,
          has_changes: false,
          additions: 0,
          deletions: 0,
          lines: [
            {
              change_type: "Equal",
              old_line_number: 1,
              new_line_number: 1,
              content: "identical content",
            },
          ],
        };
        return Promise.resolve(identicalDiff);
      }
      const mockDiff: FileDiffResultDto = {
        stage_id: "stage-1",
        is_binary: false,
        has_changes: true,
        additions: 1,
        deletions: 1,
        lines: [
          {
            change_type: "Equal",
            old_line_number: 1,
            new_line_number: 1,
            content: "function hello() {",
          },
          {
            change_type: "Delete",
            old_line_number: 2,
            new_line_number: null,
            content: "  console.log('original');",
          },
          {
            change_type: "Insert",
            old_line_number: null,
            new_line_number: 2,
            content: "  console.log('staged modified');",
          },
          {
            change_type: "Equal",
            old_line_number: 3,
            new_line_number: 3,
            content: "}",
          },
        ],
      };
      return Promise.resolve(mockDiff);
    }
    if (cmd === "reset_staged_file") {
      return Promise.resolve({
        stage_id: "stage-1",
        original_item_id: "item-1",
        raw_base64: "L2hvbWUvc3VzaWUvYXBwLnRz",
        display_name: "app.ts",
        staged_raw_base64: "L2hvbWUvc3VzaWUvLmN1ZGRsZS10ZW1wL3N0YWdlLTFfYXBwLnRz",
        staged_path: "/home/susie/.cuddle-temp/stage-1_app.ts",
        size: 100n,
        clone_method: "Reflink",
        status: "Staged",
        staged_at_ms: 1729750000000n,
        original_snapshot: {
          item_id: "item-1",
          raw_base64: "L2hvbWUvc3VzaWUvYXBwLnRz",
          display_name: "app.ts",
          canonical_path: "/home/susie/app.ts",
          inode: 1234n,
          dev: 66305n,
          size: 100n,
          mtime_ms: 1729750000000n,
          sha256: "hash123",
        },
      });
    }
    return Promise.resolve("");
  }),
}));

describe("BottomTabPanel & Diff Operations (Phase 4.2)", () => {
  const sampleItem: FileItem = {
    item_id: "item-1",
    raw_base64: "L2hvbWUvc3VzaWUvYXBwLnRz",
    display_name: "app.ts",
    parent_raw_base64: null,
    is_dir: false,
    is_symlink: false,
    size: 100n,
    mtime_ms: 1729750000000n,
    readonly: false,
    inode: 1234n,
    dev: 66305n,
  };

  const sampleStaged: StagedItemDto = {
    stage_id: "stage-1",
    original_item_id: "item-1",
    raw_base64: "L2hvbWUvc3VzaWUvYXBwLnRz",
    display_name: "app.ts",
    staged_raw_base64: "L2hvbWUvc3VzaWUvLmN1ZGRsZS10ZW1wL3N0YWdlLTFfYXBwLnRz",
    staged_path: "/home/susie/.cuddle-temp/stage-1_app.ts",
    size: 100n,
    clone_method: "Reflink",
    status: "Modified",
    staged_at_ms: 1729750000000n,
    original_snapshot: {
      item_id: "item-1",
      raw_base64: "L2hvbWUvc3VzaWUvYXBwLnRz",
      display_name: "app.ts",
      canonical_path: "/home/susie/app.ts",
      inode: 1234n,
      dev: 66305n,
      size: 100n,
      mtime_ms: 1729750000000n,
      sha256: "hash123",
    },
  };

  it("fetches diff and displays additions and deletions in Japanese", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const handleReset = vi.fn();

    render(
      <BottomTabPanel
        selectedItem={sampleItem}
        extractedInfo={null}
        appConfig={null}
        logs={["log1"]}
        history={["hist1"]}
        stagedItems={[sampleStaged]}
        onResetStagedItem={handleReset}
        onClose={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("差分プレビュー (原本 vs 作業コピー)")).toBeDefined();
      expect(screen.getByText("+1 行 / -1 行")).toBeDefined();
    });

    // Verify diff lines rendered
    expect(screen.getByText("console.log('original');")).toBeDefined();
    expect(screen.getByText("console.log('staged modified');")).toBeDefined();

    // Click reset to original button
    const resetBtn = screen.getByText("原本状態へ復元 (リセット)");
    await act(async () => {
      fireEvent.click(resetBtn);
    });
    expect(handleReset).toHaveBeenCalledWith("stage-1");
  });

  it("displays diff in English correctly", async () => {
    await act(async () => {
      await changeLanguage("en");
    });

    render(
      <BottomTabPanel
        selectedItem={sampleItem}
        extractedInfo={null}
        appConfig={null}
        logs={[]}
        history={[]}
        stagedItems={[sampleStaged]}
        onClose={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("Diff Preview (Original vs Staging)")).toBeDefined();
      expect(screen.getByText("+1 line(s) / -1 line(s)")).toBeDefined();
    });
  });

  it("opens reflection confirmation dialog and executes reflection (Phase 4.3)", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const handleReflect = vi.fn().mockResolvedValue(undefined);

    render(
      <BottomTabPanel
        selectedItem={sampleItem}
        extractedInfo={null}
        appConfig={null}
        logs={[]}
        history={[]}
        stagedItems={[sampleStaged]}
        onReflectStagedItem={handleReflect}
        onClose={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("原本へ変更を反映")).toBeDefined();
    });

    // Click reflect button in diff header
    const reflectHeaderBtn = screen.getByText("原本へ変更を反映");
    await act(async () => {
      fireEvent.click(reflectHeaderBtn);
    });

    // Confirm dialog is displayed
    expect(screen.getByText("原本反映の安全確認")).toBeDefined();
    expect(screen.getByText("/home/susie/app.ts")).toBeDefined();
    expect(screen.getByText("/home/susie/.cuddle-temp/stage-1_app.ts")).toBeDefined();

    // Click confirm reflect button inside dialog
    const confirmBtn = screen.getByText("原本への反映を実行");
    await act(async () => {
      fireEvent.click(confirmBtn);
    });

    expect(handleReflect).toHaveBeenCalledWith("stage-1");
  });

  it("displays identical content message without reflect button when no changes exist (Test 2)", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const identicalStaged: StagedItemDto = {
      ...sampleStaged,
      stage_id: "stage-identical",
      display_name: "clean.txt",
    };

    render(
      <BottomTabPanel
        selectedItem={null}
        extractedInfo={null}
        appConfig={null}
        logs={[]}
        history={[]}
        stagedItems={[identicalStaged]}
        onClose={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("原本と作業コピーの内容は完全に一致しています (変更なし)")).toBeDefined();
    });

    // Verify reflect button is NOT rendered
    expect(screen.queryByText("原本へ変更を反映")).toBeNull();
  });

  it("displays conflict error banner when reflection fails (Test 6)", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const handleReflectFail = vi
      .fn()
      .mockRejectedValue("Policy error: Conflict detected: original file has been modified externally since staging");

    render(
      <BottomTabPanel
        selectedItem={sampleItem}
        extractedInfo={null}
        appConfig={null}
        logs={[]}
        history={[]}
        stagedItems={[sampleStaged]}
        onReflectStagedItem={handleReflectFail}
        onClose={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("原本へ変更を反映")).toBeDefined();
    });

    // Open modal
    fireEvent.click(screen.getByText("原本へ変更を反映"));

    // Click confirm reflect
    const confirmBtn = screen.getByText("原本への反映を実行");
    await act(async () => {
      fireEvent.click(confirmBtn);
    });

    // Error banner should be rendered in modal
    await waitFor(() => {
      expect(screen.getByText("原本への反映に失敗しました (原本保護ガード作動)")).toBeDefined();
      expect(screen.getByText(/Conflict detected/)).toBeDefined();
    });

    // Close button should now be available
    expect(screen.getByText("閉じる")).toBeDefined();
  });
});


