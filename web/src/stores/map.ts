// Map view state shared between the persistent map and the contextual panel.
// Zustand is justified: the MapLibre canvas and the side-panel trees are siblings; panels
// declare *what* to show, MapCanvas owns *how* (it is the only code calling MapLibre APIs).

import { create } from "zustand";

import type { Geometry, GridInfo, OccurrencePoint, RasterLayer } from "@/api/types";

export interface RasterView {
  jobId: string;
  speciesKey: number;
  /** A present layer, or a future projection layer `future_<period>_<ssp>_<kind>` (ADR-0016). */
  layer: RasterLayer | `future_${string}`;
  /** Layer of the extrapolation hatch: present MESS by default, the projection's future MESS. */
  extrapolationLayer?: string;
  threshold?: number;
  opacity: number; // 0..1
  extrapolationOverlay: boolean;
  /** Hatch cells outside the area of applicability (ADR-0014). */
  aoaOverlay?: boolean;
  /** DI threshold of the area of applicability (point readings). */
  aoaThreshold?: number;
}

/** A layer of a decision-support analysis drawn above the model raster (ADR-0018). */
export interface AnalysisView {
  jobId: string;
  analysisId: string;
  layer: string;
  opacity: number;
}

export interface DrawState {
  active: boolean;
  vertices: [number, number][];
}

export type DrawFinish = (ring: [number, number][]) => void;
/** Receives a clicked point while a panel picks places on the map. */
export type PickHandler = (lng: number, lat: number) => void;

interface MapState {
  studyArea: Geometry | null;
  points: OccurrencePoint[];
  showPoints: boolean;
  grid: GridInfo | null;
  raster: RasterView | null;
  analysis: AnalysisView | null;
  /** Project footprint outline (decision support). */
  footprint: GeoJSON.GeoJSON | null;
  draw: DrawState;
  onDrawFinish: DrawFinish | null;
  pick: PickHandler | null;
  inspect: { lng: number; lat: number } | null;
  fitRequest: { bounds: [number, number, number, number]; nonce: number } | null;
  basemapFallback: boolean;
  /** Height in px of the map covered by the narrow-screen bottom sheet (0 on wide screens). */
  sheetInset: number;

  setStudyArea: (g: Geometry | null) => void;
  setPoints: (p: OccurrencePoint[]) => void;
  setShowPoints: (v: boolean) => void;
  setGrid: (g: GridInfo | null) => void;
  setRaster: (r: RasterView | null) => void;
  setAnalysis: (a: AnalysisView | null) => void;
  setFootprint: (g: GeoJSON.GeoJSON | null) => void;
  startDraw: (onFinish: DrawFinish) => void;
  addVertex: (v: [number, number]) => void;
  undoVertex: () => void;
  cancelDraw: () => void;
  /** Ends drawing; hands the closed ring to the onFinish callback (ignored when < 3 vertices). */
  finishDraw: () => void;
  setInspect: (p: { lng: number; lat: number } | null) => void;
  /** While set (and not drawing), map clicks go to this handler instead of the inspector. */
  setPick: (h: PickHandler | null) => void;
  fitTo: (b: [number, number, number, number]) => void;
  setBasemapFallback: (v: boolean) => void;
  setSheetInset: (px: number) => void;
}

export const useMapStore = create<MapState>()((set, get) => ({
  studyArea: null,
  points: [],
  showPoints: true,
  grid: null,
  raster: null,
  analysis: null,
  footprint: null,
  draw: { active: false, vertices: [] },
  onDrawFinish: null,
  pick: null,
  inspect: null,
  fitRequest: null,
  basemapFallback: false,
  sheetInset: 0,

  setStudyArea: (studyArea) => set({ studyArea }),
  setPoints: (points) => set({ points }),
  setShowPoints: (showPoints) => set({ showPoints }),
  setGrid: (grid) => set({ grid }),
  setRaster: (raster) => set({ raster }),
  setAnalysis: (analysis) => set({ analysis }),
  setFootprint: (footprint) => set({ footprint }),
  startDraw: (onDrawFinish) => set({ draw: { active: true, vertices: [] }, onDrawFinish, inspect: null }),
  addVertex: (v) => set((s) => ({ draw: { ...s.draw, vertices: [...s.draw.vertices, v] } })),
  undoVertex: () => set((s) => ({ draw: { ...s.draw, vertices: s.draw.vertices.slice(0, -1) } })),
  cancelDraw: () => set({ draw: { active: false, vertices: [] }, onDrawFinish: null }),
  finishDraw: () => {
    const { draw, onDrawFinish } = get();
    // the two clicks of a double-click land on the same spot: drop consecutive duplicates
    const vs = draw.vertices.filter((v, i, arr) => {
      const p = arr[i - 1];
      return !p || Math.abs(p[0] - v[0]) > 1e-5 || Math.abs(p[1] - v[1]) > 1e-5;
    });
    if (vs.length < 3) return; // keep drawing; the toolbar explains the minimum
    set({ draw: { active: false, vertices: [] }, onDrawFinish: null });
    const first = vs[0];
    if (first && onDrawFinish) onDrawFinish([...vs, first]);
  },
  setInspect: (inspect) => set({ inspect }),
  setPick: (pick) => set({ pick }),
  fitTo: (bounds) => set({ fitRequest: { bounds, nonce: Date.now() } }),
  setBasemapFallback: (basemapFallback) => set({ basemapFallback }),
  setSheetInset: (sheetInset) => set({ sheetInset }),
}));
