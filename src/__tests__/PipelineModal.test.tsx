import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { PipelineModal } from "../components/PipelineModal";
import { changeLanguage } from "../i18n";
import type { PipelinePresetDto } from "../types/PipelinePresetDto";
import type { ExternalToolStatusDto } from "../types/ExternalToolStatusDto";
import type { StagedItemDto } from "../types/StagedItemDto";

const mockPresets: PipelinePresetDto[] = [
  {
    id: "text_mask_secrets",
    category: "Text",
    name_key: "pipeline.presets.text_mask_secrets_name",
    desc_key: "pipeline.presets.text_mask_secrets_desc",
    required_tools: [],
    is_builtin: true,
    default_suffix: "_masked.log",
    supported_extensions: ["log", "txt"],
    parameters: [],
  },
  {
    id: "image_web_light",
    category: "Image",
    name_key: "pipeline.presets.img_web_light_name",
    desc_key: "pipeline.presets.img_web_light_desc",
    required_tools: ["imagemagick"],
    is_builtin: false,
    default_suffix: "_web.webp",
    supported_extensions: ["png", "jpg"],
    parameters: [
      {
        key: "quality",
        label_key: "pipeline.params.quality",
        param_type: "number",
        default_value: "80",
        min: 1,
        max: 100,
        step: 1,
        options: null,
      },
    ],
  },
];

const mockTools: ExternalToolStatusDto[] = [
  {
    tool_id: "imagemagick",
    display_name: "ImageMagick",
    is_available: true,
    version: "7.1.1",
    executable_path: "/usr/bin/magick",
  },
  {
    tool_id: "ffmpeg",
    display_name: "FFmpeg",
    is_available: false,
    version: null,
    executable_path: null,
  },
];

const mockStagedFiles: StagedItemDto[] = [
  {
    stage_id: "stage-1",
    original_item_id: "item-1",
    raw_base64: "L2FwcC5sb2c=",
    display_name: "app.log",
    staged_raw_base64: "L3RlbXAvYXBwLmxvZw==",
    staged_path: "/tmp/app.log",
    size: 2048n,
    clone_method: "Reflink",
    status: "Staged",
    staged_at_ms: 1729800000000n,
    original_snapshot: {
      item_id: "item-1",
      raw_base64: "L2FwcC5sb2c=",
      display_name: "app.log",
      canonical_path: "/home/user/app.log",
      inode: 12345n,
      dev: 66305n,
      size: 2048n,
      mtime_ms: 1729800000000n,
      sha256: "hash123",
    },
  },
];

let progressListenerCallback: ((event: { payload: unknown }) => void) | null = null;

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, cb: (e: { payload: unknown }) => void) => {
    if (event === "pipeline-progress") {
      progressListenerCallback = cb;
    }
    return Promise.resolve(() => {
      progressListenerCallback = null;
    });
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "get_pipeline_presets") {
      return Promise.resolve(mockPresets);
    }
    if (cmd === "probe_external_tools") {
      return Promise.resolve(mockTools);
    }
    if (cmd === "cancel_pipeline") {
      return Promise.resolve(true);
    }
    if (cmd === "execute_pipeline_preset") {
      return Promise.resolve({
        preset_id: "text_mask_secrets",
        total_count: 1,
        success_count: 1,
        failure_count: 0,
        is_cancelled: false,
        item_results: [
          {
            stage_id: "stage-1",
            original_name: "app.log",
            output_name: "app_masked.log",
            success: true,
            error_message: null,
            execution_time_ms: 25,
          },
        ],
      });
    }
    return Promise.resolve("");
  }),
}));

