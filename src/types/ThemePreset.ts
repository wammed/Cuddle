// src/types/ThemePreset.ts
// Cuddle - 30 Color Theme Presets Specification (Classic, Modern, Neon, Cyber)

export type ThemeCategory = "cyber" | "neon" | "modern" | "classic";

export interface ThemeColors {
  bgCanvas: string;
  bgSurfaceBase: string;
  bgSurfaceCard: string;
  bgSurfaceElevated: string;
  bgSurfaceInput: string;
  borderSubtle: string;
  borderMedium: string;
  borderGlow: string;
  accentBrand: string;
  accentBrandBright: string;
  accentSafety: string;
  accentWarning: string;
  accentDanger: string;
  textPrimary: string;
  textSecondary: string;
  textMuted: string;
}

export interface ThemePreset {
  id: string;
  category: ThemeCategory;
  nameKey: string;
  fallbackName: string;
  descriptionKey: string;
  fallbackDesc: string;
  colors: ThemeColors;
}
