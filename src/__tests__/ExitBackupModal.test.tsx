import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent, act } from "@testing-library/react";
import { ExitBackupModal } from "../components/ExitBackupModal";
import { changeLanguage } from "../i18n";
import type { BackupResultDto } from "../types/BackupResultDto";

const mockDefaultPath = "/home/susie/.local/share/Cuddle/backups/backup_20261025_120000.tar.gz";

const mockBackupSuccessResult: BackupResultDto = {
  archive_path: mockDefaultPath,
  file_count: 5,
  total_bytes: 1048576n,
  timestamp_ms: 1729828800000n,
};

let defaultPathValue = mockDefaultPath;
let shouldFailBackup = false;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: any) => {
    if (cmd === "get_default_backup_path") {
      return Promise.resolve(defaultPathValue);
    }
    if (cmd === "create_workspace_backup") {
      if (shouldFailBackup) {
        return Promise.reject("Simulated disk full error");
      }
      return Promise.resolve({
        ...mockBackupSuccessResult,
        archive_path: args?.destinationPath ?? mockDefaultPath,
      });
    }
    return Promise.resolve(null);
  }),
}));

describe("ExitBackupModal (Phase 6.3)", () => {
  beforeEach(async () => {
    await act(async () => {
      await changeLanguage("ja");
    });
    defaultPathValue = mockDefaultPath;
    shouldFailBackup = false;
    vi.clearAllMocks();
  });

  it("does not render when isOpen is false", () => {
    const { container } = render(
      <ExitBackupModal
        isOpen={false}
        stagedCount={3}
        onClose={vi.fn()}
        onDiscard={vi.fn()}
        onBackupSuccess={vi.fn()}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders properly with default path when isOpen is true", async () => {
    render(
      <ExitBackupModal
        isOpen={true}
        stagedCount={3}
        onClose={vi.fn()}
        onDiscard={vi.fn()}
        onBackupSuccess={vi.fn()}
      />
    );

    expect(screen.getByText("終了時バックアップ & 安全確認")).toBeDefined();
    expect(screen.getByText(/3 件の作業コピーが存在します/)).toBeDefined();

    await waitFor(() => {
      const input = screen.getByLabelText("保存先アーカイブパス (tar.gz)") as HTMLInputElement;
      expect(input.value).toBe(mockDefaultPath);
    });
  });

  it("allows editing the destination path and resetting to default", async () => {
    render(
      <ExitBackupModal
        isOpen={true}
        stagedCount={3}
        onClose={vi.fn()}
        onDiscard={vi.fn()}
        onBackupSuccess={vi.fn()}
      />
    );

    const input = (await screen.findByLabelText(
      "保存先アーカイブパス (tar.gz)"
    )) as HTMLInputElement;
    expect(input.value).toBe(mockDefaultPath);

    // Edit path
    await act(async () => {
      fireEvent.change(input, { target: { value: "/custom/backup.tar.gz" } });
    });
    expect(input.value).toBe("/custom/backup.tar.gz");

    // Reset button appears
    const resetBtn = screen.getByTitle("デフォルトパスに戻す");
    expect(resetBtn).toBeDefined();

    // Click reset
    await act(async () => {
      fireEvent.click(resetBtn);
    });
    expect(input.value).toBe(mockDefaultPath);
  });

  it("calls onBackupSuccess and onClose when backup succeeds", async () => {
    const onBackupSuccess = vi.fn();
    const onClose = vi.fn();

    render(
      <ExitBackupModal
        isOpen={true}
        stagedCount={5}
        onClose={onClose}
        onDiscard={vi.fn()}
        onBackupSuccess={onBackupSuccess}
      />
    );

    await screen.findByLabelText("保存先アーカイブパス (tar.gz)");

    const backupBtn = screen.getByRole("button", { name: "アーカイブ保存して終了" });
    await act(async () => {
      fireEvent.click(backupBtn);
    });

    await waitFor(() => {
      expect(onBackupSuccess).toHaveBeenCalledTimes(1);
      expect(onBackupSuccess).toHaveBeenCalledWith(
        expect.objectContaining({
          archive_path: mockDefaultPath,
          file_count: 5,
        })
      );
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  it("calls onDiscard when discard button is clicked", async () => {
    const onDiscard = vi.fn();

    render(
      <ExitBackupModal
        isOpen={true}
        stagedCount={2}
        onClose={vi.fn()}
        onDiscard={onDiscard}
        onBackupSuccess={vi.fn()}
      />
    );

    await screen.findByLabelText("保存先アーカイブパス (tar.gz)");

    const discardBtn = screen.getByTitle("作業コピーを破棄して終了");
    await act(async () => {
      fireEvent.click(discardBtn);
    });

    expect(onDiscard).toHaveBeenCalledTimes(1);
  });

  it("calls onClose when cancel button is clicked", async () => {
    const onClose = vi.fn();

    render(
      <ExitBackupModal
        isOpen={true}
        stagedCount={2}
        onClose={onClose}
        onDiscard={vi.fn()}
        onBackupSuccess={vi.fn()}
      />
    );

    await screen.findByLabelText("保存先アーカイブパス (tar.gz)");

    const cancelBtn = screen.getByRole("button", { name: "作業を継続する" });
    await act(async () => {
      fireEvent.click(cancelBtn);
    });

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("displays error message when backup fails", async () => {
    shouldFailBackup = true;

    render(
      <ExitBackupModal
        isOpen={true}
        stagedCount={2}
        onClose={vi.fn()}
        onDiscard={vi.fn()}
        onBackupSuccess={vi.fn()}
      />
    );

    await screen.findByLabelText("保存先アーカイブパス (tar.gz)");

    const backupBtn = screen.getByRole("button", { name: "アーカイブ保存して終了" });
    await act(async () => {
      fireEvent.click(backupBtn);
    });

    await waitFor(() => {
      expect(screen.getByText("Simulated disk full error")).toBeDefined();
    });
  });

  it("renders correctly in English", async () => {
    await act(async () => {
      await changeLanguage("en");
    });

    render(
      <ExitBackupModal
        isOpen={true}
        stagedCount={4}
        onClose={vi.fn()}
        onDiscard={vi.fn()}
        onBackupSuccess={vi.fn()}
      />
    );

    await screen.findByLabelText("Destination Archive Path (tar.gz)");

    expect(screen.getByText("Session Exit Backup & Safety Check")).toBeDefined();
    expect(screen.getByText(/4 staged file\(s\) exist in workspace-temp/)).toBeDefined();
    expect(screen.getByRole("button", { name: "Archive Backup & Exit" })).toBeDefined();
    expect(screen.getByRole("button", { name: "Continue Working" })).toBeDefined();
  });
});
