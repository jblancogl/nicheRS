// Study-area geometry helpers: GeoJSON upload validation (Zod), presets, area and bbox.
// All coordinates are WGS84 lon/lat (RFC 7946).

import { z } from "zod";

import type { Geometry, StudyArea } from "@/api/types";
import { geoMessages } from "@/features/setup/messages";

export const MAX_UPLOAD_BYTES = 5 * 1024 * 1024;
export const MAX_VERTICES = 50_000;

// Zod messages below are keys of geoMessages, translated when the issue is reported.
type GeoMessageKey = "position" | "wgs84" | "ring";

const position = z
  .array(z.number().finite())
  .min(2, "position" satisfies GeoMessageKey)
  .refine(
    ([lng, lat]) =>
      lng !== undefined && lat !== undefined && lng >= -180 && lng <= 180 && lat >= -90 && lat <= 90,
    "wgs84" satisfies GeoMessageKey,
  );
const ring = z.array(position).min(4, "ring" satisfies GeoMessageKey);
const polygon = z.object({ type: z.literal("Polygon"), coordinates: z.array(ring).min(1) });
const multiPolygon = z.object({
  type: z.literal("MultiPolygon"),
  coordinates: z.array(z.array(ring).min(1)).min(1),
});
const polygonal = z.discriminatedUnion("type", [polygon, multiPolygon]);
const anyGeometry = z.looseObject({ type: z.string() });
const feature = z.object({
  type: z.literal("Feature"),
  geometry: anyGeometry.nullable(),
  properties: z.record(z.string(), z.unknown()).nullable().optional(),
});
const featureCollection = z.object({
  type: z.literal("FeatureCollection"),
  features: z.array(feature),
});
const geojson = z.union([featureCollection, feature, anyGeometry]);

const to2d = (p: number[]): number[] => [p[0] ?? 0, p[1] ?? 0];

function normalise(g: z.infer<typeof polygonal>): Geometry {
  return g.type === "Polygon"
    ? { type: "Polygon", coordinates: g.coordinates.map((r) => r.map(to2d)) }
    : { type: "MultiPolygon", coordinates: g.coordinates.map((p) => p.map((r) => r.map(to2d))) };
}

export function vertexCount(g: Geometry): number {
  return g.type === "Polygon"
    ? g.coordinates.reduce((n, r) => n + r.length, 0)
    : g.coordinates.reduce((n, p) => n + p.reduce((m, r) => m + r.length, 0), 0);
}

export type ParseResult = { ok: true; geometry: Geometry; name: string | null } | { ok: false; error: string };

function firstIssue(err: z.ZodError): string {
  const m = geoMessages.get();
  const i = err.issues[0];
  if (!i) return m.invalid;
  return i.message === "position" || i.message === "wgs84" || i.message === "ring" ? m[i.message] : i.message;
}

/** Parse an uploaded GeoJSON text and return the first polygonal geometry. */
export function parseStudyAreaGeoJson(text: string): ParseResult {
  const m = geoMessages.get();
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return { ok: false, error: m.notJson };
  }
  const top = geojson.safeParse(raw);
  if (!top.success) return { ok: false, error: m.notGeoJson };

  const candidates: { geom: unknown; name: string | null }[] = [];
  const v = top.data;
  const nameOf = (props: Record<string, unknown> | null | undefined) => {
    const n = props?.["name"] ?? props?.["NAME"] ?? props?.["nom"];
    return typeof n === "string" ? n : null;
  };
  if (v.type === "FeatureCollection" && "features" in v && Array.isArray(v.features)) {
    for (const f of v.features as z.infer<typeof feature>[]) candidates.push({ geom: f.geometry, name: nameOf(f.properties) });
  } else if (v.type === "Feature" && "geometry" in v) {
    const f = v as z.infer<typeof feature>;
    candidates.push({ geom: f.geometry, name: nameOf(f.properties) });
  } else {
    candidates.push({ geom: v, name: null });
  }

  for (const c of candidates) {
    const t = (c.geom as { type?: unknown } | null)?.type;
    if (t !== "Polygon" && t !== "MultiPolygon") continue;
    const parsed = polygonal.safeParse(c.geom);
    if (!parsed.success) return { ok: false, error: firstIssue(parsed.error) };
    const geometry = normalise(parsed.data);
    const n = vertexCount(geometry);
    if (n > MAX_VERTICES)
      return {
        ok: false,
        error: m.tooManyVertices(n.toLocaleString("en-GB"), MAX_VERTICES.toLocaleString("en-GB")),
      };
    return { ok: true, geometry, name: c.name };
  }
  return { ok: false, error: m.noPolygon };
}

