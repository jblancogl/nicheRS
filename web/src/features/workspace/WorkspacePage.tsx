// The organisation's workspace, entry point of NicheRS: projects on the left, the selected
// project's studies on the right (in progress first, then a table), the study being prepared,
// and a welcome block while there are no studies yet. A plain page: the map opens inside a
// study or the setup wizard.

import {
  ArrowRight,
  BookOpen,
  Copy,
  EllipsisVertical,
  FlaskConical,
  FolderInput,
  FolderOpen,
  GitCompare,
  Folders,
  Layers,
  LayoutGrid,
  List,
  LoaderCircle,
  Map as MapIcon,
  Pencil,
  Plus,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useState, type FormEvent, type ReactNode } from "react";
import { Link, useNavigate, useSearchParams } from "react-router";

import { errorMessage, urls } from "@/api/client";
import {
  isTerminal,
  useCreateProject,
  useDeleteJob,
  useDeleteProject,
  useGbifWait,
  useJobs,
  useMe,
  useProjects,
  useRenameProject,
  useUpdateJob,
} from "@/api/hooks";
import type { EvidenceLevel, Job, Project } from "@/api/types";
import { AreaSilhouette } from "@/components/domain/AreaSilhouette";
import { ErrorState, SciName, SyntheticBadge } from "@/components/domain/common";
import { EvidenceBadge, JobStatusBadge } from "@/components/domain/status";
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Skeleton } from "@/components/ui/skeleton";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { useJobsMessages } from "@/features/jobs/messages";
import { useHelp } from "@/features/help/store";
import { useHomeMessages } from "@/features/setup/homeMessages";
import { useStepLabels } from "@/features/setup/messages";
import { formatDateTime, formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";
import { STEPS, pathSteps, useDraft } from "@/stores/draft";

import { useWorkspaceMessages } from "./messages";
import { queuedText, useJobText } from "@/features/execution/jobText";

const LEVELS: EvidenceLevel[] = ["validated", "supported", "indicative", "exploratory", "insufficient"];
/** URL value of the "studies without project" bucket. */
const NONE = "none";

const active = (j: Job) => j.status === "queued" || j.status === "running";
const studyHref = (j: Job) => (j.status === "succeeded" ? `/jobs/${j.id}/results` : `/jobs/${j.id}`);

// ---- dialogs ---------------------------------------------------------------------------------

function NameDialog({
  open,
  onOpenChange,
  title,
  label,
  hint,
  initial,
  action,
  pending,
  error,
  onSubmit,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  title: string;
  label: string;
  hint?: string;
  initial: string;
  action: string;
  pending: boolean;
  error: unknown;
  onSubmit: (name: string) => void;
}) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        {/* Remounted on every opening, so the field starts from the current name. */}
        {open && (
          <NameForm title={title} label={label} hint={hint} initial={initial} action={action} pending={pending} error={error} onSubmit={onSubmit} />
        )}
      </AlertDialogContent>
    </AlertDialog>
  );
}

function NameForm({
  title,
  label,
  hint,
  initial,
  action,
  pending,
  error,
  onSubmit,
}: {
  title: string;
  label: string;
  hint?: string;
  initial: string;
  action: string;
  pending: boolean;
  error: unknown;
  onSubmit: (name: string) => void;
}) {
  const w = useWorkspaceMessages();
  const [name, setName] = useState(initial);
  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (name.trim()) onSubmit(name.trim());
  };
  return (
    <form onSubmit={submit} className="space-y-4">
      <AlertDialogHeader>
        <AlertDialogTitle>{title}</AlertDialogTitle>
      </AlertDialogHeader>
      <div className="space-y-1.5">
        <Label htmlFor="ws-name">{label}</Label>
        <Input id="ws-name" value={name} onChange={(e) => setName(e.target.value)} autoFocus maxLength={200} />
        {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
        {error !== null && error !== undefined && <p className="text-sm text-destructive">{errorMessage(error)}</p>}
      </div>
      <AlertDialogFooter>
        <AlertDialogCancel type="button">{w.cancel}</AlertDialogCancel>
        <Button type="submit" disabled={pending || !name.trim()}>
          {action}
        </Button>
      </AlertDialogFooter>
    </form>
  );
}

function MoveDialog({ job, projects, open, onOpenChange }: { job: Job; projects: Project[]; open: boolean; onOpenChange: (o: boolean) => void }) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>{open && <MoveForm job={job} projects={projects} onDone={() => onOpenChange(false)} />}</AlertDialogContent>
    </AlertDialog>
  );
}

