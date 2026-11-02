// Results view state lives in the URL (P9: shareable links, reload restores the view).

import { useCallback } from "react";
import { useSearchParams } from "react-router";

import type { RasterLayer, ThresholdArea } from "@/api/types";

export type Rule = ThresholdArea["rule"];
export type ResultsTab =
  "summary" | "validation" | "variables" | "data" | "future" | "decision";
/** Map layer of a future projection (ADR-0016). */
export type FutureKind = "mean" | "change" | "sd";
/** Section of the Decision support tab (ADR-0018), so links can open one directly. */
export type DecisionSection = "priority" | "footprint" | "richness" | "compare";
/** Simple: plain-language reading for visitors; full: every tab (metrics, variables, exports). */
export type ResultsReading = "simple" | "full";

// "effort" is offered only for results with a sampling-bias diagnostic (ResultsView falls back).
const LAYERS: RasterLayer[] = [
  "suitability",
  "uncertainty",
  "extrapolation",
  "binary",
  "effort",
];
const RULES: Rule[] = ["p10", "mtp", "max_sss"];
const TABS: ResultsTab[] = [
  "summary",
  "validation",
  "variables",
  "data",
  "future",
  "decision",
];
const FUTURE_KINDS: FutureKind[] = ["mean", "change", "sd"];
const SECTIONS: DecisionSection[] = [
  "priority",
  "footprint",
  "richness",
  "compare",
];
const VIEWS: ResultsReading[] = ["simple", "full"];

export interface ResultsParams {
  species: number | null;
  layer: RasterLayer;
  rule: Rule;
  opacity: number; // 0..1
  ext: boolean;
  /** Area-of-applicability hatching (only when the result has an AOA; off by default). */
  aoa: boolean;
  obs: boolean;
  tab: ResultsTab;
  /** null: not in the URL, the viewer's setup path decides (ResultsView.tsx). */
  view: ResultsReading | null;
  /** Future projection shown on the map (`<period>_<ssp>`), null = present. ResultsView checks it. */
  proj: string | null;
  /** Layer of that projection. */
  flayer: FutureKind;
  /** Open section of the Decision support tab. */
  ds: DecisionSection | null;
}

export function useResultsParams(): [
  ResultsParams,
  (patch: Partial<ResultsParams>) => void,
] {
  const [sp, setSp] = useSearchParams();
  const layer = sp.get("layer") as RasterLayer | null;
  const rule = sp.get("rule") as Rule | null;
  const tab = sp.get("tab") as ResultsTab | null;
  const view = sp.get("view") as ResultsReading | null;
  const flayer = sp.get("flayer") as FutureKind | null;
  const ds = sp.get("ds") as DecisionSection | null;
  const op = Number(sp.get("opacity"));
  const species = sp.get("species");
  const params: ResultsParams = {
    species:
      species !== null && Number.isFinite(Number(species))
        ? Number(species)
        : null,
    layer: layer && LAYERS.includes(layer) ? layer : "suitability",
    rule: rule && RULES.includes(rule) ? rule : "p10",
    opacity:
      sp.has("opacity") && Number.isFinite(op)
        ? Math.min(1, Math.max(0, op / 100))
        : 0.8,
    ext: sp.get("ext") !== "0", // extrapolation hatch on by default (P7)
    aoa: sp.get("aoa") === "1",
    obs: sp.get("obs") !== "0",
    tab: tab && TABS.includes(tab) ? tab : "summary",
    view: view && VIEWS.includes(view) ? view : null,
    proj: sp.get("proj"),
    flayer: flayer && FUTURE_KINDS.includes(flayer) ? flayer : "change",
    ds: ds && SECTIONS.includes(ds) ? ds : null,
  };

  const update = useCallback(
    (patch: Partial<ResultsParams>) => {
      setSp(
        (prev) => {
          const next = new URLSearchParams(prev);
          for (const [k, v] of Object.entries(patch)) {
            if (v === undefined || v === null) next.delete(k);
            else if (k === "opacity")
              next.set(k, String(Math.round((v as number) * 100)));
            else if (typeof v === "boolean") next.set(k, v ? "1" : "0");
            else next.set(k, String(v));
          }
          return next;
        },
        { replace: true },
      );
    },
    [setSp],
  );
  return [params, update];
}
