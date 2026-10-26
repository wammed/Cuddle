import type { Density, DensityConfig } from "../types/Density";

export const DENSITY_CONFIGS: Record<Density, DensityConfig> = {
  compact: {
    id: "compact",
    rowHeight: 42,
    iconSize: 15,
    fontSize: "11px",
    paddingY: "3px",
  },
  normal: {
    id: "normal",
    rowHeight: 52,
    iconSize: 17,
    fontSize: "12px",
    paddingY: "6px",
  },
  spacious: {
    id: "spacious",
    rowHeight: 64,
    iconSize: 20,
    fontSize: "13px",
    paddingY: "9px",
  },
};

const STORAGE_KEY = "cuddle_display_density";

export function getSavedDensity(): Density {
  if (typeof window === "undefined") return "normal";
  const saved = localStorage.getItem(STORAGE_KEY) as Density | null;
  if (saved && (saved === "compact" || saved === "normal" || saved === "spacious")) {
    return saved;
  }
  return "normal";
}

export function applyDensity(density: Density): void {
  if (typeof document === "undefined") return;
  const config = DENSITY_CONFIGS[density] || DENSITY_CONFIGS.normal;
  const root = document.documentElement;

  root.style.setProperty("--list-row-height", `${config.rowHeight}px`);
  root.style.setProperty("--density-font-size", config.fontSize);
  root.style.setProperty("--density-icon-size", `${config.iconSize}px`);
  root.style.setProperty("--density-padding-y", config.paddingY);

  try {
    localStorage.setItem(STORAGE_KEY, density);
  } catch {
    // ignore
  }
}