function MoveForm({ job, projects, onDone }: { job: Job; projects: Project[]; onDone: () => void }) {
  const w = useWorkspaceMessages();
  const update = useUpdateJob();
  const [target, setTarget] = useState(job.project_id ?? NONE);
  return (
    <>
      <AlertDialogHeader>
        <AlertDialogTitle>{w.moveStudy(job.name)}</AlertDialogTitle>
      </AlertDialogHeader>
      <RadioGroup value={target} onValueChange={(v) => setTarget(String(v))} aria-label={w.projects} className="max-h-72 overflow-y-auto">
        {[{ id: NONE, name: w.noProject }, ...projects].map((p) => (
          <Label key={p.id} className="flex cursor-pointer items-center gap-2 rounded-md border p-2 font-normal has-data-checked:border-primary">
            <RadioGroupItem value={p.id} /> {p.name}
          </Label>
        ))}
      </RadioGroup>
      {update.isError && <p className="text-sm text-destructive">{errorMessage(update.error)}</p>}
      <AlertDialogFooter>
        <AlertDialogCancel>{w.cancel}</AlertDialogCancel>
        <Button
          disabled={update.isPending}
          onClick={() =>
            update.mutate(
              { id: job.id, project_id: target === NONE ? null : target },
              { onSuccess: onDone },
            )
          }
        >
          {w.move}
        </Button>
      </AlertDialogFooter>
    </>
  );
}

function DeleteStudyDialog({ job, open, onOpenChange }: { job: Job; open: boolean; onOpenChange: (o: boolean) => void }) {
  const m = useJobsMessages();
  const del = useDeleteJob();
  const me = useMe();
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{m.deleteTitle(job.name)}</AlertDialogTitle>
          <AlertDialogDescription>
            {m.deleteText}
            {me.data && m.deleteQuota}
          </AlertDialogDescription>
        </AlertDialogHeader>
        {del.isError && <p className="text-sm text-destructive">{errorMessage(del.error)}</p>}
        <AlertDialogFooter>
          <AlertDialogCancel>{m.keepRun}</AlertDialogCancel>
          <AlertDialogAction variant="destructive" onClick={() => del.mutate(job.id)}>
            {m.deleteRun}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

/** Open, duplicate, rename, move, delete. */
function StudyActions({ job, projects }: { job: Job; projects: Project[] }) {
  const m = useJobsMessages();
  const w = useWorkspaceMessages();
  const loadSpec = useDraft((s) => s.loadSpec);
  const navigate = useNavigate();
  const update = useUpdateJob();
  const [dialog, setDialog] = useState<"rename" | "move" | "delete" | null>(null);
  // "interrupted" is resumable, so it is not finished for the server.
  const deletable = isTerminal(job.status) && job.status !== "interrupted";
  const close = (o: boolean) => !o && setDialog(null);
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger render={<Button variant="ghost" size="icon-sm" aria-label={m.moreActions(job.name)} />}>
          <EllipsisVertical aria-hidden />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-60">
          <DropdownMenuItem onClick={() => void navigate(studyHref(job))}>
            <FolderOpen aria-hidden /> {w.open}
          </DropdownMenuItem>
          {job.status === "succeeded" && (
            <DropdownMenuItem onClick={() => void navigate(`/jobs/${job.id}/results?view=full&tab=decision&ds=compare`)}>
              <GitCompare aria-hidden /> {w.compareWith}
            </DropdownMenuItem>
          )}
          {job.status === "succeeded" && job.spec.species.length > 1 && (
            <DropdownMenuItem onClick={() => void navigate(`/jobs/${job.id}/results?view=full&tab=decision&ds=richness`)}>
              <Layers aria-hidden /> {w.speciesSuitable}
            </DropdownMenuItem>
          )}
          <DropdownMenuItem
            onClick={() => {
              loadSpec(job.spec, job.project_id);
              void navigate("/new");
            }}
          >
            <Copy aria-hidden /> {m.duplicate}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => setDialog("rename")}>
            <Pencil aria-hidden /> {w.rename}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => setDialog("move")}>
            <FolderInput aria-hidden /> {w.moveTo}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem disabled={!deletable} onClick={() => setDialog("delete")} className="text-destructive">
            <Trash2 aria-hidden /> {m.delete}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <NameDialog
        open={dialog === "rename"}
        onOpenChange={close}
        title={w.renameStudy}
        label={w.studyName}
        initial={job.name}
        action={w.save}
        pending={update.isPending}
        error={update.error}
        onSubmit={(name) => update.mutate({ id: job.id, name }, { onSuccess: () => setDialog(null) })}
      />
      <MoveDialog job={job} projects={projects} open={dialog === "move"} onOpenChange={close} />
      <DeleteStudyDialog job={job} open={dialog === "delete"} onOpenChange={close} />
    </>
  );
}

// ---- studies ---------------------------------------------------------------------------------