describe("PipelineModal (Phase 7.2)", () => {
  beforeEach(async () => {
    await changeLanguage("ja");
  });

  it("renders preset list, categories, and target staged files", async () => {
    render(
      <PipelineModal
        stagedFiles={mockStagedFiles}
        onClose={vi.fn()}
        onExecuteSuccess={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("パイプラインワークフロー実行 (Phase 7.2)")).toBeDefined();
      expect(screen.getAllByText("公開用ログの機密情報マスキング").length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText("Web配信用軽量化")).toBeDefined();
      expect(screen.getByText("対象ステージングファイル: 1 件")).toBeDefined();
    });
  });

  it("filters presets by category pills", async () => {
    render(
      <PipelineModal
        stagedFiles={mockStagedFiles}
        onClose={vi.fn()}
        onExecuteSuccess={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getAllByText("公開用ログの機密情報マスキング").length).toBeGreaterThanOrEqual(1);
    });

    // Click Image category
    const imgCatBtn = screen.getByText(/画像・ビジュアル/);
    await act(async () => {
      fireEvent.click(imgCatBtn);
    });

    await waitFor(() => {
      expect(screen.getAllByText("Web配信用軽量化").length).toBeGreaterThanOrEqual(1);
      expect(screen.queryAllByText("公開用ログの機密情報マスキング")).toHaveLength(0);
    });
  });

  it("executes pipeline preset successfully and triggers callback", async () => {
    const onExecuteSuccess = vi.fn();
    render(
      <PipelineModal
        stagedFiles={mockStagedFiles}
        onClose={vi.fn()}
        onExecuteSuccess={onExecuteSuccess}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("パイプラインを実行")).toBeDefined();
    });

    const runBtn = screen.getByText("パイプラインを実行");
    await act(async () => {
      fireEvent.click(runBtn);
    });

    await waitFor(() => {
      expect(screen.getByText(/パイプライン処理完了: 1 件成功/)).toBeDefined();
      expect(onExecuteSuccess).toHaveBeenCalledTimes(1);
    });
  });

  it("supports bilingual switching to English", async () => {
    await act(async () => {
      await changeLanguage("en");
    });

    render(
      <PipelineModal
        stagedFiles={mockStagedFiles}
        onClose={vi.fn()}
        onExecuteSuccess={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("Pipeline Workflow Runner (Phase 7.2)")).toBeDefined();
      expect(screen.getByText("Execute Pipeline")).toBeDefined();
      expect(screen.getAllByText("Log Secrets Masking").length).toBeGreaterThanOrEqual(1);
    });
  });

  it("handles cancellation request during pipeline execution (Phase 7.3)", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    let resolveExecute: (val: unknown) => void = () => {};
    const executePromise = new Promise((resolve) => {
      resolveExecute = resolve;
    });

    // Mock execute_pipeline_preset to remain pending until cancelled
    vi.mocked(invoke).mockImplementationOnce((cmd: string) => {
      if (cmd === "get_pipeline_presets") return Promise.resolve(mockPresets);
      return Promise.resolve("");
    });
    vi.mocked(invoke).mockImplementationOnce((cmd: string) => {
      if (cmd === "probe_external_tools") return Promise.resolve(mockTools);
      return Promise.resolve("");
    });
    let capturedJobId = "";
    vi.mocked(invoke).mockImplementation((cmd: string, args?: any) => {
      if (cmd === "execute_pipeline_preset") {
        capturedJobId = args?.request?.job_id ?? "";
        return executePromise as any;
      }
      if (cmd === "cancel_pipeline") return Promise.resolve(true);
      return Promise.resolve("");
    });

    render(
      <PipelineModal
        stagedFiles={mockStagedFiles}
        onClose={vi.fn()}
        onExecuteSuccess={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("パイプラインを実行")).toBeDefined();
    });

    const runBtn = screen.getByText("パイプラインを実行");
    await act(async () => {
      fireEvent.click(runBtn);
    });

    // Simulate backend emitting progress event
    if (progressListenerCallback) {
      await act(async () => {
        progressListenerCallback!({
          payload: {
            job_id: capturedJobId,
            preset_id: "text_mask_secrets",
            current_index: 1,
            total_items: 1,
            current_filename: "app.log",
            stage_id: "stage-1",
            status: "Processing",
            percent: 50.0,
            message: null,
          },
        });
      });
    }

    // Cancel button should appear during execution
    await waitFor(() => {
      expect(screen.getByText("処理を中断（キャンセル）")).toBeDefined();
      expect(screen.getByText(/処理中 \(1 \/ 1\)/)).toBeDefined();
    });

    // Click Cancel
    const cancelBtn = screen.getByText("処理を中断（キャンセル）");
    await act(async () => {
      fireEvent.click(cancelBtn);
    });

    // Verify cancel_pipeline was invoked
    expect(invoke).toHaveBeenCalledWith("cancel_pipeline", expect.objectContaining({ jobId: expect.any(String) }));

    // Finish execute promise with cancelled result
    await act(async () => {
      resolveExecute({
        preset_id: "text_mask_secrets",
        total_count: 1,
        success_count: 0,
        failure_count: 0,
        is_cancelled: true,
        item_results: [],
      });
    });

    await waitFor(() => {
      expect(screen.getByText(/ユーザーにより処理が中断されました/)).toBeDefined();
    });
  });
});
