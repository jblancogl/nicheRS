import { Download, ExternalLink, FileJson, FileText } from "lucide-react";
import { useState } from "react";

import { urls } from "@/api/client";
import { useManifest } from "@/api/hooks";
import type { Job, SpeciesResult } from "@/api/types";
import { ErrorState, KeyValue, Section } from "@/components/domain/common";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useExclusionText } from "@/components/domain/serverText";
import { formatCount, formatDateTime, formatKm } from "@/lib/format";
import { DEFAULT_RF_TREES } from "@/lib/schema";

import { SourceBreakdown } from "@/features/owndata/SourceBreakdown";

import { GeneralisedSection, MethodLines, SamplingBiasSection } from "./BiasSections";
import { useResultsMessages } from "./messages";

export function DataTab({ r, job, onShowEffort }: { r: SpeciesResult; job: Job; onShowEffort?: () => void }) {
  const msg = useResultsMessages();
  const reasonText = useExclusionText();
  const [showManifest, setShowManifest] = useState(false);
  const manifest = useManifest(job.id, showManifest);
  const spec = job.spec;
  return (
    <div className="space-y-5">
      <Section title={msg.data} id="data-title">
        <KeyValue
          items={[
            [msg.presences, formatCount(r.data.presences)],
            [msg.background, formatCount(r.data.background)],
            [msg.recordsFetched, formatCount(r.data.records_fetched)],
            [msg.source, r.data.source],
          ]}
        />
        {r.data.query_url && (
          <a href={r.data.query_url} target="_blank" rel="noreferrer" className="inline-flex items-center gap-1 text-sm text-primary underline-offset-4 hover:underline">
            {msg.sourceQuery} <ExternalLink className="size-3.5" aria-hidden />
          </a>
        )}
        {r.data.exclusions.length > 0 && (
          <Table>
            <caption className="sr-only">{msg.excludedCaption}</caption>
            <TableHeader>
              <TableRow>
                <TableHead>{msg.exclusionReason}</TableHead>
                <TableHead className="text-right">{msg.records}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {r.data.exclusions.map((x) => (
                <TableRow key={x.reason}>
                  <TableCell className="whitespace-normal">{reasonText(x.reason)}</TableCell>
                  <TableCell className="num text-right">{formatCount(x.count)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
        <MethodLines r={r} spec={job.spec} />
      </Section>
      <SourceBreakdown sources={r.data.by_source ?? []} sites={r.independent_validation?.sites} demo={spec.demo} />

      {r.data.generalised && <GeneralisedSection g={r.data.generalised} />}
      <SamplingBiasSection r={r} onShowEffort={onShowEffort} />

      <Section title={msg.provenance} id="prov-title">
        <KeyValue
          items={[
            [msg.runId, <span key="id" className="font-mono text-xs">{job.id}</span>],
            [msg.created, formatDateTime(job.created_at)],
            [msg.finished, formatDateTime(job.finished_at)],
            [msg.studyArea, spec.area.name],
            [msg.resolutionFactor, `×${spec.resolution_factor} (≈ ${spec.resolution_factor} km)`],
            [
              msg.filters,
              msg.filtersValue(spec.filters.year_min, formatCount(spec.filters.max_uncertainty_m), spec.filters.licences.join(", ")) +
                (spec.filters.thin_km != null ? msg.thinFilter(formatKm(spec.filters.thin_km)) : ""),
            ],
            ...(spec.model.algorithm && spec.model.algorithm !== "maxent"
              ? [[msg.algorithm, msg.algorithmName(spec.model.algorithm)] as [string, string]]
              : []),
            ...(spec.model.algorithm === "rf"
              ? []
              : [
                  [
                    msg.tuningLabel,
                    msg.tuningValue(spec.model.tuning, spec.model.feature_classes.join(", "), spec.model.beta_multipliers.join(", ")),
                  ] as [string, string],
                ]),
            ...(spec.model.algorithm === "rf" || spec.model.algorithm === "ensemble"
              ? [[msg.rfSettingsLabel, msg.rfSettingsValue(spec.model.rf_trees ?? DEFAULT_RF_TREES, spec.model.rf_tune_mtry ?? false)] as [string, string]]
              : []),
            [msg.collinearity, String(spec.model.collinearity_threshold)],
            [msg.seed, String(spec.seed)],
          ]}
        />
      </Section>

      <Section title={msg.exportsCount(r.files.length)} id="files-title">
        {r.files.length === 0 ? (
          <p className="text-sm text-muted-foreground">{msg.noFiles}</p>
        ) : (
          <ul className="divide-y rounded-lg border">
            {r.files.map((f) => (
              <li key={f.name} className="flex items-center justify-between gap-2 px-3 py-2">
                <div className="min-w-0">
                  <p className="truncate font-mono text-xs">{f.name}</p>
                  <p className="text-xs text-muted-foreground">
                    {msg.fileDescription(f.name, f.description)} · {f.media_type}
                  </p>
                </div>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  render={<a href={urls.file(job.id, r.species_key, f.name)} download={f.name} />}
                  nativeButton={false}
                  aria-label={msg.download(f.name)}
                >
                  <Download aria-hidden />
                </Button>
              </li>
            ))}
          </ul>
        )}
        <div className="flex flex-wrap gap-2">
          {r.files.some((f) => f.name === "report.html") && (
            <Button
              variant="outline"
              size="sm"
              render={<a href={urls.report(job.id, r.species_key)} target="_blank" rel="noopener noreferrer" />}
              nativeButton={false}
            >
              <FileText aria-hidden /> {msg.openReport}
            </Button>
          )}
          <Button variant="outline" size="sm" render={<a href={urls.manifest(job.id)} download={`manifest-${job.id}.json`} />} nativeButton={false}>
            <FileJson aria-hidden /> {msg.downloadManifest}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => setShowManifest((v) => !v)} aria-expanded={showManifest}>
            {showManifest ? msg.hideManifest : msg.viewManifest}
          </Button>
        </div>
        {showManifest && (
          <div>
            {manifest.isPending && <Skeleton className="h-32 w-full" />}
            {manifest.isError && <ErrorState error={manifest.error} onRetry={() => void manifest.refetch()} title={msg.manifestUnavailable} />}
            {manifest.data !== undefined && (
              <pre className="max-h-80 overflow-auto rounded-md border bg-muted/40 p-2 font-mono text-xs" tabIndex={0} aria-label={msg.manifestJson}>
                {JSON.stringify(manifest.data, null, 2)}
              </pre>
            )}
          </div>
        )}
      </Section>
    </div>
  );
}
