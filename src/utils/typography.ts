/**
 * Typography management utility for Cuddle (File Workbench).
 * Supports dynamic switching of UI Font Family, Monospace Font Family, and Base Font Size/Scale.
 * Persists settings to localStorage and injects CSS variables to document.documentElement.
 */

export interface TypographySettings {
  uiFontPreset: string;
  customUiFont: string;
  monoFontPreset: string;
  customMonoFont: string;
  fontSize: number; // 11 to 18 (px)
}

export interface FontPreset {
  id: string;
  labelKey: string;
  fallbackName: string;
  stack: string;
}

export const DEFAULT_UI_FONT_STACK =
  '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Noto Sans CJK JP", "Noto Sans JP", "Hiragino Sans", "BIZ UDPGothic", Ubuntu, Cantarell, "Helvetica Neue", sans-serif';

export const DEFAULT_MONO_FONT_STACK =
  '"JetBrains Mono", "Fira Code", "Cascadia Code", "Source Code Pro", "Noto Sans Mono CJK JP", "BIZ UDGothic", Consolas, monospace';

export const UI_FONT_PRESETS: FontPreset[] = [
  {
    id: "default",
    labelKey: "typography.font_preset_default",
    fallbackName: "Default (Inter / Noto Sans)",
    stack: DEFAULT_UI_FONT_STACK,
  },
  {
    id: "modern",
    labelKey: "typography.font_preset_modern",
    fallbackName: "Modern (Inter / Noto Sans JP)",
    stack: '"Inter", "Noto Sans JP", "Noto Sans CJK JP", "Helvetica Neue", sans-serif',
  },
  {
    id: "system",
    labelKey: "typography.font_preset_system",
    fallbackName: "System Native",
    stack: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Ubuntu, Cantarell, "Noto Sans CJK JP", sans-serif',
  },
  {
    id: "gothic",
    labelKey: "typography.font_preset_gothic",
    fallbackName: "Japanese Gothic (BIZ UDP / Yu Gothic)",
    stack: '"BIZ UDPGothic", "Hiragino Sans", "Yu Gothic", "Meiryo", "Noto Sans JP", sans-serif',
  },
  {
    id: "rounded",
    labelKey: "typography.font_preset_rounded",
    fallbackName: "Rounded Gothic",
    stack: '"M PLUS Rounded 1c", "BIZ UDPGothic", "Hiragino Maru Gothic ProN", sans-serif',
  },
  {
    id: "custom",
    labelKey: "typography.font_preset_custom",
    fallbackName: "Custom",
    stack: "",
  },
];

export const MONO_FONT_PRESETS: FontPreset[] = [
  {
    id: "default",
    labelKey: "typography.mono_preset_default",
    fallbackName: "Default (JetBrains / Fira / Cascadia)",
    stack: DEFAULT_MONO_FONT_STACK,
  },
  {
    id: "jetbrains",
    labelKey: "typography.mono_preset_jetbrains",
    fallbackName: "JetBrains Mono",
    stack: '"JetBrains Mono", "Noto Sans Mono CJK JP", monospace',
  },
  {
    id: "fira",
    labelKey: "typography.mono_preset_fira",
    fallbackName: "Fira Code",
    stack: '"Fira Code", "Noto Sans Mono CJK JP", monospace',
  },
  {
    id: "cascadia",
    labelKey: "typography.mono_preset_cascadia",
    fallbackName: "Cascadia Code",
    stack: '"Cascadia Code", "Noto Sans Mono CJK JP", monospace',
  },
  {
    id: "japanesemono",
    labelKey: "typography.mono_preset_japanesemono",
    fallbackName: "Japanese Monospace (BIZ UDGothic)",
    stack: '"BIZ UDGothic", "Noto Sans Mono CJK JP", monospace',
  },
  {
    id: "nerd",
    labelKey: "typography.mono_preset_nerd",
    fallbackName: "Symbols Nerd Font",
    stack: '"Symbols Nerd Font", "JetBrainsMono Nerd Font", "FiraCode Nerd Font", monospace',
  },
  {
    id: "custom",
    labelKey: "typography.mono_preset_custom",
    fallbackName: "Custom",
    stack: "",
  },
];

export const FONT_SIZE_STEPS = [11, 12, 13, 14, 15, 16, 18];

export const DEFAULT_TYPOGRAPHY_SETTINGS: TypographySettings = {
  uiFontPreset: "default",
  customUiFont: "",
  monoFontPreset: "default",
  customMonoFont: "",
  fontSize: 14,
};

const STORAGE_KEY = "cuddle-typography-settings";

export function getSavedTypographySettings(): TypographySettings {
  if (typeof window === "undefined" || !window.localStorage) {
    return DEFAULT_TYPOGRAPHY_SETTINGS;
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_TYPOGRAPHY_SETTINGS;
    const parsed = JSON.parse(raw);
    return {
      uiFontPreset: parsed.uiFontPreset || "default",
      customUiFont: parsed.customUiFont || "",
      monoFontPreset: parsed.monoFontPreset || "default",
      customMonoFont: parsed.customMonoFont || "",
      fontSize: typeof parsed.fontSize === "number" ? parsed.fontSize : 14,
    };
  } catch (e) {
    console.error("Failed to parse saved typography settings:", e);
    return DEFAULT_TYPOGRAPHY_SETTINGS;
  }
}

