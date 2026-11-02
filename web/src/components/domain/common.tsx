import { CloudOff, FlaskConical, Info, RotateCw } from "lucide-react";
import type { ReactNode } from "react";

import { ApiError, errorMessage } from "@/api/client";
import type { Warning } from "@/api/types";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

import { useDomainMessages } from "./messages";
import { SeverityIcon } from "./status";
import { toneBg } from "./status-meta";
import { useWarningText } from "./warningMessages";

/** Scientific name: italic, Latin language tag. Synthetic demo species are flagged. */
export function SciName({ name, className }: { name: string; className?: string }) {
  const synthetic = /\(SYNTHETIC\)/.test(name);
  const clean = name.replace(/\s*\(SYNTHETIC\)\s*/, "");
  return (
    <span className={cn("inline-flex items-center gap-1.5", className)}>
      <i lang="la">{clean}</i>
      {synthetic && <SyntheticBadge />}
    </span>
  );
}

export function SyntheticBadge({ className }: { className?: string }) {
  const m = useDomainMessages();
  return (
    <span
      className={cn(
        "inline-flex h-5 items-center gap-1 rounded-full border border-state-warning px-1.5 text-[0.7rem] font-semibold tracking-wide text-foreground uppercase not-italic",
        className,
      )}
      title={m.syntheticTitle}
    >
      <FlaskConical className="size-3 text-state-warning" aria-hidden />
      {m.synthetic}
    </span>
  );
}

/** API error state with a domain-language title and a retry action (P12). */
export function ErrorState({ error, onRetry, title, className }: { error: unknown; onRetry?: () => void; title?: string; className?: string }) {
  const m = useDomainMessages();
  const unreachable = error instanceof ApiError && error.unreachable;
  return (
    <Alert variant="destructive" className={className}>
      {unreachable ? <CloudOff aria-hidden /> : <SeverityIcon severity="blocking" />}
      <AlertTitle>{title ?? (unreachable ? m.serverDown : m.somethingWrong)}</AlertTitle>
      <AlertDescription>
        <p>{errorMessage(error)}</p>
        {error instanceof ApiError && error.problem?.type && error.problem.type !== "about:blank" && (
          <details className="mt-1 text-xs">
            <summary className="cursor-pointer">{m.technicalDetails}</summary>
            <code className="font-mono break-all">
              {error.problem.type} · HTTP {error.status}
            </code>
          </details>
        )}
        {onRetry && (
          <Button size="sm" variant="outline" className="mt-2" onClick={onRetry}>
            <RotateCw aria-hidden /> {m.retry}
          </Button>
        )}
      </AlertDescription>
    </Alert>
  );
}

export function EmptyState({ icon, title, children, action }: { icon?: ReactNode; title: string; children?: ReactNode; action?: ReactNode }) {
  return (
    <div className="flex flex-col items-center gap-2 rounded-lg border border-dashed px-4 py-8 text-center">
      {icon && <div className="text-muted-foreground">{icon}</div>}
      <p className="font-medium">{title}</p>
      {children && <div className="max-w-80 text-sm text-muted-foreground">{children}</div>}
      {action}
    </div>
  );
}

export function WarningsList({ warnings, className }: { warnings: Warning[]; className?: string }) {
  const m = useDomainMessages();
  const text = useWarningText();
  if (warnings.length === 0) return null;
  const order = { blocking: 0, warning: 1, info: 2 } as const;
  const sorted = [...warnings].sort((a, b) => order[a.severity] - order[b.severity]);
  return (
    <ul className={cn("space-y-2", className)} aria-label={m.warnings}>
      {sorted.map((w, i) => (
        <li key={`${w.code}-${i}`} className={cn("flex gap-2 rounded-md px-2.5 py-2 text-sm", toneBg[w.severity === "blocking" ? "danger" : w.severity === "warning" ? "warning" : "info"])}>
          <SeverityIcon severity={w.severity} className="mt-0.5" />
          <div>
            <span className="sr-only">{m.severity(w.severity)}: </span>
            {text(w)}
            <span className="ml-1 font-mono text-[0.7rem] text-muted-foreground">{w.code}</span>
          </div>
        </li>
      ))}
    </ul>
  );
}

/** Small info affordance. Caveats must also appear in text somewhere; this only adds detail. */
export function InfoTip({ children, label }: { children: ReactNode; label?: string }) {
  const m = useDomainMessages();
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <button type="button" className="inline-flex rounded-sm align-middle text-muted-foreground hover:text-foreground" aria-label={label ?? m.moreInfo} />
        }
      >
        <Info className="size-3.5" aria-hidden />
      </TooltipTrigger>
      <TooltipContent className="max-w-72">{children}</TooltipContent>
    </Tooltip>
  );
}

export function Section({ title, children, action, className, id }: { title: ReactNode; children: ReactNode; action?: ReactNode; className?: string; id?: string }) {
  return (
    <section className={cn("space-y-2", className)} aria-labelledby={id}>
      <div className="flex items-center justify-between gap-2">
        <h3 id={id} className="text-sm font-semibold">
          {title}
        </h3>
        {action}
      </div>
      {children}
    </section>
  );
}

export function KeyValue({ items, className }: { items: [ReactNode, ReactNode][]; className?: string }) {
  return (
    <dl className={cn("grid grid-cols-[minmax(0,auto)_1fr] gap-x-4 gap-y-1 text-sm", className)}>
      {items.map(([k, v], i) => (
        <div key={i} className="contents">
          <dt className="text-muted-foreground">{k}</dt>
          <dd className="num min-w-0 text-right break-words">{v}</dd>
        </div>
      ))}
    </dl>
  );
}
