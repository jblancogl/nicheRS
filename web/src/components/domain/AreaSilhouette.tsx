// Outline of a study area drawn from its GeoJSON (no map, no request): gives in-progress study
// cards something recognisable while there is no result to show.

import { useMemo } from "react";

import type { Geometry } from "@/api/types";

type Ring = number[][];

function rings(g: Geometry): Ring[] {
  if (g.type === "Polygon") return g.coordinates;
  if (g.type === "MultiPolygon") return (g.coordinates as Ring[][]).flat();
  return [];
}

/** SVG path of the area, equirectangular with the longitude scaled by cos(mean latitude). */
function outline(g: Geometry): { d: string; box: string } | null {
  const rs = rings(g).filter((r) => r.length >= 3);
  const pts = rs.flat();
  if (pts.length === 0) return null;
  const lats = pts.map((p) => p[1] ?? 0);
  const k = Math.cos(((Math.min(...lats) + Math.max(...lats)) / 2) * (Math.PI / 180));
  const xy = (p: number[]) => [(p[0] ?? 0) * k, -(p[1] ?? 0)] as const;
  const xs = pts.map((p) => xy(p)[0]);
  const ys = pts.map((p) => xy(p)[1]);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const pad = Math.max(x1 - x0, y1 - y0) * 0.04;
  // Thin outlines: skip vertices closer than ~1/600 of the extent (cards are a few hundred px).
  const eps = Math.max(x1 - x0, y1 - y0) / 600;
  const d = rs
    .map((r) => {
      let last: readonly [number, number] | null = null;
      const parts: string[] = [];
      for (const p of r) {
        const q = xy(p);
        if (last && Math.abs(q[0] - last[0]) < eps && Math.abs(q[1] - last[1]) < eps) continue;
        parts.push(`${parts.length ? "L" : "M"}${q[0].toFixed(4)} ${q[1].toFixed(4)}`);
        last = q;
      }
      return `${parts.join("")}Z`;
    })
    .join("");
  return { d, box: `${x0 - pad} ${y0 - pad} ${x1 - x0 + 2 * pad} ${y1 - y0 + 2 * pad}` };
}

export function AreaSilhouette({ geometry, className }: { geometry: Geometry; className?: string }) {
  const o = useMemo(() => outline(geometry), [geometry]);
  if (!o) return null;
  return (
    <svg viewBox={o.box} preserveAspectRatio="xMidYMid meet" className={className} aria-hidden>
      <path d={o.d} fill="currentColor" fillOpacity={0.1} fillRule="evenodd" stroke="currentColor" strokeOpacity={0.45} strokeWidth={1.2} vectorEffect="non-scaling-stroke" />
    </svg>
  );
}
