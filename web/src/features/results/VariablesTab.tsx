import type { Contribution, LayerInfo, ResponseCurve, SpeciesResult } from "@/api/types";
import { Section } from "@/components/domain/common";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { formatPercentValue } from "@/lib/format";
import { useVariableName } from "@/components/domain/layerNames";

import { useResultsMessages } from "./messages";

export function VariablesTab({ r, layers }: { r: SpeciesResult; layers: LayerInfo[] | undefined }) {
  const msg = useResultsMessages();
  // Results name variables by layer id (curves) or by label (contributions, used, dropped); both
  // resolve to the catalogue entry, shown with its unit (°C, m, °, fraction 0–1, km).
  const name = useVariableName(layers);
  // Collinearity reason of a dropped variable (crates/niche-app/src/fit.rs), in the interface language.
  const dropped = (reason: string) => {
    const m = reason.match(/^\|r\| = ([\d.]+) with (.+) \(threshold ([\d.]+)\)$/);
    return m ? msg.droppedReason(m[1] ?? "", name(m[2] ?? ""), m[3] ?? "") : reason;
  };
  const algorithm = r.selected.algorithm ?? "maxent";
  const sorted = [...r.contributions].sort((a, b) => b.percent - a.percent);
  // Bars by algorithm: MaxEnt percent contribution and permutation importance; RF permutation
  // importance (alone for "rf", where `percent` holds it); all three for the ensemble.
  const series: { key: string; label: string; color: string; get: (c: Contribution) => number | null | undefined }[] =
    algorithm === "rf"
      ? [{ key: "rf", label: msg.rfImportance, color: "bg-chart-3", get: (c) => c.rf_importance ?? c.percent }]
      : [
          { key: "pc", label: msg.percentContribution, color: "bg-chart-1", get: (c) => c.percent },
          ...(sorted.some((c) => c.permutation_importance != null)
            ? [{ key: "pi", label: msg.permutationImportance, color: "bg-chart-2", get: (c: Contribution) => c.permutation_importance }]
            : []),
          ...(sorted.some((c) => c.rf_importance != null)
            ? [{ key: "rf", label: msg.rfImportance, color: "bg-chart-3", get: (c: Contribution) => c.rf_importance }]
            : []),
        ];
  const max = Math.max(1, ...sorted.flatMap((c) => series.map((s) => s.get(c) ?? 0)));
  const curves = r.response_curves ?? [];
  const pd = r.partial_dependence ?? [];
  const note =
    algorithm === "rf" ? msg.contributionsNoteRf : algorithm === "ensemble" ? msg.contributionsNoteEnsemble : msg.contributionsNote;
  return (
    <div className="space-y-5">
      <Section title={algorithm === "maxent" ? msg.contributions : msg.importanceTitle} id="contrib">
        <p className="text-xs text-muted-foreground">{note}</p>
        {sorted.length === 0 ? (
          <p className="text-sm text-muted-foreground">{msg.noContributions}</p>
        ) : (
          <>
            <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground" aria-hidden>
              {series.map((s) => (
                <span key={s.key} className="flex items-center gap-1.5">
                  <span className={`inline-block h-2 w-3 rounded-sm ${s.color}`} /> {s.label}
                </span>
              ))}
            </div>
            <ul className="space-y-2.5">
              {sorted.map((c) => (
                <li key={c.variable} className="space-y-0.5">
                  <div className="flex items-baseline justify-between gap-2 text-sm">
                    <span className="min-w-0 truncate">{name(c.variable)}</span>
                    <span className="num shrink-0 text-muted-foreground">
                      {series
                        .map((s) => {
                          const v = s.get(c);
                          return v == null ? "–" : formatPercentValue(v);
                        })
                        .join(" · ")}
                    </span>
                  </div>
                  {series.map((s) => (
                    <div key={s.key} className="h-2 rounded-sm bg-muted" aria-hidden>
                      <div className={`h-2 rounded-sm ${s.color}`} style={{ width: `${((s.get(c) ?? 0) / max) * 100}%` }} />
                    </div>
                  ))}
                </li>
              ))}
            </ul>
          </>
        )}
      </Section>
      {curves.length > 0 && (
        <Section title={algorithm === "ensemble" ? msg.responseCurvesMaxent : msg.responseCurves} id="curves">
          <p className="text-xs text-muted-foreground">
            {msg.curvesNote}
          </p>
          <div className="grid grid-cols-1 gap-3">
            {curves.map((c) => (
              <ResponseCurveChart key={c.variable} curve={c} label={name(c.variable)} />
            ))}
          </div>
        </Section>
      )}
      {pd.length > 0 && (
        <Section title={msg.partialDependence} id="pd" action={<HelpTrigger target={{ article: "algorithms", section: "reading" }} />}>
          <p className="text-xs text-muted-foreground">{msg.pdNote}</p>
          <div className="grid grid-cols-1 gap-3">
            {pd.map((c) => (
              <ResponseCurveChart key={c.variable} curve={c} label={name(c.variable)} />
            ))}
          </div>
        </Section>
      )}
      <Section title={msg.variablesUsed(r.variables_used.length)} id="used">
        <ul className="list-disc pl-5 text-sm">
          {r.variables_used.map((v) => (
            <li key={v}>{name(v)}</li>
          ))}
        </ul>
      </Section>
      <Section title={msg.variablesDropped(r.variables_dropped.length)} id="dropped">
        {r.variables_dropped.length === 0 ? (
          <p className="text-sm text-muted-foreground">{msg.none}</p>
        ) : (
          <ul className="space-y-1 text-sm">
            {r.variables_dropped.map((v) => (
              <li key={v.variable}>
                <span className="font-medium">{name(v.variable)}</span>
                <span className="text-muted-foreground"> — {dropped(v.reason)}</span>
              </li>
            ))}
          </ul>
        )}
      </Section>
    </div>
  );
}

