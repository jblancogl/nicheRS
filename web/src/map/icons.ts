// Observation markers drawn on a canvas and registered as MapLibre images (no glyphs needed,
// so they also work with the offline fallback style). Shape AND colour separate the classes:
//   retained  ● white fill, near-black stroke (design-tokens --obs-presence)
//   excluded  × amber stroke with white halo (filtered occurrence)
//   generalised  dashed ring, magenta (coordinates blurred by the publisher; excluded)
//   thinned   small grey dot with white ring (removed by distance thinning)

import { OBS_COLOURS } from "@/lib/ramps";

export const ICON_RETAINED = "nichers-obs-retained";
export const ICON_EXCLUDED = "nichers-obs-excluded";
export const ICON_GENERALISED = "nichers-obs-generalised";
export const ICON_THINNED = "nichers-obs-thinned";
export const ICON_OWN = "nichers-obs-own";
export const ICON_VALIDATION = "nichers-obs-validation";
export const ICON_ABSENCE = "nichers-obs-absence";

export const ICON_VERTEX = "nichers-draw-vertex";

const PR = 2; // pixel ratio of generated images

function draw(size: number, paint: (ctx: CanvasRenderingContext2D, s: number) => void): ImageData | null {
  const c = document.createElement("canvas");
  c.width = size * PR;
  c.height = size * PR;
  const ctx = c.getContext("2d");
  if (!ctx) return null;
  ctx.scale(PR, PR);
  paint(ctx, size);
  return ctx.getImageData(0, 0, c.width, c.height);
}

export function makeIcons(): { id: string; data: ImageData; pixelRatio: number }[] {
  const out: { id: string; data: ImageData; pixelRatio: number }[] = [];
  const retained = draw(12, (ctx, s) => {
    ctx.beginPath();
    ctx.arc(s / 2, s / 2, 4, 0, Math.PI * 2);
    ctx.fillStyle = "#ffffff";
    ctx.fill();
    ctx.lineWidth = 1.5;
    ctx.strokeStyle = "#151618";
    ctx.stroke();
  });
  const excluded = draw(12, (ctx, s) => {
    const a = 3;
    const b = s - 3;
    const cross = () => {
      ctx.beginPath();
      ctx.moveTo(a, a);
      ctx.lineTo(b, b);
      ctx.moveTo(b, a);
      ctx.lineTo(a, b);
    };
    cross();
    ctx.lineCap = "round";
    ctx.lineWidth = 4;
    ctx.strokeStyle = "#ffffff";
    ctx.stroke();
    cross();
    ctx.lineWidth = 2;
    ctx.strokeStyle = OBS_COLOURS.excluded;
    ctx.stroke();
  });
  const generalised = draw(14, (ctx, s) => {
    ctx.beginPath();
    ctx.arc(s / 2, s / 2, 4.5, 0, Math.PI * 2);
    ctx.lineWidth = 3.5;
    ctx.strokeStyle = "#ffffff";
    ctx.stroke();
    ctx.setLineDash([2, 1.6]);
    ctx.lineWidth = 1.8;
    ctx.strokeStyle = OBS_COLOURS.generalised;
    ctx.stroke();
  });
  const thinned = draw(10, (ctx, s) => {
    ctx.beginPath();
    ctx.arc(s / 2, s / 2, 2.8, 0, Math.PI * 2);
    ctx.fillStyle = OBS_COLOURS.thinned;
    ctx.fill();
    ctx.lineWidth = 1.2;
    ctx.strokeStyle = "#ffffff";
    ctx.stroke();
  });
  // The organisation's data (ADR-0108): squares for its retained presences, teal diamonds for
  // independent validation sites (filled = present, hollow = absent).
  const own = draw(12, (ctx) => {
    ctx.fillStyle = "#ffffff";
    ctx.strokeStyle = "#151618";
    ctx.lineWidth = 1.5;
    ctx.fillRect(2.5, 2.5, 7, 7);
    ctx.strokeRect(2.5, 2.5, 7, 7);
  });
  const diamond = (ctx: CanvasRenderingContext2D, s: number) => {
    ctx.beginPath();
    ctx.moveTo(s / 2, 1.5);
    ctx.lineTo(s - 1.5, s / 2);
    ctx.lineTo(s / 2, s - 1.5);
    ctx.lineTo(1.5, s / 2);
    ctx.closePath();
  };
  const valPresence = draw(13, (ctx, s) => {
    diamond(ctx, s);
    ctx.fillStyle = OBS_COLOURS.validation;
    ctx.fill();
    ctx.lineWidth = 1.2;
    ctx.strokeStyle = "#ffffff";
    ctx.stroke();
  });
  const valAbsence = draw(13, (ctx, s) => {
    diamond(ctx, s);
    ctx.lineWidth = 3.2;
    ctx.strokeStyle = "#ffffff";
    ctx.stroke();
    diamond(ctx, s);
    ctx.lineWidth = 1.6;
    ctx.strokeStyle = OBS_COLOURS.validation;
    ctx.stroke();
  });
  const vertex = draw(12, (ctx, s) => {
    ctx.fillStyle = "#ffffff";
    ctx.strokeStyle = "#215d96";
    ctx.lineWidth = 2;
    ctx.fillRect(3, 3, s - 6, s - 6);
    ctx.strokeRect(3, 3, s - 6, s - 6);
  });
  if (retained) out.push({ id: ICON_RETAINED, data: retained, pixelRatio: PR });
  if (excluded) out.push({ id: ICON_EXCLUDED, data: excluded, pixelRatio: PR });
  if (generalised) out.push({ id: ICON_GENERALISED, data: generalised, pixelRatio: PR });
  if (thinned) out.push({ id: ICON_THINNED, data: thinned, pixelRatio: PR });
  if (own) out.push({ id: ICON_OWN, data: own, pixelRatio: PR });
  if (valPresence) out.push({ id: ICON_VALIDATION, data: valPresence, pixelRatio: PR });
  if (valAbsence) out.push({ id: ICON_ABSENCE, data: valAbsence, pixelRatio: PR });
  if (vertex) out.push({ id: ICON_VERTEX, data: vertex, pixelRatio: PR });
  return out;
}
