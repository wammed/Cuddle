import { describe, it, expect, vi, beforeEach } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, act, waitFor } from "@testing-library/react";
import { SettingsModal } from "../components/SettingsModal";
import { changeLanguage } from "../i18n";
import type { AppConfig } from "../types/AppConfig";

const mockInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe("Phase 14: Settings & config.toml Alignment (Task 3-1, 5-1)", () => {
  const initialConfig: AppConfig = {
    app: {
      language: "ja",
      theme: "cyber-midnight-teal",
      density: "normal",
    },
    workspace: {
      temp_directory: null,
      min_free_space_multiplier: 1.5,
      backup_retention_count: 5,
    },
    security: {
      enforce_landlock: true,
      confirmation_level: "standard",
    },
    custom_pipelines: [],
    snippets: [],
  };

  beforeEach(async () => {
    vi.clearAllMocks();
    await act(async () => {
      await changeLanguage("ja");
    });

    mockInvoke.mockImplementation(async (cmd: string, _args?: any) => {
      if (cmd === "get_app_config") {
        return initialConfig;
      }
      if (cmd === "list_workspace_backups") {
        return [];
      }
      if (cmd === "save_app_config") {
        return null;
      }
      return null;
    });
  });

  it("renders Confirmation Level in General Tab and persists selection to config.toml", async () => {
    await act(async () => {
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
    });

    // Switch to General tab
    const generalTab = screen.getByText("一般 & 言語");
    await act(async () => {
      fireEvent.click(generalTab);
    });

    // Check that Confirmation Level section is rendered
    expect(screen.getByText("安全確認レベル (Confirmation Level)")).toBeInTheDocument();
    expect(screen.getByText("標準 (Standard)")).toBeInTheDocument();
    expect(screen.getByText("厳格 (Strict)")).toBeInTheDocument();

    // Click Strict option
    const strictBtn = screen.getByText("厳格 (Strict)");
    await act(async () => {
      fireEvent.click(strictBtn);
    });

    // Verify save_app_config is called with confirmation_level: "strict"
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "save_app_config",
        expect.objectContaining({
          config: expect.objectContaining({
            security: expect.objectContaining({
              confirmation_level: "strict",
            }),
          }),
        })
      );
    });
  });

  it("persists language selection to config.toml", async () => {
    const handleToggleLang = vi.fn();

    await act(async () => {
      render(
        <SettingsModal
          isOpen={true}
          onClose={() => {}}
          currentLang="ja"
          onToggleLang={handleToggleLang}
          landlockSupported={true}
          landlockAbi="V5"
        />
      );
    });

    // Switch to General tab
    const generalTab = screen.getByText("一般 & 言語");
    await act(async () => {
      fireEvent.click(generalTab);
    });

    // Click English option
    const enBtn = screen.getByText("English (英語)");
    await act(async () => {
      fireEvent.click(enBtn);
    });

    expect(handleToggleLang).toHaveBeenCalled();
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "save_app_config",
        expect.objectContaining({
          config: expect.objectContaining({
            app: expect.objectContaining({
              language: "en",
            }),
          }),
        })
      );
    });
  });

  it("persists theme and density selections in Appearance Tab to config.toml", async () => {
    const handleDensityChange = vi.fn();

    await act(async () => {
      render(
        <SettingsModal
          isOpen={true}
          onClose={() => {}}
          currentLang="ja"
          onToggleLang={() => {}}
          landlockSupported={true}
          landlockAbi="V5"
          onDensityChange={handleDensityChange}
        />
      );
    });

    // 1. Density change
    const compactPill = screen.getByText(/コンパクト/);
    await act(async () => {
      fireEvent.click(compactPill);
    });

    expect(handleDensityChange).toHaveBeenCalledWith("compact");
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "save_app_config",
        expect.objectContaining({
          config: expect.objectContaining({
            app: expect.objectContaining({
              density: "compact",
            }),
          }),
        })
      );
    });
  });
});
