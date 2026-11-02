// Raster legend colours. Theme independent (design-tokens §3). The server renders the tiles;
// these stops document the same ramps for legends.

export const VIRIDIS: [number, string][] = [
  [0, "#440154"],
  [0.125, "#482878"],
  [0.25, "#3e4989"],
  [0.375, "#31688e"],
  [0.5, "#26828e"],
  [0.625, "#1f9e89"],
  [0.75, "#35b779"],
  [0.875, "#6ece58"],
  [1, "#fde725"],
];

export const GREYS: string[] = ["#fff2e8", "#e2d5cb", "#c5b8ae", "#a89c92", "#8d8177", "#72665e", "#584d45", "#40362e", "#291f18"];

export const BINARY_COLOUR = "#21918c";

/** Sampling effort (target-group records per cell): ColorBrewer Oranges #fee6ce → #f16913 →
 *  #7f2704 on ln(1 + v) / ln(1 + max_cell_records), as rendered by the server (colormap.rs);
 *  cells without records are transparent. Linear midpoints added for the legend. */
export const EFFORT_ORANGES: string[] = ["#fee6ce", "#f8a870", "#f16913", "#b8480c", "#7f2704"];

/** Occurrence marker colours (map/icons.ts draws the markers, Legend.tsx the swatches). */
export const OBS_COLOURS = { excluded: "#b45309", generalised: "#be185d", thinned: "#6b7280", validation: "#0f766e" } as const;