export function applyTypographySettings(settings: TypographySettings): void {
  if (typeof document === "undefined") return;

  const root = document.documentElement;

  // Resolve UI Font
  let uiStack = DEFAULT_UI_FONT_STACK;
  if (settings.uiFontPreset === "custom" && settings.customUiFont.trim()) {
    uiStack = `"${settings.customUiFont.trim()}", ${DEFAULT_UI_FONT_STACK}`;
  } else {
    const found = UI_FONT_PRESETS.find((p) => p.id === settings.uiFontPreset);
    if (found && found.stack) {
      uiStack = found.stack;
    }
  }

  // Resolve Monospace Font
  let monoStack = DEFAULT_MONO_FONT_STACK;
  if (settings.monoFontPreset === "custom" && settings.customMonoFont.trim()) {
    monoStack = `"${settings.customMonoFont.trim()}", ${DEFAULT_MONO_FONT_STACK}`;
  } else {
    const found = MONO_FONT_PRESETS.find((p) => p.id === settings.monoFontPreset);
    if (found && found.stack) {
      monoStack = found.stack;
    }
  }

  // Calculate Font Scale (14px = 1.0)
  const baseSize = Math.max(11, Math.min(22, settings.fontSize));
  const scale = baseSize / 14.0;

  // Set CSS Variables
  root.style.setProperty("--font-sans", uiStack);
  root.style.setProperty("--font-mono", monoStack);
  root.style.setProperty("--app-base-font-size", `${baseSize}px`);
  root.style.setProperty("--font-size-scale", scale.toFixed(4));

  // Persist to localStorage
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch (e) {
    console.error("Failed to save typography settings to localStorage:", e);
  }

  // Notify listeners (e.g. statusbar, other components)
  if (typeof window !== "undefined") {
    try {
      window.dispatchEvent(
        new CustomEvent("cuddle:typography-changed", { detail: settings })
      );
    } catch (e) {
      // Ignore in non-standard or headless environments
    }
  }
}

export function resetTypographySettings(): TypographySettings {
  applyTypographySettings(DEFAULT_TYPOGRAPHY_SETTINGS);
  return DEFAULT_TYPOGRAPHY_SETTINGS;
}

export type NerdFontStatus = "active" | "detected" | "recommended";

/**
 * Checks whether Nerd Font is explicitly configured in settings
 * (either by choosing the 'nerd' preset or naming a Nerd Font in custom UI/Mono font).
 */
export function isNerdFontConfigured(settings?: TypographySettings): boolean {
  const s = settings || getSavedTypographySettings();
  if (s.monoFontPreset === "nerd") {
    return true;
  }
  if (s.monoFontPreset === "custom" && /nerd/i.test(s.customMonoFont)) {
    return true;
  }
  if (s.uiFontPreset === "custom" && /nerd/i.test(s.customUiFont)) {
    return true;
  }
  const chosenMono = MONO_FONT_PRESETS.find((p) => p.id === s.monoFontPreset);
  if (chosenMono && /nerd/i.test(chosenMono.stack)) {
    return true;
  }
  const chosenUi = UI_FONT_PRESETS.find((p) => p.id === s.uiFontPreset);
  if (chosenUi && /nerd/i.test(chosenUi.stack)) {
    return true;
  }
  return false;
}

/**
 * Checks whether any Nerd Font family is installed on the host system via document.fonts API.
 */
export function detectSystemNerdFont(): boolean {
  if (
    typeof document === "undefined" ||
    !document.fonts ||
    typeof document.fonts.check !== "function"
  ) {
    return false;
  }
  try {
    const candidateFonts = [
      '12px "Symbols Nerd Font"',
      '12px "JetBrainsMono Nerd Font"',
      '12px "JetBrainsMonoNL Nerd Font"',
      '12px "FiraCode Nerd Font"',
      '12px "Hack Nerd Font"',
      '12px "CaskaydiaCove Nerd Font"',
      '12px "MesloLGS NF"',
      '12px "SauceCodePro Nerd Font"',
    ];
    return candidateFonts.some((font) => {
      try {
        return document.fonts.check(font);
      } catch {
        return false;
      }
    });
  } catch {
    return false;
  }
}

/**
 * Determines current status of Nerd Font for statusbar and settings UI:
 * - "active": User explicitly configured a Nerd Font preset or custom font.
 * - "detected": Nerd Font is available on the host system but not currently configured as primary.
 * - "recommended": Nerd Font is neither configured nor detected.
 */
export function getNerdFontStatus(settings?: TypographySettings): NerdFontStatus {
  if (isNerdFontConfigured(settings)) {
    return "active";
  }
  if (detectSystemNerdFont()) {
    return "detected";
  }
  return "recommended";
}