export function bboxOf(g: Geometry): [number, number, number, number] {
  let w = Infinity;
  let s = Infinity;
  let e = -Infinity;
  let n = -Infinity;
  const visit = (r: number[][]) => {
    for (const [x, y] of r) {
      if (x === undefined || y === undefined) continue;
      w = Math.min(w, x);
      e = Math.max(e, x);
      s = Math.min(s, y);
      n = Math.max(n, y);
    }
  };
  if (g.type === "Polygon") g.coordinates.forEach(visit);
  else g.coordinates.forEach((p) => p.forEach(visit));
  return [w, s, e, n];
}

const R = 6_371_008.8; // mean Earth radius, m
const rad = (d: number) => (d * Math.PI) / 180;

/** Ring area on the sphere (Chamberlain & Duquette 2007), m². */
function ringArea(r: number[][]): number {
  let a = 0;
  for (let i = 0; i < r.length - 1; i++) {
    const p1 = r[i];
    const p2 = r[i + 1];
    if (!p1 || !p2) continue;
    a += rad((p2[0] ?? 0) - (p1[0] ?? 0)) * (2 + Math.sin(rad(p1[1] ?? 0)) + Math.sin(rad(p2[1] ?? 0)));
  }
  return Math.abs((a * R * R) / 2);
}

function polygonArea(p: number[][][]): number {
  const [outer, ...holes] = p;
  if (!outer) return 0;
  return Math.max(0, ringArea(outer) - holes.reduce((s, h) => s + ringArea(h), 0));
}

/** Approximate geodesic area in km² (display only; the server computes the authoritative grid area). */
export function areaKm2(g: Geometry): number {
  const m2 = g.type === "Polygon" ? polygonArea(g.coordinates) : g.coordinates.reduce((s, p) => s + polygonArea(p), 0);
  return m2 / 1e6;
}

const bboxPolygon = (w: number, s: number, e: number, n: number): Geometry => ({
  type: "Polygon",
  coordinates: [
    [
      [w, s],
      [e, s],
      [e, n],
      [w, n],
      [w, s],
    ],
  ],
});

export interface AreaPreset {
  id: string;
  /** English display texts; the wizard shows the translations of usePresetMessages (by id). */
  label: string;
  note: string;
  area: StudyArea;
}

export const AREA_PRESETS: AreaPreset[] = [
  {
    id: "france",
    label: "France (metropolitan)",
    note: "Approximate outline, mainland only (Corsica excluded)",
    area: {
      name: "France metropolitan (approximate)",
      geometry: {
        type: "Polygon",
        coordinates: [
          [
            [-1.79, 43.37],
            [-1.25, 44.65],
            [-1.15, 46.2],
            [-2.2, 47.1],
            [-4.75, 47.85],
            [-4.75, 48.55],
            [-3.0, 48.85],
            [-1.6, 48.7],
            [-1.25, 49.7],
            [0.2, 49.5],
            [1.6, 50.15],
            [2.55, 51.08],
            [4.2, 49.95],
            [5.8, 49.55],
            [6.4, 49.45],
            [8.2, 48.98],
            [7.55, 47.6],
            [6.0, 46.15],
            [7.0, 45.9],
            [6.65, 45.1],
            [7.65, 44.15],
            [7.5, 43.78],
            [6.6, 43.15],
            [5.4, 43.2],
            [4.3, 43.45],
            [3.05, 43.1],
            [3.15, 42.45],
            [1.75, 42.5],
            [0.65, 42.85],
            [-0.75, 42.95],
            [-1.79, 43.37],
          ],
        ],
      },
    },
  },
  {
    id: "occitanie",
    label: "Occitanie",
    note: "Bounding box of the region (approximate)",
    area: { name: "Occitanie (approximate bbox)", geometry: bboxPolygon(-0.33, 42.33, 4.85, 45.05) },
  },
  {
    id: "bretagne",
    label: "Bretagne",
    note: "Bounding box of the region (approximate)",
    area: { name: "Bretagne (approximate bbox)", geometry: bboxPolygon(-5.15, 47.25, -1.0, 48.9) },
  },
];

export const DEMO_SPECIES = { key: 0, scientific_name: "Virtualis demonstrandum (SYNTHETIC)" } as const;
/** Second virtual species of the demo (server: demo::SECOND_SPECIES_KEY), for multi-species outputs. */
export const DEMO_SECOND_SPECIES = { key: 1_000_002, scientific_name: "Virtualis secundus (SYNTHETIC)" } as const;
/** The synthetic demo landscape covers 3.0–6.0 °E, 43.8–45.8 °N, inland (server: niche-app demo.rs). */
export const DEMO_AREA: StudyArea = {
  name: "Synthetic demo region, SE France (SYNTHETIC)",
  geometry: bboxPolygon(3.05, 43.85, 5.95, 45.75),
};
