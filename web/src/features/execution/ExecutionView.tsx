import { ArrowRight, Copy, Radio } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";

import { useJobEvents } from "@/api/events";
import { isTerminal, useCancelJob, useJob } from "@/api/hooks";
import type { JobEvent, ValidationSettings } from "@/api/types";
import { ErrorState, KeyValue, SciName, SyntheticBadge } from "@/components/domain/common";
import { Panel } from "@/components/domain/Panel";
import { useDomainMessages } from "@/components/domain/messages";
import { JobStatusBadge, SpeciesStatusBadge } from "@/components/domain/status";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { formatDateTime, formatDuration, formatTime } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useDraft } from "@/stores/draft";
import { useMapStore } from "@/stores/map";

import { useJobText } from "./jobText";
import { useExecutionMessages } from "./messages";
import { QueuedPanel, StepList } from "./WaitPanel";
import { useWaitMessages } from "./waitMessages";

function useNow(active: boolean) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!active) return;
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [active]);
  return now;
}

function EventLog({ events }: { events: JobEvent[] }) {
  const ref = useRef<HTMLOListElement>(null);
  const stick = useRef(true);
  const m = useExecutionMessages();
  const jt = useJobText();
  useEffect(() => {
    const el = ref.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
  }, [events]);
  if (events.length === 0) return <p className="text-sm text-muted-foreground">{m.noEvents}</p>;
  return (
    <ol
      ref={ref}
      onScroll={(e) => {
        const el = e.currentTarget;
        stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
      }}
      className="max-h-64 space-y-0.5 overflow-y-auto rounded-md border bg-muted/40 p-2 font-mono text-xs"
      aria-label={m.eventLog}
      tabIndex={0}
    >
      {events.map((e) => (
        <li key={e.seq} className={cn("flex gap-2", e.level === "error" && "text-destructive", e.level === "warning" && "text-state-warning")}>
          <span className="num shrink-0 text-muted-foreground">{formatTime(e.at)}</span>
          <span className="shrink-0 uppercase">{e.level === "info" ? "" : e.level}</span>
          <span className="min-w-0 break-words text-foreground">{jt.event(e.message)}</span>
        </li>
      ))}
    </ol>
  );
}

function failureHelp(status: string, m: ReturnType<typeof useExecutionMessages>): string {
  if (status === "interrupted") return m.helpInterrupted;
  if (status === "cancelled") return m.helpCancelled;
  return m.helpFailed;
}

