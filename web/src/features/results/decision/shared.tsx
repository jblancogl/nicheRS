// Pieces shared by the Decision support sections: analysis files, map-layer toggles and legends.
// Legend colours match the server's tile ramps (crates/niche-geo/src/colormap.rs).

import { Download, Eye, EyeOff, FileText } from "lucide-react";

import { urls } from "@/api/client";
import type { Analysis, AnalysisLayer, AnalysisLayerId } from "@/api/types";
import { useWarningText } from "@/components/domain/warningMessages";
import { Button } from "@/components/ui/button";
import { formatDecimal } from "@/lib/format";
import { VIRIDIS } from "@/lib/ramps";

import { useDecisionMessages } from "./messages";
import { DIVERGING, HOTSPOT, PRIORITY_CLASS_COLOURS, PRIORITY_RAMP, layerLabel, type LayerProps } from "./style";

function Ramp({ colours, ticks, label }: { colours: string[]; ticks: string[]; label: string }) {
  return (
    <div>
      <div className="flex h-3 overflow-hidden rounded-sm border" role="img" aria-label={label}>
        {colours.map((c) => (
          <span key={c} className="flex-1" style={{ backgroundColor: c }} />
        ))}
      </div>
      <div className="num mt-0.5 flex justify-between text-xs text-muted-foreground" aria-hidden>
        {ticks.map((t, i) => (
          <span key={`${t}-${i}`}>{t}</span>
        ))}
      </div>
    </div>
  );
}

export function Swatch({ colour }: { colour: string }) {
  return <span className="inline-block size-3 shrink-0 rounded-sm border align-middle" style={{ backgroundColor: colour }} aria-hidden />;
}

export function LayerLegend({ layer }: { layer: AnalysisLayer }) {
  const m = useDecisionMessages();
  const label = layerLabel(m, layer.id);
  switch (layer.id) {
    case "priority":
      return <Ramp colours={PRIORITY_RAMP} ticks={[`0 ${m.priorityLow}`, `1 ${m.priorityHigh}`]} label={label} />;
    case "priority_class":
      return (
        <ul className="space-y-1 text-xs">
          {(["prospect_first", "known", "low_unsampled", "low_sampled"] as const).map((id, i) => (
            <li key={id} className="flex items-center gap-2">
              <Swatch colour={PRIORITY_CLASS_COLOURS[i] ?? "#ccc"} /> {m.className(id)}
            </li>
          ))}
        </ul>
      );
    case "hotspot":
      return (
        <p className="flex items-center gap-2 text-xs">
          <Swatch colour={HOTSPOT} /> {label}
        </p>
      );
    case "difference": {
      const mx = Math.max(Math.abs(layer.min), Math.abs(layer.max));
      return (
        <div className="space-y-0.5">
          <Ramp colours={DIVERGING} ticks={[`−${formatDecimal(mx)}`, "0", `+${formatDecimal(mx)}`]} label={label} />
          <p className="flex justify-between text-xs text-muted-foreground">
            <span>{m.lowerInB}</span>
            <span>{m.higherInB}</span>
          </p>
        </div>
      );
    }
    default:
      return (
        <Ramp
          colours={VIRIDIS.map((v) => v[1])}
          ticks={["0", layer.id === "suitability_sum" ? formatDecimal(layer.max) : String(Math.round(layer.max))]}
          label={label}
        />
      );
  }
}

/** Show/hide buttons for the layers of an analysis, with the legend of the one shown. */
export function LayerToggles({ analysis, shown, setShown, only }: LayerProps & { analysis: Analysis; only?: AnalysisLayerId[] }) {
  const m = useDecisionMessages();
  const layers = analysis.layers.filter((l) => (only ? only.includes(l.id) : l.id !== "contributing"));
  const active = layers.find((l) => shown?.aid === analysis.id && shown.layer === l.id);
  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-2">
        {layers.map((l) => {
          const on = active?.id === l.id;
          return (
            <Button
              key={l.id}
              size="sm"
              variant={on ? "default" : "outline"}
              aria-pressed={on}
              onClick={() => setShown(on ? null : { aid: analysis.id, layer: l.id })}
            >
              {on ? <EyeOff aria-hidden /> : <Eye aria-hidden />} {layerLabel(m, l.id)}
            </Button>
          );
        })}
      </div>
      {active && <LayerLegend layer={active} />}
    </div>
  );
}

/** Files of an analysis, its printable page and its manifest. */
export function AnalysisFiles({ analysis }: { analysis: Analysis }) {
  const m = useDecisionMessages();
  const names = [...analysis.files.map((f) => f.name).filter((n) => n !== "report.html"), "manifest.json"];
  return (
    <div className="space-y-1.5">
      <p className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">{m.downloads}</p>
      <div className="flex flex-wrap gap-2">
        {analysis.files.some((f) => f.name === "report.html") && (
          <Button size="sm" variant="outline" nativeButton={false} render={<a href={urls.analysisFile(analysis.job_id, analysis.id, "report.html", true)} target="_blank" rel="noreferrer" />}>
            <FileText aria-hidden /> {m.file("report.html")}
          </Button>
        )}
        {names.map((n) => (
          <Button key={n} size="sm" variant="ghost" nativeButton={false} render={<a href={urls.analysisFile(analysis.job_id, analysis.id, n)} download />}>
            <Download aria-hidden /> {m.file(n)}
          </Button>
        ))}
      </div>
    </div>
  );
}

/** Warnings of an analysis, worded in the interface language from their code and parameters. */
export function AnalysisWarnings({ analysis }: { analysis: Analysis }) {
  const text = useWarningText();
  const ws = analysis.warnings.filter((w) => w.code !== "synthetic");
  if (ws.length === 0) return null;
  return (
    <ul className="space-y-1 text-sm">
      {ws.map((w) => (
        <li key={w.code} className={w.severity === "warning" ? "text-state-warning" : "text-muted-foreground"}>
          {text(w)}
        </li>
      ))}
    </ul>
  );
}
