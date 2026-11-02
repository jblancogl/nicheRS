import { TriangleAlert } from "lucide-react";

import { useLayers } from "@/api/hooks";
import type { LayerInfo } from "@/api/types";
import { EmptyState, ErrorState } from "@/components/domain/common";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { useDraft } from "@/stores/draft";
import { useLayerText } from "@/components/domain/layerNames";

import { groupLayers, useSelectedLayers } from "./layerSelection";
import { useVariablesMessages } from "./messages";
import { WizardFrame } from "./WizardFrame";

function CautionBadge({ text }: { text: string }) {
  const m = useVariablesMessages();
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <button
            type="button"
            className="inline-flex h-5 items-center gap-1 rounded-full bg-state-warning-subtle px-1.5 text-[0.7rem] font-medium"
            aria-label={m.cautionLabel(text)}
          />
        }
      >
        <TriangleAlert className="size-3 text-state-warning" aria-hidden /> {m.caution}
      </TooltipTrigger>
      <TooltipContent className="max-w-72">{text}</TooltipContent>
    </Tooltip>
  );
}

function Caution({ text }: { text: string }) {
  return (
    <div className="flex items-start gap-1.5 pt-0.5">
      <CautionBadge text={text} />
      <span className="text-xs text-muted-foreground">{text}</span>
    </div>
  );
}

/** Help section per group (ADR-0015 predictors have their own article). */
const GROUP_HELP = {
  climate: { article: "variables", section: "chelsa" },
  terrain: { article: "predictors", section: "terrain" },
  land_cover: { article: "predictors", section: "landcover" },
  water: { article: "predictors", section: "water" },
} as const;

export function VariablesStep() {
  const demo = useDraft((s) => s.demo);
  const q = useLayers(demo);
  const lt = useLayerText();
  const setLayers = useDraft((s) => s.setLayers);
  const threshold = useDraft((s) => s.model.collinearity_threshold);
  const setStep = useDraft((s) => s.setStep);
  const { ids, isCustom } = useSelectedLayers();
  const m = useVariablesMessages();

  // New selections go to the end: the collinearity filter keeps variables in this priority
  // order, so climate chosen first is kept before land cover (ADR-0009, ADR-0015).
  const toggle = (id: string, on: boolean) => {
    const next = on ? [...ids, id] : ids.filter((x) => x !== id);
    setLayers(next);
  };
  const clearGroup = (layers: LayerInfo[]) => {
    const drop = new Set(layers.map((l) => l.id));
    setLayers(ids.filter((x) => !drop.has(x)));
  };

  // Cautions come from the server in English; known ones are shown in the interface language.
  const groups = q.data
    ? groupLayers(q.data.map((l) => (l.caution ? { ...l, caution: m.layerCaution(l.id) || l.caution } : l)))
    : [];
  const landCover = groups.find((g) => g.group === "land_cover");
  const allLandCover = landCover !== undefined && landCover.layers.length > 1 && landCover.layers.every((l) => ids.includes(l.id));

  return (
    <WizardFrame
      title={m.title}
      subtitle={m.subtitle}
      onNext={() => setStep(4)}
      nextDisabled={ids.length === 0}
      nextReason={m.nextReason}
      nextLabel={ids.length > 0 ? m.useThese(ids.length) : undefined}
    >
      <div className="space-y-4">
        <p className="rounded-md bg-muted px-3 py-2 text-sm">
          {m.collinearityA}
          <span className="num">{threshold}</span>
          {m.collinearityB}
        </p>

        {q.isPending && (
          <div className="space-y-2" aria-busy="true">
            {[0, 1, 2, 3].map((i) => (
              <Skeleton key={i} className="h-14 w-full" />
            ))}
          </div>
        )}
        {q.isError && <ErrorState error={q.error} onRetry={() => void q.refetch()} title={m.catalogueUnavailable} />}
        {q.data && q.data.length === 0 && <EmptyState title={m.noLayers}>{m.noLayersText}</EmptyState>}
        {q.data && q.data.length > 0 && (
          <>
            <div className="flex flex-wrap items-center justify-between gap-2 text-sm">
              <span className="num text-muted-foreground">
                {m.selectedCount(ids.length, q.data.length, isCustom)}
              </span>
              {isCustom && (
                <Button variant="outline" size="sm" onClick={() => setLayers(null)}>
                  {groups.length > 1 ? m.climateOnly : m.resetRecommended}
                </Button>
              )}
            </div>
            {groups.map(({ group, layers, sharedCaution }) => {
              const selected = layers.filter((l) => ids.includes(l.id)).length;
              const first = layers[0];
              const headingId = `layer-group-${group}`;
              return (
                <fieldset key={group} aria-labelledby={headingId} className="space-y-2">
                  <div className="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
                    <h3 id={headingId} className="flex items-center gap-1.5 text-sm font-semibold">
                      {m.group(group)}
                      <span className="num text-xs font-normal text-muted-foreground">{m.groupCount(selected, layers.length)}</span>
                      {!demo && <HelpTrigger target={GROUP_HELP[group]} />}
                    </h3>
                    {selected > 0 && group !== "climate" && (
                      <Button variant="ghost" size="sm" onClick={() => clearGroup(layers)}>
                        {m.clearGroup}
                      </Button>
                    )}
                  </div>
                  {!demo && <p className="text-xs text-muted-foreground">{m.groupIntro(group)}</p>}
                  {first && group !== "climate" && (
                    <p className="text-xs text-muted-foreground">
                      {lt.source(first)} · {first.licence}
                      {first.attribution && (
                        <>
                          {" "}
                          · {m.attribution}: {first.attribution}
                        </>
                      )}
                    </p>
                  )}
                  {sharedCaution && <Caution text={sharedCaution} />}
                  {group === "land_cover" && (
                    <p
                      className={`rounded-md px-3 py-2 text-xs ${allLandCover ? "bg-state-warning-subtle" : "bg-muted"}`}
                      role={allLandCover ? "status" : undefined}
                    >
                      {allLandCover ? <b>{m.compositionAll} </b> : null}
                      {m.compositionHint}
                    </p>
                  )}
                  <ul className="divide-y rounded-lg border">
                    {layers.map((l) => {
                      const checked = ids.includes(l.id);
                      const cid = `layer-${l.id}`;
                      return (
                        <li key={l.id} className="flex items-start gap-3 px-3 py-2.5">
                          <Checkbox id={cid} checked={checked} onCheckedChange={(c) => toggle(l.id, c)} className="mt-0.5" />
                          <div className="min-w-0 flex-1 space-y-0.5">
                            <label htmlFor={cid} className="flex cursor-pointer flex-wrap items-center gap-x-2 text-sm font-medium">
                              {lt.name(l)}
                              <span className="font-mono text-xs font-normal text-muted-foreground">{l.short_name}</span>
                              <span className="text-xs font-normal text-muted-foreground">{lt.unit(l)}</span>
                              {l.default_selected && <span className="text-xs font-normal text-muted-foreground">{m.recommended}</span>}
                            </label>
                            {group === "climate" && (
                              <p className="text-xs text-muted-foreground">
                                {lt.source(l)} · {l.licence}
                              </p>
                            )}
                            {l.caution && l.caution !== sharedCaution && <Caution text={l.caution} />}
                          </div>
                        </li>
                      );
                    })}
                  </ul>
                </fieldset>
              );
            })}
          </>
        )}
      </div>
    </WizardFrame>
  );
}
