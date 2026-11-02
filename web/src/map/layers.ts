// Idempotent synchronisation of NicheRS overlays onto a MapLibre map.
// Called on every relevant store change and after every style (re)load, so a basemap/theme
// switch never loses the study area, points or model rasters.
// Layer stack (information-architecture §5): raster → extrapolation hatch → AOA hatch → mask → grid →
// [basemap labels] → study-area outline → observations → draw handles.

import type { GeoJSONSource, Map as MapLibreMap } from "maplibre-gl";

import { urls } from "@/api/client";
import type { Geometry, GridInfo, OccurrencePoint } from "@/api/types";
import type { AnalysisView, DrawState, RasterView } from "@/stores/map";
import type { ResolvedTheme } from "@/stores/theme";

import { ICON_ABSENCE, ICON_EXCLUDED, ICON_GENERALISED, ICON_OWN, ICON_RETAINED, ICON_THINNED, ICON_VALIDATION, ICON_VERTEX, makeIcons } from "./icons";

type FC = GeoJSON.FeatureCollection;
const EMPTY: FC = { type: "FeatureCollection", features: [] };

export interface OverlayState {
  studyArea: Geometry | null;
  points: OccurrencePoint[];
  showPoints: boolean;
  grid: GridInfo | null;
  raster: RasterView | null;
  analysis: AnalysisView | null;
  footprint: GeoJSON.GeoJSON | null;
  draw: DrawState;
  theme: ResolvedTheme;
}

const S = {
  area: "nr-area",
  mask: "nr-mask",
  grid: "nr-grid",
  points: "nr-points",
  draw: "nr-draw",
  raster: "nr-raster",
  extrap: "nr-extrap",
  aoa: "nr-aoa",
  analysis: "nr-analysis",
  footprint: "nr-footprint",
};

function footprintFeature(g: GeoJSON.GeoJSON | null): FC {
  if (!g) return EMPTY;
  if (g.type === "FeatureCollection") return g;
  if (g.type === "Feature") return { type: "FeatureCollection", features: [g] };
  return { type: "FeatureCollection", features: [{ type: "Feature", properties: {}, geometry: g }] };
}

function firstLabelLayer(map: MapLibreMap): string | undefined {
  return map.getStyle().layers.find((l) => l.type === "symbol" && !l.id.startsWith("nr-"))?.id;
}

function setData(map: MapLibreMap, id: string, data: FC) {
  const src = map.getSource<GeoJSONSource>(id);
  if (src) void src.setData(data);
  else map.addSource(id, { type: "geojson", data });
}

function areaFeature(g: Geometry | null): FC {
  return g ? { type: "FeatureCollection", features: [{ type: "Feature", properties: {}, geometry: g }] } : EMPTY;
}

/** World polygon with the study area's outer rings as holes: dims everything outside. */
function maskFeature(g: Geometry | null): FC {
  if (!g) return EMPTY;
  const outers = g.type === "Polygon" ? [g.coordinates[0]] : g.coordinates.map((p) => p[0]);
  const world = [
    [-180, -85],
    [180, -85],
    [180, 85],
    [-180, 85],
    [-180, -85],
  ];
  const holes = outers.filter((r): r is number[][] => Array.isArray(r)).map((r) => [...r].reverse());
  return {
    type: "FeatureCollection",
    features: [{ type: "Feature", properties: {}, geometry: { type: "Polygon", coordinates: [world, ...holes] } }],
  };
}

const MAX_GRID_LINES = 400;

function gridFeature(grid: GridInfo | null): FC {
  if (!grid) return EMPTY;
  const [w, s, e, n] = grid.bounds;
  const lines: GeoJSON.Position[][] = [];
  if (grid.width + grid.height <= MAX_GRID_LINES) {
    const dx = (e - w) / grid.width;
    const dy = (n - s) / grid.height;
    for (let i = 0; i <= grid.width; i++) lines.push([[w + i * dx, s], [w + i * dx, n]]);
    for (let j = 0; j <= grid.height; j++) lines.push([[w, s + j * dy], [e, s + j * dy]]);
  } else {
    lines.push([[w, s], [e, s], [e, n], [w, n], [w, s]]);
  }
  return { type: "FeatureCollection", features: [{ type: "Feature", properties: {}, geometry: { type: "MultiLineString", coordinates: lines } }] };
}

