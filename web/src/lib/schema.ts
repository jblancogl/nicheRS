// Zod schema shared by the guided summary and Expert mode (one config object, UX-3).

import { z } from "zod";

import type { Algorithm, FeatureClassSet, ModelSettings, OccurrenceFilters, ValidationSettings } from "@/api/types";

import { schemaMessages } from "./schemaMessages";

/** Messages in the current interface language (errors are resolved when a field is validated). */
const t = () => schemaMessages.get();

export const FEATURE_SETS: { id: FeatureClassSet; label: string }[] = [
  { id: "L", label: "Linear" },
  { id: "LQ", label: "Linear + quadratic" },
  { id: "H", label: "Hinge" },
  { id: "LQH", label: "Linear + quadratic + hinge" },
  { id: "LQHP", label: "Linear + quadratic + hinge + product" },
];

export const DEFAULT_MODEL: ModelSettings = {
  tuning: "auto",
  feature_classes: ["L", "LQ", "H", "LQH", "LQHP"],
  beta_multipliers: [0.5, 1, 2, 3, 4],
  background_points: 10_000,
  collinearity_threshold: 0.7,
};

/** Trees per random forest when rf_trees is omitted (server DEFAULT_RF_TREES, ADR-0017). */
export const DEFAULT_RF_TREES = 500;
export const MIN_RF_TREES = 100;
export const MAX_RF_TREES = 2000;

export const DEFAULT_VALIDATION: ValidationSettings = { method: "spatial_block", folds: 4 };
export const DEFAULT_SEED = 42;

export const DEFAULT_FILTERS: OccurrenceFilters = {
  year_min: 1981,
  max_uncertainty_m: 1000,
  licences: ["CC0_1_0", "CC_BY_4_0"],
};

export function parseBetaList(text: string): number[] | null {
  const parts = text
    .split(/[,;\s]+/)
    .map((s) => s.trim())
    .filter(Boolean);
  if (parts.length === 0) return null;
  const nums = parts.map(Number);
  return nums.every((n) => Number.isFinite(n)) ? nums : null;
}

/** Largest distance-thinning distance the server accepts (km). */
export const MAX_THIN_KM = 100;
/** GBIF class name as the server accepts it (letters, digits, " -_(),."; ≤ 64 characters). */
const TARGET_GROUP_RE = /^[\p{L}\p{N} \-_(),.]{1,64}$/u;

/** A decimal number typed with a dot or a comma; null when not a finite number. */
function parseKm(text: string): number | null {
  const n = Number(text.trim().replace(",", "."));
  return text.trim() !== "" && Number.isFinite(n) ? n : null;
}

export const modelFormSchema = z
  .object({
    tuning: z.enum(["auto", "fixed"]),
    feature_classes: z
      .array(z.enum(["L", "LQ", "H", "LQH", "LQHP"]))
      .min(1, { error: () => t().selectFeatureSet }),
    beta_text: z.string().trim().min(1, { error: () => t().enterBeta }),
    background_points: z
      .number({ error: () => t().wholeNumber })
      .int({ error: () => t().wholeNumber })
      .min(1_000, { error: () => t().bgMin })
      .max(100_000, { error: () => t().bgMax }),
    collinearity_threshold: z
      .number({ error: () => t().rNumber })
      .min(0.3, { error: () => t().rMin })
      .max(1, { error: () => t().rMax }),
    method: z.enum(["spatial_block", "spatial_grid", "random_kfold"]),
    /** spatial_grid block side in km; empty = automatic (autocorrelation range). */
    block_km_text: z.string().trim(),
    null_models: z
      .number({ error: () => t().wholeNumber })
      .int({ error: () => t().wholeNumber })
      .min(0, { error: () => t().nullModelsRange })
      .max(200, { error: () => t().nullModelsRange })
      // 0 = off; otherwise ≥ 19, the fewest nulls for which p can reach 0.05 (1/(n + 1)).
      .refine((n) => n === 0 || n >= 19, { error: () => t().nullModelsRange }),
    aoa: z.boolean(),
    /** Background sampling (ADR-0013): random (default) or target group. */
    background_method: z.enum(["random", "target_group"]),
    /** target_group only: GBIF class; empty = the species' own class. */
    target_group: z.string().trim(),
    /** Distance thinning of presence cells in km; empty = off (stored in filters.thin_km). */
    thin_km_text: z.string().trim(),
    /** Modelling algorithm (ADR-0017): MaxEnt (default), down-sampled random forest or ensemble. */
    algorithm: z.enum(["maxent", "rf", "ensemble"]),
    rf_trees: z
      .number({ error: () => t().wholeNumber })
      .int({ error: () => t().wholeNumber })
      .min(MIN_RF_TREES, { error: () => t().rfTreesRange })
      .max(MAX_RF_TREES, { error: () => t().rfTreesRange }),
    rf_tune_mtry: z.boolean(),
    folds: z
      .number({ error: () => t().wholeNumber })
      .int({ error: () => t().wholeNumber })
      .min(2, { error: () => t().foldsMin })
      .max(10, { error: () => t().foldsMax }),
    seed: z
      .number({ error: () => t().wholeNumber })
      .int({ error: () => t().wholeNumber })
      .min(0, { error: () => t().seedMin })
      .max(4_294_967_295, { error: () => t().seedMax }),
  })
  .superRefine((v, ctx) => {
    const betas = parseBetaList(v.beta_text);
    // MaxEnt settings are hidden and not used by the random forest alone.
    const maxent = v.algorithm !== "rf";
    if (maxent && !betas) {
      ctx.addIssue({ code: "custom", path: ["beta_text"], message: t().betaList });
    } else if (maxent && betas) {
      if (betas.some((b) => b <= 0 || b > 20))
        ctx.addIssue({ code: "custom", path: ["beta_text"], message: t().betaRange });
      if (betas.length > 10)
        ctx.addIssue({ code: "custom", path: ["beta_text"], message: t().betaCount });
      if (v.tuning === "fixed" && betas.length !== 1)
        ctx.addIssue({ code: "custom", path: ["beta_text"], message: t().fixedOneBeta });
    }
    if (v.algorithm !== "rf" && v.tuning === "fixed" && v.feature_classes.length !== 1)
      ctx.addIssue({ code: "custom", path: ["feature_classes"], message: t().fixedOneSet });
    if (v.method === "spatial_block" && v.folds !== 4)
      ctx.addIssue({ code: "custom", path: ["folds"], message: t().blocksFour });
    if (v.method === "spatial_grid" && v.block_km_text !== "") {
      const km = Number(v.block_km_text.replace(",", "."));
      if (!Number.isFinite(km) || km < 1 || km > 1000)
        ctx.addIssue({ code: "custom", path: ["block_km_text"], message: t().blockKmRange });
    }
    if (v.background_method === "target_group" && v.target_group !== "" && !TARGET_GROUP_RE.test(v.target_group))
      ctx.addIssue({ code: "custom", path: ["target_group"], message: t().targetGroupInvalid });
    if (v.thin_km_text !== "") {
      const km = parseKm(v.thin_km_text);
      if (km === null || km <= 0 || km > MAX_THIN_KM)
        ctx.addIssue({ code: "custom", path: ["thin_km_text"], message: t().thinKmRange });
    }
  });

