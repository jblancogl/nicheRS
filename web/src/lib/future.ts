import type { FuturePeriod, FutureSsp, ProjectionResult } from "@/api/types";

/** Catalogue of CHELSA v2.1 CMIP6 climatologies (niche_data::chelsa, ADR-0016). */
export const FUTURE_PERIODS: FuturePeriod[] = ["2011-2040", "2041-2070", "2071-2100"];
export const FUTURE_SSPS: FutureSsp[] = ["ssp126", "ssp370", "ssp585"];
export const FUTURE_GCMS = ["GFDL-ESM4", "IPSL-CM6A-LR", "MPI-ESM1-2-HR", "MRI-ESM2-0", "UKESM1-0-LL"];
/** Most scenario × model runs per study (engine validation). */
export const MAX_PROJECTION_RUNS = 15;

/** Tile/raster layer of a projection output, e.g. `future_2041-2070_ssp370_change`. */
export function futureLayer(p: { period: string; ssp: string }, kind: "mean" | "sd" | "change" | "mess"): `future_${string}` {
  return `future_${p.period}_${p.ssp}_${kind}`;
}

/** Key of a projection in selectors. */
export function projectionKey(p: { period: string; ssp: string }): string {
  return `${p.period}_${p.ssp}`;
}

/** Change-class colours (Okabe–Ito; same as the server tiles and report): unsuitable in both,
 * no longer suitable, newly suitable, suitable in both. */
export const CHANGE_COLOURS = ["#E0E0E0", "#D55E00", "#0072B2", "#009E73"];

/** Agreement needed to show a class at full strength: a majority of the GCMs. */
export function majority(gcms: number): number {
  return Math.floor(gcms / 2) + 1;
}

/** Lowest–highest of a per-GCM value. */
export function gcmRange(p: ProjectionResult, f: (g: ProjectionResult["per_gcm"][number]) => number): [number, number] {
  const v = p.per_gcm.map(f);
  return v.length ? [Math.min(...v), Math.max(...v)] : [NaN, NaN];
}

/** Whether a predictor changes in future projections (CHELSA climate; the demo's synthetic climate). */
export function isClimateLayer(id: string): boolean {
  return id.startsWith("chelsa_") || id === "demo_temp" || id === "demo_prec";
}

export type ProjectionProblem = "tooMany" | "noGcm" | "duplicate" | "noClimate";

/** Problems the engine would reject (engine validation, ADR-0016); `layers` null = defaults. */
export function projectionProblems(
  projections: { period: string; ssp: string; gcms: string[] }[],
  layers: string[] | null,
): ProjectionProblem[] {
  if (projections.length === 0) return [];
  const out: ProjectionProblem[] = [];
  const runs = projections.reduce((n, p) => n + p.gcms.length, 0);
  if (runs > MAX_PROJECTION_RUNS) out.push("tooMany");
  if (projections.some((p) => p.gcms.length === 0)) out.push("noGcm");
  const keys = projections.map(projectionKey);
  if (new Set(keys).size !== keys.length) out.push("duplicate");
  if (layers !== null && !layers.some(isClimateLayer)) out.push("noClimate");
  return out;
}
