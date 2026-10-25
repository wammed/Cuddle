export type Density = "compact" | "normal" | "spacious";

export interface DensityConfig {
  id: Density;
  rowHeight: number;
  iconSize: number;
  fontSize: string;
  paddingY: string;
}