export type ModelFormValues = z.infer<typeof modelFormSchema>;

export function toFormValues(m: ModelSettings, v: ValidationSettings, seed: number, thinKm: number | null = null): ModelFormValues {
  return {
    tuning: m.tuning,
    feature_classes: [...m.feature_classes],
    beta_text: m.beta_multipliers.join(", "),
    background_points: m.background_points,
    collinearity_threshold: m.collinearity_threshold,
    method: v.method,
    folds: v.folds,
    block_km_text: v.block_km != null ? String(v.block_km) : "",
    null_models: v.null_models ?? 0,
    aoa: v.aoa ?? false,
    background_method: m.background_method ?? "random",
    target_group: m.target_group ?? "",
    thin_km_text: thinKm != null ? String(thinKm) : "",
    algorithm: m.algorithm ?? "maxent",
    rf_trees: m.rf_trees ?? DEFAULT_RF_TREES,
    rf_tune_mtry: m.rf_tune_mtry ?? false,
    seed,
  };
}

export function fromFormValues(f: ModelFormValues): {
  model: ModelSettings;
  validation: ValidationSettings;
  seed: number;
  /** Distance thinning (km) for filters.thin_km; null = off. */
  thin_km: number | null;
} {
  const model: ModelSettings = {
    tuning: f.tuning,
    feature_classes: f.feature_classes,
    beta_multipliers: parseBetaList(f.beta_text) ?? DEFAULT_MODEL.beta_multipliers,
    background_points: f.background_points,
    collinearity_threshold: f.collinearity_threshold,
  };
  // Left out when random (the default), so a default study keeps its specification hash.
  if (f.background_method === "target_group") {
    model.background_method = "target_group";
    if (f.target_group !== "") model.target_group = f.target_group;
  }
  // Left out for MaxEnt (the default) and at default values, so a default study keeps its hash.
  if (f.algorithm !== "maxent") {
    model.algorithm = f.algorithm;
    if (f.rf_trees !== DEFAULT_RF_TREES) model.rf_trees = f.rf_trees;
    if (f.rf_tune_mtry) model.rf_tune_mtry = true;
  }
  const km = f.thin_km_text === "" ? null : parseKm(f.thin_km_text);
  return { model, validation: validationFrom(f), seed: f.seed, thin_km: km !== null && km > 0 ? km : null };
}

/** Filters with distance thinning set (or removed: the key is omitted when off). */
export function withThinning(f: OccurrenceFilters, km: number | null): OccurrenceFilters {
  const rest = { ...f };
  delete rest.thin_km;
  return km !== null ? { ...rest, thin_km: km } : rest;
}

