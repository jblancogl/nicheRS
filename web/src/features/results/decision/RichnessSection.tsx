// Multi-species aggregation (ADR-0018 §3): "species with suitable conditions", never "species
// present"; the stacking caveat is always shown.

import { useState } from "react";

import { errorMessage } from "@/api/client";
import { useCreateAnalysis } from "@/api/hooks";
import type { Analysis, EvidenceLevel } from "@/api/types";
import { SciName } from "@/components/domain/common";
import { EvidenceBadge } from "@/components/domain/status";
import { useRichnessReasonText } from "@/components/domain/serverText";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { formatArea, formatCount, formatPercent } from "@/lib/format";

import { useDecisionMessages } from "./messages";
import { AnalysisFiles, AnalysisWarnings, LayerToggles } from "./shared";
import type { LayerProps } from "./style";

const LEVELS: EvidenceLevel[] = ["exploratory", "indicative", "supported", "validated"];
const LEVEL_NAMES: Record<EvidenceLevel, string> = {
  insufficient: "Insufficient",
  exploratory: "Exploratory",
  indicative: "Indicative",
  supported: "Supported",
  validated: "Validated",
};

export function RichnessSection({ jobId, nSpecies, analyses, ...layer }: LayerProps & { jobId: string; nSpecies: number; analyses: Analysis[] }) {
  const m = useDecisionMessages();
  const reasonText = useRichnessReasonText();
  const create = useCreateAnalysis(jobId);
  const [level, setLevel] = useState<EvidenceLevel>("indicative");
  const a = analyses.find((x) => x.kind === "richness" && x.richness?.min_evidence === level);
  const r = a?.richness;

  if (nSpecies < 2) return <p className="text-sm text-muted-foreground">{m.oneSpecies}</p>;
  return (
    <div className="space-y-3">
      <p className="text-sm">{m.richnessLead}</p>
      <p className="rounded-md border border-state-warning/40 bg-state-warning/5 px-3 py-2 text-sm">{m.caveat}</p>
      <div className="flex flex-wrap items-end gap-2">
        <div className="space-y-1">
          <Label htmlFor="richness-level">{m.minEvidence}</Label>
          <Select items={LEVELS.map((l) => ({ value: l, label: LEVEL_NAMES[l] }))} value={level} onValueChange={(v) => v && setLevel(v)}>
            <SelectTrigger id="richness-level" className="w-44">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {LEVELS.map((l) => (
                <SelectItem key={l} value={l}>
                  {LEVEL_NAMES[l]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        {!a && (
          <Button
            size="sm"
            disabled={create.isPending}
            onClick={() => create.mutate({ kind: "richness", min_evidence: level }, { onSuccess: (x) => layer.setShown({ aid: x.id, layer: "richness" }) })}
          >
            {create.isPending ? m.computing : m.richnessCompute}
          </Button>
        )}
      </div>
      <p className="text-xs text-muted-foreground">{m.exploratoryNote}</p>
      {create.isError && (
        <Alert variant="destructive">
          <AlertDescription>{errorMessage(create.error)}</AlertDescription>
        </Alert>
      )}
      {a && r && (
        <>
          <AnalysisWarnings analysis={a} />
          <LayerToggles analysis={a} {...layer} />
          <p className="text-sm">
            {m.hotspotLine(r.hotspot.cells, formatArea(r.hotspot.area_km2), formatPercent(r.hotspot.fraction_of_study_area), r.hotspot.min_species)}
          </p>
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{m.colSpecies}</TableHead>
                  <TableHead>{m.colEvidence}</TableHead>
                  <TableHead className="text-right">{m.colP10}</TableHead>
                  <TableHead>{m.included}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {r.included.map((s) => (
                  <TableRow key={s.species_key}>
                    <TableCell className="max-w-40 whitespace-normal">
                      <SciName name={s.scientific_name} />
                    </TableCell>
                    <TableCell>{s.evidence_level ? <EvidenceBadge level={s.evidence_level} /> : "—"}</TableCell>
                    <TableCell className="num text-right">{formatArea(s.suitable_km2)}</TableCell>
                    <TableCell>✓</TableCell>
                  </TableRow>
                ))}
                {r.excluded.map((s) => (
                  <TableRow key={s.species_key} className="text-muted-foreground">
                    <TableCell className="max-w-40 whitespace-normal">
                      <SciName name={s.scientific_name} />
                    </TableCell>
                    <TableCell>{s.evidence_level ? <EvidenceBadge level={s.evidence_level} /> : "—"}</TableCell>
                    <TableCell className="text-right">—</TableCell>
                    <TableCell className="whitespace-normal">
                      {m.excluded}: {s.reason ? reasonText(s.reason) : "—"}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
          <div>
            <p className="mb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{m.histogram}</p>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="text-right">{m.colCount}</TableHead>
                  <TableHead className="text-right">{m.colCells}</TableHead>
                  <TableHead className="text-right">{m.colArea}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody className="num">
                {r.histogram.map((b) => (
                  <TableRow key={b.species}>
                    <TableCell className="text-right">{b.species}</TableCell>
                    <TableCell className="text-right">{formatCount(b.cells)}</TableCell>
                    <TableCell className="text-right">{formatArea(b.area_km2)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
          <AnalysisFiles analysis={a} />
        </>
      )}
    </div>
  );
}