function Outcome({ job }: { job: Job }) {
  const m = useJobsMessages();
  const counts = LEVELS.map((l) => [l, job.species.filter((s) => s.status === "succeeded" && s.evidence_level === l).length] as const).filter(
    ([, n]) => n > 0,
  );
  const noModel = job.species.filter((s) => s.status === "skipped").length;
  const failed = job.species.filter((s) => s.status === "failed").length;
  return (
    <div className="flex flex-wrap items-center gap-1.5 text-xs">
      {counts.map(([level, n]) => (
        <span key={level} className="inline-flex items-center gap-1">
          <EvidenceBadge level={level} />
          {n > 1 && <span className="num text-muted-foreground">×{n}</span>}
        </span>
      ))}
      {noModel > 0 && <span className="text-muted-foreground">{m.noModel(noModel)}</span>}
      {failed > 0 && <span className="text-destructive">{m.failedSpecies(failed)}</span>}
    </div>
  );
}

function ActiveCard({ job, now, projects }: { job: Job; now: number; projects: Project[] }) {
  const jt = useJobText();
  const m = useJobsMessages();
  const done = job.species.filter((s) => s.status !== "pending" && s.status !== "running").length;
  const started = job.started_at ? Date.parse(job.started_at) : null;
  const pct = Math.round(Math.max(0, Math.min(1, job.progress)) * 100);
  return (
    <li className="space-y-2 rounded-lg border border-primary/30 bg-primary/5 p-3">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0 space-y-1">
          <Link to={studyHref(job)} className="block truncate font-medium underline-offset-4 hover:underline">
            {job.name}
          </Link>
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
            <JobStatusBadge status={job.status} />
            {job.spec.demo && <SyntheticBadge />}
            <span className="truncate">{job.spec.area.name}</span>
          </div>
        </div>
        <StudyActions job={job} projects={projects} />
      </div>
      {job.status === "queued" ? (
        <p className="text-xs text-muted-foreground">{queuedText(job, m.waiting, jt.stage)}</p>
      ) : (
        <div className="space-y-1">
          <div className="flex items-baseline justify-between gap-2 text-xs">
            <span className="min-w-0 truncate">
              {job.species.length > 1 && <span className="font-medium">{m.speciesDone(done, job.species.length)} · </span>}
              {jt.stage(job.stage)}
            </span>
            <span className="num shrink-0 text-muted-foreground">{pct} %</span>
          </div>
          <Progress value={pct} aria-label={job.name} />
          {started !== null && <p className="num text-xs text-muted-foreground">{m.elapsed(formatDuration(now - started))}</p>}
        </div>
      )}
      <Button size="sm" variant="outline" render={<Link to={studyHref(job)} />} nativeButton={false}>
        {m.viewProgress}
      </Button>
    </li>
  );
}

