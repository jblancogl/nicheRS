// Progress a non-specialist can read: the study's steps, and — while GBIF prepares the
// occurrence download — what is happening, for how long, and the proof that it is alive.

import { Check, Circle, ExternalLink, LoaderCircle } from "lucide-react";
import { useState } from "react";

import { errorMessage } from "@/api/client";
import { useGbifWait, useSkipDownload } from "@/api/hooks";
import type { Job } from "@/api/types";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
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
import { Button } from "@/components/ui/button";
import { formatCount, formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";

import { useWaitMessages } from "./waitMessages";

type StepState = "done" | "active" | "todo";

/** Environmental data take the first 15 % of a running study (engine.rs). */
const ENV_SHARE = 0.15;

function stepStates(job: Job): StepState[] {
  if (job.status === "succeeded") return ["done", "done", "done", "done"];
  if (job.status === "queued") return ["active", "todo", "todo", "todo"];
  if (job.status === "running") return job.progress < ENV_SHARE ? ["done", "active", "todo", "todo"] : ["done", "done", "active", "todo"];
  return ["todo", "todo", "todo", "todo"];
}

export function StepList({ job }: { job: Job }) {
  const m = useWaitMessages();
  const labels = [m.stepData, m.stepEnv, m.stepModel, m.stepResults];
  const states = stepStates(job);
  return (
    <ol aria-label={m.steps} className="space-y-1.5 text-sm">
      {labels.map((label, i) => {
        const st = states[i] ?? "todo";
        return (
          <li key={label} className={cn("flex items-center gap-2", st === "todo" && "text-muted-foreground")} aria-current={st === "active" ? "step" : undefined}>
            {st === "done" ? (
              <Check className="size-4 shrink-0 text-state-success" aria-hidden />
            ) : st === "active" ? (
              <LoaderCircle className="size-4 shrink-0 text-primary motion-safe:animate-spin" aria-hidden />
            ) : (
              <Circle className="size-4 shrink-0" aria-hidden />
            )}
            <span className={cn(st === "active" && "font-medium")}>{label}</span>
            <span className="sr-only">({st === "done" ? m.done : st === "active" ? m.inProgress : m.toDo})</span>
          </li>
        );
      })}
    </ol>
  );
}

const secondsSince = (iso: string, now: number) => Math.max(0, Math.round((now - new Date(iso).getTime()) / 1000));

/** A queued study: the GBIF wait when there is one, else the plain queue. */
export function QueuedPanel({ job, now }: { job: Job; now: number }) {
  const m = useWaitMessages();
  const wait = useGbifWait(job.id, job.status === "queued");
  const skip = useSkipDownload();
  const [confirm, setConfirm] = useState(false);
  const w = wait.data;

  if (!w) {
    return (
      <section className="space-y-1 rounded-lg border p-3" aria-live="polite">
        <h3 className="text-sm font-semibold">{m.waitStart}</h3>
        <p className="text-sm text-muted-foreground">{m.waitStartText}</p>
      </section>
    );
  }
  const waited = secondsSince(w.since, now);
  const checked = secondsSince(w.checked_at, now);
  const slow = waited >= w.slow_after_s;
  const canSkip = waited >= w.skip_after_s && w.status !== "downloading";
  return (
    <section className="space-y-3 rounded-lg border p-3" aria-labelledby="gbif-wait-title">
      <div className="flex items-center gap-2">
        <LoaderCircle className="size-4 shrink-0 text-primary motion-safe:animate-spin" aria-hidden />
        <h3 id="gbif-wait-title" className="text-sm font-semibold">
          {m.waitTitle}
        </h3>
      </div>
      <p className="text-sm">{m.waitText}</p>
      <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-sm">
        <dt className="text-muted-foreground">{m.waitingFor}</dt>
        <dd className="num">{formatDuration(waited * 1000)}</dd>
        <dt className="text-muted-foreground">{m.gbifStatus}</dt>
        <dd>{m.status(w.status)}</dd>
        <dt className="text-muted-foreground">{m.lastCheck}</dt>
        <dd className="num">{m.ago(checked)}</dd>
      </dl>
      {checked >= 120 && <p className="text-sm text-state-warning">{m.stale(Math.floor(checked / 60))}</p>}
      {w.url && (
        <a href={w.url} target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-1 text-sm text-primary underline-offset-4 hover:underline">
          {m.follow} <ExternalLink className="size-3.5" aria-hidden />
        </a>
      )}
      <p className="text-xs text-muted-foreground">{m.canClose}</p>

      {slow && (
        <Alert>
          <AlertTitle>{m.slowTitle}</AlertTitle>
          <AlertDescription>
            <p>{m.slowText}</p>
            {canSkip ? (
              <Button variant="outline" size="sm" className="mt-2" onClick={() => setConfirm(true)} disabled={skip.isPending || skip.isSuccess}>
                {m.skip}
              </Button>
            ) : (
              <p className="text-xs">{m.skipHint(Math.round(w.skip_after_s / 60))}</p>
            )}
          </AlertDescription>
        </Alert>
      )}
      {skip.isError && <p className="text-sm text-destructive">{m.skipFailed}: {errorMessage(skip.error)}</p>}

      <AlertDialog open={confirm} onOpenChange={setConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{m.skipTitle}</AlertDialogTitle>
            <AlertDialogDescription>{m.skipText(formatCount(w.search_api_max_records))}</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{m.keepWaiting}</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                setConfirm(false);
                skip.mutate(job.id);
              }}
            >
              {m.skipConfirm}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </section>
  );
}