function JobExecution({ id }: { id: string }) {
  const navigate = useNavigate();
  const job = useJob(id, false);
  const terminal = job.data ? isTerminal(job.data.status) : false;
  const stream = useJobEvents(id, Boolean(job.data) && !terminal);
  // Second observer of the same query: polls every 2 s only when SSE is unavailable.
  useJob(id, stream.state === "polling");
  const cancel = useCancelJob();
  const [confirm, setConfirm] = useState(false);
  const loadSpec = useDraft((s) => s.loadSpec);
  const now = useNow(!terminal);
  const m = useExecutionMessages();
  const dm = useDomainMessages();
  const wm = useWaitMessages();
  const jt = useJobText();

  // Map shows the run's study area.
  const area = job.data?.spec.area.geometry ?? null;
  useEffect(() => {
    const m = useMapStore.getState();
    m.setRaster(null);
    m.setPoints([]);
    m.setGrid(null);
    m.setStudyArea(area);
  }, [area]);

  // Completed → results (interaction-specifications §5).
  const status = job.data?.status;
  useEffect(() => {
    if (status === "succeeded") void navigate(`/jobs/${id}/results`, { replace: true });
  }, [status, id, navigate]);

  if (job.isPending) {
    return (
      <Panel title={m.modelRun}>
        <Skeleton className="h-24 w-full" />
      </Panel>
    );
  }
  if (job.isError) {
    return (
      <Panel title={m.modelRun}>
        <ErrorState error={job.error} onRetry={() => void job.refetch()} title={m.cannotLoad} />
      </Panel>
    );
  }
  const j = job.data;
  const started = j.started_at ? new Date(j.started_at).getTime() : null;
  const finished = j.finished_at ? new Date(j.finished_at).getTime() : null;
  const elapsed = started ? (finished ?? now) - started : null;
  const pct = Math.round(Math.min(1, Math.max(0, j.progress)) * 100);
  // live region: announces stage transitions and terminal states only (not every % tick)
  const announcement = j.status === "running" ? jt.stage(j.stage) : dm.job(j.status);
  const warnings = stream.events.filter((e) => e.level === "warning").length;
  const canCancel = j.status === "queued" || j.status === "running";

  return (
    <Panel
      title={j.name}
      subtitle={
        <span className="flex flex-wrap items-center gap-2">
          <JobStatusBadge status={j.status} />
          <span className="num">{m.created(formatDateTime(j.created_at))}</span>
          {j.spec.demo && !/SYNTHETIC/.test(j.spec.species[0]?.scientific_name ?? "") && <SyntheticBadge />}
        </span>
      }
      focusKey={j.status}
      footer={
        <>
          {terminal ? (
            <>
              <Button
                variant="outline"
                onClick={() => {
                  loadSpec(j.spec, j.project_id);
                  void navigate("/new");
                }}
              >
                <Copy aria-hidden /> {m.duplicate}
              </Button>
              {j.status === "succeeded" && (
                <Button render={<Link to={`/jobs/${id}/results`} />} nativeButton={false}>
                  {m.openResults} <ArrowRight aria-hidden />
                </Button>
              )}
            </>
          ) : (
            <Button variant="destructive" onClick={() => setConfirm(true)} disabled={!canCancel || cancel.isPending}>
              {cancel.isPending ? m.cancelling : m.cancelRun}
            </Button>
          )}
        </>
      }
    >
      <div className="space-y-5">
        <div role="status" aria-live="polite" className="sr-only">
          {announcement}
        </div>
        {j.status === "queued" && (
          <section className="space-y-4" aria-label={m.progress}>
            <QueuedPanel job={j} now={now} />
            <StepList job={j} />
          </section>
        )}
        {j.status !== "queued" && (
        <section className="space-y-2" aria-label={m.progress}>
          <div className="flex items-baseline justify-between gap-2 text-sm">
            <span className="font-medium">{jt.stage(j.stage) || "—"}</span>
            <span className="num whitespace-nowrap text-muted-foreground">{pct} %</span>
          </div>
          <Progress value={pct} aria-label={m.overallProgress} />
          <div className="num flex flex-wrap justify-between gap-2 text-xs text-muted-foreground">
            <span>{elapsed !== null ? m.elapsed(formatDuration(elapsed)) : m.notStarted}</span>
            {!terminal && (
              <span className="inline-flex items-center gap-1">
                <Radio className="size-3" aria-hidden /> {m.stream(stream.state)}
              </span>
            )}
          </div>
          {warnings > 0 && (
            <p className="text-sm text-state-warning">{m.warningsSoFar(warnings)}</p>
          )}
          {j.status === "running" && <StepList job={j} />}
        </section>
        )}

        {(j.status === "failed" || j.status === "interrupted" || j.status === "cancelled") && (
          <Alert variant={j.status === "cancelled" ? "default" : "destructive"}>
            <AlertTitle>
              {j.status === "failed" ? m.failed : j.status === "interrupted" ? m.interrupted : m.cancelled}
              {j.stage ? m.atStage(jt.stage(j.stage)) : ""}
            </AlertTitle>
            <AlertDescription>
              {j.error && <p className="font-medium text-foreground">{j.error}</p>}
              <p>{failureHelp(j.status, m)}</p>
            </AlertDescription>
          </Alert>
        )}
        {cancel.isError && <ErrorState error={cancel.error} title={m.cancelFailed} />}

        <section className="space-y-2" aria-labelledby="species-status">
          <h3 id="species-status" className="text-sm font-semibold">
            {m.species(j.species.length)}
          </h3>
          <ul className="divide-y rounded-lg border">
            {j.species.map((s) => (
              <li key={s.key} className="space-y-1 px-3 py-2">
                <div className="flex items-center justify-between gap-2">
                  <SciName name={s.scientific_name} className="min-w-0 truncate text-sm" />
                  <SpeciesStatusBadge status={s.status} jobFinished={terminal} />
                </div>
                {s.error && <p className="text-xs text-destructive">{s.error}</p>}
              </li>
            ))}
          </ul>
        </section>

        <details className="group space-y-2" open={j.status === "failed" || undefined}>
          <summary className="cursor-pointer text-sm font-semibold text-muted-foreground hover:text-foreground">
            {wm.details(stream.events.length)}
          </summary>
          <div className="mt-2 space-y-2">
            <EventLog events={stream.events} />
            {stream.state === "polling" && (
              <p className="text-xs text-muted-foreground">
                {m.pollingNote}
              </p>
            )}
          </div>
        </details>

        <section className="space-y-2" aria-labelledby="run-config">
          <h3 id="run-config" className="text-sm font-semibold">
            {m.configuration}
          </h3>
          <KeyValue
            items={[
              [m.studyArea, j.spec.area.name],
              [m.variables, String(j.spec.layers.length)],
              [m.resolution, `×${j.spec.resolution_factor}`],
              [m.validation, validationText(j.spec.validation, m)],
              [m.seed, String(j.spec.seed)],
              [m.runId, <span key="id" className="font-mono text-xs">{j.id}</span>],
            ]}
          />
        </section>
      </div>

      <AlertDialog open={confirm} onOpenChange={setConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{m.confirmTitle(j.name)}</AlertDialogTitle>
            <AlertDialogDescription>{m.confirmText}</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{m.keepRunning}</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => {
                setConfirm(false);
                cancel.mutate(id);
              }}
            >
              {m.cancelRun}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Panel>
  );
}

export function ExecutionView() {
  const { id } = useParams();
  if (!id) return null;
  // keyed so the event log and stream state reset when switching runs
  return <JobExecution key={id} id={id} />;
}

/** Validation scheme of a study, with the optional evaluations (ADR-0014). */
function validationText(v: ValidationSettings, m: ReturnType<typeof useExecutionMessages>): string {
  const parts = [
    v.method === "spatial_block"
      ? m.spatialBlocks(v.folds)
      : v.method === "spatial_grid"
        ? m.spatialGrid(v.folds, v.block_km ?? null)
        : m.randomKfold(v.folds),
  ];
  if ((v.null_models ?? 0) > 0) parts.push(m.nullModels(v.null_models ?? 0));
  if (v.aoa) parts.push(m.aoa);
  return parts.join(" · ");
}