/** A specification loaded back into the draft ("Duplicate & edit") without default-valued
 *  optional keys, so a default study keeps the same specification JSON (and hash). */
export function normaliseModel(m: ModelSettings): ModelSettings {
  const { background_method, target_group, algorithm, rf_trees, rf_tune_mtry, ...base } = m;
  const rest: ModelSettings =
    algorithm === "rf" || algorithm === "ensemble"
      ? {
          ...base,
          algorithm,
          ...(rf_trees != null && rf_trees !== DEFAULT_RF_TREES ? { rf_trees } : {}),
          ...(rf_tune_mtry ? { rf_tune_mtry: true } : {}),
        }
      : base;
  if (background_method !== "target_group") return rest;
  return { ...rest, background_method, ...(target_group ? { target_group } : {}) };
}

export function normaliseFilters(f: OccurrenceFilters): OccurrenceFilters {
  return withThinning(f, f.thin_km ?? null);
}

/** Validation settings from the form. Options left off are omitted (not sent as defaults), so a
 *  default study keeps the same specification JSON and hash as before they existed (ADR-0014). */
function validationFrom(f: ModelFormValues): ValidationSettings {
  const v: ValidationSettings = { method: f.method, folds: f.method === "spatial_block" ? 4 : f.folds };
  if (f.method === "spatial_grid" && f.block_km_text !== "") {
    const km = Number(f.block_km_text.replace(",", "."));
    if (Number.isFinite(km)) v.block_km = km;
  }
  if (f.null_models > 0) v.null_models = f.null_models;
  if (f.aoa) v.aoa = true;
  return v;
}

/** Validation scheme in words (summary, review). */
function describeValidation(v: ValidationSettings): string {
  const msg = t();
  if (v.method === "spatial_block") return msg.valBlocks(v.folds);
  if (v.method === "spatial_grid") return msg.valGrid(v.folds, v.block_km ?? null);
  return msg.valRandom(v.folds);
}

/** Plain-language summary of the model configuration (guided view and review). */
export function describeModel(m: ModelSettings, v: ValidationSettings): string {
  const msg = t();
  let val = describeValidation(v);
  if ((v.null_models ?? 0) > 0) val += msg.withNulls(v.null_models ?? 0);
  if (v.aoa) val += msg.withAoa;
  if (m.background_method === "target_group") val += msg.withTargetGroup(m.target_group ?? null);
  const algorithm: Algorithm = m.algorithm ?? "maxent";
  const trees = m.rf_trees ?? DEFAULT_RF_TREES;
  if (algorithm === "rf") return msg.rfModel(trees, m.rf_tune_mtry ?? false, val);
  const maxent = (v: string) =>
    m.tuning === "auto"
      ? msg.tuned(m.feature_classes.length, m.beta_multipliers.length, v)
      : msg.fixed(m.feature_classes.join(", "), m.beta_multipliers.join(", "), v);
  if (algorithm === "ensemble")
    return msg.ensembleModel(trees, m.rf_tune_mtry ?? false, maxent("").replace(/,\s*$/, ""), val);
  return maxent(val);
}

/** Non-default parameters, for the review step (P4: list every non-default value). */
export function modelDiffs(m: ModelSettings, v: ValidationSettings, seed: number, thinKm: number | null = null): string[] {
  const msg = t();
  const out: string[] = [];
  const d = DEFAULT_MODEL;
  if (m.tuning !== d.tuning) out.push(msg.diffTuning(m.tuning === "auto" ? msg.tuningAuto : msg.tuningFixed));
  if (m.feature_classes.join() !== d.feature_classes.join()) out.push(msg.diffSets(m.feature_classes.join(", ")));
  if (m.beta_multipliers.join() !== d.beta_multipliers.join()) out.push(msg.diffBetas(m.beta_multipliers.join(", ")));
  if (m.background_points !== d.background_points) out.push(msg.diffBg(m.background_points));
  if (m.collinearity_threshold !== d.collinearity_threshold) out.push(msg.diffR(m.collinearity_threshold));
  if (v.method !== DEFAULT_VALIDATION.method || v.folds !== DEFAULT_VALIDATION.folds)
    out.push(msg.diffVal(describeValidation(v)));
  if ((v.null_models ?? 0) > 0) out.push(msg.diffNulls(v.null_models ?? 0));
  if (v.aoa) out.push(msg.diffAoa);
  if (m.background_method === "target_group") out.push(msg.diffBackground(m.target_group ?? null));
  if (thinKm !== null) out.push(msg.diffThin(thinKm));
  if (m.algorithm === "rf" || m.algorithm === "ensemble") {
    out.push(msg.diffAlgorithm(m.algorithm));
    if ((m.rf_trees ?? DEFAULT_RF_TREES) !== DEFAULT_RF_TREES) out.push(msg.diffTrees(m.rf_trees ?? DEFAULT_RF_TREES));
    if (m.rf_tune_mtry) out.push(msg.diffTuneMtry);
  }
  if (seed !== DEFAULT_SEED) out.push(msg.diffSeed(seed));
  return out;
}
