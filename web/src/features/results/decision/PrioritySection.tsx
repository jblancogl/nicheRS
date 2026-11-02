// Survey priority of the selected species (ADR-0018 §1): areas worth surveying, never "where the
// species is".

import { Crosshair } from "lucide-react";

import { errorMessage } from "@/api/client";
import { useCreateAnalysis } from "@/api/hooks";
import type { Analysis, SpeciesResult } from "@/api/types";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { formatArea, formatDecimal, formatLngLat, formatPercent, formatScore } from "@/lib/format";
import { useMapStore } from "@/stores/map";

import { useDecisionMessages } from "./messages";
import { AnalysisFiles, AnalysisWarnings, LayerToggles, Swatch } from "./shared";
import { PRIORITY_CLASS_COLOURS, type LayerProps } from "./style";

export function PrioritySection({ jobId, result, analyses, ...layer }: LayerProps & { jobId: string; result: SpeciesResult; analyses: Analysis[] }) {
  const m = useDecisionMessages();
  const create = useCreateAnalysis(jobId);
  const a = analyses.find((x) => x.kind === "priority" && x.species_key === result.species_key);
  const p = a?.priority;

  if (result.status !== "succeeded") return <p className="text-sm text-muted-foreground">{m.noModel}</p>;
  return (
    <div className="space-y-3">
      <p className="text-sm">{m.priorityLead}</p>
      {!a && (
        <Button
          size="sm"
          disabled={create.isPending}
          onClick={() =>
            create.mutate({ kind: "priority", species_key: result.species_key }, { onSuccess: (r) => layer.setShown({ aid: r.id, layer: "priority_class" }) })
          }
        >
          {create.isPending ? m.computing : m.priorityCompute}
        </Button>
      )}
      {create.isError && (
        <Alert variant="destructive">
          <AlertDescription>{errorMessage(create.error)}</AlertDescription>
        </Alert>
      )}
      {a && p && (
        <>
          <p className="text-sm text-muted-foreground">
            {p.method === "effort" ? m.methodEffort(p.target_group ?? "—", Boolean(p.effort_synthetic)) : m.methodDistance(p.known_records ?? 0)}{" "}
            {m.thresholdLine(formatDecimal(p.suitability_threshold, 3), formatDecimal(p.sampling_split, 1), p.method === "effort")}
          </p>
          <AnalysisWarnings analysis={a} />
          <LayerToggles analysis={a} {...layer} />
          <div>
            <p className="mb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{m.classes}</p>
            <ul className="space-y-1 text-sm">
              {p.classes.map((c) => (
                <li key={c.id} className="flex items-start gap-2">
                  <Swatch colour={PRIORITY_CLASS_COLOURS[c.code - 1] ?? "#ccc"} />
                  <span className="flex-1">{m.className(c.id)}</span>
                  <span className="num text-muted-foreground">
                    {formatArea(c.area_km2)} · {formatPercent(c.fraction_of_study_area)}
                  </span>
                </li>
              ))}
            </ul>
          </div>
          <div>
            <p className="mb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{m.candidates(p.candidates.length, p.candidate_groups)}</p>
            <p className="mb-1 text-xs text-muted-foreground">{m.candidatesMethod(formatDecimal(p.candidate_min_priority, 2))}</p>
            {p.candidates.length === 0 ? (
              <p className="text-sm text-muted-foreground">{m.noCandidates}</p>
            ) : (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className="text-right">{m.colRank}</TableHead>
                      <TableHead>{m.colCentroid}</TableHead>
                      <TableHead className="text-right">{m.colArea}</TableHead>
                      <TableHead className="text-right">{m.colPriority}</TableHead>
                      <TableHead className="text-right">{m.colSuitability}</TableHead>
                      <TableHead className="text-right">{p.method === "effort" ? m.colEffort : m.colDistance}</TableHead>
                      <TableHead className="text-right">{m.colExtrapolated}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody className="num">
                    {p.candidates.map((c) => (
                      <TableRow key={c.rank}>
                        <TableCell className="text-right">
                          <Button variant="ghost" size="xs" aria-label={m.zoomTo(c.rank)} onClick={() => useMapStore.getState().fitTo(c.bbox)}>
                            <Crosshair aria-hidden /> {c.rank}
                          </Button>
                        </TableCell>
                        <TableCell>{formatLngLat(c.lng, c.lat)}</TableCell>
                        <TableCell className="text-right">{formatArea(c.area_km2)}</TableCell>
                        <TableCell className="text-right">{formatScore(c.mean_priority)}</TableCell>
                        <TableCell className="text-right">
                          {formatScore(c.mean_suitability)} / {formatScore(c.max_suitability)}
                        </TableCell>
                        <TableCell className="text-right">
                          {p.method === "effort" ? formatDecimal(c.mean_effort, 1) : `${formatDecimal(c.mean_distance_km, 1)} km`}
                        </TableCell>
                        <TableCell className="text-right">{formatPercent(c.extrapolated_share)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </div>
          <AnalysisFiles analysis={a} />
        </>
      )}
    </div>
  );
}
