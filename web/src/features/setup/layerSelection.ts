import { useLayers } from "@/api/hooks";
import type { LayerGroup, LayerInfo } from "@/api/types";
import { useDraft } from "@/stores/draft";

export function defaultLayerIds(layers: LayerInfo[]): string[] {
  return layers.filter((l) => l.default_selected).map((l) => l.id);
}

export interface LayerGroupView {
  group: LayerGroup;
  layers: LayerInfo[];
  /** Caution shared by every layer of the group (shown once), if any. */
  sharedCaution: string | null;
}

const ORDER: LayerGroup[] = ["climate", "terrain", "land_cover", "water"];

/** Catalogue split by group (climate, terrain, land cover, water), in catalogue order. */
export function groupLayers(layers: LayerInfo[]): LayerGroupView[] {
  return ORDER.map((group) => {
    const ls = layers.filter((l) => (l.group ?? "climate") === group);
    // The caution most layers of the group share (e.g. the 2021 temporal mismatch of land cover);
    // layers with another caution (built-up) still show theirs.
    const counts = new Map<string, number>();
    for (const l of ls) if (l.caution) counts.set(l.caution, (counts.get(l.caution) ?? 0) + 1);
    let sharedCaution: string | null = null;
    let best = 1;
    for (const [c, n] of counts)
      if (n > best) {
        sharedCaution = c;
        best = n;
      }
    return { group, layers: ls, sharedCaution };
  }).filter((g) => g.layers.length > 0);
}

/** The effective selection: the user's list, or the server defaults until edited. */
export function useSelectedLayers(): { ids: string[]; layers: LayerInfo[] | undefined; isCustom: boolean } {
  const chosen = useDraft((s) => s.layers);
  const demo = useDraft((s) => s.demo);
  const q = useLayers(demo);
  const ids = chosen ?? (q.data ? defaultLayerIds(q.data) : []);
  return { ids, layers: q.data, isCustom: chosen !== null };
}
