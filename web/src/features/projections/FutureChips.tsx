import { ArrowRight } from "lucide-react";

import type { SpeciesResult } from "@/api/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { formatArea, formatAreaRange } from "@/lib/format";
import { gcmRange, projectionKey } from "@/lib/future";

import { useProjectionMessages } from "./messages";

/** Summary tab: present vs future suitable area per scenario, with the spread across models. */
export function FutureChips({ r, onOpen }: { r: SpeciesResult; onOpen: () => void }) {
  const m = useProjectionMessages();
  const projections = r.projections ?? [];
  if (projections.length === 0) return null;
  return (
    <section className="space-y-2 rounded-lg border p-3" aria-labelledby="future-chips">
      <h3 id="future-chips" className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
        {m.title}
      </h3>
      <ul className="space-y-1.5">
        {projections.map((p) => {
          const [lo, hi] = gcmRange(p, (g) => g.future_suitable_km2);
          return (
            <li key={projectionKey(p)} className="space-y-0.5 text-sm">
              <span className="num block text-xs text-muted-foreground">
                {p.period} · {m[`ssp_${p.ssp}`]}
              </span>
              <Badge variant="secondary" className="num whitespace-normal">
                {m.chip(formatArea(p.present_suitable_km2), formatArea(p.future_suitable_km2))}
              </Badge>{" "}
              <span className="num text-xs text-muted-foreground">{m.rangeGcm(formatAreaRange(lo, hi))}</span>
            </li>
          );
        })}
      </ul>
      {projections.some((p) => p.synthetic) && <p className="text-xs text-state-warning">{m.synthetic}</p>}
      <Button type="button" variant="ghost" size="sm" onClick={onOpen}>
        {m.tab} <ArrowRight aria-hidden />
      </Button>
    </section>
  );
}
