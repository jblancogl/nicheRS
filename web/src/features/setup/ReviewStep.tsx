import { useEffect, useRef, useState, type ReactNode } from "react";
import { useNavigate } from "react-router";

import { errorMessage } from "@/api/client";
import { useCreateJob, usePreview, useProjects } from "@/api/hooks";
import { ErrorState, SciName, SyntheticBadge, WarningsList } from "@/components/domain/common";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { formatArea, formatCount, formatDate, resolutionLabel } from "@/lib/format";
import { areaKm2 } from "@/lib/geo";
import { useProjectionMessages } from "@/features/projections/messages";
import { DEFAULT_FILTERS, describeModel, modelDiffs } from "@/lib/schema";
import { draftToSpec, useDraft, type StepIndex } from "@/stores/draft";
import { useOwnDataMessages } from "@/features/owndata/messages";
import { useWorkspaceMessages } from "@/features/workspace/messages";

import { useSelectedLayers } from "./layerSelection";
import { useReviewMessages } from "./messages";
import { useExtentMessages } from "./extentMessages";
import { usePathMessages } from "./pathMessages";
import { usePreviewRequest, useResolutionFit } from "./previewRequest";
import { WizardFrame } from "./WizardFrame";

function Row({ label, step, editText, children }: { label: string; step: StepIndex; editText?: string; children: ReactNode }) {
  const setStep = useDraft((s) => s.setStep);
  const setPath = useDraft((s) => s.setPath);
  const quick = useDraft((s) => s.path === "quick");
  const m = useReviewMessages();
  // The quick path does not show the data, variables and model steps: editing them opens Expert.
  const edit = () => {
    if (quick && step > 1) setPath("expert");
    setStep(step);
  };
  return (
    <div className="flex items-start gap-3 border-b py-2.5 last:border-b-0">
      <div className="w-24 shrink-0 text-sm text-muted-foreground">{label}</div>
      <div className="min-w-0 flex-1 text-sm">{children}</div>
      <Button variant="link" size="sm" className="h-auto px-0" onClick={edit} aria-label={editText ?? m.editLabel(label)}>
        {m.edit}
      </Button>
    </div>
  );
}

