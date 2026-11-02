import type { ProjectionResult } from "@/api/types";
import { Ramp } from "@/components/domain/Legend";
import type { FutureKind } from "@/features/results/params";
import { CHANGE_COLOURS, majority } from "@/lib/future";
import { GREYS, VIRIDIS } from "@/lib/ramps";

import { useProjectionMessages } from "./messages";

/** Legend of a future projection layer (ADR-0016). */
export function FutureLegend({ p, kind, opacity, hatch }: { p: ProjectionResult; kind: FutureKind; opacity: number; hatch: boolean }) {
  const m = useProjectionMessages();
  const n = p.gcms.length;
  return (
    <div className="space-y-2 text-sm">
      <p className="num text-xs font-semibold tracking-wide text-muted-foreground uppercase">
        {p.period} · {m[`ssp_${p.ssp}`]}
      </p>
      {kind === "mean" && (
        <div className="space-y-1">
          <p className="font-medium">{m.layerMean}</p>
          <Ramp colours={VIRIDIS.map(([, c]) => c)} opacity={opacity} ticks={["0", "0.25", "0.5", "0.75", "1"]} label={m.layerMean} />
        </div>
      )}
      {kind === "sd" && (
        <div className="space-y-1">
          <p className="font-medium">{m.layerSd}</p>
          <Ramp colours={GREYS} opacity={opacity} ticks={["0", "0.125", "≥ 0.25"]} label={m.layerSd} />
        </div>
      )}
      {kind === "change" && (
        <div className="space-y-1">
          <p className="font-medium">{m.legendChange}</p>
          <ul className="space-y-0.5">
            {(
              [
                [CHANGE_COLOURS[3], m.stable],
                [CHANGE_COLOURS[2], m.gained],
                [CHANGE_COLOURS[1], m.lost],
                [CHANGE_COLOURS[0], m.unsuitable],
              ] as const
            ).map(([c, label]) => (
              <li key={label} className="flex items-center gap-2">
                <span aria-hidden className="inline-block size-3 shrink-0 rounded-sm border" style={{ background: c, opacity }} />
                {label}
              </li>
            ))}
          </ul>
          {n > 1 && <p className="text-xs text-muted-foreground">{m.legendFaded(majority(n), n)}</p>}
        </div>
      )}
      {hatch && (
        <div className="flex items-center gap-2">
          <span className="hatch-extrapolation size-4 shrink-0 rounded-sm border" aria-hidden />
          <span>{m.legendHatch}</span>
        </div>
      )}
    </div>
  );
}
