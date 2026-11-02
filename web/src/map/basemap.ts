// Basemap: OpenFreeMap (free, commercial use allowed; attribution kept via the style's sources).
// When the style cannot be fetched (offline, firewall), fall back to a plain background so the
// app keeps working — study areas, points and model rasters do not depend on the basemap.

import type { ExpressionSpecification, Map as MapLibreMap, StyleSpecification } from "maplibre-gl";

import type { Lang } from "@/i18n/lang";
import type { ResolvedTheme } from "@/stores/theme";

const STYLE_URL: Record<ResolvedTheme, string> = {
  light: "https://tiles.openfreemap.org/styles/positron",
  dark: "https://tiles.openfreemap.org/styles/dark",
};

const cache = new Map<ResolvedTheme, StyleSpecification>();

export function fallbackStyle(theme: ResolvedTheme): StyleSpecification {
  return {
    version: 8,
    sources: {},
    layers: [
      {
        id: "background",
        type: "background",
        paint: { "background-color": theme === "dark" ? "#1b1d20" : "#f2f3f0" },
      },
    ],
  };
}

function isStyle(v: unknown): v is StyleSpecification {
  return (
    typeof v === "object" &&
    v !== null &&
    (v as { version?: unknown }).version === 8 &&
    Array.isArray((v as { layers?: unknown }).layers)
  );
}

export async function loadBasemap(
  theme: ResolvedTheme,
): Promise<{ style: StyleSpecification; fallback: boolean }> {
  const hit = cache.get(theme);
  if (hit) return { style: hit, fallback: false };
  try {
    const res = await fetch(STYLE_URL[theme], { signal: AbortSignal.timeout(6000) });
    if (!res.ok) throw new Error(String(res.status));
    const json: unknown = await res.json();
    if (!isStyle(json)) throw new Error("invalid style");
    cache.set(theme, json);
    return { style: json, fallback: false };
  } catch {
    return { style: fallbackStyle(theme), fallback: true };
  }
}

/** Each basemap label layer's own text-field, captured once per loaded style. */
const labelFields = new WeakMap<MapLibreMap, Map<string, unknown>>();

/** Call on "style.load": the new style's text-fields become the originals. */
export function resetLabelFields(map: MapLibreMap) {
  labelFields.delete(map);
}

/**
 * Place labels in the interface language. OpenMapTiles vector tiles carry `name:fr`/`name:es`;
 * French and Spanish prefer them and fall back to the style's own expression (`name:latin`,
 * `name_en`, `name`). English keeps the style unchanged. Only layout properties change, so the
 * style is not reloaded and our overlays stay in place.
 */
export function localizeLabels(map: MapLibreMap, lang: Lang) {
  let orig = labelFields.get(map);
  if (!orig) {
    orig = new Map();
    labelFields.set(map, orig);
  }
  for (const l of map.getStyle().layers) {
    if (l.type !== "symbol" || l.id.startsWith("nr-")) continue;
    if (!orig.has(l.id)) orig.set(l.id, map.getLayoutProperty(l.id, "text-field"));
    const base = orig.get(l.id) as ExpressionSpecification | undefined;
    if (!Array.isArray(base) || !JSON.stringify(base).includes('"name')) continue;
    map.setLayoutProperty(l.id, "text-field", lang === "en" ? base : ["coalesce", ["get", `name:${lang}`], base]);
  }
}
