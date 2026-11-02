import { ExternalLink, TriangleAlert } from "lucide-react";
import { useEffect } from "react";

import { usePreview } from "@/api/hooks";
import type { Licence } from "@/api/types";
import { ErrorState, KeyValue, SciName, Section, WarningsList } from "@/components/domain/common";
import { ObservedLegend } from "@/components/domain/Legend";
import { useExclusionText } from "@/components/domain/serverText";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Checkbox } from "@/components/ui/checkbox";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { OwnDataSection } from "@/features/owndata/OwnDataSection";
import { SourceBreakdown } from "@/features/owndata/SourceBreakdown";
import { formatArea, formatCount, formatDecimal, formatKm, resolutionLabel } from "@/lib/format";
import { bboxOf } from "@/lib/geo";
import { useDraft } from "@/stores/draft";
import { useMapStore } from "@/stores/map";

import { useDataMessages } from "./messages";
import { useExtentMessages } from "./extentMessages";
import { usePreviewRequest, useResolutionFit } from "./previewRequest";
import { WizardFrame } from "./WizardFrame";

const RESOLUTIONS = [1, 2, 5, 10] as const;

const LICENCES: { id: Licence; label: string; note?: "ccByNote" | "ccByNcNote" }[] = [
  { id: "CC0_1_0", label: "CC0 1.0" },
  { id: "CC_BY_4_0", label: "CC BY 4.0", note: "ccByNote" },
  { id: "CC_BY_NC_4_0", label: "CC BY-NC 4.0", note: "ccByNcNote" },
];

function NumberField({ id, label, value, onChange, unit, min, max }: { id: string; label: string; value: number; onChange: (v: number) => void; unit?: string; min: number; max: number }) {
  return (
    <div className="space-y-1">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex items-center gap-2">
        <Input
          id={id}
          type="number"
          inputMode="numeric"
          className="num w-32"
          min={min}
          max={max}
          defaultValue={value}
          onBlur={(e) => {
            const v = Number(e.target.value);
            if (Number.isFinite(v) && v >= min && v <= max) onChange(v);
            else e.target.value = String(value);
          }}
        />
        {unit && <span className="text-sm text-muted-foreground">{unit}</span>}
      </div>
    </div>
  );
}

