import { describe, it, expect, vi, beforeEach } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { notifyUser } from "../utils/notifications";
import { SettingsModal } from "../components/SettingsModal";
import { PipelineModal } from "../components/PipelineModal";
import { changeLanguage } from "../i18n";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { AppConfig } from "../types/AppConfig";

// Mock @tauri-apps/api/core
const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Mock @tauri-apps/api/event
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe("Phase 12: Pipeline Flexibility & Desktop Notifications", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await changeLanguage("ja");
  });

  describe("Task 2-3: Desktop Notification Abstraction (notifyUser)", () => {
    it("successfully dispatches notification via backend IPC", async () => {
      mockInvoke.mockResolvedValueOnce(true);

      const success = await notifyUser("Cuddle Pipeline", "Batch processing completed", "success");
      expect(success).toBe(true);
      expect(mockInvoke).toHaveBeenCalledWith("notify_user", {
        title: "Cuddle Pipeline",
        body: "Batch processing completed",
        level: "success",
      });
    });

    it("gracefully catches errors without throwing when backend fails", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("D-Bus daemon unavailable"));

      const success = await notifyUser("Cuddle Pipeline", "Failure fallback", "error");
      expect(success).toBe(false);
    });
  });

  describe("Task 4-1 & Task 4-2: Custom Pipeline Definition Form in SettingsModal", () => {
    const mockConfig: AppConfig = {
      app: { language: "ja", theme: "dark", density: "normal" },
      workspace: { temp_directory: null, min_free_space_multiplier: 1.5, backup_retention_count: 5 },
      security: { enforce_landlock: true, confirmation_level: "standard" },
      custom_pipelines: [
        {
          id: "custom-batch-tar",
          name: "Tar Gz Packager",
          description: "Archive multiple files",
          executable: "tar",
          args: ["-czf", "{WorkspaceDirectory}/archive.tar.gz", "{InputList}"],
          supported_extensions: ["*"],
          output_extension: "tar.gz",
          default_suffix: "",
          allowed_read_paths: ["/usr/share/doc", "/opt/data"],
          input_mode: "batch_list",
        },
      ],
      snippets: [],
    };

    it("renders custom pipeline card with Mode badge and Landlock Read paths", async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === "get_app_config") return Promise.resolve(mockConfig);
        if (cmd === "list_workspace_backups") return Promise.resolve([]);
        return Promise.resolve(null);
      });

      render(
        <SettingsModal
          isOpen={true}
          onClose={() => {}}
          currentLang="ja"
          onToggleLang={() => {}}
          landlockSupported={true}
          landlockAbi="V5"
        />
      );

      // Switch to custom pipelines tab
      const pipeTab = await screen.findByText("カスタムパイプライン");
      fireEvent.click(pipeTab);

      // Verify custom pipeline card rendered
      expect(await screen.findByText("Tar Gz Packager")).toBeInTheDocument();

      // Verify Mode badge rendered
      expect(screen.getByText("batch_list")).toBeInTheDocument();

      // Verify Landlock Read paths rendered
      expect(screen.getByText("/usr/share/doc, /opt/data")).toBeInTheDocument();
    });

    it("supports selecting input_mode and appending placeholders in the form", async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === "get_app_config") return Promise.resolve(mockConfig);
        if (cmd === "list_workspace_backups") return Promise.resolve([]);
        if (cmd === "save_app_config") return Promise.resolve();
        return Promise.resolve(null);
      });

      render(
        <SettingsModal
          isOpen={true}
          onClose={() => {}}
          currentLang="ja"
          onToggleLang={() => {}}
          landlockSupported={true}
          landlockAbi="V5"
        />
      );

      const pipeTab = await screen.findByText("カスタムパイプライン");
      fireEvent.click(pipeTab);

      const addBtn = await screen.findByText("+ 新規パイプラインを追加");
      fireEvent.click(addBtn);

      // Verify Input Mode select exists
      const modeSelect = screen.getByDisplayValue("ファイル個別実行 (per_file: {input}, {output})") as HTMLSelectElement;
      expect(modeSelect).toBeInTheDocument();

      // Change input mode to standard_input
      fireEvent.change(modeSelect, { target: { value: "standard_input" } });
      expect(modeSelect.value).toBe("standard_input");

      // Verify placeholder tags exist and append to args
      const inputListBtn = screen.getByText("+ {InputList}");
      const argsInput = screen.getByPlaceholderText("-q 80 {input} -o {output} or {InputList}") as HTMLInputElement;
      expect(argsInput).toBeInTheDocument();

      fireEvent.click(inputListBtn);
      expect(argsInput.value).toContain("{InputList}");

      // Verify allowed_read_paths input
      const allowedReadsInput = screen.getByPlaceholderText("/usr/share/fonts, /opt/data, /etc/magic") as HTMLInputElement;
      fireEvent.change(allowedReadsInput, { target: { value: "/etc/magic, /usr/share/data" } });
      expect(allowedReadsInput.value).toBe("/etc/magic, /usr/share/data");
    });
  });

  describe("PipelineModal: Dry Run Preview with Input Mode & Allowed Read Paths", () => {
    const stagedFiles: StagedItemDto[] = [
      {
        stage_id: "stage-1",
        original_item_id: "orig-1",
        raw_base64: "aGVsbG8=",
        display_name: "sample.txt",
        staged_raw_base64: "aGVsbG8=",
        staged_path: "/tmp/cuddle_ws/stage-1_sample.txt",
        size: 1024n,
        clone_method: "Reflink",
        status: "Staged",
        staged_at_ms: 1729800000000n,
        original_snapshot: {
          item_id: "orig-1",
          raw_base64: "aGVsbG8=",
          display_name: "sample.txt",
          canonical_path: "/tmp/orig/sample.txt",
          inode: 100n,
          dev: 200n,
          size: 1024n,
          mtime_ms: 1729800000000n,
          sha256: "sha256-test",
        },
      },
    ];

    it("displays Input Mode and Landlock Read in Step 1 Risk Specification box", async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === "get_pipeline_presets") return Promise.resolve([]);
        if (cmd === "probe_external_tools") return Promise.resolve([]);
        if (cmd === "get_app_config") {
          return Promise.resolve({
            app: { language: "ja", theme: "dark" },
            workspace: { temp_directory: null, min_free_space_multiplier: 1.5, backup_retention_count: 5 },
            security: { enforce_landlock: true },
            custom_pipelines: [
              {
                id: "custom-stdin-proc",
                name: "Stdin Processor",
                description: "Process via stdin list",
                executable: "cat",
                args: [],
                supported_extensions: ["txt"],
                output_extension: null,
                default_suffix: "",
                allowed_read_paths: ["/usr/share/dict"],
                input_mode: "standard_input",
              },
            ],
          });
        }
        return Promise.resolve(null);
      });

      render(
        <PipelineModal
          stagedFiles={stagedFiles}
          onClose={() => {}}
          onExecuteSuccess={() => {}}
        />
      );

      // Verify Input Mode: standard_input is displayed in Risk Spec box
      expect(await screen.findByText("standard_input")).toBeInTheDocument();

      // Verify Landlock Read path is displayed
      expect(screen.getByText("/usr/share/dict")).toBeInTheDocument();
    });
  });
});
