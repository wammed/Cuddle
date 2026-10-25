import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { ChmodModal } from "../components/ChmodModal";
import { changeLanguage } from "../i18n";
import type { StagedItemDto } from "../types/StagedItemDto";
import type { ChmodPlanDto } from "../types/ChmodPlanDto";
import type { ChmodExecutionResultDto } from "../types/ChmodExecutionResultDto";

const mockNormalPlan: ChmodPlanDto = {
  plan_id: "plan-normal-1",
  items: [
    {
      item_id: "item-1",
      path: "/home/user/script.sh",
      display_name: "script.sh",
      is_dir: false,
      current_mode: 0o644,
      current_mode_str: "-rw-r--r--",
      current_octal: "0644",
      target_mode: 0o755,
      target_mode_str: "-rwxr-xr-x",
      target_octal: "0755",
      is_changed: true,
      has_special_bits: false,
      warning: null,
    },
  ],
  total_count: 1,
  changed_count: 1,
  has_warnings: false,
  has_special_bits: false,
  is_executable: true,
  error_message: null,
};

const mockSpecialBitsPlan: ChmodPlanDto = {
  plan_id: "plan-special-1",
  items: [
    {
      item_id: "item-2",
      path: "/home/user/bin/binary",
      display_name: "binary",
      is_dir: false,
      current_mode: 0o755,
      current_mode_str: "-rwxr-xr-x",
      current_octal: "0755",
      target_mode: 0o4755,
      target_mode_str: "-rwsr-xr-x",
      target_octal: "4755",
      is_changed: true,
      has_special_bits: true,
      warning: null,
    },
  ],
  total_count: 1,
  changed_count: 1,
  has_warnings: false,
  has_special_bits: true,
  is_executable: true,
  error_message: null,
};

let currentPlan: ChmodPlanDto = mockNormalPlan;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "generate_chmod_plan") {
      return Promise.resolve(currentPlan);
    }
    if (cmd === "execute_chmod_plan") {
      const result: ChmodExecutionResultDto = {
        plan_id: currentPlan.plan_id,
        success: true,
        total_count: currentPlan.total_count,
        succeeded_count: currentPlan.changed_count,
        failed_count: 0,
        is_rolled_back: false,
        error_message: null,
      };
      return Promise.resolve(result);
    }
    return Promise.reject(new Error(`Unhandled mock invoke: ${cmd}`));
  }),
}));

const mockItems: StagedItemDto[] = [
  {
    stage_id: "item-1",
    original_item_id: "orig-1",
    raw_base64: "c2NyaXB0LnNo",
    staged_raw_base64: "c2NyaXB0LnNo",
    display_name: "script.sh",
    staged_path: "/tmp/cuddle-workspace/script.sh",
    size: 120n,
    clone_method: "Reflink",
    status: "Staged",
    staged_at_ms: 1700000000000n,
    original_snapshot: {
      item_id: "orig-1",
      raw_base64: "c2NyaXB0LnNo",
      display_name: "script.sh",
      canonical_path: "/home/user/script.sh",
      size: 120n,
      mtime_ms: 1700000000000n,
      sha256: "hash-1",
      inode: 12345n,
      dev: 66305n,
    },
  },
];

describe("ChmodModal Component", () => {
  beforeEach(async () => {
    await changeLanguage("ja");
    currentPlan = mockNormalPlan;
    vi.clearAllMocks();
  });

  it("does not render when isOpen is false", () => {
    const { container } = render(
      <ChmodModal isOpen={false} onClose={vi.fn()} items={mockItems} />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders modal header and tab options when isOpen is true", async () => {
    render(<ChmodModal isOpen={true} onClose={vi.fn()} items={mockItems} />);

    await waitFor(() => {
      expect(screen.getByText("ファイル属性（権限）の一括変更")).toBeDefined();
      expect(screen.getByText("8進数指定 (Octal)")).toBeDefined();
      expect(screen.getByText("シンボリック指定 (Symbolic)")).toBeDefined();
    });
  });

  it("updates octal input when preset buttons are clicked", async () => {
    render(<ChmodModal isOpen={true} onClose={vi.fn()} items={mockItems} />);

    await waitFor(() => {
      expect(screen.getByText(/644 \(標準ファイル/)).toBeDefined();
    });

    const presetBtn = screen.getByText(/644 \(標準ファイル/);
    fireEvent.click(presetBtn);

    const input = screen.getByLabelText("8進数パーミッション") as HTMLInputElement;
    expect(input.value).toBe("644");
  });

  it("switches to symbolic tab and shows symbolic presets", async () => {
    render(<ChmodModal isOpen={true} onClose={vi.fn()} items={mockItems} />);

    const symbolicTab = screen.getByText("シンボリック指定 (Symbolic)");
    fireEvent.click(symbolicTab);

    await waitFor(() => {
      expect(screen.getByText(/\+x \(全員に実行権限を付与\)/)).toBeDefined();
    });

    const plusXBtn = screen.getByText(/\+x \(全員に実行権限を付与\)/);
    fireEvent.click(plusXBtn);

    const input = screen.getByLabelText("シンボリック表現 (chmod)") as HTMLInputElement;
    expect(input.value).toBe("+x");
  });

  it("displays special bits warning banner when plan has special bits", async () => {
    currentPlan = mockSpecialBitsPlan;

    render(<ChmodModal isOpen={true} onClose={vi.fn()} items={mockItems} />);

    await waitFor(() => {
      expect(
        screen.getByText(/特殊ビット \(setuid\/setgid\/sticky\) が指定されています/)
      ).toBeDefined();
    });
  });

  it("calls execute_chmod_plan when clicking apply button", async () => {
    const handleSuccess = vi.fn();
    render(
      <ChmodModal
        isOpen={true}
        onClose={vi.fn()}
        items={mockItems}
        onSuccess={handleSuccess}
      />
    );

    await waitFor(() => {
      const applyBtn = screen.getByRole("button", { name: /属性を一括変更/ }) as HTMLButtonElement;
      expect(applyBtn.disabled).toBe(false);
    });

    const applyBtn = screen.getByRole("button", { name: /属性を一括変更/ });
    fireEvent.click(applyBtn);

    await waitFor(() => {
      expect(handleSuccess).toHaveBeenCalledWith(
        expect.objectContaining({
          success: true,
          succeeded_count: 1,
        })
      );
    });
  });
});