const STATUS_RANK: Record<string, number> = { own: 3, retained: 3, validation: 3, validation_absence: 2, absence: 2, thinned: 2, generalised: 1 };

/** Marker kind: the status, except the organisation's retained presences (ADR-0108). */
const kindOf = (p: OccurrencePoint) => (p.status === "retained" && p.source === "own" ? "own" : p.status);

function pointsFeature(points: OccurrencePoint[], show: boolean): FC {
  if (!show) return EMPTY;
  return {
    type: "FeatureCollection",
    features: points.map((p) => ({
      type: "Feature",
      // Draw order: excluded below generalised below thinned below retained.
      properties: { retained: p.status === "retained" ? 1 : 0, rank: STATUS_RANK[kindOf(p)] ?? 0, status: kindOf(p) },
      geometry: { type: "Point", coordinates: [p.lng, p.lat] },
    })),
  };
}

function drawFeature(d: DrawState): FC {
  if (!d.active || d.vertices.length === 0) return EMPTY;
  const features: GeoJSON.Feature[] = d.vertices.map((v) => ({
    type: "Feature",
    properties: { kind: "vertex" },
    geometry: { type: "Point", coordinates: v },
  }));
  if (d.vertices.length >= 2) {
    const ring = d.vertices.length >= 3 ? [...d.vertices, d.vertices[0] as [number, number]] : d.vertices;
    features.push({ type: "Feature", properties: { kind: "line" }, geometry: { type: "LineString", coordinates: ring } });
  }
  return { type: "FeatureCollection", features };
}

export function ensureImages(map: MapLibreMap) {
  for (const icon of makeIcons()) {
    if (!map.hasImage(icon.id)) map.addImage(icon.id, icon.data, { pixelRatio: icon.pixelRatio });
  }
}

/** Tile URL currently attached to each of our raster sources. */
const rasterUrls = new Map<string, string>();

function syncRaster(map: MapLibreMap, id: string, tiles: string | null, opacity: number, before: string | undefined) {
  const layerId = `${id}-layer`;
  const existing = map.getSource(id);
  const currentUrl = existing ? rasterUrls.get(id) : undefined;
  if (!tiles) {
    if (map.getLayer(layerId)) map.removeLayer(layerId);
    if (existing) map.removeSource(id);
    return;
  }
  if (existing && currentUrl !== tiles) {
    if (map.getLayer(layerId)) map.removeLayer(layerId);
    map.removeSource(id);
  }
  if (!map.getSource(id)) {
    map.addSource(id, { type: "raster", tiles: [tiles], tileSize: 256, minzoom: 0, maxzoom: 14 });
    rasterUrls.set(id, tiles);
  }
  if (!map.getLayer(layerId)) {
    map.addLayer(
      { id: layerId, type: "raster", source: id, paint: { "raster-opacity": opacity, "raster-fade-duration": 0, "raster-resampling": "nearest" } },
      before && map.getLayer(before) ? before : undefined,
    );
  } else {
    map.setPaintProperty(layerId, "raster-opacity", opacity);
  }
}

