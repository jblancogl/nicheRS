// Map legend per raster layer + a separate "Observed" section (P5: observations ≠ predictions).
// Swatches are drawn as discrete segments of the documented stops (no decorative gradients).

import type { RasterLayer } from "@/api/types";
import { formatCount, formatScore } from "@/lib/format";
import { BINARY_COLOUR, EFFORT_ORANGES, GREYS, OBS_COLOURS, VIRIDIS } from "@/lib/ramps";
import { cn } from "@/lib/utils";
import { useMapStore } from "@/stores/map";

import { useDomainMessages } from "./messages";

export function Ramp({ colours, opacity, ticks, label }: { colours: string[]; opacity: number; ticks: string[]; label: string }) {
  return (
    <div>
      <div className="flex h-3 overflow-hidden rounded-sm border" role="img" aria-label={label} style={{ opacity }}>
        {colours.map((c) => (
          <span key={c} className="flex-1" style={{ backgroundColor: c }} />
        ))}
      </div>
      <div className="num mt-0.5 flex justify-between text-xs text-muted-foreground" aria-hidden>
        {ticks.map((t) => (
          <span key={t}>{t}</span>
        ))}
      </div>
    </div>
  );
}

export function LayerLegend({
  layer,
  opacity,
  threshold,
  ruleLabel,
  showExtrapolation,
  showAoa = false,
  effort,
  algorithm,
  className,
}: {
  layer: RasterLayer;
  opacity: number;
  threshold?: number;
  ruleLabel?: string;
  showExtrapolation: boolean;
  /** Area-of-applicability hatching (ADR-0014). */
  showAoa?: boolean;
  /** Sampling-effort layer scale (ADR-0013): highest records in one cell, synthetic surface. */
  effort?: { maxCellRecords: number; synthetic: boolean };
  /** Modelling algorithm of the result (ADR-0017): names what the suitability score is. */
  algorithm?: string | null;
  className?: string;
}) {
  const m = useDomainMessages();
  return (
    <div className={cn("space-y-2 text-sm", className)} aria-label={m.mapLegend}>
      <p className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">{m.modelled}</p>
      {layer === "suitability" && (
        <div className="space-y-1">
          <p className="font-medium">{m.suitabilityTitle(algorithm)}</p>
          <Ramp
            colours={VIRIDIS.map(([, c]) => c)}
            opacity={opacity}
            ticks={["0", "0.25", "0.5", "0.75", "1"]}
            label={m.suitabilityRamp}
          />
          <details className="text-xs text-muted-foreground">
            <summary className="cursor-pointer">{m.whatMeans}</summary>
            <p className="mt-1">{m.suitabilityExplain}</p>
          </details>
        </div>
      )}
      {layer === "uncertainty" && (
        <div className="space-y-1">
          <p className="font-medium">{m.uncertaintyTitle}</p>
          <Ramp colours={GREYS} opacity={opacity} ticks={["0", "0.125", "≥ 0.25"]} label={m.uncertaintyRamp} />
          <p className="text-xs text-muted-foreground">{m.uncertaintyExplain}</p>
        </div>
      )}
      {layer === "extrapolation" && (
        <div className="flex items-center gap-2">
          <span className="hatch-extrapolation size-4 shrink-0 rounded-sm border" aria-hidden />
          <span>{m.extrapolation}</span>
        </div>
      )}
      {layer === "binary" && (
        <div className="flex items-center gap-2">
          <span className="size-4 shrink-0 rounded-sm border" style={{ backgroundColor: BINARY_COLOUR, opacity: 0.85 * opacity }} aria-hidden />
          <span>
            {m.suitableAt} <span className="num">{formatScore(threshold)}</span>
            {ruleLabel ? ` (${ruleLabel})` : ""}
          </span>
        </div>
      )}
      {layer === "effort" && effort && (
        <div className="space-y-1">
          <p className="font-medium">
            {m.effortTitle}
            {effort.synthetic && <span className="ml-1 text-xs font-normal text-state-warning">({m.syntheticLower})</span>}
          </p>
          <Ramp
            colours={EFFORT_ORANGES}
            opacity={opacity}
            ticks={["1", formatCount(effort.maxCellRecords)]}
            label={m.effortRamp(formatCount(effort.maxCellRecords))}
          />
          <p className="text-xs text-muted-foreground">{m.effortScale(formatCount(effort.maxCellRecords))}</p>
          <p className="text-xs text-muted-foreground">{m.effortNote}</p>
        </div>
      )}
      {showExtrapolation && layer !== "extrapolation" && (
        <div className="flex items-center gap-2">
          <span className="hatch-extrapolation size-4 shrink-0 rounded-sm border" aria-hidden />
          <span>{m.hatched}</span>
        </div>
      )}
      {showAoa && (
        <div className="flex items-center gap-2">
          <span className="hatch-aoa size-4 shrink-0 rounded-sm border" aria-hidden />
          <span>{m.aoaHatched}</span>
        </div>
      )}
    </div>
  );
}