export function DataStep() {
  const species = useDraft((s) => s.species);
  const resolution = useDraft((s) => s.resolution_factor);
  const setResolution = useDraft((s) => s.setResolution);
  const filters = useDraft((s) => s.filters);
  const setFilters = useDraft((s) => s.setFilters);
  const setStep = useDraft((s) => s.setStep);
  const req = usePreviewRequest();
  const preview = usePreview(req);
  const d = preview.data;
  const m = useDataMessages();
  const xm = useExtentMessages();
  const fit = useResolutionFit();
  const reasonText = useExclusionText();

  // A resolution the study area does not fit at (e.g. a draft from before the area changed)
  // moves to the finest one it fits at.
  useEffect(() => {
    if (!fit.currentFits && fit.finest !== null) setResolution(fit.finest);
  }, [fit.currentFits, fit.finest, setResolution]);

  // Plot occurrences and the modelling grid on the map.
  useEffect(() => {
    const m = useMapStore.getState();
    m.setPoints(d?.points ?? []);
    m.setGrid(d?.grid ?? null);
    if (d && d.points.length > 0) {
      const [w, s, e, n] = d.grid.bounds;
      m.fitTo([w, s, e, n]);
    } else if (req) {
      m.fitTo(bboxOf(req.area.geometry));
    }
  }, [d, req]);

  const retainedPts = d ? d.points.filter((p) => p.status === "retained").length : 0;
  // Validation sites and absences of the organisation's data (ADR-0108) are not exclusions.
  const excludedPts = d ? d.points.filter((p) => p.status !== "retained" && !p.status.startsWith("validation") && p.status !== "absence").length : 0;
  const licenceTotal = d ? Object.values(d.licences).reduce((a, b) => a + b, 0) : 0;
  const first = species[0];

  return (
    <WizardFrame
      title={m.title}
      subtitle={m.subtitle}
      onNext={() => setStep(3)}
    >
      <div className="space-y-6">
        <div className="space-y-2">
          <p className="text-sm font-medium" id="res-label">
            {m.resolution}
          </p>
          <ToggleGroup
            aria-labelledby="res-label"
            variant="outline"
            className="w-full"
            value={[String(resolution)]}
            onValueChange={(v) => {
              const n = Number(v[0]);
              if (Number.isFinite(n) && n > 0) setResolution(n);
            }}
          >
            {RESOLUTIONS.map((r) => (
              <ToggleGroupItem
                key={r}
                value={String(r)}
                disabled={!fit.fits(r)}
                title={fit.fits(r) ? undefined : xm.unavailable(r)}
                className="num flex-1"
              >
                ≈ {r} km
              </ToggleGroupItem>
            ))}
          </ToggleGroup>
          <p className="text-xs text-muted-foreground">
            {m.resolutionHint(resolutionLabel(resolution))}
          </p>
          {fit.finest !== null && fit.finest > 1 && <p className="text-xs text-muted-foreground">{xm.disabledHint(fit.finest)}</p>}
        </div>

        <Collapsible>
          <CollapsibleTrigger className="flex w-full items-center justify-between rounded-md text-sm font-medium hover:underline">
            {m.filters}
            <span className="num text-xs font-normal text-muted-foreground">
              {m.filtersSummary(filters.year_min, formatCount(filters.max_uncertainty_m), filters.licences.length)}
            </span>
          </CollapsibleTrigger>
          <CollapsibleContent className="space-y-3 pt-3">
            <div className="grid grid-cols-2 gap-3">
              <NumberField id="year-min" label={m.yearMin} value={filters.year_min} min={1900} max={2100} onChange={(v) => setFilters({ ...filters, year_min: v })} />
              <NumberField
                id="max-unc"
                label={m.maxUnc}
                unit="m"
                value={filters.max_uncertainty_m}
                min={1}
                max={100_000}
                onChange={(v) => setFilters({ ...filters, max_uncertainty_m: v })}
              />
            </div>
            <p className="text-xs text-muted-foreground">{m.yearHint}</p>
            {filters.thin_km != null && (
              <p className="flex items-center gap-1 text-xs text-muted-foreground">
                {m.thinningOn(formatKm(filters.thin_km))}
                <HelpTrigger target={{ article: "data", section: "thinning" }} />
              </p>
            )}
            <fieldset className="space-y-1.5">
              <legend className="mb-1 flex items-center gap-1 text-sm font-medium">
                {m.licences} <HelpTrigger target={{ article: "data", section: "licences" }} />
              </legend>
              {LICENCES.map((l) => (
                <Label key={l.id} className="flex items-center gap-2 font-normal">
                  <Checkbox
                    checked={filters.licences.includes(l.id)}
                    onCheckedChange={(c) =>
                      setFilters({
                        ...filters,
                        licences: c ? [...filters.licences, l.id] : filters.licences.filter((x) => x !== l.id),
                      })
                    }
                  />
                  {l.label}
                  {l.note && <span className="text-xs text-muted-foreground">— {m[l.note]}</span>}
                </Label>
              ))}
              {filters.licences.includes("CC_BY_NC_4_0") && (
                <p className="text-xs text-state-warning">{m.ncWarning}</p>
              )}
            </fieldset>
          </CollapsibleContent>
        </Collapsible>

        <OwnDataSection />

        <Section title={m.preview} id="preview-title">
          {first && (
            <p className="text-sm text-muted-foreground">
              {m.showing}
              <SciName name={first.scientific_name} />
              {species.length > 1 && m.others}
            </p>
          )}
          {species.length > 1 && (
            <ul className="list-disc pl-5 text-sm">
              {species.slice(1).map((s) => (
                <li key={s.key}>
                  <SciName name={s.scientific_name} />
                </li>
              ))}
            </ul>
          )}

          {preview.isPending && (
            <div className="space-y-2" aria-busy="true" aria-label={m.loadingPreview}>
              <p className="text-sm text-muted-foreground">{m.fetching}</p>
              <Skeleton className="h-16 w-full" />
              <Skeleton className="h-24 w-full" />
            </div>
          )}
          {preview.isError && (
            <ErrorState
              error={preview.error}
              onRetry={() => void preview.refetch()}
              title={m.previewUnavailable}
            />
          )}
          {d && (
            <div className="space-y-4">
              {d.truncated && (
                <Alert>
                  <TriangleAlert className="text-state-warning" aria-hidden />
                  <AlertTitle>{m.truncatedTitle}</AlertTitle>
                  <AlertDescription>
                    {m.truncatedText(formatCount(d.records_fetched), formatCount(d.records_total_matching))}
                  </AlertDescription>
                </Alert>
              )}
              <WarningsList warnings={d.warnings} />
              <div className="rounded-lg border p-3">
                <p className="mb-1 text-2xl font-semibold num">
                  {formatCount(d.retained_cells)} <span className="text-sm font-normal text-muted-foreground">{m.retainedCells}</span>
                </p>
                <KeyValue
                  items={[
                    [m.source, d.source],
                    [m.fetchedMatching, `${formatCount(d.records_fetched)} / ${formatCount(d.records_total_matching)}`],
                    [m.pointsShown, m.pointsValue(formatCount(retainedPts), formatCount(excludedPts))],
                    [m.grid, m.gridValue(formatCount(d.grid.width), formatCount(d.grid.height), formatDecimal(d.grid.cell_size_deg, 4))],
                    [m.cellsInArea, formatCount(d.grid.cells_in_area)],
                    [m.areaGrid, formatArea(d.grid.area_km2)],
                  ]}
                />
                {d.query_url && (
                  <a href={d.query_url} target="_blank" rel="noreferrer" className="mt-2 inline-flex items-center gap-1 text-sm text-primary underline-offset-4 hover:underline">
                    {m.sourceQuery} <ExternalLink className="size-3.5" aria-hidden />
                  </a>
                )}
              </div>

              <ObservedLegend />

              <SourceBreakdown sources={d.by_source ?? []} sites={d.validation_sites} demo={req?.demo} />

              <div>
                <h4 className="mb-1 flex items-center gap-1 text-sm font-medium">
                  {m.exclusions} <HelpTrigger target={{ article: "data", section: "rules" }} />
                </h4>
                {d.exclusions.length === 0 ? (
                  <p className="text-sm text-muted-foreground">{m.noExclusions}</p>
                ) : (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>{m.reason}</TableHead>
                        <TableHead className="text-right">{m.records}</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {d.exclusions.map((x) => (
                        <TableRow key={x.reason}>
                          <TableCell className="whitespace-normal">{reasonText(x.reason)}</TableCell>
                          <TableCell className="num text-right">{formatCount(x.count)}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                )}
              </div>

              <div>
                <h4 className="mb-1 text-sm font-medium">{m.fetchedLicences}</h4>
                {licenceTotal === 0 ? (
                  <p className="text-sm text-muted-foreground">{m.noLicence}</p>
                ) : (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>{m.licence}</TableHead>
                        <TableHead className="text-right">{m.records}</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {Object.entries(d.licences).map(([k, v]) => (
                        <TableRow key={k}>
                          <TableCell>{LICENCES.find((l) => l.id === k)?.label ?? k}</TableCell>
                          <TableCell className="num text-right">{formatCount(v)}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                )}
              </div>
            </div>
          )}
        </Section>
      </div>
    </WizardFrame>
  );
}
