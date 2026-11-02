import type { SpeciesResult } from "@/api/types";
import { EM_DASH, formatAreaRange, formatDecimal, formatPercent } from "@/lib/format";

import { useResultsMessages } from "./messages";

export function sd(values: number[]): number | null {
  if (values.length < 2) return null;
  const m = values.reduce((a, b) => a + b, 0) / values.length;
  return Math.sqrt(values.reduce((a, b) => a + (b - m) ** 2, 0) / (values.length - 1));
}

/** "0.71 ± 0.09" using the server mean and the SD across folds. */
export function meanSd(mean: number | null, folds: (number | null)[], digits = 2): string {
  if (mean === null || !Number.isFinite(mean)) return EM_DASH;
  const s = sd(folds.filter((v): v is number => v !== null && Number.isFinite(v)));
  return s === null ? formatDecimal(mean, digits) : `${formatDecimal(mean, digits)} ± ${formatDecimal(s, digits)}`;
}

export function percentMeanSd(mean: number, folds: number[]): string {
  const s = sd(folds);
  return s === null ? formatPercent(mean) : `${formatPercent(mean)} ± ${formatPercent(s)}`;
}

export function areaBand(r: SpeciesResult): { lo: number; hi: number } | null {
  const areas = r.thresholds.map((t) => t.area_km2).filter(Number.isFinite);
  if (areas.length === 0) return null;
  return { lo: Math.min(...areas), hi: Math.max(...areas) };
}

export function areaBandText(r: SpeciesResult): string {
  const b = areaBand(r);
  return b ? formatAreaRange(b.lo, b.hi) : EM_DASH;
}

export function schemeText(r: SpeciesResult): string {
  const k = r.validation.folds.length;
  const m = r.validation.method;
  const t = useResultsMessages.get();
  const b = r.validation.blocks;
  if (m === "spatial_grid" && b) return t.schemeGrid(k, Math.round(b.block_km));
  return m === "spatial_block" || /spatial/i.test(m) ? t.schemeSpatial(k) : t.schemeOther(m, k);
}
