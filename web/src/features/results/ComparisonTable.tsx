import type { SpeciesResult } from "@/api/types";
import { SciName } from "@/components/domain/common";
import { EvidenceBadge, SpeciesStatusBadge } from "@/components/domain/status";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { formatArea } from "@/lib/format";
import { cn } from "@/lib/utils";

import { meanSd, percentMeanSd } from "./derive";
import { useResultsMessages } from "./messages";

/** Batch comparison. No AUC column: AUC is not comparable across species (AC-BAT-6). */
export function ComparisonTable({ species, selected, onSelect }: { species: SpeciesResult[]; selected: number; onSelect: (key: number) => void }) {
  const msg = useResultsMessages();
  return (
    <Table>
      <caption className="sr-only">{msg.comparisonCaption}</caption>
      <TableHeader>
        <TableRow>
          <TableHead>{msg.species}</TableHead>
          <TableHead>{msg.evidence}</TableHead>
          <TableHead className="text-right">CBI</TableHead>
          <TableHead className="text-right">OR10</TableHead>
          <TableHead className="text-right">{msg.areaP10}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody className="num">
        {species.map((s) => {
          const ok = s.status === "succeeded";
          const p10 = s.thresholds.find((t) => t.rule === "p10");
          return (
            <TableRow key={s.species_key} className={cn(s.species_key === selected && "bg-accent")}>
              <TableCell className="max-w-36 whitespace-normal">
                <button type="button" className="text-left underline-offset-4 hover:underline" onClick={() => onSelect(s.species_key)} aria-current={s.species_key === selected}>
                  <SciName name={s.scientific_name} />
                </button>
              </TableCell>
              <TableCell>{ok ? <EvidenceBadge level={s.evidence.level} /> : <SpeciesStatusBadge status={s.status} />}</TableCell>
              <TableCell className="text-right">{ok ? meanSd(s.validation.cbi_mean, s.validation.folds.map((f) => f.cbi)) : "—"}</TableCell>
              <TableCell className="text-right">{ok ? percentMeanSd(s.validation.or10_mean, s.validation.folds.map((f) => f.or10)) : "—"}</TableCell>
              <TableCell className="text-right">{ok && p10 ? formatArea(p10.area_km2) : "—"}</TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}
