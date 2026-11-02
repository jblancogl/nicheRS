import { ArrowLeft, Copy, Crosshair, Package } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import { Link, useNavigate, useParams } from "react-router";

import { useJob, useJobOccurrences, useLayers, useResults } from "@/api/hooks";
import type { JobResults } from "@/api/types";
import {
  ErrorState,
  SciName,
  SyntheticBadge,
} from "@/components/domain/common";
import { Panel } from "@/components/domain/Panel";
import { JobStatusBadge } from "@/components/domain/status";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { useProjectionMessages } from "@/features/projections/messages";
import { ProjectionsTab } from "@/features/projections/ProjectionsTab";
import { formatDateTime } from "@/lib/format";
import { futureLayer, projectionKey } from "@/lib/future";
import { useDraft } from "@/stores/draft";
import { useMapStore } from "@/stores/map";

import { ComparisonTable } from "./ComparisonTable";
import { DataTab } from "./DataTab";
import { DecisionTab } from "./decision/DecisionTab";
import { useDecisionMessages } from "./decision/messages";
import { MapOverlay } from "./MapOverlay";
import { useResultsMessages } from "./messages";
import {
  useResultsParams,
  type ResultsReading,
  type ResultsTab,
} from "./params";
import { SimpleSummary } from "./SimpleSummary";
import { useSimpleMessages } from "./simpleMessages";
import { SummaryTab } from "./SummaryTab";
import { ValidationTab } from "./ValidationTab";
import { VariablesTab } from "./VariablesTab";

