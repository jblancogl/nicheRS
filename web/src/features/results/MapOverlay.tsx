import { ChevronDown, Layers } from "lucide-react";
import { useState } from "react";

import type { RasterLayer, SpeciesResult } from "@/api/types";
import { LayerLegend, ObservedLegend } from "@/components/domain/Legend";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Separator } from "@/components/ui/separator";
import { Slider } from "@/components/ui/slider";
import { cn } from "@/lib/utils";
import { useMapStore } from "@/stores/map";

import { FutureLegend } from "@/features/projections/FutureLegend";
import { useProjectionMessages } from "@/features/projections/messages";
import { projectionKey } from "@/lib/future";

import { useResultsMessages } from "./messages";
import type { FutureKind, ResultsParams } from "./params";

type LayerLabel = "layerSuitability" | "layerUncertainty" | "layerExtrapolation" | "layerBinary" | "layerEffort";

const LAYER_OPTIONS: { id: RasterLayer; label: LayerLabel }[] = [
  { id: "suitability", label: "layerSuitability" },
  { id: "uncertainty", label: "layerUncertainty" },
  { id: "extrapolation", label: "layerExtrapolation" },
  { id: "binary", label: "layerBinary" },
];
/** Observed sampling effort, after the modelled layers (only with a target-group effort surface). */
const EFFORT_OPTION: { id: RasterLayer; label: LayerLabel } = { id: "effort", label: "layerEffort" };

