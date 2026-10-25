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

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "get_pipeline_presets") {
      return Promise.resolve(mockPresets);
    }
    if (cmd === "probe_external_tools") {
      return Promise.resolve(mockTools);
    }
    if (cmd === "execute_pipeline_preset") {
      return Promise.resolve({
        preset_id: "text_mask_secrets",
        total_count: 1,
        success_count: 1,
        failure_count: 0,
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
});