/** Project the study is created in (organisational only: not part of the specification). */
function ProjectField() {
  const w = useWorkspaceMessages();
  const projects = useProjects().data ?? [];
  const projectId = useDraft((s) => s.projectId);
  const setProject = useDraft((s) => s.setProject);
  if (projects.length === 0) return null;
  return (
    <div className="space-y-1.5">
      <Label htmlFor="study-project">{w.project}</Label>
      <select
        id="study-project"
        value={projectId ?? ""}
        onChange={(e) => setProject(e.target.value || null)}
        className="h-9 w-full rounded-md border border-input bg-transparent px-2 text-sm shadow-xs focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none"
      >
        <option value="">{w.noProject}</option>
        {projects.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>
    </div>
  );
}

export function ReviewStep() {
  const draft = useDraft();
  const navigate = useNavigate();
  const create = useCreateJob();
  const { ids, layers } = useSelectedLayers();
  const preview = usePreview(usePreviewRequest());
  const [ack, setAck] = useState(false);
  const fit = useResolutionFit();
  const xm = useExtentMessages();
  const m = useReviewMessages();
  const p = usePathMessages();
  const pm = useProjectionMessages();
  const own = useOwnDataMessages();
  const quick = draft.path === "quick";
  const errRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (create.isError) errRef.current?.scrollIntoView({ block: "nearest" });
  }, [create.isError]);

  const spec = draftToSpec(draft, ids);
  const blocking = preview.data?.warnings.filter((w) => w.severity === "blocking") ?? [];
  const needsAck = blocking.length > 0;
  const diffs = modelDiffs(draft.model, draft.validation, draft.seed, draft.filters.thin_km ?? null);
  const layerNames = (layers ?? []).filter((l) => ids.includes(l.id)).map((l) => l.short_name);
  // The quick path does not hide settings changed in Expert mode (or copied from a duplicated
  // study): it lists them like the Expert review does (AC-SET-8).
  const customised =
    draft.layers !== null ||
    diffs.length > 0 ||
    (draft.projections ?? []).length > 0 ||
    JSON.stringify(draft.filters) !== JSON.stringify(DEFAULT_FILTERS) ||
    (fit.finest !== null && draft.resolution_factor !== fit.finest);

  // The quick path has no Data step to choose a coarser grid: use the finest the area fits at.
  const { setResolution } = draft;
  useEffect(() => {
    if (quick && fit.known && !fit.currentFits && fit.finest !== null) setResolution(fit.finest);
  }, [quick, fit.known, fit.currentFits, fit.finest, setResolution]);

  const launch = () => {
    if (!spec) return;
    create.mutate({ spec, projectId: draft.projectId }, {
      onSuccess: (job) => {
        void navigate(`/jobs/${job.id}`);
      },
    });
  };

  return (
    <WizardFrame
      title={quick ? p.reviewTitle : m.title}
      subtitle={quick ? p.reviewSubtitle : m.subtitle}
      nextLabel={m.launch}
      onNext={launch}
      nextPending={create.isPending}
      nextDisabled={!spec || !fit.currentFits || (needsAck && !ack)}
      nextReason={!spec ? m.incomplete : !fit.currentFits ? xm.reviewBlocked : m.ackFirst}
      secondary={
        <AlertDialog>
          <AlertDialogTrigger render={<Button variant="ghost" />}>{m.startOver}</AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>{m.discardTitle}</AlertDialogTitle>
              <AlertDialogDescription>
                {m.discardText}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>{m.keep}</AlertDialogCancel>
              <AlertDialogAction
                variant="destructive"
                onClick={() => {
                  draft.reset();
                  void navigate("/");
                }}
              >
                {m.discard}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      }
    >
      <div className="space-y-5">
        {/* Quick studies are named after their species and area (draftToSpec), unless a name was
            given in Expert mode. */}
        {(!quick || draft.name.trim() !== "") && (
          <div className="space-y-1.5">
            <Label htmlFor="study-name">{m.runName}</Label>
            <Input
              id="study-name"
              value={draft.name}
              placeholder={spec?.name ?? m.runNamePlaceholder}
              onChange={(e) => draft.setName(e.target.value)}
            />
          </div>
        )}

        <ProjectField />

        {draft.demo && (
          <p className="flex items-center gap-2 rounded-md bg-state-warning-subtle px-3 py-2 text-sm">
            <SyntheticBadge /> {m.demoNote}
          </p>
        )}

        <div className="rounded-lg border px-3">
          <Row label={m.species} step={0}>
            <ul className="space-y-0.5">
              {draft.species.map((s) => (
                <li key={s.key}>
                  <SciName name={s.scientific_name} className="flex-wrap" />
                  {s.own && draft.ownDataset && (
                    <span className="block text-xs text-muted-foreground">
                      {draft.ownDataset.name}: {own.source(s.own.role === "validation" ? "validation" : s.own.gbif ? "both" : "own")}
                    </span>
                  )}
                </li>
              ))}
            </ul>
          </Row>
          <Row label={m.area} step={1}>
            {draft.area ? (
              <>
                {draft.area.name} <span className="num text-muted-foreground">≈ {formatArea(areaKm2(draft.area.geometry))}</span>
              </>
            ) : (
              "—"
            )}
          </Row>
          {quick && !customised ? (
            <Row label={p.settings} step={2} editText={p.adjust}>
              {p.recommended}
            </Row>
          ) : (
          <>
          {quick && <p className="border-b py-2.5 text-sm text-muted-foreground">{p.custom}</p>}
          <Row label={m.data} step={2}>
            <span className="num">
              {m.dataValue(
                resolutionLabel(draft.resolution_factor),
                draft.filters.year_min,
                formatCount(draft.filters.max_uncertainty_m),
                draft.filters.licences.length,
              )}
            </span>
          </Row>
          <Row label={m.variables} step={3}>
            <span className="num">{m.selected(ids.length)}</span>
            {layerNames.length > 0 && <span className="block font-mono text-xs text-muted-foreground">{layerNames.join(", ")}</span>}
          </Row>
          <Row label={m.model} step={4}>
            {describeModel(draft.model, draft.validation)}
            {diffs.length > 0 && (
              <ul className="mt-1 list-disc pl-4 text-xs text-muted-foreground">
                {diffs.map((d) => (
                  <li key={d}>{d}</li>
                ))}
              </ul>
            )}
            <span className="num block text-xs text-muted-foreground">{m.seed(draft.seed)}</span>
            {(draft.projections ?? []).length > 0 && (
              <span className="block text-xs text-muted-foreground">
                {pm.summary(
                  (draft.projections ?? []).length,
                  (draft.projections ?? []).reduce((n, x) => n + x.gcms.length, 0),
                )}
                {": "}
                <span className="num">{(draft.projections ?? []).map((x) => `${x.period} ${x.ssp}`).join(", ")}</span>
              </span>
            )}
          </Row>
          </>
          )}
        </div>

        <section className="space-y-2" aria-labelledby="preflight">
          <h3 id="preflight" className="text-sm font-semibold">
            {quick ? p.checks : m.preflight} {draft.species.length > 1 && <span className="font-normal text-muted-foreground">{m.firstSpecies}</span>}
          </h3>
          {preview.isPending && (quick ? <p className="text-sm text-muted-foreground" role="status">{p.searching}</p> : <Skeleton className="h-12 w-full" />)}
          {preview.isError && (
            <p className="text-sm text-muted-foreground">
              {m.previewUnavailable(errorMessage(preview.error))}
            </p>
          )}
          {preview.data && (
            <>
              {quick ? (
                <>
                  <p className="num text-sm">{p.found(formatCount(preview.data.retained_cells), formatCount(preview.data.records_fetched))}</p>
                  {preview.data.truncated && <p className="text-xs text-muted-foreground">{p.truncated}</p>}
                </>
              ) : (
                <p className="num text-sm">
                  {m.previewLine(formatCount(preview.data.retained_cells), formatCount(preview.data.records_fetched), preview.data.truncated)}
                </p>
              )}
              {preview.data.warnings.length === 0 ? (
                <p className="text-sm text-muted-foreground">{m.noWarnings}</p>
              ) : (
                <WarningsList warnings={preview.data.warnings} />
              )}
              {needsAck && (
                <Label className="flex items-start gap-2 font-normal">
                  <Checkbox checked={ack} onCheckedChange={setAck} className="mt-0.5" />
                  <span>{m.ack(blocking.length)}</span>
                </Label>
              )}
            </>
          )}
        </section>

        {fit.mirror && !draft.demo && <p className="text-sm text-muted-foreground">{m.mirrorNote(formatDate(fit.mirror))}</p>}

        {quick && <p className="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground">{p.meaning}</p>}

        <div ref={errRef}>
          {create.isError && <ErrorState error={create.error} title={m.launchFailed} />}
        </div>
      </div>
    </WizardFrame>
  );
}