/** Floating layer control + legend on the map (Modelled radio group, Observed toggles). */
export function MapOverlay({
  result,
  params,
  update,
  occurrencesAvailable,
  simple = false,
}: {
  result: SpeciesResult;
  params: ResultsParams;
  update: (p: Partial<ResultsParams>) => void;
  occurrencesAvailable: boolean;
  /** Simple reading: the suitability layer with its hatch, without layer, opacity or hatch choices. */
  simple?: boolean;
}) {
  const msg = useResultsMessages();
  const pm = useProjectionMessages();
  const projections = simple ? [] : (result.projections ?? []);
  const projection = projections.find((p) => projectionKey(p) === params.proj);
  // Collapsed by default on narrow screens so the map stays visible (map is the primary surface).
  const [open, setOpen] = useState(() => !window.matchMedia("(max-width: 767px)").matches);
  // Narrow screens: the panel sits under the zoom buttons and must end above the bottom sheet.
  const sheetInset = useMapStore((s) => s.sheetInset);
  const threshold = result.thresholds.find((t) => t.rule === params.rule);
  const bias = result.sampling_bias;
  const options = bias ? [...LAYER_OPTIONS, EFFORT_OPTION] : LAYER_OPTIONS;
  return (
    <div
      className="absolute bottom-8 left-3 z-10 flex w-72 max-w-[calc(100%-1.5rem)] flex-col rounded-lg border bg-card shadow-md max-md:top-14 max-md:bottom-auto"
      style={sheetInset > 0 ? { maxHeight: `calc(100% - ${sheetInset}px - 4.5rem)` } : undefined}
    >
      <button
        type="button"
        className="flex w-full shrink-0 items-center justify-between gap-2 px-3 py-2 text-sm font-medium"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        <span className="inline-flex items-center gap-2">
          <Layers className="size-4" aria-hidden /> {msg.layersLegend}
        </span>
        <ChevronDown className={cn("size-4 transition-transform", open && "rotate-180")} aria-hidden />
      </button>
      {open && (
        <div className="min-h-0 space-y-3 overflow-y-auto border-t px-3 py-3 md:max-h-[60vh]">
          {!simple && (
            <>
          {projections.length > 0 && (
            <fieldset>
              <legend className="mb-1.5 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{pm.mapPeriod}</legend>
              <RadioGroup
                value={projection ? projectionKey(projection) : "present"}
                onValueChange={(v) => update({ proj: v === "present" ? null : (v as string) })}
                aria-label={pm.mapPeriod}
              >
                <Label className="flex items-center gap-2 font-normal">
                  <RadioGroupItem value="present" />
                  {pm.present}
                </Label>
                {projections.map((p) => (
                  <Label key={projectionKey(p)} className="num flex items-center gap-2 font-normal">
                    <RadioGroupItem value={projectionKey(p)} />
                    {p.period} · {pm[`ssp_${p.ssp}`]}
                  </Label>
                ))}
              </RadioGroup>
            </fieldset>
          )}
          <fieldset>
            <legend className="mb-1.5 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{msg.mapLayer}</legend>
            {projection ? (
              <RadioGroup value={params.flayer} onValueChange={(v) => update({ flayer: v as FutureKind })} aria-label={msg.mapLayer}>
                {(
                  [
                    ["change", pm.layerChange],
                    ["mean", pm.layerMean],
                    ["sd", pm.layerSd],
                  ] as const
                ).map(([id, label]) => (
                  <Label key={id} className="flex items-center gap-2 font-normal">
                    <RadioGroupItem value={id} />
                    {label}
                  </Label>
                ))}
              </RadioGroup>
            ) : (
            <RadioGroup value={params.layer} onValueChange={(v) => update({ layer: v as RasterLayer })} aria-label={msg.mapLayer}>
              {options.map((o) => (
                <Label key={o.id} className="flex items-center gap-2 font-normal">
                  <RadioGroupItem value={o.id} />
                  {msg[o.label]}
                </Label>
              ))}
            </RadioGroup>
            )}
          </fieldset>
          <div className="space-y-1.5">
            <Label className="flex items-center justify-between font-normal" id="opacity-label">
              {msg.opacity} <span className="num text-muted-foreground">{Math.round(params.opacity * 100)} %</span>
            </Label>
            <Slider
              aria-labelledby="opacity-label"
              min={0}
              max={100}
              step={5}
              value={[Math.round(params.opacity * 100)]}
              onValueChange={(v: number | readonly number[]) => {
                const n = typeof v === "number" ? v : v[0];
                if (typeof n === "number") update({ opacity: n / 100 });
              }}
            />
          </div>
            </>
          )}
          <div className="space-y-2">
            {!simple && (
            <Label className="flex min-h-6 items-center gap-2 font-normal">
              <Checkbox checked={params.ext} onCheckedChange={(c) => update({ ext: c })} disabled={!projection && params.layer === "extrapolation"} />
              {projection ? pm.hatching : msg.hatching}
            </Label>
            )}
            {!simple && !projection && result.aoa && (
              <Label className="flex min-h-6 items-center gap-2 font-normal">
                <Checkbox checked={params.aoa} onCheckedChange={(c) => update({ aoa: c })} />
                {msg.aoaHatching}
              </Label>
            )}
            <Label className="flex min-h-6 items-center gap-2 font-normal">
              <Checkbox checked={params.obs && occurrencesAvailable} onCheckedChange={(c) => update({ obs: c })} disabled={!occurrencesAvailable} />
              {msg.occurrences}
            </Label>
            {!occurrencesAvailable && (
              <p className="text-xs text-muted-foreground">{msg.noOccurrences}</p>
            )}
          </div>
          <Separator />
          {projection ? (
            <FutureLegend p={projection} kind={params.flayer} opacity={params.opacity} hatch={params.ext} />
          ) : (
          <LayerLegend
            layer={params.layer}
            opacity={params.opacity}
            threshold={threshold?.threshold}
            ruleLabel={threshold ? msg.ruleInline(threshold.rule) : undefined}
            showExtrapolation={params.ext}
            showAoa={!simple && params.aoa && Boolean(result.aoa)}
            effort={bias ? { maxCellRecords: bias.max_cell_records, synthetic: Boolean(bias.synthetic) } : undefined}
            algorithm={result.selected.algorithm}
          />
          )}
          {params.obs && occurrencesAvailable && <ObservedLegend />}
          <p className="text-xs text-muted-foreground">{msg.clickToRead}</p>
        </div>
      )}
    </div>
  );
}
