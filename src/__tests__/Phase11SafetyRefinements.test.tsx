import { describe, it, expect, vi, beforeEach } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { CriticalSecurityModal } from "../components/CriticalSecurityModal";
import { ExitBackupModal } from "../components/ExitBackupModal";
import { BottomTabPanel } from "../components/BottomTabPanel";
import { changeLanguage } from "../i18n";
import type { FileItem } from "../types/FileItem";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { BackupArchiveInfoDto } from "../types/BackupArchiveInfoDto";

const mockStagedItem: StagedItemDto = {
  stage_id: "stage-p11-1",
  original_item_id: "orig-p11-1",
  raw_base64: "L2hvbWUvc3VzaWUvZG9jLnBkZg==",
  display_name: "doc.pdf",
  staged_raw_base64: "L3RtcC9kb2MucGRm",
  staged_path: "/tmp/doc.pdf",
  size: 1024n,
  clone_method: "Reflink",
  status: "Conflict",
  staged_at_ms: 1729800000000n,
  original_snapshot: {
    item_id: "orig-p11-1",
    raw_base64: "L2hvbWUvc3VzaWUvZG9jLnBkZg==",
    display_name: "doc.pdf",
    canonical_path: "/home/susie/doc.pdf",
    inode: 12345n,
    dev: 66305n,
    size: 1024n,
    mtime_ms: 1729800000000n,
    sha256: "sha256_mock_hash",
  },
};

const mockFileItem: FileItem = {
  item_id: "orig-p11-1",
  raw_base64: "L2hvbWUvc3VzaWUvZG9jLnBkZg==",
  display_name: "doc.pdf",
  parent_raw_base64: null,
  is_dir: false,
  is_symlink: false,
  size: 1024n,
  mtime_ms: 1729800000000n,
  readonly: false,
  inode: 12345n,
  dev: 66305n,
};

const mockBackupsList: BackupArchiveInfoDto[] = [
  {
    filename: "backup_20261028_120000.tar.gz",
    full_path: "/home/susie/.local/share/Cuddle/backups/backup_20261028_120000.tar.gz",
    created_at_ms: 1729828800000n,
    size_bytes: 204800n,
    file_count: 3,
  },
  {
    filename: "backup_20261027_100000.tar.gz",
    full_path: "/home/susie/.local/share/Cuddle/backups/backup_20261027_100000.tar.gz",
    created_at_ms: 1729728800000n,
    size_bytes: 102400n,
    file_count: 2,
  },
];

let deleteBackupCalledWith: string | null = null;
let createBackupCalledWith: any = null;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: any) => {
    if (cmd === "get_landlock_status") {
      return Promise.resolve({ supported: false, abi_version: null });
    }
    if (cmd === "get_default_backup_path") {
      return Promise.resolve("/home/susie/.local/share/Cuddle/backups/backup_20261029_090000.tar.gz");
    }
    if (cmd === "get_app_config") {
      return Promise.resolve({
        app: { language: "ja", theme: "dark" },
        workspace: { temp_directory: null, min_free_space_multiplier: 1.5, backup_retention_count: 5 },
        security: { enforce_landlock: true },
        custom_pipelines: [],
      });
    }
    if (cmd === "list_workspace_backups") {
      return Promise.resolve(mockBackupsList);
    }
    if (cmd === "delete_workspace_backup") {
      deleteBackupCalledWith = args?.filename;
      return Promise.resolve();
    }
    if (cmd === "create_workspace_backup") {
      createBackupCalledWith = args;
      return Promise.resolve({
        archive_path: "/home/susie/.local/share/Cuddle/backups/backup_20261029_090000.tar.gz",
        file_count: 1,
        total_bytes: 102400n,
        timestamp_ms: 1729828800000n,
      });
    }
    if (cmd === "get_file_diff") {
      return Promise.resolve({
        stage_id: "stage-p11-1",
        is_binary: false,
        has_changes: true,
        additions: 3,
        deletions: 1,
        lines: [
          { old_line_no: 1, new_line_no: 1, change_type: "Equal", content: "Common line" },
          { old_line_no: 2, new_line_no: null, change_type: "Delete", content: "Old line" },
          { old_line_no: null, new_line_no: 2, change_type: "Insert", content: "New line" },
        ],
      });
    }
    if (cmd === "get_origin_backup_preview") {
      return Promise.resolve("/home/susie/doc_20261029_090000.origin.pdf");
    }
    if (cmd === "get_alternate_copy_preview") {
      const suffix = args?.suffix || ".edit";
      return Promise.resolve(`/home/susie/doc${suffix}.pdf`);
    }
    if (cmd === "reflect_staged_file_as_copy") {
      return Promise.resolve({
        staged_item: { ...mockStagedItem, status: "Reflected" },
        copy_path: "/home/susie/doc.edit.pdf",
      });
    }
    if (cmd === "reflect_staged_file") {
      return Promise.reject("Conflict detected: original file has been modified externally since staging");
    }
    return Promise.resolve(null);
  }),
}));