const W = 320;
const H = 150;
const PAD = { l: 30, r: 8, t: 8, b: 26 };

function tick(x: number): string {
  const a = Math.abs(x);
  return a >= 100 ? x.toFixed(0) : a >= 1 ? x.toFixed(1) : x.toFixed(2);
}

/** One marginal response curve; y fixed to 0–1 so curves are comparable across variables. */
function ResponseCurveChart({ curve, label }: { curve: ResponseCurve; label: string }) {
  const msg = useResultsMessages();
  const x0 = curve.x[0] ?? 0;
  const x1 = curve.x[curve.x.length - 1] ?? 1;
  const span = x1 > x0 ? x1 - x0 : 1;
  const px = (x: number) => PAD.l + ((x - x0) / span) * (W - PAD.l - PAD.r);
  const py = (y: number) => PAD.t + (1 - Math.min(1, Math.max(0, y))) * (H - PAD.t - PAD.b);
  const peak = curve.y.reduce((m, y, i) => (y > (curve.y[m] ?? -Infinity) ? i : m), 0);
  const summary = msg.curveSummary(
    label,
    tick(Math.min(...curve.y)),
    tick(Math.max(...curve.y)),
    tick(curve.x[peak] ?? x0),
    tick(curve.training_min),
    tick(curve.training_max),
  );
  return (
    <figure className="space-y-1 rounded-md border p-2">
      <figcaption className="truncate text-xs font-medium">{label}</figcaption>
      <svg viewBox={`0 0 ${W} ${H}`} className="h-auto w-full text-chart-1" role="img" aria-label={summary}>
        {!curve.categorical &&
          (
            [
              [x0, curve.training_min],
              [curve.training_max, x1],
            ] as const
          ).map(([a, b]) =>
            b > a ? (
              <rect key={a} x={px(a)} y={PAD.t} width={px(b) - px(a)} height={H - PAD.t - PAD.b} className="fill-muted" />
            ) : null,
          )}
        <line x1={PAD.l} y1={H - PAD.b} x2={W - PAD.r} y2={H - PAD.b} className="stroke-border" />
        <line x1={PAD.l} y1={PAD.t} x2={PAD.l} y2={H - PAD.b} className="stroke-border" />
        {[0, 0.5, 1].map((t) => (
          <text key={t} x={PAD.l - 4} y={py(t) + 3} textAnchor="end" className="fill-muted-foreground text-[9px]">
            {t}
          </text>
        ))}
        <text x={PAD.l} y={H - 8} className="fill-muted-foreground text-[9px]">
          {tick(x0)}
        </text>
        <text x={W - PAD.r} y={H - 8} textAnchor="end" className="fill-muted-foreground text-[9px]">
          {tick(x1)}
        </text>
        {curve.categorical ? (
          curve.x.map((x, i) => {
            const bw = Math.max(2, ((W - PAD.l - PAD.r) / curve.x.length) * 0.6);
            return (
              <rect
                key={x}
                x={px(x) - bw / 2}
                y={py(curve.y[i] ?? 0)}
                width={bw}
                height={py(0) - py(curve.y[i] ?? 0)}
                fill="currentColor"
              />
            );
          })
        ) : (
          <polyline
            points={curve.x.map((x, i) => `${px(x).toFixed(1)},${py(curve.y[i] ?? 0).toFixed(1)}`).join(" ")}
            fill="none"
            stroke="currentColor"
            strokeWidth={1.8}
          />
        )}
      </svg>
    </figure>
  );
}
