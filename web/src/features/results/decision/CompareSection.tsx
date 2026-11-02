// Comparison of two studies of the same species (ADR-0018 §4): a sensitivity check. When the grids
// differ the server refuses with a 422 whose message is shown as is.

import { useState } from "react";

import { errorMessage } from "@/api/client";
import { useCreateAnalysis, useJobs, useLayers } from "@/api/hooks";
import type { Analysis, ComparedStudy, SpeciesResult } from "@/api/types";
import { useLayerText } from "@/components/domain/layerNames";
import { EvidenceBadge } from "@/components/domain/status";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { formatArea, formatCount, formatDate, formatDecimal, formatPercent, formatScore } from "@/lib/format";

import { useResultsMessages } from "../messages";
import { useDecisionMessages } from "./messages";
import { AnalysisFiles, AnalysisWarnings, LayerToggles } from "./shared";
import type { LayerProps } from "./style";

const RULE_LABEL: Record<string, string> = { p10: "P10", mtp: "MTP", max_sss: "max SSS" };

function show(v: unknown, dflt: string): string {
  if (v === null || v === undefined) return dflt;
  return typeof v === "string" ? v : JSON.stringify(v);
}

export function CompareSection({ jobId, result, analyses, ...layer }: LayerProps & { jobId: string; result: SpeciesResult; analyses: Analysis[] }) {
  const m = useDecisionMessages();
  const rm = useResultsMessages();
  const lt = useLayerText();
  const jobs = useJobs();
  // Variables are result labels ("SHORT — name"; "<name> (SYNTHETIC)" in the demo): worded from the catalogue.
  const demo = Boolean(jobs.data?.find((j) => j.id === jobId)?.spec.demo);
  const layers = useLayers(demo).data;
  const variable = (label: string) => {
    const l = layers?.find((x) => x.id === label || label.startsWith(`${x.short_name} — `) || label === `${x.name} (SYNTHETIC)`);
    return !l ? label : demo ? `${lt.name(l)} (SYNTHETIC)` : `${l.short_name} — ${lt.name(l)}`;
  };
  const create = useCreateAnalysis(jobId);
  const key = result.species_key;
  const candidates = (jobs.data ?? []).filter((j) => j.id !== jobId && j.status === "succeeded" && j.spec.species.some((s) => s.key === key));
  const [other, setOther] = useState<string | null>(null);
  const done = analyses.filter((a) => a.kind === "comparison" && a.species_key === key);
  const a = (other ? done.find((x) => x.comparison?.b.job_id === other) : undefined) ?? (other ? undefined : done[done.length - 1]);
  const c = a?.comparison;

  if (result.status !== "succeeded") return <p className="text-sm text-muted-foreground">{m.noModel}</p>;
  return (
    <div className="space-y-3">
      <p className="text-sm">{m.compareLead}</p>
      {jobs.isSuccess && candidates.length === 0 ? (
        <p className="text-sm text-muted-foreground">{m.compareNone}</p>
      ) : (
        <div className="flex flex-wrap items-end gap-2">
          <div className="min-w-0 flex-1 space-y-1">
            <Label htmlFor="compare-with">{m.compareWith}</Label>
            <Select
              items={candidates.map((j) => ({ value: j.id, label: `${j.name} · ${formatDate(j.finished_at ?? j.created_at)}` }))}
              value={other}
              onValueChange={(v) => {
                create.reset();
                setOther(v);
              }}
            >
              <SelectTrigger id="compare-with" className="w-full">
                <SelectValue placeholder={m.comparePick} />
              </SelectTrigger>
              <SelectContent>
                {candidates.map((j) => (
                  <SelectItem key={j.id} value={j.id}>
                    {j.name} · {formatDate(j.finished_at ?? j.created_at)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {other && !a && (
            <Button
              size="sm"
              disabled={create.isPending}
              onClick={() =>
                create.mutate({ kind: "comparison", other_job_id: other, species_key: key }, { onSuccess: (x) => layer.setShown({ aid: x.id, layer: "difference" }) })
              }
            >
              {create.isPending ? m.computing : m.compareRun}
            </Button>
          )}
        </div>
      )}
      {create.isError && (
        <Alert variant="destructive">
          <AlertDescription>{errorMessage(create.error)}</AlertDescription>
        </Alert>
      )}
      {a && c && (
        <>
          <AnalysisWarnings analysis={a} />
          <LayerToggles analysis={a} {...layer} />
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead />
                  <TableHead>{m.studyA}</TableHead>
                  <TableHead>
                    {m.studyB}: {c.b.name}
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody className="num">
                <Row label={m.rowAlgorithm} a={rm.algorithmShort(c.a.algorithm)} b={rm.algorithmShort(c.b.algorithm)} />
                <TableRow>
                  <TableCell className="text-muted-foreground">{m.rowEvidence}</TableCell>
                  <TableCell>
                    <EvidenceBadge level={c.a.evidence_level} />
                  </TableCell>
                  <TableCell>
                    <EvidenceBadge level={c.b.evidence_level} />
                  </TableCell>
                </TableRow>
                <Row label={m.rowPresences} a={formatCount(c.a.presences)} b={formatCount(c.b.presences)} />
                <Row label="CBI" a={formatScore(c.a.cbi_mean)} b={formatScore(c.b.cbi_mean)} />
                <Row label="OR10" a={formatScore(c.a.or10_mean)} b={formatScore(c.b.or10_mean)} />
                <Row label="AUC" a={formatScore(c.a.auc_mean)} b={formatScore(c.b.auc_mean)} />
                <Row label={m.rowValidation} a={m.methodCode(c.a.validation_method)} b={m.methodCode(c.b.validation_method)} />
                <Row label={m.rowBackground} a={m.backgroundCode(c.a.background)} b={m.backgroundCode(c.b.background)} />
                {["p10", "mtp", "max_sss"].map((rule) => {
                  const f = (s: ComparedStudy) => {
                    const t = s.thresholds.find((x) => x.rule === rule);
                    return t ? `${formatArea(t.area_km2)} (≥ ${formatDecimal(t.threshold, 3)})` : "—";
                  };
                  return <Row key={rule} label={m.rowArea(RULE_LABEL[rule] ?? rule)} a={f(c.a)} b={f(c.b)} />;
                })}
                <Row label={m.rowExtrapolated} a={formatPercent(c.a.extrapolated_share)} b={formatPercent(c.b.extrapolated_share)} />
                <Row
                  label={m.rowAoa}
                  a={c.a.outside_aoa_share != null ? formatPercent(c.a.outside_aoa_share) : m.notComputed}
                  b={c.b.outside_aoa_share != null ? formatPercent(c.b.outside_aoa_share) : m.notComputed}
                />
                <Row label={m.rowVariables} a={c.a.variables.map(variable).join(", ")} b={c.b.variables.map(variable).join(", ")} />
              </TableBody>
            </Table>
          </div>
          <div>
            <p className="mb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{m.settingsDiff}</p>
            {c.settings.length === 0 ? (
              <p className="text-sm text-muted-foreground">{m.noSettingsDiff}</p>
            ) : (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{m.colSetting}</TableHead>
                      <TableHead>A</TableHead>
                      <TableHead>B</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {c.settings.map((d) => (
                      <TableRow key={d.path}>
                        <TableCell className="font-mono text-xs">{d.path}</TableCell>
                        <TableCell className="max-w-40 font-mono text-xs break-all whitespace-normal">{show(d.a, m.defaultValue)}</TableCell>
                        <TableCell className="max-w-40 font-mono text-xs break-all whitespace-normal">{show(d.b, m.defaultValue)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </div>
          <div>
            <p className="mb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{m.agreement}</p>
            <p className="mb-1 text-xs text-muted-foreground">{m.commonCells(c.common_cells, formatArea(c.common_km2))}</p>
            <Table>
              <TableBody className="num">
                <Row label={m.schoener} a={formatDecimal(c.schoener_d, 3)} />
                <Row label={m.spearman} a={formatDecimal(c.spearman, 3)} />
                <Row label={m.meanDiff} a={`${c.mean_difference >= 0 ? "+" : ""}${formatDecimal(c.mean_difference, 3)}`} />
                <Row label={m.meanAbsDiff} a={formatDecimal(c.mean_abs_difference, 3)} />
                <Row label={m.p10Both} a={formatArea(c.p10_agreement.both_km2)} />
                <Row label={m.p10OnlyA} a={formatArea(c.p10_agreement.only_a_km2)} />
                <Row label={m.p10OnlyB} a={formatArea(c.p10_agreement.only_b_km2)} />
                <Row label={m.p10Neither} a={formatArea(c.p10_agreement.neither_km2)} />
              </TableBody>
            </Table>
          </div>
          <AnalysisFiles analysis={a} />
        </>
      )}
    </div>
  );
}

function Row({ label, a, b }: { label: string; a: string; b?: string }) {
  return (
    <TableRow>
      <TableCell className="text-muted-foreground">{label}</TableCell>
      <TableCell className="whitespace-normal">{a}</TableCell>
      {b !== undefined && <TableCell className="whitespace-normal">{b}</TableCell>}
    </TableRow>
  );
}