describe("Phase 11: Safety, Conflict Resolution & Recovery Refinements", () => {
  beforeEach(async () => {
    await act(async () => {
      await changeLanguage("ja");
    });
    deleteBackupCalledWith = null;
    createBackupCalledWith = null;
    vi.clearAllMocks();
  });

  describe("Task 4-3: CriticalSecurityModal (Landlock LSM Warning & Consent)", () => {
    it("does not render when isOpen is false", () => {
      const { container } = render(
        <CriticalSecurityModal isOpen={false} onConsent={vi.fn()} onExit={vi.fn()} />
      );
      expect(container.firstChild).toBeNull();
    });

    it("renders warning title and security explanations when isOpen is true", () => {
      render(<CriticalSecurityModal isOpen={true} onConsent={vi.fn()} onExit={vi.fn()} />);

      expect(screen.getByText("重大セキュリティ警告: Landlock LSM が利用できません")).toBeInTheDocument();
      expect(screen.getByText(/現在の Linux カーネルは Landlock LSM/)).toBeInTheDocument();
      expect(screen.getByText("セキュリティ上の注意事項")).toBeInTheDocument();
      expect(screen.getByText(/外部ツールやカスタムパイプラインを実行する際/)).toBeInTheDocument();
      expect(screen.getByText(/Cuddle 内部処理/)).toBeInTheDocument();
    });

    it("triggers onExit when clicking Exit Application button", () => {
      const onExit = vi.fn();
      render(<CriticalSecurityModal isOpen={true} onConsent={vi.fn()} onExit={onExit} />);

      const exitBtn = screen.getByText("アプリケーションを終了");
      fireEvent.click(exitBtn);

      expect(onExit).toHaveBeenCalledTimes(1);
    });

    it("triggers onConsent with rememberChoice false by default", () => {
      const onConsent = vi.fn();
      render(<CriticalSecurityModal isOpen={true} onConsent={onConsent} onExit={vi.fn()} />);

      const proceedBtn = screen.getByText("リスクを理解して操作をアンロック");
      fireEvent.click(proceedBtn);

      expect(onConsent).toHaveBeenCalledWith(false);
    });

    it("triggers onConsent with rememberChoice true when checkbox is checked", () => {
      const onConsent = vi.fn();
      render(<CriticalSecurityModal isOpen={true} onConsent={onConsent} onExit={vi.fn()} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const proceedBtn = screen.getByText("リスクを理解して操作をアンロック");
      fireEvent.click(proceedBtn);

      expect(onConsent).toHaveBeenCalledWith(true);
    });
  });

  describe("Task 3-3: Backup Retention Management & Pruning in ExitBackupModal", () => {
    it("displays retention limit and allows toggling existing archive list", async () => {
      render(
        <ExitBackupModal
          isOpen={true}
          stagedCount={1}
          onClose={vi.fn()}
          onDiscard={vi.fn()}
          onBackupSuccess={vi.fn()}
        />
      );

      await waitFor(() => {
        expect(screen.getByText(/バックアップ保持世代数/)).toBeInTheDocument();
      });

      // Toggle archive list
      const toggleBtn = screen.getByText(/保存済みバックアップ一覧/);
      fireEvent.click(toggleBtn);

      await waitFor(() => {
        expect(screen.getByText("backup_20261028_120000.tar.gz")).toBeInTheDocument();
        expect(screen.getByText("backup_20261027_100000.tar.gz")).toBeInTheDocument();
      });
    });

    it("deletes a backup archive when delete button is confirmed", async () => {
      vi.spyOn(window, "confirm").mockReturnValue(true);

      render(
        <ExitBackupModal
          isOpen={true}
          stagedCount={1}
          onClose={vi.fn()}
          onDiscard={vi.fn()}
          onBackupSuccess={vi.fn()}
        />
      );

      const toggleBtn = await screen.findByText(/保存済みバックアップ一覧/);
      fireEvent.click(toggleBtn);

      const deleteButtons = await screen.findAllByTitle("削除");
      expect(deleteButtons.length).toBeGreaterThan(0);

      fireEvent.click(deleteButtons[0]);

      expect(window.confirm).toHaveBeenCalled();
      expect(deleteBackupCalledWith).toBe("backup_20261028_120000.tar.gz");
    });

    it("submits autoPruneLimit when executing workspace backup", async () => {
      const onBackupSuccess = vi.fn();
      render(
        <ExitBackupModal
          isOpen={true}
          stagedCount={1}
          onClose={vi.fn()}
          onDiscard={vi.fn()}
          onBackupSuccess={onBackupSuccess}
        />
      );

      const backupBtn = await screen.findByText("アーカイブ保存して終了");
      fireEvent.click(backupBtn);

      await waitFor(() => {
        expect(createBackupCalledWith).toEqual({
          destinationPath: "/home/susie/.local/share/Cuddle/backups/backup_20261029_090000.tar.gz",
          autoPruneLimit: 5,
        });
        expect(onBackupSuccess).toHaveBeenCalled();
      });
    });
  });

  describe("Task 3-2: Alternate Filename UI on Original Conflict in BottomTabPanel", () => {
    it("renders alternate copy UI and button when reflection encounters conflict", async () => {
      const onReflectCopy = vi.fn();
      render(
        <BottomTabPanel
          selectedItem={mockFileItem}
          extractedInfo={null}
          appConfig={null}
          logs={[]}
          history={[]}
          stagedItems={[mockStagedItem]}
          activeTab="diff"
          onClose={vi.fn()}
          onReflectStagedItemAsCopy={onReflectCopy}
        />
      );

      // 1. Open reflection modal
      const openReflectBtn = await screen.findByText("原本へ変更を反映");
      fireEvent.click(openReflectBtn);

      // 2. Click confirm reflection to trigger conflict error
      const confirmBtn = await screen.findByText("原本への反映を実行");
      fireEvent.click(confirmBtn);

      // 3. Conflict detected banner should appear with alternate copy options
      await waitFor(() => {
        expect(screen.getByText("原本競合時の別名保存")).toBeInTheDocument();
        expect(screen.getAllByText(".edit を付加して別名保存").length).toBeGreaterThan(0);
      });

      // 4. Click Save as Alternate Copy button
      const saveCopyBtns = screen.getAllByText(".edit を付加して別名保存");
      fireEvent.click(saveCopyBtns[0]);

      await waitFor(() => {
        expect(onReflectCopy).toHaveBeenCalledWith("stage-p11-1", ".edit");
      });
    });
  });
});
