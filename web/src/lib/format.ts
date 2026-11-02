// formatScientific — every number shown in the UI goes through here (design-tokens §8).
// Rules follow the interface language (en-GB, fr-FR, es-ES: decimal point or comma, month names),
// with the thousands separator always a narrow no-break space (U+202F) as documented:
// "1 240 km²". Units are joined with a no-break space. Display only: nothing parses these strings.

import { useLang, type Lang } from "@/i18n/lang";

const NNBSP = " ";
const NBSP = " ";
export const EM_DASH = "—";

const LOCALES: Record<Lang, string> = { en: "en-GB", fr: "fr-FR", es: "es-ES" };

function locale(): string {
  return LOCALES[useLang.getState().lang];
}

function groupThin(parts: Intl.NumberFormatPart[]): string {
  return parts.map((p) => (p.type === "group" ? NNBSP : p.value)).join("");
}

function fmt(value: number, opts: Intl.NumberFormatOptions): string {
  // "negative": no sign on zero, so -0 and values rounding to zero never print as "-0".
  return groupThin(new Intl.NumberFormat(locale(), { signDisplay: "negative", ...opts }).formatToParts(value));
}

const missing = (v: number | null | undefined): v is null | undefined =>
  v === null || v === undefined || !Number.isFinite(v);

/** Integer count with thin-space grouping: 1 204. */
export function formatCount(v: number | null | undefined): string {
  if (missing(v)) return EM_DASH;
  return fmt(Math.round(v), { maximumFractionDigits: 0 });
}

/** A distance or size typed by the user (km), as given: up to 2 decimals, no padding (2.5, 10). */
export function formatKm(v: number | null | undefined): string {
  if (missing(v)) return EM_DASH;
  return fmt(v, { maximumFractionDigits: 2 });
}

/** Fixed decimals (scores, CBI, AUC, thresholds): 0.73. */
export function formatDecimal(v: number | null | undefined, digits = 2): string {
  if (missing(v)) return EM_DASH;
  return fmt(v, { minimumFractionDigits: digits, maximumFractionDigits: digits });
}

/** Suitability score, relative 0–1 — never a percentage. */
export const formatScore = (v: number | null | undefined) => formatDecimal(v, 2);

/** Percentage from a 0–1 fraction: 0 decimals, 1 decimal below 1 %. */
export function formatPercent(fraction: number | null | undefined): string {
  if (missing(fraction)) return EM_DASH;
  const pct = fraction * 100;
  const digits = pct !== 0 && Math.abs(pct) < 1 ? 1 : 0;
  return `${fmt(pct, { minimumFractionDigits: digits, maximumFractionDigits: digits })}${NBSP}%`;
}

/** Percentage already expressed in 0–100 (variable contributions). */
export function formatPercentValue(pct: number | null | undefined, digits = 1): string {
  if (missing(pct)) return EM_DASH;
  return `${fmt(pct, { minimumFractionDigits: digits, maximumFractionDigits: digits })}${NBSP}%`;
}

/** Round to n significant figures (area: 3 sf, no false precision). */
export function roundSig(v: number, sig = 3): number {
  if (v === 0 || !Number.isFinite(v)) return v;
  const d = Math.ceil(Math.log10(Math.abs(v)));
  const p = sig - d;
  const m = 10 ** p;
  return Math.round(v * m) / m;
}

/** Area number only (3 sf; < 1 km² switches to ha). */
export function formatAreaNumber(km2: number): { value: string; unit: "km²" | "ha" } {
  if (km2 < 1) {
    const ha = roundSig(km2 * 100, 3);
    return { value: fmt(ha, { maximumFractionDigits: 2 }), unit: "ha" };
  }
  const r = roundSig(km2, 3);
  return { value: fmt(r, { maximumFractionDigits: r < 10 ? 2 : r < 100 ? 1 : 0 }), unit: "km²" };
}

export function formatArea(km2: number | null | undefined): string {
  if (missing(km2)) return EM_DASH;
  const { value, unit } = formatAreaNumber(km2);
  return `${value}${NBSP}${unit}`;
}

/** "980–1 560 km²" */
export function formatAreaRange(lo: number, hi: number): string {
  if (missing(lo) || missing(hi)) return EM_DASH;
  const a = formatAreaNumber(lo);
  const b = formatAreaNumber(hi);
  if (a.unit === b.unit) return `${a.value}–${b.value}${NBSP}${b.unit}`;
  return `${formatArea(lo)}–${formatArea(hi)}`;
}

/** mean ± SD, SD rounded to one significant figure and the mean to the same decimal. */
export function formatMeanSd(values: (number | null)[], digits = 2): string {
  const xs = values.filter((v): v is number => v !== null && Number.isFinite(v));
  if (xs.length === 0) return EM_DASH;
  const mean = xs.reduce((a, b) => a + b, 0) / xs.length;
  if (xs.length < 2) return formatDecimal(mean, digits);
  const sd = Math.sqrt(xs.reduce((a, b) => a + (b - mean) ** 2, 0) / (xs.length - 1));
  return `${formatDecimal(mean, digits)} ± ${formatDecimal(sd, digits)}`;
}

/** Coordinates: 5 decimals with hemisphere. */
export function formatLngLat(lng: number, lat: number): string {
  const ew = lng >= 0 ? "E" : "W";
  const ns = lat >= 0 ? "N" : "S";
  return `${formatDecimal(Math.abs(lng), 5)}${NBSP}${ew}, ${formatDecimal(Math.abs(lat), 5)}${NBSP}${ns} (WGS${NBSP}84)`;
}

export function formatDateTime(iso: string | null | undefined): string {
  if (!iso) return EM_DASH;
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return new Intl.DateTimeFormat(locale(), {
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(d);
}

/** Calendar date only ("14 Oct 2026"); a YYYY-MM-DD string is read as that day, not UTC midnight. */
export function formatDate(iso: string | null | undefined): string {
  if (!iso) return EM_DASH;
  const day = /^\d{4}-\d{2}-\d{2}$/.test(iso) ? `${iso}T12:00:00` : iso;
  const d = new Date(day);
  if (Number.isNaN(d.getTime())) return iso;
  return new Intl.DateTimeFormat(locale(), { day: "numeric", month: "short", year: "numeric" }).format(d);
}

export function formatTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return new Intl.DateTimeFormat(locale(), { hour: "2-digit", minute: "2-digit", second: "2-digit" }).format(d);
}

/** "4 min 10 s" */
export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return EM_DASH;
  const s = Math.round(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}${NBSP}h ${m}${NBSP}min`;
  if (m > 0) return `${m}${NBSP}min ${sec}${NBSP}s`;
  return `${sec}${NBSP}s`;
}

/** Approximate ground size of a grid cell at the given resolution factor (30 arc-seconds base). */
export function resolutionLabel(factor: number): string {
  return `≈${NBSP}${factor}${NBSP}km (${factor * 30}″)`;
}

/** "1.2 GB", "340 MB" (decimal units, as storage is sold). */
export function formatBytes(bytes: number | null | undefined): string {
  if (missing(bytes)) return EM_DASH;
  const units = ["B", "kB", "MB", "GB", "TB"];
  let v = bytes;
  let i = 0;
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000;
    i += 1;
  }
  return `${fmt(v, { maximumFractionDigits: v < 10 && i > 0 ? 1 : 0 })}${NBSP}${units[i]}`;
}
