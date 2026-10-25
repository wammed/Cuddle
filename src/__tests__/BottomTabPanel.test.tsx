import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { BottomTabPanel } from "../components/BottomTabPanel";
import { changeLanguage } from "../i18n";
import type { FileItem } from "../types/FileItem";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { FileDiffResultDto } from "../types/FileDiffResultDto";
import type { OperationHistoryDto } from "../types/OperationHistoryDto";
import type { ExtractedTextDto } from "../types/ExtractedTextDto";

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
    if (cmd === "probe_external_tools") {
      return Promise.resolve([
        {
          tool_id: "imagemagick",
          display_name: "ImageMagick",
          is_available: true,
          executable_path: "/usr/bin/magick",
          version: "7.1.1",
          error_message: null,
        },
        {
          tool_id: "ffmpeg",
          display_name: "FFmpeg",
          is_available: false,
          executable_path: null,
          version: null,
          error_message: "not found",
        },
      ]);
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

    // Switch to separated "差分表示" tab
    const diffTab = screen.getByText("差分表示");
    await act(async () => {
      fireEvent.click(diffTab);
    });

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

    // Switch to separated "Diff" tab
    const diffTab = screen.getByText("Diff");
    await act(async () => {
      fireEvent.click(diffTab);
    });

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

    // Switch to "差分表示" tab
    const diffTab = screen.getByText("差分表示");
    await act(async () => {
      fireEvent.click(diffTab);
    });

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

    expect(handleReflect).toHaveBeenCalledWith("stage-1", "BackupOrigin");
  });

  it("supports switching reflection mode to Mode A (Atomic Overwrite) (Design 5.2)", async () => {
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

    const diffTab = screen.getByText("差分表示");
    await act(async () => {
      fireEvent.click(diffTab);
    });

    await waitFor(() => {
      expect(screen.getByText("原本へ変更を反映")).toBeDefined();
    });

    await act(async () => {
      fireEvent.click(screen.getByText("原本へ変更を反映"));
    });

    // Check mode options exist
    expect(screen.getByText(/方式 B: 原本を \.origin として退避保存/)).toBeDefined();
    expect(screen.getByText(/方式 A: 原本を直接上書き/)).toBeDefined();

    // Select Mode A
    await act(async () => {
      fireEvent.click(screen.getByText(/方式 A: 原本を直接上書き/));
    });

    // Confirm reflect
    const confirmBtn = screen.getByText("原本への反映を実行");
    await act(async () => {
      fireEvent.click(confirmBtn);
    });

    expect(handleReflect).toHaveBeenCalledWith("stage-1", "Overwrite");
  });

  it("renders selective undo button on undoable history items and invokes callback (Design 6.2)", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const handleUndo = vi.fn();
    vi.spyOn(window, "confirm").mockReturnValue(true);

    const undoableHistory: OperationHistoryDto[] = [
      {
        id: 1n,
        operation_id: "op-rename-batch-101",
        timestamp_ms: 1729750000000n,
        operation_type: "BulkRename",
        display_name: "10 files renamed",
        original_path: null,
        staged_path: null,
        inode: null,
        status: "Success",
        can_undo: true,
        details: "Batch ID: batch-101",
      },
      {
        id: 2n,
        operation_id: "op-chmod-202",
        timestamp_ms: 1729750005000n,
        operation_type: "chmod",
        display_name: "script.sh",
        original_path: "/home/susie/script.sh",
        staged_path: "/home/susie/script.sh",
        inode: null,
        status: "Success",
        can_undo: true,
        details: "Mode changed: 0644 (-rw-r--r--) -> 0755 (-rwxr-xr-x)",
      },
    ];

    render(
      <BottomTabPanel
        selectedItem={null}
        extractedInfo={null}
        appConfig={null}
        logs={[]}
        history={[]}
        historyItems={undoableHistory}
        onUndoHistoryItem={handleUndo}
        onClose={vi.fn()}
      />
    );

    const historyTab = screen.getByText(/操作履歴/);
    await act(async () => {
      fireEvent.click(historyTab);
    });

    // Verify UNDO buttons are rendered for can_undo == true
    const undoButtons = screen.getAllByText("元に戻す (UNDO)");
    expect(undoButtons.length).toBe(2);

    // Click the first UNDO button
    await act(async () => {
      fireEvent.click(undoButtons[0]);
    });

    expect(window.confirm).toHaveBeenCalled();
    expect(handleUndo).toHaveBeenCalledWith("op-rename-batch-101");
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

    // Switch to "差分表示" tab
    const diffTab = screen.getByText("差分表示");
    await act(async () => {
      fireEvent.click(diffTab);
    });

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

    // Switch to "差分表示" tab
    const diffTab = screen.getByText("差分表示");
    await act(async () => {
      fireEvent.click(diffTab);
    });

    await waitFor(() => {
      expect(screen.getByText("原本へ変更を反映")).toBeDefined();
    });

    // Open modal
    await act(async () => {
      fireEvent.click(screen.getByText("原本へ変更を反映"));
    });

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

  it("displays SQLite WAL operation history timeline with badges and details (Phase 6.1)", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const sampleHistories: OperationHistoryDto[] = [
      {
        id: 1n,
        operation_id: "op-rename-1",
        timestamp_ms: 1729750000000n,
        operation_type: "BulkRename",
        display_name: "photo_001.jpg",
        original_path: "/home/susie/pictures/photo_001.jpg",
        staged_path: "/home/susie/.cuddle-temp/stage-1_photo_001.jpg",
        inode: 12345n,
        status: "Success",
        can_undo: true,
        details: "Prefix: 20261025_",
      },
      {
        id: 2n,
        operation_id: "op-reflect-1",
        timestamp_ms: 1729750005000n,
        operation_type: "Reflect",
        display_name: "doc_v2.md",
        original_path: "/home/susie/docs/doc_v2.md",
        staged_path: "/home/susie/.cuddle-temp/stage-2_doc_v2.md",
        inode: 23456n,
        status: "Success",
        can_undo: false,
        details: "SHA-256 verified match",
      },
    ];

    const handleClear = vi.fn();

    render(
      <BottomTabPanel
        selectedItem={null}
        extractedInfo={null}
        appConfig={null}
        logs={[]}
        history={[]}
        historyItems={sampleHistories}
        onClearHistory={handleClear}
        stagedItems={[]}
        onClose={vi.fn()}
      />
    );

    // Switch to history tab
    const historyTabBtn = screen.getByText("操作履歴");
    await act(async () => {
      fireEvent.click(historyTabBtn);
    });

    // Verify history panel header
    expect(screen.getByText("操作履歴 (SQLite WAL)")).toBeDefined();
    expect(screen.getByText("2 件の記録 (上限 50 件)")).toBeDefined();

    // Verify operation badges
    expect(screen.getByText("一括リネーム")).toBeDefined();
    expect(screen.getByText("原本反映")).toBeDefined();

    // Verify file display names and details
    expect(screen.getByText("photo_001.jpg")).toBeDefined();
    expect(screen.getByText("Prefix: 20261025_")).toBeDefined();
    expect(screen.getByText("doc_v2.md")).toBeDefined();
    expect(screen.getByText("SHA-256 verified match")).toBeDefined();

    // Click clear history button
    const clearBtn = screen.getByText("履歴を消去");
    await act(async () => {
      fireEvent.click(clearBtn);
    });
    expect(handleClear).toHaveBeenCalled();
  });

  it("displays empty history message when historyItems is empty (Phase 6.1)", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    render(
      <BottomTabPanel
        selectedItem={null}
        extractedInfo={null}
        appConfig={null}
        logs={[]}
        history={[]}
        historyItems={[]}
        onClose={vi.fn()}
      />
    );

    // Switch to history tab
    const historyTabBtn = screen.getByText("操作履歴");
    await act(async () => {
      fireEvent.click(historyTabBtn);
    });

    expect(screen.getByText("操作履歴はありません")).toBeDefined();
  });

  it("displays text preview independently from diffs in preview tab", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    const sampleExtracted: ExtractedTextDto = {
      item_id: "item-readme",
      raw_base64: "cmVhZG1lLnR4dA==",
      display_name: "readme.txt",
      extracted_chars: 42,
      tier_used: "Tier1Direct",
      encoding_used: "UTF-8",
      truncated: false,
      warning: null,
      text: "This is a clean preview text independent of diff.",
    };

    render(
      <BottomTabPanel
        selectedItem={sampleItem}
        extractedInfo={sampleExtracted}
        appConfig={null}
        logs={[]}
        history={[]}
        stagedItems={[sampleStaged]}
        onClose={vi.fn()}
      />
    );

    // Initial tab is "preview", which shows the extracted text, not diff
    expect(screen.getByText(/clean preview text independent of diff/)).toBeDefined();
    expect(screen.getByText("readme.txt")).toBeDefined();
    expect(screen.getByText("42 chars")).toBeDefined();
    expect(screen.queryByText("差分プレビュー (原本 vs 作業コピー)")).toBeNull();
  });

  it("renders external tools status in security & tools diagnostic tab (Phase 7.1)", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    render(
      <BottomTabPanel
        selectedItem={null}
        extractedInfo={null}
        appConfig={null}
        logs={[]}
        history={[]}
        onClose={vi.fn()}
      />
    );

    const securityTabBtn = screen.getByText("セキュリティ & ツール診断");
    await act(async () => {
      fireEvent.click(securityTabBtn);
    });

    await waitFor(() => {
      expect(screen.getByText("外部ツール連携環境 (Phase 7.1)")).toBeDefined();
      expect(screen.getByText("ImageMagick")).toBeDefined();
      expect(screen.getByText("FFmpeg")).toBeDefined();
      expect(screen.getByText("利用可能")).toBeDefined();
      expect(screen.getByText("未インストール")).toBeDefined();
    });
  });

  it("renders Git status tab and switches to it", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    render(
      <BottomTabPanel
        selectedItem={null}
        extractedInfo={null}
        appConfig={null}
        logs={[]}
        history={[]}
        onClose={vi.fn()}
      />
    );

    const gitTabBtn = screen.getByText("Git 情報");
    expect(gitTabBtn).toBeDefined();

    await act(async () => {
      fireEvent.click(gitTabBtn);
    });

    // Git tab is active
    expect(gitTabBtn.closest("button")?.className).toContain("active");
  });
});


