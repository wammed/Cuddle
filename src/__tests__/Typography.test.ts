import { describe, it, expect, beforeEach } from "vitest";
import {
  DEFAULT_TYPOGRAPHY_SETTINGS,
  UI_FONT_PRESETS,
  MONO_FONT_PRESETS,
  FONT_SIZE_STEPS,
  applyTypographySettings,
  getSavedTypographySettings,
  resetTypographySettings,
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
});
