// Legend colours of the decision-support layers: the server's tile ramps
// (crates/niche-geo/src/colormap.rs), and display names of the layers.

import type { AnalysisLayerId } from "@/api/types";

import type { useDecisionMessages } from "./messages";

export const PRIORITY_RAMP = ["#ffffcc", "#fd8d3c", "#e31a1c", "#542788"];
export const PRIORITY_CLASS_COLOURS = ["#d95f02", "#1b9e77", "#fdd49e", "#c7e9c0"];
export const DIVERGING = ["#2166ac", "#67a9cf", "#f7f7f7", "#ef8a62", "#b2182b"];
export const HOTSPOT = "#e7298a";

/** The analysis layer drawn on the map (one at a time). */
export interface ShownLayer {
  aid: string;
  layer: AnalysisLayerId;
}

export interface LayerProps {
  shown: ShownLayer | null;
  setShown: (s: ShownLayer | null) => void;
}

export function layerLabel(m: ReturnType<typeof useDecisionMessages>, id: AnalysisLayerId): string {
  switch (id) {
    case "priority":
      return m.layerPriority;
    case "priority_class":
      return m.layerPriorityClass;
    case "richness":
      return m.layerRichness;
    case "suitability_sum":
      return m.layerSum;
    case "hotspot":
      return m.layerHotspot;
    case "difference":
      return m.layerDifference;
    default:
      return id;
  }
}
