// Independent presence–absence validation of the final map on the organisation's data
// (ADR-0108). Calibration is shown as a diagnostic only: suitability is not a probability.

import type { IndependentValidation } from "@/api/types";
import { KeyValue, Section } from "@/components/domain/common";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { formatCount, formatDecimal, formatKm, formatPercent } from "@/lib/format";

import { useIndependentValidationMessages, useOwnDataMessages } from "./messages";

export function IndependentValidationSection({ v }: { v: IndependentValidation }) {
  const m = useIndependentValidationMessages();
  const o = useOwnDataMessages();
  const s = v.sites;
  const excluded = Object.entries(s.excluded);
  return (
    <Section title={m.title} id="val-independent" action={<HelpTrigger target={{ article: "field-data", section: "validation" }} />}>
      <p className="text-sm">
        {m.intro}{" "}
        <span className="font-medium">{v.dataset_name}</span> · {v.role === "validation" ? m.heldOut : m.trainingRole}
        {v.synthetic && (
          <Badge variant="outline" className="ml-1">
            {m.synthetic}
          </Badge>
        )}
      </p>
      <KeyValue
        className="grid-cols-[minmax(0,1fr)_auto]"
        items={[
          [m.presenceSites, formatCount(s.presence_sites)],
          [m.absenceSites, formatCount(s.absence_sites)],
          [m.leakage, formatCount(s.leakage_excluded)],
          ...(s.conflicting_cells > 0 ? [[m.conflicting, formatCount(s.conflicting_cells)] as [string, string]] : []),
          ...(excluded.length > 0
            ? [[m.excluded, excluded.map(([k, n]) => `${o.statusLabel(k)}: ${formatCount(n)}`).join(" · ")] as [string, string]]
            : []),
          ...(v.spread
            ? [
                [
                  m.spread,
                  `${m.spreadValue(formatKm(v.spread.extent_km[0]), formatKm(v.spread.extent_km[1]), formatPercent(v.spread.share_of_study_extent))}${
                    v.spread.mean_nearest_km != null ? ` · ${m.nearest(`${formatKm(v.spread.mean_nearest_km)} km`)}` : ""
                  }`,
                ] as [string, string],
              ]
            : []),
        ]}
      />
      <p className="text-xs text-muted-foreground">{m.leakageHint}</p>
      {v.note && <p className="rounded-md bg-muted px-2.5 py-2 text-sm">{v.note}</p>}
      {v.auc != null && (
        <>
          <KeyValue className="grid-cols-[minmax(0,1fr)_auto]" items={[[m.auc, formatDecimal(v.auc, 3)]]} />
          <p className="text-xs text-muted-foreground">{m.aucHint}</p>
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{m.threshold}</TableHead>
                  <TableHead className="text-right">{m.sensitivity}</TableHead>
                  <TableHead className="text-right">{m.specificity}</TableHead>
                  <TableHead className="text-right">{m.tss}</TableHead>
                  <TableHead className="text-right">{m.kappa}</TableHead>
                  <TableHead className="text-right">{m.confusion}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody className="num">
                {v.thresholds.map((t) => (
                  <TableRow key={t.rule}>
                    <TableCell className="whitespace-normal">
                      {m.rule(t.rule) ?? t.rule} ({formatDecimal(t.threshold, 3)})
                    </TableCell>
                    <TableCell className="text-right">{formatDecimal(t.sensitivity)}</TableCell>
                    <TableCell className="text-right">{formatDecimal(t.specificity)}</TableCell>
                    <TableCell className="text-right">{formatDecimal(t.tss)}</TableCell>
                    <TableCell className="text-right">{formatDecimal(t.kappa)}</TableCell>
                    <TableCell className="text-right">
                      {t.true_pos}/{t.false_pos}/{t.false_neg}/{t.true_neg}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
          <p className="text-xs text-muted-foreground">{m.tssHint}</p>
          {v.calibration.length > 0 && (
            <div className="space-y-1">
              <p className="text-sm font-medium">{m.calibration}</p>
              <p className="rounded-md bg-state-warning/10 px-2.5 py-2 text-xs">{m.calibrationCaveat}</p>
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{m.range}</TableHead>
                      <TableHead className="text-right">{m.meanSuitability}</TableHead>
                      <TableHead className="text-right">{m.observedShare}</TableHead>
                      <TableHead className="text-right">{m.nSites}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody className="num">
                    {v.calibration.map((c) => (
                      <TableRow key={`${c.lower}-${c.upper}`}>
                        <TableCell>
                          {formatDecimal(c.lower, 3)}–{formatDecimal(c.upper, 3)}
                        </TableCell>
                        <TableCell className="text-right">{formatDecimal(c.mean_suitability, 3)}</TableCell>
                        <TableCell className="text-right">{formatPercent(c.observed_presence_share)}</TableCell>
                        <TableCell className="text-right">{formatCount(c.n)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </div>
          )}
          <KeyValue className="grid-cols-[minmax(0,1fr)_auto]" items={[[m.brier, formatDecimal(v.brier, 3)]]} />
        </>
      )}
      <p className="text-sm">{v.validated ? m.validatedMet : m.validatedNotMet}</p>
      <p className="text-xs text-muted-foreground">{m.criteria}</p>
    </Section>
  );
}