function StudiesTable({ jobs, projects, showProject }: { jobs: Job[]; projects: Project[]; showProject: boolean }) {
  const m = useJobsMessages();
  const w = useWorkspaceMessages();
  const projectName = (id?: string | null) => projects.find((p) => p.id === id)?.name;
  return (
    <div className="overflow-hidden rounded-lg border">
      <table className="w-full text-sm">
        <thead className="bg-muted/50 text-left text-xs text-muted-foreground">
          <tr>
            <th className="px-3 py-2 font-medium">{w.colStudy}</th>
            <th className="hidden px-3 py-2 font-medium md:table-cell">{w.colOutcome}</th>
            <th className="hidden px-3 py-2 font-medium lg:table-cell">{w.colCreated}</th>
            <th className="w-10 px-1 py-2">
              <span className="sr-only">{m.moreActions("")}</span>
            </th>
          </tr>
        </thead>
        <tbody className="divide-y">
          {jobs.map((j) => {
            const took = j.started_at && j.finished_at ? Date.parse(j.finished_at) - Date.parse(j.started_at) : null;
            const first = j.spec.species[0];
            return (
              <tr key={j.id} className="align-top hover:bg-muted/30">
                <td className="space-y-1 px-3 py-2.5">
                  <Link to={studyHref(j)} className="font-medium underline-offset-4 hover:underline">
                    {j.name}
                  </Link>
                  <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
                    <JobStatusBadge status={j.status} />
                    {/* The demo species name already carries its SYNTHETIC badge. */}
                    {j.spec.demo && !/SYNTHETIC/.test(first?.scientific_name ?? "") && <SyntheticBadge />}
                    {first && <SciName name={first.scientific_name} />}
                    {j.spec.species.length > 1 && <span className="num">+{j.spec.species.length - 1}</span>}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {j.spec.area.name}
                    {showProject && projectName(j.project_id) && <> · {w.inProject(projectName(j.project_id) ?? "")}</>}
                  </p>
                  <div className="md:hidden">
                    <Outcome job={j} />
                  </div>
                  {j.status === "interrupted" && <p className="text-xs text-state-warning">{m.resumes}</p>}
                </td>
                <td className="hidden px-3 py-2.5 md:table-cell">
                  <Outcome job={j} />
                </td>
                <td className="num hidden px-3 py-2.5 text-xs whitespace-nowrap text-muted-foreground lg:table-cell">
                  {formatDateTime(j.created_at)}
                  {j.created_by_name && <div>{w.by(j.created_by_name)}</div>}
                  {took !== null && took >= 0 && <div>{m.tookTime(formatDuration(took))}</div>}
                </td>
                <td className="px-1 py-2">
                  <StudyActions job={j} projects={projects} />
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

// ---- gallery ---------------------------------------------------------------------------------

/** Species name or area already in the study name are not repeated under it. */
function mentions(name: string, text: string): boolean {
  return name.toLowerCase().includes(text.toLowerCase());
}

function CardMedia({ job, now }: { job: Job; now: number }) {
  const jt = useJobText();
  const w = useWorkspaceMessages();
  const m = useJobsMessages();
  const [broken, setBroken] = useState(false);
  const fitted = job.species.find((s) => s.status === "succeeded");
  const gbif = useGbifWait(job.id, job.status === "queued").data;
  if (active(job)) {
    const pct = Math.round(Math.max(0, Math.min(1, job.progress)) * 100);
    const done = job.species.filter((s) => s.status !== "pending" && s.status !== "running").length;
    const started = job.started_at ? Date.parse(job.started_at) : null;
    const queued = job.status === "queued";
    const secondsSince = (iso: string) => Math.max(0, Math.round((now - Date.parse(iso)) / 1000));
    // The card already names the species: show only the step ("Cross-validation: 31/60 models").
    const sp = job.species.find((s) => job.stage.startsWith(`${s.scientific_name}: `));
    const step = sp ? job.stage.slice(sp.scientific_name.length + 2) : job.stage;
    const headline = queued ? (gbif ? w.cardGbif(Math.floor(secondsSince(gbif.since) / 60)) : w.cardQueued) : `${pct} % · ${jt.stage(step)}`;
    const detail = queued
      ? gbif
        ? w.cardGbifDetail(secondsSince(gbif.checked_at))
        : w.cardQueuedDetail
      : [job.species.length > 1 ? m.speciesDone(done, job.species.length) : null, started !== null ? m.elapsed(formatDuration(now - started)) : null]
          .filter(Boolean)
          .join(" · ");
    return (
      <div className="relative h-full overflow-hidden bg-primary/5">
        <AreaSilhouette geometry={job.spec.area.geometry} className="absolute inset-x-6 top-5 bottom-20 h-[calc(100%-6.25rem)] w-[calc(100%-3rem)] text-primary" />
        <div className="absolute inset-x-0 bottom-0 space-y-1.5 bg-gradient-to-t from-card via-card/90 to-transparent p-4 pt-8">
          <p className="flex items-center gap-1.5 text-sm font-medium">
            <LoaderCircle className="size-3.5 shrink-0 text-primary motion-safe:animate-spin" aria-hidden />
            <span className="line-clamp-1">{headline}</span>
          </p>
          {!queued && <Progress value={pct} aria-label={job.name} />}
          {detail && <p className="line-clamp-2 text-xs text-muted-foreground">{detail}</p>}
        </div>
      </div>
    );
  }
  if (fitted && !broken) {
    return (
      <img
        src={urls.thumbnail(job.id, fitted.key)}
        alt={w.thumbAlt(job.name)}
        loading="lazy"
        onError={() => setBroken(true)}
        className="h-full w-full object-contain"
      />
    );
  }
  const failed = job.status === "failed";
  return (
    <div
      className={cn(
        "flex h-full items-end p-4 text-xs",
        failed ? "text-destructive" : "text-muted-foreground",
        "bg-[repeating-linear-gradient(135deg,transparent_0_8px,var(--color-muted)_8px_9px)]",
      )}
    >
      <p className="line-clamp-3">{failed ? (job.error ?? w.failedCard) : job.status === "succeeded" ? w.noModelCard : null}</p>
    </div>
  );
}

function StudyCard({ job, now, projects, showProject }: { job: Job; now: number; projects: Project[]; showProject: boolean }) {
  const w = useWorkspaceMessages();
  const first = job.spec.species[0];
  const more = job.spec.species.length - 1;
  const project = showProject ? projects.find((p) => p.id === job.project_id)?.name : undefined;
  return (
    <li className="group flex flex-col overflow-hidden rounded-xl border bg-card transition-shadow focus-within:ring-2 focus-within:ring-ring/50 hover:shadow-md">
      <Link to={studyHref(job)} tabIndex={-1} aria-hidden className="relative block aspect-[4/3] overflow-hidden border-b bg-muted/40">
        <CardMedia job={job} now={now} />
        <span className="absolute top-2 left-2 flex flex-wrap gap-1">
          {job.status !== "succeeded" && <JobStatusBadge status={job.status} className="bg-card/90 shadow-sm" />}
        </span>
        {more > 0 && (
          <span className="num absolute right-2 bottom-2 rounded-full bg-card/90 px-2 py-0.5 text-[0.7rem] font-medium shadow-sm">
            {w.moreSpecies(more)}
          </span>
        )}
      </Link>
      <div className="flex flex-1 flex-col gap-2 p-3">
        <div className="flex items-start justify-between gap-1">
          <div className="min-w-0 space-y-0.5">
            <Link to={studyHref(job)} className="line-clamp-2 font-medium leading-snug underline-offset-4 hover:underline">
              {job.name}
            </Link>
            {first && !mentions(job.name, first.scientific_name) && (
              // Not truncated: for the demo the name carries the SYNTHETIC badge, which must stay visible.
              <SciName name={first.scientific_name} className="flex flex-wrap text-sm text-muted-foreground" />
            )}
            {!mentions(job.name, job.spec.area.name) && <p className="truncate text-xs text-muted-foreground">{job.spec.area.name}</p>}
          </div>
          <StudyActions job={job} projects={projects} />
        </div>
        {job.spec.demo && !/SYNTHETIC/.test(first?.scientific_name ?? "") && <SyntheticBadge />}
        {job.status === "succeeded" && <Outcome job={job} />}
        <p className="num mt-auto text-xs text-muted-foreground">
          {formatDateTime(job.created_at)}
          {job.created_by_name && <> · {job.created_by_name}</>}
          {project && <> · {project}</>}
        </p>
      </div>
    </li>
  );
}

const EVIDENCE_BAR: Record<EvidenceLevel, string> = {
  validated: "bg-state-success",
  supported: "bg-state-success",
  indicative: "bg-state-info",
  exploratory: "bg-state-warning",
  insufficient: "bg-muted-foreground/40",
};

/** What the studies on screen produced: evidence level of every modelled species. */
function EvidenceBalance({ jobs }: { jobs: Job[] }) {
  const w = useWorkspaceMessages();
  const species = jobs.flatMap((j) => (j.status === "succeeded" ? j.species : []));
  const fitted = species.filter((s) => s.status === "succeeded" && s.evidence_level);
  const noModel = species.filter((s) => s.status === "skipped").length;
  if (fitted.length === 0) return null;
  const counts = LEVELS.map((l) => [l, fitted.filter((s) => s.evidence_level === l).length] as const).filter(([, n]) => n > 0);
  const total = fitted.length + noModel;
  return (
    <section aria-label={w.balanceLabel(fitted.length)} className="space-y-2">
      <div className="flex items-baseline justify-between gap-2">
        <h2 className="text-sm font-semibold">
          {w.balance} <span className="num font-normal text-muted-foreground">{fitted.length}</span>
        </h2>
      </div>
      <div className="flex h-2.5 overflow-hidden rounded-full bg-muted" aria-hidden>
        {counts.map(([l, n]) => (
          <span key={l} className={EVIDENCE_BAR[l]} style={{ width: `${(n / total) * 100}%` }} />
        ))}
      </div>
      <ul className="flex flex-wrap gap-x-4 gap-y-1 text-xs">
        {counts.map(([l, n]) => (
          <li key={l} className="inline-flex items-center gap-1.5">
            <EvidenceBadge level={l} /> <span className="num text-muted-foreground">{n}</span>
          </li>
        ))}
        {noModel > 0 && (
          <li className="num inline-flex items-center gap-1.5 text-muted-foreground">
            <span className="size-2 rounded-full bg-muted" aria-hidden /> {noModel} {w.noModel}
          </li>
        )}
      </ul>
    </section>
  );
}

type View = "gallery" | "list";
const VIEW_KEY = "nichers.workspace.view";

function useView(): [View, (v: View) => void] {
  const [view, setView] = useState<View>(() => {
    try {
      return window.localStorage.getItem(VIEW_KEY) === "list" ? "list" : "gallery";
    } catch {
      return "gallery";
    }
  });
  const set = (v: View) => {
    setView(v);
    try {
      window.localStorage.setItem(VIEW_KEY, v);
    } catch {
      // storage unavailable: the choice lasts for this page only
    }
  };
  return [view, set];
}

// ---- projects --------------------------------------------------------------------------------

function NavItem({ selected, onClick, icon, label, count }: { selected: boolean; onClick: () => void; icon: ReactNode; label: string; count: number }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={selected ? "page" : undefined}
      className={cn(
        "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm",
        selected ? "bg-primary/10 font-medium text-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground",
      )}
    >
      {icon}
      <span className="min-w-0 flex-1 truncate">{label}</span>
      <span className="num text-xs">{count}</span>
    </button>
  );
}

function ProjectNav({
  projects,
  jobs,
  current,
  select,
}: {
  projects: Project[];
  jobs: Job[];
  current: string | null;
  select: (p: string | null) => void;
}) {
  const w = useWorkspaceMessages();
  const [localCreating, setLocalCreating] = useState(false);
  const create = useCreateProject();
  const [params, setParams] = useSearchParams();
  // "New project" from the global search arrives as ?newProject=1.
  const creating = params.get("newProject") === "1" || localCreating;
  const setCreating = (o: boolean) => {
    setLocalCreating(o);
    if (!o && params.has("newProject")) {
      params.delete("newProject");
      setParams(params, { replace: true });
    }
  };
  const without = jobs.filter((j) => !j.project_id).length;
  return (
    <nav aria-label={w.projects} className="space-y-3">
      <h2 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">{w.projects}</h2>
      {projects.length > 0 ? (
        <div className="space-y-0.5">
          <NavItem selected={current === null} onClick={() => select(null)} icon={<Folders className="size-4" aria-hidden />} label={w.allStudies} count={jobs.length} />
          {projects.map((p) => (
            <NavItem key={p.id} selected={current === p.id} onClick={() => select(p.id)} icon={<FolderOpen className="size-4" aria-hidden />} label={p.name} count={p.studies} />
          ))}
          {without > 0 && (
            <NavItem selected={current === NONE} onClick={() => select(NONE)} icon={<FolderOpen className="size-4 opacity-50" aria-hidden />} label={w.noProject} count={without} />
          )}
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">{w.projectsHint}</p>
      )}
      <Button variant="outline" size="sm" className="w-full" onClick={() => setCreating(true)}>
        <Plus aria-hidden /> {w.newProject}
      </Button>
      <NameDialog
        open={creating}
        onOpenChange={setCreating}
        title={w.newProject}
        label={w.projectName}
        hint={w.projectNameHint}
        initial=""
        action={w.create}
        pending={create.isPending}
        error={create.error}
        onSubmit={(name) =>
          create.mutate(name, {
            onSuccess: (p) => {
              setCreating(false);
              select(p.id);
            },
          })
        }
      />
    </nav>
  );
}

function ProjectMenu({ project, onDeleted }: { project: Project; onDeleted: () => void }) {
  const w = useWorkspaceMessages();
  const rename = useRenameProject();
  const del = useDeleteProject();
  const [dialog, setDialog] = useState<"rename" | "delete" | null>(null);
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger render={<Button variant="ghost" size="icon-sm" aria-label={w.projectActions(project.name)} />}>
          <EllipsisVertical aria-hidden />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-56">
          <DropdownMenuItem onClick={() => setDialog("rename")}>
            <Pencil aria-hidden /> {w.renameProject}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => setDialog("delete")} className="text-destructive">
            <Trash2 aria-hidden /> {w.deleteProject}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <NameDialog
        open={dialog === "rename"}
        onOpenChange={(o) => !o && setDialog(null)}
        title={w.renameProject}
        label={w.projectName}
        initial={project.name}
        action={w.save}
        pending={rename.isPending}
        error={rename.error}
        onSubmit={(name) => rename.mutate({ id: project.id, name }, { onSuccess: () => setDialog(null) })}
      />
      <AlertDialog open={dialog === "delete"} onOpenChange={(o) => !o && setDialog(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{w.deleteProjectTitle(project.name)}</AlertDialogTitle>
            <AlertDialogDescription>{w.deleteProjectText(project.studies)}</AlertDialogDescription>
          </AlertDialogHeader>
          {del.isError && <p className="text-sm text-destructive">{errorMessage(del.error)}</p>}
          <AlertDialogFooter>
            <AlertDialogCancel>{w.cancel}</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={() => del.mutate(project.id, { onSuccess: onDeleted })}>
              {w.deleteProject}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

// ---- draft and welcome -----------------------------------------------------------------------

function DraftCard() {
  const h = useHomeMessages();
  const navigate = useNavigate();
  const species = useDraft((s) => s.species);
  const area = useDraft((s) => s.area);
  const step = useDraft((s) => s.step);
  const path = useDraft((s) => s.path);
  const demo = useDraft((s) => s.demo);
  const reset = useDraft((s) => s.reset);
  if (species.length === 0 && area === null) return null;
  const steps = pathSteps(path);
  const first = species[0];
  const label = useStepLabels.get()[STEPS[step]?.id ?? "species"];
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-primary/40 bg-primary/5 p-3">
      <div className="min-w-0 space-y-0.5">
        <p className="text-xs font-medium text-muted-foreground uppercase">{h.inProgress}</p>
        <p className="flex flex-wrap items-center gap-1.5 text-sm">
          {first ? <SciName name={first.scientific_name} /> : <span className="text-muted-foreground">{h.notChosen}</span>}
          {species.length > 1 && <span className="num text-muted-foreground">+{species.length - 1}</span>}
          {area && <span className="text-muted-foreground">· {area.name}</span>}
          {demo && !/SYNTHETIC/.test(first?.scientific_name ?? "") && <SyntheticBadge />}
        </p>
        <p className="text-xs text-muted-foreground">{h.saved(Math.max(steps.indexOf(step), 0) + 1, steps.length, label)}</p>
      </div>
      <div className="flex gap-2">
        <Button size="sm" onClick={() => void navigate("/new")}>
          {h.continue} <ArrowRight aria-hidden />
        </Button>
        <Button size="sm" variant="ghost" onClick={() => reset()}>
          <Trash2 aria-hidden /> {h.discard}
        </Button>
      </div>
    </div>
  );
}

function Welcome() {
  const h = useHomeMessages();
  const w = useWorkspaceMessages();
  const m = useJobsMessages();
  const navigate = useNavigate();
  const startDemo = useDraft((s) => s.startDemo);
  const showHelp = useHelp((s) => s.show);
  return (
    <section className="space-y-5 rounded-lg border p-5" data-welcome aria-labelledby="ws-welcome">
      <div className="space-y-2">
        <p className="text-xs font-medium text-muted-foreground uppercase">{m.empty}</p>
        <h2 id="ws-welcome" className="text-lg font-semibold">
          {w.welcomeTitle}
        </h2>
        <p className="text-sm text-muted-foreground">
          {h.leadA}
          <strong className="text-foreground">{h.leadStrong}</strong>
          {h.leadB}
        </p>
        <p className="text-sm text-muted-foreground">{w.welcomeText}</p>
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-3">
          <h3 className="text-sm font-semibold">{h.howTitle}</h3>
          <ol className="space-y-3">
            {(
              [
                [h.how1t, h.how1],
                [h.how2t, h.how2],
                [h.how3t, h.how3],
              ] as const
            ).map(([title, text], i) => (
              <li key={title} className="flex gap-3">
                <span className="num flex size-6 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-medium">{i + 1}</span>
                <span className="space-y-0.5">
                  <span className="block text-sm font-medium">{title}</span>
                  <span className="block text-sm text-muted-foreground">{text}</span>
                </span>
              </li>
            ))}
          </ol>
        </div>
        <div className="space-y-3">
          <div className="rounded-lg border border-dashed p-3">
            <div className="flex items-start gap-3">
              <FlaskConical className="mt-0.5 size-4 shrink-0 text-state-warning" aria-hidden />
              <div className="space-y-2">
                <p className="text-sm font-medium">{h.demoTitle}</p>
                <p className="text-sm text-muted-foreground">
                  {h.demoA}
                  <strong>{h.demoStrong}</strong>
                  {h.demoB}
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    startDemo();
                    void navigate("/new");
                  }}
                >
                  {h.demoButton}
                </Button>
              </div>
            </div>
          </div>
          <p className="flex gap-2 rounded-md bg-muted/60 p-2 text-xs text-muted-foreground">
            <MapIcon className="mt-0.5 size-3.5 shrink-0" aria-hidden />
            {h.notProbability}
          </p>
          <Button variant="link" size="sm" className="h-auto px-0" onClick={() => showHelp({ article: "getting-started" })} aria-haspopup="dialog">
            <BookOpen aria-hidden /> {h.guide}
          </Button>
        </div>
      </div>
    </section>
  );
}

// ---- page ------------------------------------------------------------------------------------

function useNow(enabled: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!enabled) return;
    const t = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(t);
  }, [enabled]);
  return now;
}

export function WorkspacePage() {
  const w = useWorkspaceMessages();
  const m = useJobsMessages();
  const me = useMe();
  const jobs = useJobs();
  const projects = useProjects();
  const navigate = useNavigate();
  const reset = useDraft((s) => s.reset);
  const [params, setParams] = useSearchParams();
  const current = params.get("project");
  const [view, setView] = useView();

  const all = useMemo(() => [...(jobs.data ?? [])].sort((a, b) => b.created_at.localeCompare(a.created_at)), [jobs.data]);
  const plist = projects.data ?? [];
  const project = current && current !== NONE ? plist.find((p) => p.id === current) : undefined;
  const inScope = all.filter((j) => (current === null ? true : current === NONE ? !j.project_id : j.project_id === current));
  const shown = inScope;
  const running = shown.filter(active);
  const finished = shown.filter((j) => !active(j));
  const now = useNow(running.length > 0);
  const select = (p: string | null) => setParams(p ? { project: p } : {});

  // A project that no longer exists (deleted elsewhere) falls back to all studies.
  useEffect(() => {
    if (current && current !== NONE && projects.data && !project) setParams({});
  }, [current, projects.data, project, setParams]);

  const title = project ? project.name : current === NONE ? w.noProject : (me.data?.organisation ?? w.myStudies);
  const firstTime = jobs.data !== undefined && all.length === 0 && plist.length === 0;

  return (
    <div className="min-h-0 flex-1 overflow-y-auto bg-background" data-workspace>
      <div className="mx-auto grid max-w-[110rem] gap-8 px-4 py-6 lg:grid-cols-[15rem_minmax(0,1fr)] lg:px-8">
        <aside className="lg:sticky lg:top-0 lg:self-start">
          {projects.isError ? (
            <ErrorState error={projects.error} onRetry={() => void projects.refetch()} title={w.projects} />
          ) : (
            <ProjectNav projects={plist} jobs={all} current={current} select={select} />
          )}
        </aside>

        <main className="min-w-0 space-y-6" aria-labelledby="ws-title">
          <header className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex min-w-0 items-center gap-1">
              <h1 id="ws-title" className="truncate text-2xl font-semibold tracking-tight">
                {title}
              </h1>
              {project && <ProjectMenu project={project} onDeleted={() => select(null)} />}
            </div>
            <div className="flex items-center gap-2">
              {inScope.length > 0 && (
                <ToggleGroup
                  aria-label={w.view}
                  value={[view]}
                  onValueChange={(v) => v[0] && setView(v[0] as View)}
                  variant="outline"
                  size="sm"
                >
                  <ToggleGroupItem value="gallery" aria-label={w.viewGallery}>
                    <LayoutGrid aria-hidden /> <span className="hidden sm:inline">{w.viewGallery}</span>
                  </ToggleGroupItem>
                  <ToggleGroupItem value="list" aria-label={w.viewList}>
                    <List aria-hidden /> <span className="hidden sm:inline">{w.viewList}</span>
                  </ToggleGroupItem>
                </ToggleGroup>
              )}
              <Button
                onClick={() => {
                  reset(project ? project.id : null);
                  void navigate("/new");
                }}
              >
                <Plus aria-hidden /> {project ? w.newStudyIn(project.name) : w.newStudy}
              </Button>
            </div>
          </header>

          <DraftCard />

          {jobs.isPending && (
            <div className="grid grid-cols-[repeat(auto-fill,minmax(17rem,1fr))] gap-4" aria-busy="true">
              {[0, 1, 2, 3].map((i) => (
                <Skeleton key={i} className="aspect-[4/5] w-full rounded-xl" />
              ))}
            </div>
          )}
          {jobs.isError && <ErrorState error={jobs.error} onRetry={() => void jobs.refetch()} title={m.cannotList} />}

          {firstTime && <Welcome />}

          {inScope.some((j) => j.status === "succeeded") && (
            <div className="rounded-xl border bg-card p-4">
              <EvidenceBalance jobs={inScope} />
            </div>
          )}

          {view === "gallery" && shown.length > 0 && (
            <ul className="grid grid-cols-[repeat(auto-fill,minmax(17rem,1fr))] gap-4" aria-label={m.title}>
              {[...running, ...finished].map((j) => (
                <StudyCard key={j.id} job={j} now={now} projects={plist} showProject={current === null} />
              ))}
            </ul>
          )}

          {view === "list" && running.length > 0 && (
            <section aria-labelledby="ws-active" className="space-y-2">
              <h2 id="ws-active" className="text-sm font-semibold">
                {m.inProgress(running.length)}
              </h2>
              <ul className="grid gap-2 md:grid-cols-2">
                {running.map((j) => (
                  <ActiveCard key={j.id} job={j} now={now} projects={plist} />
                ))}
              </ul>
            </section>
          )}

          {view === "list" && finished.length > 0 && (
            <section aria-labelledby="ws-finished" className="space-y-2">
              <h2 id="ws-finished" className="text-sm font-semibold">
                {m.finished(finished.length)}
              </h2>
              <StudiesTable jobs={finished} projects={plist} showProject={current === null} />
            </section>
          )}

          {jobs.data && !firstTime && shown.length === 0 && (
            <p className="rounded-xl border border-dashed p-6 text-center text-sm text-muted-foreground">
              {project ? w.emptyProject : current === NONE ? w.emptyNoProject : m.empty}
            </p>
          )}
        </main>
      </div>
    </div>
  );
}
