// Project-footprint overlay (ADR-0018 §2): a pre-diagnostic. The disclaimer is always visible.

import { PenLine, Trash2, Upload } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";

import { errorMessage } from "@/api/client";
import { useCreateAnalysis, useDeleteAnalysis } from "@/api/hooks";
import type { Analysis, Geometry } from "@/api/types";
import { SciName } from "@/components/domain/common";
import { EvidenceBadge } from "@/components/domain/status";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bboxOf, parseStudyAreaGeoJson } from "@/lib/geo";
import { formatArea, formatAreaRange, formatDecimal, formatPercent, formatScore } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useMapStore } from "@/stores/map";

import { useDecisionMessages } from "./messages";
import { AnalysisFiles, AnalysisWarnings } from "./shared";

const MAX_BYTES = 8 << 20;

function vertices(g: Geometry): number {
  return g.type === "Polygon" ? g.coordinates.reduce((n, r) => n + r.length, 0) : g.coordinates.reduce((n, p) => n + p.reduce((k, r) => k + r.length, 0), 0);
}

export function FootprintSection({ jobId, analyses }: { jobId: string; analyses: Analysis[] }) {
  const m = useDecisionMessages();
  const create = useCreateAnalysis(jobId);
  const del = useDeleteAnalysis(jobId);
  const footprints = analyses.filter((a) => a.kind === "footprint");
  const [chosen, setChosen] = useState<string | null>(null);
  const current = footprints.find((f) => f.id === chosen) ?? footprints[footprints.length - 1];
  const [name, setName] = useState("");
  const [geometry, setGeometry] = useState<Geometry | null>(null);
  const [fileError, setFileError] = useState<string | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const nameId = useId();
  const fileId = useId();
  const drawing = useMapStore((s) => s.draw.active);

  // Outline of the pending or the chosen footprint on the map.
  const outline = geometry ?? current?.footprint?.geometry ?? null;
  useEffect(() => {
    useMapStore.getState().setFootprint(outline);
  }, [outline]);
  useEffect(() => () => useMapStore.getState().setFootprint(null), []);

  const onFile = async (file: File | undefined) => {
    setFileError(null);
    if (!file) return;
    if (file.size > MAX_BYTES) {
      setFileError(m.fileTooBig((file.size / 1024 / 1024).toFixed(1)));
      return;
    }
    const r = parseStudyAreaGeoJson(await file.text());
    if (!r.ok) {
      setFileError(r.error);
      return;
    }
    setGeometry(r.geometry);
    if (!name.trim()) setName(r.name ?? file.name.replace(/\.(geo)?json$/i, ""));
    useMapStore.getState().fitTo(bboxOf(r.geometry));
  };

  const onDraw = () =>
    useMapStore.getState().startDraw((ring) => {
      setGeometry({ type: "Polygon", coordinates: [ring] });
      setName((n) => (n.trim() ? n : m.footprintDrawn));
    });

  const f = current?.footprint;
  return (
    <div className="space-y-3">
      <p className="text-sm">{m.footprintLead}</p>
      <p className="rounded-md border border-state-warning/40 bg-state-warning/5 px-3 py-2 text-sm font-medium">{m.disclaimer}</p>
      <div className="space-y-2 rounded-md border p-3">
        <div className="space-y-1">
          <Label htmlFor={nameId}>{m.footprintName}</Label>
          <Input id={nameId} value={name} maxLength={120} onChange={(e) => setName(e.target.value)} />
        </div>
        <div className="flex flex-wrap gap-2">
          <input ref={fileRef} id={fileId} type="file" accept=".geojson,.json,application/geo+json,application/json" className="sr-only" onChange={(e) => void onFile(e.target.files?.[0])} />
          <Button size="sm" variant="outline" onClick={() => fileRef.current?.click()}>
            <Upload aria-hidden /> {m.footprintFile}
          </Button>
          <Button size="sm" variant="outline" onClick={onDraw} disabled={drawing}>
            <PenLine aria-hidden /> {m.footprintDraw}
          </Button>
        </div>
        {fileError && <p className="text-sm text-destructive">{fileError}</p>}
        {geometry && <p className="text-xs text-muted-foreground">{m.footprintReady(vertices(geometry))}</p>}
        <Button
          size="sm"
          disabled={!geometry || !name.trim() || create.isPending}
          onClick={() =>
            geometry &&
            create.mutate(
              { kind: "footprint", name: name.trim(), geometry },
              {
                onSuccess: (a) => {
                  setChosen(a.id);
                  setGeometry(null);
                  setName("");
                  if (fileRef.current) fileRef.current.value = "";
                },
              },
            )
          }
        >
          {create.isPending ? m.computing : m.footprintRun}
        </Button>
        {create.isError && (
          <Alert variant="destructive">
            <AlertDescription>{errorMessage(create.error)}</AlertDescription>
          </Alert>
        )}
      </div>

      <div>
        <p className="mb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{m.footprints}</p>
        {footprints.length === 0 ? (
          <p className="text-sm text-muted-foreground">{m.noFootprints}</p>
        ) : (
          <ul className="space-y-1">
            {footprints.map((x) => (
              <li key={x.id} className="flex items-center gap-2">
                <button
                  type="button"
                  className={cn("flex-1 truncate text-left text-sm underline-offset-4 hover:underline", x.id === current?.id && "font-semibold")}
                  aria-current={x.id === current?.id}
                  onClick={() => {
                    setChosen(x.id);
                    const g = x.footprint?.geometry;
                    if (g) useMapStore.getState().fitTo(bboxOf(g));
                  }}
                >
                  {x.footprint?.name ?? x.title}
                </button>
                <Button size="icon-xs" variant="ghost" aria-label={m.deleteFootprint(x.footprint?.name ?? x.title)} disabled={del.isPending} onClick={() => del.mutate(x.id)}>
                  <Trash2 aria-hidden />
                </Button>
              </li>
            ))}
          </ul>
        )}
        {del.isError && <p className="text-sm text-destructive">{errorMessage(del.error)}</p>}
      </div>

      {current && f && (
        <div className="space-y-2">
          <p className="text-sm">{m.footprintSummary(formatArea(f.area_km2), formatArea(f.in_study_km2), f.cells, formatArea(f.mean_cell_km2))}</p>
          <AnalysisWarnings analysis={current} />
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{m.colSpecies}</TableHead>
                  <TableHead>{m.colEvidence}</TableHead>
                  <TableHead className="text-right">{m.colP10}</TableHead>
                  <TableHead className="text-right">{m.colShareFootprint}</TableHead>
                  <TableHead className="text-right">{m.colShareSpecies}</TableHead>
                  <TableHead className="text-right">{m.colBand}</TableHead>
                  <TableHead className="text-right">{m.colSuitability}</TableHead>
                  <TableHead className="text-right">{m.colUncertainty}</TableHead>
                  <TableHead className="text-right">{m.colExtrapolated}</TableHead>
                  <TableHead className="text-right">{m.colAoa}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody className="num">
                {f.species.map((s) => {
                  const t = (rule: string) => s.thresholds.find((x) => x.rule === rule);
                  const p10 = t("p10");
                  const mtp = t("mtp");
                  const sss = t("max_sss");
                  return (
                    <TableRow key={s.species_key}>
                      <TableCell className="max-w-40 whitespace-normal">
                        <SciName name={s.scientific_name} />
                        {s.note && <p className="text-xs text-muted-foreground">{s.note}</p>}
                      </TableCell>
                      <TableCell>{s.evidence_level ? <EvidenceBadge level={s.evidence_level} /> : "—"}</TableCell>
                      <TableCell className="text-right">{p10 ? formatArea(p10.suitable_km2) : "—"}</TableCell>
                      <TableCell className="text-right">{p10 ? formatPercent(p10.share_of_footprint) : "—"}</TableCell>
                      <TableCell className="text-right">{p10 ? formatPercent(p10.share_of_species_area) : "—"}</TableCell>
                      <TableCell className="text-right">
                        {mtp && sss ? formatAreaRange(Math.min(mtp.suitable_km2, sss.suitable_km2), Math.max(mtp.suitable_km2, sss.suitable_km2)) : "—"}
                      </TableCell>
                      <TableCell className="text-right">
                        {s.mean_suitability != null ? `${formatScore(s.mean_suitability)} / ${formatScore(s.max_suitability)}` : "—"}
                      </TableCell>
                      <TableCell className="text-right">{formatDecimal(s.mean_uncertainty, 3)}</TableCell>
                      <TableCell className="text-right">{formatPercent(s.extrapolated_share)}</TableCell>
                      <TableCell className="text-right">{s.outside_aoa_share != null ? formatPercent(s.outside_aoa_share) : m.notComputed}</TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>
          <AnalysisFiles analysis={current} />
        </div>
      )}
    </div>
  );
}
