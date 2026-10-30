import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QuickEditorModal } from "../components/QuickEditorModal";
import { changeLanguage } from "../i18n";
import type { StagedItemDto } from "../types/StagedItemDto";

const mockStagedItem: StagedItemDto = {
  stage_id: "test-stage-1234",
  original_item_id: "orig-1234",
  raw_base64: "LmJhc2hyYw==",
  display_name: ".bashrc",
  staged_raw_base64: "c3RhZ2VkLWJhc2hyYw==",
  staged_path: "/home/susie/.local/share/Cuddle/workspace-temp/test-stage-1234",
  size: 120n,
  clone_method: "Reflink",
  status: "Staged",
  staged_at_ms: 1700000000000n,
  original_snapshot: {
    item_id: "orig-1234",
    raw_base64: "LmJhc2hyYw==",
    display_name: ".bashrc",
    canonical_path: "/home/susie/.bashrc",
    inode: 9999n,
    dev: 1n,
    size: 120n,
    mtime_ms: 1700000000000n,
    sha256: "hash123",
  },
};

let currentTextContent = "# ~/.bashrc initialization\nexport PATH=\"$HOME/bin:$PATH\"\nalias ll='ls -la'\n";
let lastSavedContent = "";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args: any) => {
    if (cmd === "read_staged_file_text") {
      if (args.stageId === "test-stage-1234") {
        return Promise.resolve(currentTextContent);
      }
      return Promise.reject("File not found");
    }
    if (cmd === "write_staged_file_text") {
      lastSavedContent = args.content;
      return Promise.resolve();
    }
    return Promise.resolve();
  }),
}));

describe("QuickEditorModal Component", () => {
  beforeEach(async () => {
    await changeLanguage("ja");
    currentTextContent = "# ~/.bashrc initialization\nexport PATH=\"$HOME/bin:$PATH\"\nalias ll='ls -la'\n";
    lastSavedContent = "";
    vi.clearAllMocks();
  });

  it("renders modal header, paths, and loads text content into editor", async () => {
    const handleClose = vi.fn();
    const handleSaveSuccess = vi.fn();

    render(
      <QuickEditorModal
        isOpen={true}
        stagedItem={mockStagedItem}
        onClose={handleClose}
        onSaveSuccess={handleSaveSuccess}
      />
    );

    // Check title and file name
    expect(screen.getByText("簡易設定エディタ")).toBeDefined();
    expect(screen.getByText(".bashrc")).toBeDefined();
    expect(screen.getByText(/shell/i)).toBeDefined();

    // Check paths
    expect(screen.getByText("/home/susie/.bashrc")).toBeDefined();
    expect(screen.getByText("/home/susie/.local/share/Cuddle/workspace-temp/test-stage-1234")).toBeDefined();

    // Wait for text loading
    await waitFor(() => {
      const textarea = document.querySelector(".quick-editor-textarea") as HTMLTextAreaElement;
      expect(textarea).toBeDefined();
      expect(textarea.value).toContain("# ~/.bashrc initialization");
      expect(textarea.value).toContain("alias ll='ls -la'");
    });

    // Check line numbers
    const lineNumbers = document.querySelectorAll(".gutter-line-number");
    expect(lineNumbers.length).toBeGreaterThanOrEqual(3);
  });

  it("inserts preset snippets correctly into textarea", async () => {
    render(
      <QuickEditorModal
        isOpen={true}
        stagedItem={mockStagedItem}
        onClose={vi.fn()}
        onSaveSuccess={vi.fn()}
      />
    );

    await waitFor(() => {
      const textarea = document.querySelector(".quick-editor-textarea") as HTMLTextAreaElement;
      expect(textarea).toBeDefined();
      expect(textarea.value).toContain(".bashrc");
    });

    const textarea = document.querySelector(".quick-editor-textarea") as HTMLTextAreaElement;

    // Click snippet button: Add PATH
    const pathBtn = screen.getByText("+ PATH 追加");
    fireEvent.click(pathBtn);

    expect(textarea.value).toContain('export PATH="$HOME/.local/bin:$PATH"');

    // Click snippet button: Add alias
    const aliasBtn = screen.getByText("+ alias 定義");
    fireEvent.click(aliasBtn);

    expect(textarea.value).toContain("alias ll='ls -la'");
  });

  it("saves modified content and triggers onSaveSuccess callback", async () => {
    const handleSaveSuccess = vi.fn();

    render(
      <QuickEditorModal
        isOpen={true}
        stagedItem={mockStagedItem}
        onClose={vi.fn()}
        onSaveSuccess={handleSaveSuccess}
      />
    );

    await waitFor(() => {
      const textarea = document.querySelector(".quick-editor-textarea") as HTMLTextAreaElement;
      expect(textarea).toBeDefined();
    });

    const textarea = document.querySelector(".quick-editor-textarea") as HTMLTextAreaElement;

    // Type new text
    fireEvent.change(textarea, {
      target: { value: textarea.value + "\nexport EDITOR=cuddle\n" },
    });

    // Save button should now be enabled
    const saveBtn = screen.getByRole("button", { name: /作業コピーを保存/i });
    expect((saveBtn as HTMLButtonElement).disabled).toBe(false);

    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(lastSavedContent).toContain("export EDITOR=cuddle");
      expect(handleSaveSuccess).toHaveBeenCalledWith("test-stage-1234");
    });
  });

  it("warns about unsaved changes when trying to close", async () => {
    const handleClose = vi.fn();

    render(
      <QuickEditorModal
        isOpen={true}
        stagedItem={mockStagedItem}
        onClose={handleClose}
        onSaveSuccess={vi.fn()}
      />
    );

    await waitFor(() => {
      const textarea = document.querySelector(".quick-editor-textarea") as HTMLTextAreaElement;
      expect(textarea).toBeDefined();
    });

    const textarea = document.querySelector(".quick-editor-textarea") as HTMLTextAreaElement;

    // Modify content
    fireEvent.change(textarea, {
      target: { value: "modified without saving" },
    });

    // Click close button in header
    const closeHeaderBtn = screen.getByTitle("閉じる");
    fireEvent.click(closeHeaderBtn);

    // Confirmation dialog should appear
    expect(screen.getByText("未保存の編集内容があります。破棄して閉じますか？")).toBeDefined();
    expect(handleClose).not.toHaveBeenCalled();

    // Confirm discarding changes
    const confirmCloseBtns = screen.getAllByRole("button", { name: "閉じる" });
    // The danger button inside dialog
    fireEvent.click(confirmCloseBtns[confirmCloseBtns.length - 1]);

    expect(handleClose).toHaveBeenCalledTimes(1);
  });
});