function Results({ id, data }: { id: string; data: JobResults }) {
  const msg = useResultsMessages();
  const dm = useDecisionMessages();
  const navigate = useNavigate();
  const sm = useSimpleMessages();
  const pm = useProjectionMessages();
  const [urlParams, update] = useResultsParams();
  // Simple reading by default for visitors who set studies up in Quick mode (unless the link says).
  const quickVisitor = useDraft((s) => s.path === "quick");
  const reading: ResultsReading =
    urlParams.view ?? (quickVisitor ? "simple" : "full");
  const simple = reading === "simple";
  // The simple reading explains the suitability layer with its extrapolation hatch: the map shows
  // exactly that, whatever layer the link asked for.
  const shownParams = useMemo(
    () =>
      simple
        ? { ...urlParams, layer: "suitability" as const, ext: true, aoa: false }
        : urlParams,
    [simple, urlParams],
  );
  const layersQ = useLayers(Boolean(data.job.spec.demo));
  const loadSpec = useDraft((s) => s.loadSpec);
  const [mapEl] = useState(() => document.getElementById("map"));
  const { job, species } = data;

  const selected =
    species.find((s) => s.species_key === shownParams.species) ??
    species.find((s) => s.status === "succeeded") ??
    species[0];
  // The sampling-effort layer exists only for results with a target-group effort surface.
  const hasEffort = Boolean(selected?.sampling_bias);
  const params = useMemo(
    () =>
      shownParams.layer === "effort" && !hasEffort
        ? { ...shownParams, layer: "suitability" as const }
        : shownParams,
    [shownParams, hasEffort],
  );
  const ok = selected?.status === "succeeded";
  // Future projection on the map (ADR-0016): only one the selected result has.
  const projection = selected?.projections?.find(
    (p) => projectionKey(p) === params.proj,
  );
  const futureTabOk = Boolean(selected?.projections?.length);
  const occ = useJobOccurrences(id, ok ? selected.species_key : undefined);
  const occAvailable = Boolean(occ.data && occ.data.length > 0);
  const threshold =
    selected?.thresholds.find((t) => t.rule === params.rule)?.threshold ??
    selected?.thresholds[0]?.threshold;

  // Drive the map from the URL view state.
  const area = job.spec.area.geometry;
  useEffect(() => {
    const m = useMapStore.getState();
    m.setStudyArea(area);
    m.setGrid(null);
  }, [area]);
  const key = selected?.species_key;
  const aoaThreshold = selected?.aoa?.threshold;
  useEffect(() => {
    const m = useMapStore.getState();
    if (key === undefined || !ok) {
      m.setRaster(null);
      return;
    }
    m.setRaster({
      jobId: id,
      speciesKey: key,
      layer: projection ? futureLayer(projection, params.flayer) : params.layer,
      extrapolationLayer: projection
        ? futureLayer(projection, "mess")
        : undefined,
      threshold,
      opacity: params.opacity,
      extrapolationOverlay: params.ext,
      aoaOverlay: !projection && params.aoa && aoaThreshold !== undefined,
      aoaThreshold,
    });
  }, [
    id,
    key,
    ok,
    params.layer,
    params.opacity,
    params.ext,
    params.aoa,
    aoaThreshold,
    threshold,
    projection,
    params.flayer,
  ]);
  useEffect(() => {
    const m = useMapStore.getState();
    m.setPoints(params.obs && occ.data ? occ.data : []);
  }, [params.obs, occ.data]);
  const bounds = selected?.bounds;
  const boundsKey = bounds?.join(",");
  useEffect(() => {
    if (bounds) useMapStore.getState().fitTo(bounds);
    // fit only when the species (its bounds) changes
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [boundsKey]);
  useEffect(
    () => () => {
      const m = useMapStore.getState();
      m.setRaster(null);
      m.setPoints([]);
      m.setInspect(null);
    },
    [],
  );

  const warningsCount = useMemo(
    () => species.reduce((n, s) => n + s.warnings.length, 0),
    [species],
  );

  if (!selected) {
    return (
      <Panel title={job.name}>
        <p className="text-sm text-muted-foreground">{msg.noSpeciesResults}</p>
      </Panel>
    );
  }

  return (
    <Panel
      title={job.name}
      subtitle={
        <span className="flex flex-wrap items-center gap-2">
          <JobStatusBadge
            status={job.status}
            withWarnings={warningsCount > 0}
          />
          <span className="num">
            {formatDateTime(job.finished_at ?? job.created_at)}
          </span>
          {job.spec.demo &&
            !/SYNTHETIC/.test(job.spec.species[0]?.scientific_name ?? "") && (
              <SyntheticBadge />
            )}
        </span>
      }
      focusKey={selected.species_key}
      header={
        <div className="space-y-2">
          <ToggleGroup
            aria-label={sm.viewLabel}
            value={[reading]}
            onValueChange={(v) => {
              const next = v[0] as ResultsReading | undefined;
              if (next) update({ view: next });
            }}
            variant="outline"
            size="sm"
            className="w-full"
          >
            <ToggleGroupItem value="simple" className="flex-1">
              {sm.simple}
            </ToggleGroupItem>
            <ToggleGroupItem value="full" className="flex-1">
              {sm.full}
            </ToggleGroupItem>
          </ToggleGroup>
          {species.length > 1 ? (
            <div className="space-y-1">
              <label
                className="text-xs text-muted-foreground"
                htmlFor="species-switch"
              >
                {msg.speciesSwitch(species.length)}
              </label>
              <Select
                items={species.map((s) => ({
                  value: String(s.species_key),
                  label: s.scientific_name,
                }))}
                value={String(selected.species_key)}
                onValueChange={(v) =>
                  v !== null && update({ species: Number(v) })
                }
              >
                <SelectTrigger id="species-switch" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {species.map((s) => (
                    <SelectItem
                      key={s.species_key}
                      value={String(s.species_key)}
                    >
                      <SciName name={s.scientific_name} />
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          ) : (
            <SciName name={selected.scientific_name} className="text-sm" />
          )}
        </div>
      }
      footer={
        <>
          <Button
            variant="outline"
            onClick={() => {
              loadSpec(job.spec, job.project_id);
              void navigate("/new");
            }}
          >
            <Copy aria-hidden /> {msg.duplicate}
          </Button>
          <Button onClick={() => update({ view: "full", tab: "data" })}>
            <Package aria-hidden /> {msg.exports}
          </Button>
        </>
      }
    >
      {job.spec.demo && (
        <div className="mb-4 space-y-1 rounded-md bg-state-warning-subtle px-3 py-2 text-sm">
          <p>{msg.demoNotice}</p>
          <HelpTrigger
            target={{ article: "getting-started", section: "demo" }}
            text="demoHelp"
          />
        </div>
      )}
      {!ok ? (
        <Alert variant="destructive">
          <AlertTitle>{msg.noModel(selected.status)}</AlertTitle>
          <AlertDescription>
            {selected.error ?? msg.notModelled}
          </AlertDescription>
        </Alert>
      ) : simple ? (
        <SimpleSummary
          r={selected}
          spec={job.spec}
          layers={layersQ.data}
          onFull={() => update({ view: "full", tab: "summary" })}
        />
      ) : (
        <Tabs
          value={
            params.tab === "future" && !futureTabOk ? "summary" : params.tab
          }
          onValueChange={(v) => update({ tab: v as ResultsTab })}
        >
          {/* Six tabs do not fit the side panel on one row: they wrap rather than scroll out of sight. */}
          <TabsList className="mb-4 w-full flex-wrap group-data-horizontal/tabs:h-auto [&>*]:h-7">
            <TabsTrigger value="summary">{msg.tabSummary}</TabsTrigger>
            <TabsTrigger value="validation">{msg.tabValidation}</TabsTrigger>
            <TabsTrigger value="variables">{msg.tabVariables}</TabsTrigger>
            <TabsTrigger value="data">{msg.tabData}</TabsTrigger>
            {futureTabOk && <TabsTrigger value="future">{pm.tab}</TabsTrigger>}
            <TabsTrigger value="decision">{dm.tab}</TabsTrigger>
          </TabsList>
          <TabsContent value="summary">
            <SummaryTab
              r={selected}
              spec={job.spec}
              params={params}
              update={update}
            />
            <Button
              variant="ghost"
              size="sm"
              className="mt-3"
              onClick={() => useMapStore.getState().fitTo(selected.bounds)}
            >
              <Crosshair aria-hidden /> {msg.zoomToResults}
            </Button>
          </TabsContent>
          <TabsContent value="validation">
            <ValidationTab r={selected} spec={job.spec} layers={layersQ.data} />
          </TabsContent>
          <TabsContent value="variables">
            <VariablesTab r={selected} layers={layersQ.data} />
          </TabsContent>
          <TabsContent value="data">
            <DataTab
              r={selected}
              job={job}
              onShowEffort={
                hasEffort ? () => update({ layer: "effort" }) : undefined
              }
            />
          </TabsContent>
          {futureTabOk && (
            <TabsContent value="future">
              <ProjectionsTab r={selected} params={params} update={update} />
            </TabsContent>
          )}
          <TabsContent value="decision">
            <DecisionTab
              job={job}
              result={selected}
              section={params.ds}
              onSection={(ds) => update({ ds })}
            />
          </TabsContent>
        </Tabs>
      )}
      {species.length > 1 && (
        <section className="mt-6 space-y-2" aria-labelledby="compare-title">
          <h3 id="compare-title" className="text-sm font-semibold">
            {msg.compareSpecies}
          </h3>
          <ComparisonTable
            species={species}
            selected={selected.species_key}
            onSelect={(k) => update({ species: k })}
          />
          <p className="text-xs text-muted-foreground">{msg.compareNote}</p>
        </section>
      )}
      {ok &&
        mapEl &&
        createPortal(
          <MapOverlay
            result={selected}
            params={params}
            update={update}
            occurrencesAvailable={occAvailable}
            simple={simple}
          />,
          mapEl,
        )}
    </Panel>
  );
}

export function ResultsView() {
  const { id } = useParams();
  const job = useJob(id, false);
  const succeeded = job.data?.status === "succeeded";
  const results = useResults(id, succeeded);
  const msg = useResultsMessages();

  if (!id) return null;
  if (job.isPending || (succeeded && results.isPending)) {
    return (
      <Panel title={msg.results}>
        <div className="space-y-3" aria-busy="true">
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-32 w-full" />
          <Skeleton className="h-24 w-full" />
        </div>
      </Panel>
    );
  }
  if (job.isError) {
    return (
      <Panel title={msg.results}>
        <ErrorState
          error={job.error}
          onRetry={() => void job.refetch()}
          title={msg.cannotLoadRun}
        />
      </Panel>
    );
  }
  if (!succeeded) {
    return (
      <Panel
        title={job.data.name}
        subtitle={<JobStatusBadge status={job.data.status} />}
      >
        <p className="mb-3 text-sm text-muted-foreground">{msg.notCompleted}</p>
        <Button
          variant="outline"
          render={<Link to={`/jobs/${id}`} />}
          nativeButton={false}
        >
          <ArrowLeft aria-hidden /> {msg.goToRun}
        </Button>
      </Panel>
    );
  }
  if (results.isError) {
    return (
      <Panel title={job.data.name}>
        <ErrorState
          error={results.error}
          onRetry={() => void results.refetch()}
          title={msg.resultsUnavailable}
        />
      </Panel>
    );
  }
  if (!results.data) return null;
  return <Results key={id} id={id} data={results.data} />;
}
