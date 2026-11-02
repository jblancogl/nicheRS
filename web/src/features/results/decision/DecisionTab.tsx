// Decision support tab of a finished study (ADR-0018): survey priority, project footprints,
// several species, comparison with another study. Every output is computed on demand by the
// server from the study's saved results; the analysis layer shown is drawn above the model raster.

import { useEffect, useState } from "react";

import { useAnalyses } from "@/api/hooks";
import type { Job, SpeciesResult } from "@/api/types";
import { ErrorState } from "@/components/domain/common";
import { Skeleton } from "@/components/ui/skeleton";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { useMapStore } from "@/stores/map";

import type { DecisionSection } from "../params";
import { CompareSection } from "./CompareSection";
import { FootprintSection } from "./FootprintSection";
import { useDecisionMessages } from "./messages";
import { PrioritySection } from "./PrioritySection";
import { RichnessSection } from "./RichnessSection";
import type { ShownLayer } from "./style";

const SECTIONS: DecisionSection[] = ["priority", "footprint", "richness", "compare"];

export function DecisionTab({
  job,
  result,
  section,
  onSection,
}: {
  job: Job;
  result: SpeciesResult;
  section: DecisionSection | null;
  onSection: (s: DecisionSection) => void;
}) {
  const m = useDecisionMessages();
  const analyses = useAnalyses(job.id);
  const [shown, setShown] = useState<ShownLayer | null>(null);
  const multi = job.spec.species.length > 1;
  const current: DecisionSection = section ?? "priority";
  const list = analyses.data ?? [];
  // A shown layer whose analysis was deleted disappears.
  const visible = shown && list.some((a) => a.id === shown.aid) ? shown : null;

  useEffect(() => {
    useMapStore.getState().setAnalysis(visible ? { jobId: job.id, analysisId: visible.aid, layer: visible.layer, opacity: 0.85 } : null);
  }, [job.id, visible]);
  useEffect(() => () => useMapStore.getState().setAnalysis(null), []);

  const label: Record<DecisionSection, string> = {
    priority: m.sectionPriority,
    footprint: m.sectionFootprint,
    richness: m.sectionRichness,
    compare: m.sectionCompare,
  };
  const layer = { shown: visible, setShown };
  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">{m.intro}</p>
      {job.spec.demo && <p className="rounded-md bg-state-warning-subtle px-3 py-2 text-sm">{m.synthetic}</p>}
      <HelpTrigger target={{ article: "decision" }} text="decisionHelp" />
      <ToggleGroup
        aria-label={m.tab}
        value={[current]}
        onValueChange={(v) => {
          const next = v[0] as DecisionSection | undefined;
          if (next) onSection(next);
        }}
        variant="outline"
        size="sm"
        className="flex w-full flex-wrap"
      >
        {SECTIONS.filter((s) => s !== "richness" || multi).map((s) => (
          <ToggleGroupItem key={s} value={s} className="h-auto min-h-7 flex-1 py-1 whitespace-normal">
            {label[s]}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
      {analyses.isPending ? (
        <Skeleton className="h-24 w-full" />
      ) : analyses.isError ? (
        <ErrorState error={analyses.error} onRetry={() => void analyses.refetch()} />
      ) : (
        <section aria-label={label[current]}>
          {current === "priority" && <PrioritySection jobId={job.id} result={result} analyses={list} {...layer} />}
          {current === "footprint" && <FootprintSection jobId={job.id} analyses={list} />}
          {current === "richness" && <RichnessSection jobId={job.id} nSpecies={job.spec.species.length} analyses={list} {...layer} />}
          {current === "compare" && <CompareSection jobId={job.id} result={result} analyses={list} {...layer} />}
        </section>
      )}
    </div>
  );
}
