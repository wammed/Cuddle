import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { TimestampModal } from "../components/TimestampModal";
import { changeLanguage } from "../i18n";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { TimestampPlanDto } from "../types/TimestampPlanDto";
import type { TimestampExecutionResultDto } from "../types/TimestampExecutionResultDto";

const mockNormalPlan: TimestampPlanDto = {
  plan_id: "plan-ts-1",
  items: [
    {
      item_id: "item-1",
      path: "/home/user/document.txt",
      display_name: "document.txt",
      is_dir: false,
      current_mtime_ms: 1700000000000n,
      current_mtime_formatted: "2023-11-14 22:13:20",
      target_mtime_ms: 1700003600000n,
      target_mtime_formatted: "2023-11-14 23:13:20",
      is_changed: true,
      warning: null,
    },
  ],
  total_count: 1,
  changed_count: 1,
  has_warnings: false,
  is_executable: true,
  error_message: null,
};

let currentPlan: TimestampPlanDto = mockNormalPlan;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "generate_timestamp_plan") {
      return Promise.resolve(currentPlan);
    }
    if (cmd === "execute_timestamp_plan") {
      const res: TimestampExecutionResultDto = {
        success: true,
        plan_id: "plan-ts-1",
        total_count: 1,
        succeeded_count: 1,
        failed_count: 0,
        is_rolled_back: false,
        error_message: null,
      };
      return Promise.resolve(res);
    }
    return Promise.resolve({});
  }),
}));

const mockStagedItem: StagedItemDto = {
  stage_id: "item-1",
  original_item_id: "orig-1",
  display_name: "document.txt",
  raw_base64: "L2hvbWUvdXNlci9kb2N1bWVudC50eHQ=",
  staged_raw_base64: "L2hvbWUvdXNlci93b3Jrc3BhY2UvZG9jdW1lbnQudHh0",
  staged_path: "/home/user/workspace/document.txt",
  size: 1024n,
  staged_at_ms: 1700000000000n,
  status: "Staged",
  clone_method: "Reflink",
  original_snapshot: {
    item_id: "orig-1",
    raw_base64: "L2hvbWUvdXNlci9kb2N1bWVudC50eHQ=",
    display_name: "document.txt",
    canonical_path: "/home/user/document.txt",
    inode: 12345n,
    dev: 67890n,
    size: 1024n,
    mtime_ms: 1700000000000n,
    sha256: "abcdef",
  },
};

describe("TimestampModal Component", () => {
  beforeEach(async () => {
    currentPlan = mockNormalPlan;
    vi.clearAllMocks();
    await changeLanguage("ja");
  });

  it("renders nothing when isOpen is false", () => {
    const { container } = render(
      <TimestampModal
        isOpen={false}
        onClose={vi.fn()}
        items={[mockStagedItem]}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders modal header, tabs, and plan preview when open", async () => {
    render(
      <TimestampModal
        isOpen={true}
        onClose={vi.fn()}
        items={[mockStagedItem]}
      />
    );

    // Title should be visible
    expect(screen.getByText("ファイルタイムスタンプ（更新日時）の一括変更")).toBeDefined();

    // Tabs should be present
    expect(screen.getByText("現在時刻 (touch)")).toBeDefined();
    expect(screen.getByText("絶対日時指定")).toBeDefined();
    expect(screen.getByText("相対時間シフト")).toBeDefined();
    expect(screen.getByText("EXIF撮影日時同期")).toBeDefined();

    // Table should show planned item
    await waitFor(() => {
      expect(screen.getByText("document.txt")).toBeDefined();
    });
  });

  it("switches tabs and updates plan parameters", async () => {
    render(
      <TimestampModal
        isOpen={true}
        onClose={vi.fn()}
        items={[mockStagedItem]}
      />
    );

    // Switch to Relative Tab
    fireEvent.click(screen.getByText("相対時間シフト"));
    expect(screen.getByText("+1 時間")).toBeDefined();
    expect(screen.getByText("-1 時間")).toBeDefined();

    // Switch to Absolute Tab
    fireEvent.click(screen.getByText("絶対日時指定"));
    expect(screen.getByPlaceholderText("例: 2026-10-25 15:30:00")).toBeDefined();

    // Switch to EXIF Tab
    fireEvent.click(screen.getByText("EXIF撮影日時同期"));
    expect(screen.getByText(/EXIF メタデータから撮影日時/)).toBeDefined();
  });

  it("calls execute_timestamp_plan on confirm button click", async () => {
    const handleSuccess = vi.fn();
    render(
      <TimestampModal
        isOpen={true}
        onClose={vi.fn()}
        items={[mockStagedItem]}
        onSuccess={handleSuccess}
      />
    );

    await waitFor(() => {
      const applyBtn = screen.getByText("タイムスタンプを一括変更");
      expect(applyBtn).toBeDefined();
      expect((applyBtn as HTMLButtonElement).disabled).toBe(false);
    });

    const applyBtn = screen.getByText("タイムスタンプを一括変更");
    fireEvent.click(applyBtn);

    await waitFor(() => {
      expect(handleSuccess).toHaveBeenCalledTimes(1);
    });
  });
});
