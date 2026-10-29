import { describe, it, expect, beforeEach } from "vitest";
import {
  DEFAULT_TYPOGRAPHY_SETTINGS,
  UI_FONT_PRESETS,
  MONO_FONT_PRESETS,
  FONT_SIZE_STEPS,
  applyTypographySettings,
  getSavedTypographySettings,
  resetTypographySettings,
  isNerdFontConfigured,
  detectSystemNerdFont,
  getNerdFontStatus,
  TypographySettings,
} from "../utils/typography";

describe("Typography Utility (Phase 8.10)", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.style.removeProperty("--font-sans");
    document.documentElement.style.removeProperty("--font-mono");
    document.documentElement.style.removeProperty("--app-base-font-size");
    document.documentElement.style.removeProperty("--font-size-scale");
  });

  it("returns default settings when localStorage is empty", () => {
    const settings = getSavedTypographySettings();
    expect(settings).toEqual(DEFAULT_TYPOGRAPHY_SETTINGS);
    expect(settings.fontSize).toBe(14);
    expect(settings.uiFontPreset).toBe("default");
    expect(settings.monoFontPreset).toBe("default");
  });

  it("applies settings and sets CSS root variables correctly", () => {
    const custom: TypographySettings = {
      uiFontPreset: "modern",
      customUiFont: "",
      monoFontPreset: "jetbrains",
      customMonoFont: "",
      fontSize: 16,
    };

    applyTypographySettings(custom);

    const root = document.documentElement;
    expect(root.style.getPropertyValue("--app-base-font-size")).toBe("16px");
    const scale = parseFloat(root.style.getPropertyValue("--font-size-scale"));
    expect(scale).toBeCloseTo(16 / 14, 3);

    const modernPreset = UI_FONT_PRESETS.find((p) => p.id === "modern");
    expect(root.style.getPropertyValue("--font-sans")).toBe(modernPreset?.stack);

    const jbPreset = MONO_FONT_PRESETS.find((p) => p.id === "jetbrains");
    expect(root.style.getPropertyValue("--font-mono")).toBe(jbPreset?.stack);

    // Verify localStorage persistence
    const saved = getSavedTypographySettings();
    expect(saved).toEqual(custom);
  });

  it("supports custom font family input with fallback stack", () => {
    const custom: TypographySettings = {
      uiFontPreset: "custom",
      customUiFont: "Comic Sans MS",
      monoFontPreset: "custom",
      customMonoFont: "CustomCode Nerd Font",
      fontSize: 12,
    };

    applyTypographySettings(custom);

    const root = document.documentElement;
    expect(root.style.getPropertyValue("--font-sans")).toContain('"Comic Sans MS"');
    expect(root.style.getPropertyValue("--font-mono")).toContain('"CustomCode Nerd Font"');
    expect(root.style.getPropertyValue("--app-base-font-size")).toBe("12px");
  });

  it("resets to default settings correctly", () => {
    const custom: TypographySettings = {
      uiFontPreset: "gothic",
      customUiFont: "",
      monoFontPreset: "fira",
      customMonoFont: "",
      fontSize: 18,
    };

    applyTypographySettings(custom);
    expect(document.documentElement.style.getPropertyValue("--app-base-font-size")).toBe("18px");

    const reset = resetTypographySettings();
    expect(reset).toEqual(DEFAULT_TYPOGRAPHY_SETTINGS);
    expect(document.documentElement.style.getPropertyValue("--app-base-font-size")).toBe("14px");
  });

  it("provides valid presets and font size steps", () => {
    expect(UI_FONT_PRESETS.length).toBeGreaterThanOrEqual(5);
    expect(MONO_FONT_PRESETS.length).toBeGreaterThanOrEqual(6);
    expect(FONT_SIZE_STEPS).toContain(14);
    expect(FONT_SIZE_STEPS[0]).toBe(11);
    expect(FONT_SIZE_STEPS[FONT_SIZE_STEPS.length - 1]).toBe(18);
  });

  describe("Nerd Font Status Detection", () => {
    it("identifies when Nerd Font is configured via mono preset", () => {
      expect(isNerdFontConfigured({ ...DEFAULT_TYPOGRAPHY_SETTINGS, monoFontPreset: "nerd" })).toBe(true);
      expect(isNerdFontConfigured({ ...DEFAULT_TYPOGRAPHY_SETTINGS, monoFontPreset: "default" })).toBe(false);
    });

    it("identifies when Nerd Font is configured via custom mono or ui font", () => {
      expect(
        isNerdFontConfigured({
          ...DEFAULT_TYPOGRAPHY_SETTINGS,
          monoFontPreset: "custom",
          customMonoFont: "Hack Nerd Font",
        })
      ).toBe(true);

      expect(
        isNerdFontConfigured({
          ...DEFAULT_TYPOGRAPHY_SETTINGS,
          uiFontPreset: "custom",
          customUiFont: "Symbols Nerd Font Mono",
        })
      ).toBe(true);

      expect(
        isNerdFontConfigured({
          ...DEFAULT_TYPOGRAPHY_SETTINGS,
          monoFontPreset: "custom",
          customMonoFont: "Consolas",
        })
      ).toBe(false);
    });

    it("detects system fonts safely when document.fonts is missing or mocked", () => {
      // Missing document.fonts
      const origFonts = (document as any).fonts;
      (document as any).fonts = undefined;
      expect(detectSystemNerdFont()).toBe(false);

      // Mocked document.fonts.check returning true for Nerd Font
      (document as any).fonts = {
        check: (font: string) => font.includes("JetBrainsMono Nerd Font"),
      };
      expect(detectSystemNerdFont()).toBe(true);

      // Restore
      (document as any).fonts = origFonts;
    });

    it("evaluates getNerdFontStatus properly", () => {
      // 1. When explicitly configured -> active
      const configured: TypographySettings = {
        ...DEFAULT_TYPOGRAPHY_SETTINGS,
        monoFontPreset: "nerd",
      };
      expect(getNerdFontStatus(configured)).toBe("active");

      // 2. When not configured and no fonts detected -> recommended
      expect(getNerdFontStatus(DEFAULT_TYPOGRAPHY_SETTINGS)).toBe("recommended");
    });

    it("dispatches cuddle:typography-changed event on applyTypographySettings", () => {
      let receivedDetail: TypographySettings | null = null;
      const handler = (e: Event) => {
        receivedDetail = (e as CustomEvent).detail;
      };
      window.addEventListener("cuddle:typography-changed", handler);

      const targetSettings: TypographySettings = {
        uiFontPreset: "default",
        customUiFont: "",
        monoFontPreset: "nerd",
        customMonoFont: "",
        fontSize: 15,
      };

      applyTypographySettings(targetSettings);

      expect(receivedDetail).toEqual(targetSettings);
      window.removeEventListener("cuddle:typography-changed", handler);
    });
  });
});