export function ObservedLegend({ excluded = true }: { excluded?: boolean }) {
  const m = useDomainMessages();
  // Generalised and thinned markers are listed only when the map shows such points.
  const hasGeneralised = useMapStore((s) => s.points.some((p) => p.status === "generalised"));
  const hasThinned = useMapStore((s) => s.points.some((p) => p.status === "thinned"));
  // The organisation's data (ADR-0108).
  const hasOwn = useMapStore((s) => s.points.some((p) => p.source === "own" && p.status === "retained"));
  const hasValidation = useMapStore((s) => s.points.some((p) => p.status === "validation"));
  const hasAbsence = useMapStore((s) => s.points.some((p) => p.status === "validation_absence" || p.status === "absence"));
  return (
    <div className="space-y-1 text-sm">
      <p className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">{m.observed}</p>
      <div className="flex items-center gap-2">
        <svg className="size-3.5" viewBox="0 0 12 12" aria-hidden>
          <circle cx="6" cy="6" r="4" fill="#fff" stroke="#151618" strokeWidth="1.5" />
        </svg>
        <span>{m.retained}</span>
      </div>
      {excluded && (
        <div className="flex items-center gap-2">
          <svg className="size-3.5" viewBox="0 0 12 12" aria-hidden>
            <path d="M3 3l6 6M9 3l-6 6" stroke="#fff" strokeWidth="4" strokeLinecap="round" />
            <path d="M3 3l6 6M9 3l-6 6" stroke={OBS_COLOURS.excluded} strokeWidth="2" strokeLinecap="round" />
          </svg>
          <span>{m.excluded}</span>
        </div>
      )}
      {excluded && hasGeneralised && (
        <div className="flex items-center gap-2">
          <svg className="size-3.5" viewBox="0 0 14 14" aria-hidden>
            <circle cx="7" cy="7" r="4.5" fill="none" stroke="#fff" strokeWidth="3.5" />
            <circle cx="7" cy="7" r="4.5" fill="none" stroke={OBS_COLOURS.generalised} strokeWidth="1.8" strokeDasharray="2 1.6" />
          </svg>
          <span>{m.generalised}</span>
        </div>
      )}
      {hasOwn && (
        <div className="flex items-center gap-2">
          <svg className="size-3.5" viewBox="0 0 12 12" aria-hidden>
            <rect x="2.5" y="2.5" width="7" height="7" fill="#fff" stroke="#151618" strokeWidth="1.5" />
          </svg>
          <span>{m.ownRetained}</span>
        </div>
      )}
      {hasValidation && (
        <div className="flex items-center gap-2">
          <svg className="size-3.5" viewBox="0 0 13 13" aria-hidden>
            <path d="M6.5 1.5L11.5 6.5L6.5 11.5L1.5 6.5Z" fill={OBS_COLOURS.validation} stroke="#fff" strokeWidth="1.2" />
          </svg>
          <span>{m.validationPresence}</span>
        </div>
      )}
      {hasAbsence && (
        <div className="flex items-center gap-2">
          <svg className="size-3.5" viewBox="0 0 13 13" aria-hidden>
            <path d="M6.5 1.5L11.5 6.5L6.5 11.5L1.5 6.5Z" fill="none" stroke={OBS_COLOURS.validation} strokeWidth="1.6" />
          </svg>
          <span>{m.validationAbsence}</span>
        </div>
      )}
      {excluded && hasThinned && (
        <div className="flex items-center gap-2">
          <svg className="size-3.5" viewBox="0 0 12 12" aria-hidden>
            <circle cx="6" cy="6" r="2.8" fill={OBS_COLOURS.thinned} stroke="#fff" strokeWidth="1.2" />
          </svg>
          <span>{m.thinned}</span>
        </div>
      )}
    </div>
  );
}
