// Sampling bias, background method, distance thinning and generalised coordinates in the
// results (ADR-0011, ADR-0013). The bias diagnostic is descriptive: it never changes the
// evidence level, and the UI says so wherever it is shown.

import { Map as MapIcon } from "lucide-react";

import type { GeneralisedRecords, JobSpec, SpeciesResult } from "@/api/types";
import { KeyValue, Section } from "@/components/domain/common";
import { useFallbackText } from "@/components/domain/serverText";
import { Button } from "@/components/ui/button";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { formatCount, formatKm, formatPercent } from "@/lib/format";

import { useResultsMessages } from "./messages";

/** The background actually used, in words (random, target group, or requested but fell back). */
function useBackgroundText(r: SpeciesResult, spec: JobSpec): { text: string; detail: string | null } {
  const msg = useResultsMessages();
  const fallback = useFallbackText();
  const b = r.background_sampling;
  if (!b) {
    // Default random background (older results of target-group studies have no record either).
    return spec.model.background_method === "target_group"
      ? { text: msg.bgTargetGroup(spec.model.target_group ?? "—"), detail: null }
      : { text: msg.bgRandom, detail: null };
  }
  if (b.method === "target_group") {
    return { text: msg.bgTargetGroup(b.target_group ?? "—"), detail: msg.bgEligible(formatCount(b.eligible_cells)) };
  }
  return {
    text: msg.bgFallback(b.target_group ?? null),
    detail: b.fallback_reason ? msg.bgFallbackReason(fallback(b.fallback_reason)) : null,
  };
}

/** Method lines: background and distance thinning (Summary and Validation tabs). */
export function MethodLines({ r, spec }: { r: SpeciesResult; spec: JobSpec }) {
  const msg = useResultsMessages();
  const bg = useBackgroundText(r, spec);
  const fellBack = r.background_sampling?.requested === "target_group" && r.background_sampling.method !== "target_group";
  const km = spec.filters.thin_km;
  return (
    <ul className="space-y-1.5 text-sm">
      <li>
        <span className="inline-flex items-center gap-1 text-muted-foreground">
          {msg.bgLabel} <HelpTrigger target={{ article: "data", section: "targetgroup" }} />
        </span>{" "}
        <span className={fellBack ? "font-medium text-state-warning" : undefined}>{bg.text}</span>
        {bg.detail && <span className="block text-xs text-muted-foreground">{bg.detail}</span>}
      </li>
      {km != null && (
        <li>
          <span className="inline-flex items-center gap-1 text-muted-foreground">
            {msg.thinLabel} <HelpTrigger target={{ article: "data", section: "thinning" }} />
          </span>{" "}
          {msg.thinValue(formatKm(km))}
        </li>
      )}
    </ul>
  );
}

/** Sampling-bias diagnostic from the target-group effort surface (Data tab). */
export function SamplingBiasSection({ r, onShowEffort }: { r: SpeciesResult; onShowEffort?: () => void }) {
  const msg = useResultsMessages();
  const b = r.sampling_bias;
  if (!b) return null;
  return (
    <Section title={msg.biasTitle} id="bias-title" action={<HelpTrigger target={{ article: "data", section: "targetgroup" }} />}>
      {b.synthetic && <p className="rounded-md bg-state-warning-subtle px-2.5 py-1.5 text-sm">{msg.biasSynthetic}</p>}
      <KeyValue
        items={[
          [msg.biasGroup, b.target_group],
          [msg.biasRecords, formatCount(b.records)],
          [msg.biasCells, msg.biasCellsValue(formatCount(b.cells_with_effort), formatPercent(b.share_of_area_with_effort))],
          [msg.biasMaxCell, formatCount(b.max_cell_records)],
        ]}
      />
      <p className="text-sm">{msg.biasConcentration(formatPercent(b.presences_in_top10pct_effort_cells), formatPercent(b.expected))}</p>
      <p className="text-xs text-muted-foreground">{msg.biasDescriptive}</p>
      {onShowEffort && (
        <Button variant="outline" size="sm" onClick={onShowEffort}>
          <MapIcon aria-hidden /> {msg.biasShowMap}
        </Button>
      )}
    </Section>
  );
}

/** Records whose coordinates were generalised by their publishers (Data tab). */
export function GeneralisedSection({ g }: { g: GeneralisedRecords }) {
  const msg = useResultsMessages();
  return (
    <Section title={msg.generalisedTitle} id="gen-title" action={<HelpTrigger target={{ article: "data", section: "generalised" }} />}>
      {g.flags_available ? (
        <KeyValue
          items={[
            [msg.genRecords, formatCount(g.records)],
            [msg.genExcluded, formatCount(g.excluded)],
            [msg.genRetained, formatCount(g.retained)],
          ]}
        />
      ) : (
        <p className="text-sm text-muted-foreground">{msg.genFlagsUnavailable}</p>
      )}
    </Section>
  );
}
