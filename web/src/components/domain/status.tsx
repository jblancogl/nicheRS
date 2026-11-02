// Status vocabulary: colour is never the only cue — every state has an icon and a text label.

import {
  BadgeCheck,
  CircleCheck,
  CircleDashed,
  CircleX,
  Info,
  LoaderCircle,
  OctagonAlert,
  TriangleAlert,
  type LucideIcon,
} from "lucide-react";

import type { EvidenceLevel, JobStatus, Severity, SpeciesRunStatus } from "@/api/types";
import { cn } from "@/lib/utils";

import { useDomainMessages } from "./messages";
import { toneBg, toneClass, type Tone } from "./status-meta";

const JOB: Record<JobStatus, { tone: Tone; icon: LucideIcon }> = {
  queued: { tone: "neutral", icon: CircleDashed },
  running: { tone: "running", icon: LoaderCircle },
  succeeded: { tone: "success", icon: CircleCheck },
  failed: { tone: "danger", icon: CircleX },
  cancelled: { tone: "neutral", icon: CircleDashed },
  interrupted: { tone: "warning", icon: TriangleAlert },
};

const SPECIES: Record<SpeciesRunStatus, { tone: Tone; icon: LucideIcon }> = {
  pending: { tone: "neutral", icon: CircleDashed },
  running: { tone: "running", icon: LoaderCircle },
  succeeded: { tone: "success", icon: CircleCheck },
  failed: { tone: "danger", icon: CircleX },
  skipped: { tone: "warning", icon: TriangleAlert },
  cancelled: { tone: "neutral", icon: CircleDashed },
};

function Pill({ tone, icon: Icon, label, spin, className }: { tone: Tone; icon: LucideIcon; label: string; spin?: boolean; className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex h-5 shrink-0 items-center gap-1 rounded-full px-2 text-xs font-medium whitespace-nowrap",
        toneBg[tone],
        toneClass[tone],
        className,
      )}
    >
      <Icon className={cn("size-3", spin && "motion-safe:animate-spin")} aria-hidden />
      <span className="text-foreground">{label}</span>
    </span>
  );
}

export function JobStatusBadge({ status, withWarnings, className }: { status: JobStatus; withWarnings?: boolean; className?: string }) {
  const m = useDomainMessages();
  const s = JOB[status];
  if (status === "succeeded" && withWarnings)
    return <Pill tone="warning" icon={TriangleAlert} label={m.completedWithWarnings} className={className} />;
  return <Pill tone={s.tone} icon={s.icon} label={m.job(status)} spin={status === "running"} className={className} />;
}

/** `jobFinished`: a species still pending in a finished job was never run (the job stopped first). */
export function SpeciesStatusBadge({ status, jobFinished = false }: { status: SpeciesRunStatus; jobFinished?: boolean }) {
  const m = useDomainMessages();
  if (jobFinished && (status === "pending" || status === "running")) {
    return <Pill tone="neutral" icon={CircleDashed} label={m.notRun} />;
  }
  const s = SPECIES[status];
  return <Pill tone={s.tone} icon={s.icon} label={m.species(status)} spin={status === "running"} />;
}

const SEVERITY: Record<Severity, { tone: Tone; icon: LucideIcon }> = {
  info: { tone: "info", icon: Info },
  warning: { tone: "warning", icon: TriangleAlert },
  blocking: { tone: "danger", icon: OctagonAlert },
};

export function SeverityIcon({ severity, className }: { severity: Severity; className?: string }) {
  const m = useDomainMessages();
  const s = SEVERITY[severity];
  const Icon = s.icon;
  return <Icon className={cn("size-4 shrink-0", toneClass[s.tone], className)} aria-label={m.severity(severity)} role="img" />;
}


const EVIDENCE: Record<EvidenceLevel, { label: string; tone: Tone; icon: LucideIcon }> = {
  insufficient: {
    label: "Insufficient",
    tone: "danger",
    icon: CircleX,
  },
  exploratory: {
    label: "Exploratory",
    tone: "warning",
    icon: TriangleAlert,
  },
  indicative: {
    label: "Indicative",
    tone: "info",
    icon: Info,
  },
  validated: {
    label: "Validated",
    tone: "success",
    icon: BadgeCheck,
  },
  supported: {
    label: "Supported",
    tone: "success",
    icon: CircleCheck,
  },
};

export function EvidenceBadge({ level, className }: { level: EvidenceLevel; className?: string }) {
  const e = EVIDENCE[level];
  return <Pill tone={e.tone} icon={e.icon} label={e.label} className={className} />;
}

