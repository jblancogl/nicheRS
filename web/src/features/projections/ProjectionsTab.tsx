import { MapIcon } from "lucide-react";

import type { ProjectionResult, SpeciesResult } from "@/api/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { Card } from "@/features/results/SummaryTab";
import type { ResultsParams } from "@/features/results/params";
import { formatArea, formatAreaRange, formatCount, formatDecimal, formatPercent } from "@/lib/format";
import { CHANGE_COLOURS, gcmRange, projectionKey } from "@/lib/future";
import { cn } from "@/lib/utils";

import { useProjectionMessages } from "./messages";

function Swatch({ colour }: { colour: string | undefined }) {
  return <span aria-hidden className="inline-block size-3 shrink-0 rounded-sm border" style={{ background: colour }} />;
}

/** Share of the present suitable area. */
const ofPresent = (x: number, p: ProjectionResult) => (p.present_suitable_km2 > 0 ? formatPercent(x / p.present_suitable_km2) : "—");

function Scenario({ p, shown, onShow }: { p: ProjectionResult; shown: boolean; onShow: () => void }) {
  const m = useProjectionMessages();
  const [lo, hi] = gcmRange(p, (g) => g.future_suitable_km2);
  const id = `proj-${projectionKey(p)}`;
  const rows: [string | undefined, string, number, number | null][] = [
    [CHANGE_COLOURS[3], m.stable, p.stable_km2, p.stable_all_gcms_km2],
    [CHANGE_COLOURS[2], m.gained, p.gained_km2, p.gained_all_gcms_km2],
    [CHANGE_COLOURS[1], m.lost, p.lost_km2, p.lost_all_gcms_km2],
  ];
  return (
    <Card
      labelledBy={id}
      help={{ article: "future", section: "reading" }}
      title={
        <span className="num normal-case">
          {p.period} · {m[`ssp_${p.ssp}`]}
        </span>
      }
    >
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant="secondary" className="num whitespace-normal">
          {m.chip(formatArea(p.present_suitable_km2), formatArea(p.future_suitable_km2))}
        </Badge>
        <span className="num text-xs text-muted-foreground">{m.rangeGcm(formatAreaRange(lo, hi))}</span>
      </div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{m.changeClass}</TableHead>
            <TableHead className="text-right">km²</TableHead>
            <TableHead className="text-right">%</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody className="num">
          {rows.map(([c, label, km2, all]) => (
            <TableRow key={label}>
              <TableCell className="whitespace-normal">
                <span className="flex items-center gap-2">
                  <Swatch colour={c} /> {label}
                </span>
                {all !== null && <span className="block pl-5 text-xs text-muted-foreground">{m.allAgree(formatArea(all))}</span>}
              </TableCell>
              <TableCell className="text-right">{formatArea(km2)}</TableCell>
              <TableCell className="text-right">{ofPresent(km2, p)}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <p className="text-xs text-muted-foreground">{m.ofPresent("%")}</p>
      <dl className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 text-sm">
        <dt className="text-muted-foreground">{m.spread}</dt>
        <dd className="num text-right">{formatDecimal(p.mean_gcm_sd, 3)}</dd>
        <dt className={cn("text-muted-foreground", p.novel_fraction > 0.1 && "text-state-warning")}>{m.extrapolated}</dt>
        <dd className="num text-right">{formatPercent(p.novel_fraction)}</dd>
      </dl>
      <p className="text-xs text-muted-foreground">{m.extrapolatedHint}</p>
      {p.cells_missing > 0 && <p className="text-xs text-state-warning">{m.missingCells(formatCount(p.cells_missing))}</p>}
      <details>
        <summary className="cursor-pointer text-sm">{m.perGcm}</summary>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{m.gcm}</TableHead>
              <TableHead className="text-right">{m.futureSuitable}</TableHead>
              <TableHead className="text-right">{m.gained}</TableHead>
              <TableHead className="text-right">{m.lost}</TableHead>
              <TableHead className="text-right">MESS &lt; 0</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody className="num">
            {p.per_gcm.map((g) => (
              <TableRow key={g.gcm}>
                <TableCell className="font-mono text-xs">{g.gcm}</TableCell>
                <TableCell className="text-right">{formatArea(g.future_suitable_km2)}</TableCell>
                <TableCell className="text-right">{formatArea(g.gained_km2)}</TableCell>
                <TableCell className="text-right">{formatArea(g.lost_km2)}</TableCell>
                <TableCell className="text-right">{formatPercent(g.novel_fraction)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </details>
      <Button type="button" variant={shown ? "secondary" : "outline"} size="sm" onClick={onShow} aria-pressed={shown}>
        <MapIcon aria-hidden /> {m.showOnMap}
      </Button>
    </Card>
  );
}

/** Results tab of the future-climate projections (ADR-0016). */
export function ProjectionsTab({
  r,
  params,
  update,
}: {
  r: SpeciesResult;
  params: ResultsParams;
  update: (p: Partial<ResultsParams>) => void;
}) {
  const m = useProjectionMessages();
  const projections = r.projections ?? [];
  const statics = projections[0]?.static_variables ?? [];
  return (
    <div className="space-y-4">
      {projections.some((p) => p.synthetic) && (
        <p className="rounded-md bg-state-warning-subtle px-3 py-2 text-sm">{m.synthetic}</p>
      )}
      <p className="text-sm">
        {m.method} <HelpTrigger target={{ article: "future", section: "limits" }} />
      </p>
      {statics.length > 0 && <p className="text-xs text-muted-foreground">{m.staticNote(statics.join(", "))}</p>}
      {projections.map((p) => (
        <Scenario
          key={projectionKey(p)}
          p={p}
          shown={params.proj === projectionKey(p)}
          onShow={() => update({ proj: projectionKey(p), flayer: params.flayer })}
        />
      ))}
      <section className="space-y-1" aria-labelledby="proj-limits">
        <h3 id="proj-limits" className="text-sm font-semibold">
          {m.limitsTitle}
        </h3>
        <ul className="list-disc space-y-0.5 pl-5 text-sm text-muted-foreground">
          <li>{m.limit1}</li>
          <li>{m.limit2}</li>
          <li>{m.limit3}</li>
          <li>{m.limit4}</li>
        </ul>
      </section>
    </div>
  );
}
