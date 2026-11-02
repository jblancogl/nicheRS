// Records per source (ADR-0108): GBIF and the organisation's data, with the presence cells each
// gives and why the other records were not kept. Shown in the data step and the Data tab.

import type { SourceCount, ValidationSites } from "@/api/types";
import { Section } from "@/components/domain/common";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { formatCount } from "@/lib/format";

import { useOwnDataMessages } from "./messages";

export function SourceBreakdown({ sources, sites, demo }: { sources: SourceCount[]; sites?: ValidationSites | null; demo?: boolean }) {
  const m = useOwnDataMessages();
  if (sources.length === 0) return null;
  const gbif = sources.find((s) => s.source === "gbif");
  return (
    <Section title={m.bySource} id="by-source">
      <div className="overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead />
              <TableHead className="text-right">{m.recordsN}</TableHead>
              <TableHead className="text-right">{m.cellsN}</TableHead>
              <TableHead>{m.detail}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {sources.map((s) => (
              <TableRow key={s.source}>
                <TableCell>{m.sourceName(s.source === "gbif" && demo ? "demo" : s.source) ?? s.source}</TableCell>
                <TableCell className="num text-right">{formatCount(s.records)}</TableCell>
                <TableCell className="num text-right">{formatCount(s.retained_cells)}</TableCell>
                <TableCell className="whitespace-normal text-xs">
                  {Object.entries(s.statuses)
                    .filter(([k]) => k !== "retained")
                    .map(([k, n]) => `${m.statusLabel(k)}: ${formatCount(n)}`)
                    .join(" · ")}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
      {gbif?.in_own_cells != null && gbif.in_own_cells > 0 && <p className="text-xs text-muted-foreground">{m.inOwnCells(formatCount(gbif.in_own_cells))}</p>}
      {sites && (
        <p className="text-sm">
          {m.sites(formatCount(sites.presence_sites), formatCount(sites.absence_sites))}
          {sites.leakage_excluded > 0 && <span className="block text-xs text-muted-foreground">{m.leakage(formatCount(sites.leakage_excluded))}</span>}
        </p>
      )}
    </Section>
  );
}