/** Caller guarantees the style has fired "style.load" (Map.isStyleLoaded() also waits for tiles). */
export function syncOverlays(map: MapLibreMap, st: OverlayState) {
  ensureImages(map);
  const labels = firstLabelLayer(map);
  const before = (id: string) => (map.getLayer(id) ? id : labels);
  const dark = st.theme === "dark";
  const fg = dark ? "#f1f2f4" : "#151618";
  const halo = dark ? "#0c0d0f" : "#ffffff";

  // Model rasters (below everything we draw, below basemap labels).
  const r = st.raster;
  syncRaster(
    map,
    S.raster,
    r ? urls.tiles(r.jobId, r.speciesKey, r.layer, r.layer === "binary" ? r.threshold : undefined) : null,
    r ? r.opacity : 1,
    before(`${S.extrap}-layer`) ?? labels,
  );
  syncRaster(
    map,
    S.extrap,
    r && r.extrapolationOverlay && r.layer !== "extrapolation" ? urls.tiles(r.jobId, r.speciesKey, r.extrapolationLayer ?? "extrapolation") : null,
    1,
    labels,
  );
  syncRaster(map, S.aoa, r && r.aoaOverlay ? urls.tiles(r.jobId, r.speciesKey, "aoa") : null, 1, labels);
  // Decision-support layer (ADR-0018): above the model raster and its hatching, below labels.
  const an = st.analysis;
  syncRaster(map, S.analysis, an ? urls.analysisTiles(an.jobId, an.analysisId, an.layer) : null, an ? an.opacity : 1, labels);

  // Study-area mask and outline.
  setData(map, S.mask, maskFeature(st.studyArea));
  if (!map.getLayer(`${S.mask}-fill`))
    map.addLayer({ id: `${S.mask}-fill`, type: "fill", source: S.mask, paint: { "fill-color": "#808080", "fill-opacity": 0.35 } }, labels);

  setData(map, S.grid, gridFeature(st.grid));
  if (!map.getLayer(`${S.grid}-line`))
    map.addLayer({ id: `${S.grid}-line`, type: "line", source: S.grid, paint: { "line-color": fg, "line-opacity": 0.25, "line-width": 0.5 } }, labels);

  setData(map, S.area, areaFeature(st.studyArea));
  if (!map.getLayer(`${S.area}-halo`))
    map.addLayer({ id: `${S.area}-halo`, type: "line", source: S.area, paint: { "line-color": halo, "line-width": 3.5 } });
  if (!map.getLayer(`${S.area}-line`))
    map.addLayer({ id: `${S.area}-line`, type: "line", source: S.area, paint: { "line-color": fg, "line-width": 1.5 } });
  map.setPaintProperty(`${S.area}-halo`, "line-color", halo);
  map.setPaintProperty(`${S.area}-line`, "line-color", fg);
  map.setPaintProperty(`${S.grid}-line`, "line-color", fg);

  // Project footprint (decision support): magenta outline over a white halo.
  setData(map, S.footprint, footprintFeature(st.footprint));
  if (!map.getLayer(`${S.footprint}-halo`))
    map.addLayer({ id: `${S.footprint}-halo`, type: "line", source: S.footprint, paint: { "line-color": halo, "line-width": 4 } });
  if (!map.getLayer(`${S.footprint}-line`))
    map.addLayer({ id: `${S.footprint}-line`, type: "line", source: S.footprint, paint: { "line-color": "#e7298a", "line-width": 2 } });
  map.setPaintProperty(`${S.footprint}-halo`, "line-color", halo);

  // Observations: shape + colour coded, excluded drawn below retained.
  setData(map, S.points, pointsFeature(st.points, st.showPoints));
  if (!map.getLayer(`${S.points}-sym`))
    map.addLayer({
      id: `${S.points}-sym`,
      type: "symbol",
      source: S.points,
      layout: {
        "icon-image": [
          "match",
          ["get", "status"],
          "retained",
          ICON_RETAINED,
          "generalised",
          ICON_GENERALISED,
          "thinned",
          ICON_THINNED,
          "own",
          ICON_OWN,
          "validation",
          ICON_VALIDATION,
          "validation_absence",
          ICON_ABSENCE,
          "absence",
          ICON_ABSENCE,
          ICON_EXCLUDED,
        ],
        "icon-allow-overlap": true,
        "icon-ignore-placement": true,
        "symbol-sort-key": ["get", "rank"],
      },
    });

  // Draw handles on top.
  setData(map, S.draw, drawFeature(st.draw));
  if (!map.getLayer(`${S.draw}-line`))
    map.addLayer({
      id: `${S.draw}-line`,
      type: "line",
      source: S.draw,
      filter: ["==", ["get", "kind"], "line"],
      paint: { "line-color": "#215d96", "line-width": 2, "line-dasharray": [2, 1] },
    });
  if (!map.getLayer(`${S.draw}-vtx`))
    map.addLayer({
      id: `${S.draw}-vtx`,
      type: "symbol",
      source: S.draw,
      filter: ["==", ["get", "kind"], "vertex"],
      layout: { "icon-image": ICON_VERTEX, "icon-allow-overlap": true, "icon-ignore-placement": true },
    });
}

export const NR_RASTER_SOURCES = [S.raster, S.extrap, S.aoa, S.analysis];
