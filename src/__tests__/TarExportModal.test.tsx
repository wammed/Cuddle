import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { TarExportModal } from "../components/TarExportModal";
import { changeLanguage } from "../i18n";
import type { FileItem } from "../types/FileItem";
import type { TarExportResultDto } from "../types/TarExportResultDto";

const mockExportResult: TarExportResultDto = {
  success: true,
  archive_path: "/tmp/cuddle_workspace_test.tar.gz",
  file_count: 3,
  total_uncompressed_bytes: 1048576n,
  archive_bytes: 524288n,
  error_message: null,
};

let currentExportResult: TarExportResultDto = mockExportResult;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "export_workspace_tar") {
      return Promise.resolve(currentExportResult);
    }
    return Promise.resolve({});
  }),
}));

const mockFiles: FileItem[] = [
  {
    item_id: "item-1",
    raw_base64: "L2hvbWUvdXNlci9maWxlMS50eHQ=",
    display_name: "file1.txt",
    parent_raw_base64: null,
    is_dir: false,
    is_symlink: false,
    size: 1024n,
    mtime_ms: 1700000000000n,
    readonly: false,
    inode: 101n,
    dev: 1n,
  },
  {
    item_id: "item-2",
    raw_base64: "L2hvbWUvdXNlci9maWxlMi50eHQ=",
    display_name: "file2.txt",
    parent_raw_base64: null,
    is_dir: false,
    is_symlink: false,
    size: 2048n,
    mtime_ms: 1700000000000n,
    readonly: false,
    inode: 102n,
    dev: 1n,
  },
];

describe("TarExportModal Component", () => {
  beforeEach(async () => {
    currentExportResult = mockExportResult;
    vi.clearAllMocks();
    await changeLanguage("ja");
  });

  it("renders nothing when isOpen is false", () => {
    const { container } = render(
      <TarExportModal
        isOpen={false}
        onClose={vi.fn()}
        stagedItems={mockFiles}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders modal header, scopes, and formats when open", () => {
    render(
      <TarExportModal
        isOpen={true}
        onClose={vi.fn()}
        stagedItems={mockFiles}
        selectedIds={["item-1"]}
      />
    );

    expect(screen.getByText("ワークスペースのアーカイブ (tar エクスポート)")).toBeDefined();
    expect(screen.getByText("エクスポート対象")).toBeDefined();
    expect(screen.getByText("変更されたファイルのみ (推奨)")).toBeDefined();
    expect(screen.getByText("ステージング全ファイル")).toBeDefined();
    expect(screen.getByText("gzip 圧縮 tar (.tar.gz)")).toBeDefined();
    expect(screen.getByText("非圧縮 POSIX tar (.tar)")).toBeDefined();
  });

  it("adjusts destination path extension when format toggles", () => {
    render(
      <TarExportModal
        isOpen={true}
        onClose={vi.fn()}
        stagedItems={mockFiles}
      />
    );

    const destInput = screen.getByPlaceholderText("例: /home/user/archive.tar.gz") as HTMLInputElement;
    expect(destInput.value.endsWith(".tar.gz")).toBe(true);

    // Switch to uncompressed tar
    fireEvent.click(screen.getByText("非圧縮 POSIX tar (.tar)"));
    expect(destInput.value.endsWith(".tar")).toBe(true);

    // Switch back to tar.gz
    fireEvent.click(screen.getByText("gzip 圧縮 tar (.tar.gz)"));
    expect(destInput.value.endsWith(".tar.gz")).toBe(true);
  });

  it("calls export_workspace_tar on submit and displays success banner", async () => {
    const handleSuccess = vi.fn();
    render(
      <TarExportModal
        isOpen={true}
        onClose={vi.fn()}
        stagedItems={mockFiles}
        onSuccess={handleSuccess}
      />
    );

    const exportBtn = screen.getByText("アーカイブを出力").closest("button") as HTMLButtonElement;
    expect(exportBtn.disabled).toBe(false);

    fireEvent.click(exportBtn);

    await waitFor(() => {
      expect(handleSuccess).toHaveBeenCalledTimes(1);
      expect(screen.getByText(/エクスポート完了/)).toBeDefined();
    });
  });
});
