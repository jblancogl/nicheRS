// Landing page of the administration: the instance in five numbers, what needs a look (quotas
// running out, failed studies, open feedback, system checks) and the latest studies. Every item
// links to the section where it is handled; nothing is decided here.

import { CircleAlert, CircleCheck, OctagonAlert } from "lucide-react";
import type { ReactNode } from "react";
import { Link } from "react-router";

import { useAdminActivity, useAdminFeedback, useAdminOrgs, useAdminSystem, useAdminUsers } from "@/api/hooks";
import { JobStatusBadge } from "@/components/domain/status";
import { Skeleton } from "@/components/ui/skeleton";
import { formatCount, formatDateTime } from "@/lib/format";
import { cn } from "@/lib/utils";

import { backupHealth, gbifHealth, mirrorHealth, storageHealth } from "./health";
import { jobUrl } from "./links";
import { useAdminMessages } from "./messages";
import { InlineError, QUOTA_WARN } from "./shared";
import { adminUrl, type AdminSectionId } from "./sections";

interface Attention {
  tone: "bad" | "warn";
  text: string;
  to: string;
}

function Tile({ to, label, value, sub, danger }: { to: string; label: string; value: ReactNode; sub?: ReactNode; danger?: boolean }) {
  return (
    <Link to={to} className="space-y-0.5 rounded-lg border bg-card px-3 py-2.5 hover:bg-muted/40">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className={cn("num text-2xl font-semibold", danger && "text-destructive")}>{value}</p>
      {sub && <p className="num text-xs text-muted-foreground">{sub}</p>}
    </Link>
  );
}

function Panel({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return (
    <section aria-label={title} className="space-y-3 rounded-lg border bg-card p-4">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold">{title}</h2>
        {action}
      </div>
      {children}
    </section>
  );
}

export function OverviewSection() {
  const m = useAdminMessages();
  const orgs = useAdminOrgs();
  const sys = useAdminSystem();
  const feedback = useAdminFeedback();
  const act = useAdminActivity();
  const users = useAdminUsers({ limit: 1 });
  const go = (id: AdminSectionId) => adminUrl(id);

  const active = (orgs.data ?? []).filter((o) => !o.disabled_at);
  const disabledOrgs = (orgs.data?.length ?? 0) - active.length;
  const used = active.reduce((n, o) => n + o.quota.used, 0);
  const quota = active.reduce((n, o) => n + o.quota.total, 0);
  const s = sys.data;
  const inProgress = s ? s.jobs.running + s.jobs.queued + s.jobs.preparing : null;
  const openFeedback = (feedback.data ?? []).filter((f) => f.status === "open").length;

  const attention: Attention[] = [];
  for (const o of active) {
    if (o.quota.total === 0) continue;
    const share = (o.quota.used + o.quota.reserved) / o.quota.total;
    if (share >= 1) attention.push({ tone: "bad", text: m.attQuotaFull(o.name), to: adminUrl("orgs", o.id) });
    else if (share >= QUOTA_WARN) attention.push({ tone: "warn", text: m.attQuotaNear(o.name, Math.round(share * 100)), to: adminUrl("orgs", o.id) });
  }
  if (s && s.jobs.failed_7d > 0) attention.push({ tone: "warn", text: m.attFailed(s.jobs.failed_7d), to: go("activity") });
  if (openFeedback > 0) attention.push({ tone: "warn", text: m.attFeedback(openFeedback), to: go("feedback") });
  if (s) {
    const checks = [
      [m.sysStorage, storageHealth(s)],
      [m.sysBackups, backupHealth(s, sys.dataUpdatedAt)],
      [m.sysGbif, gbifHealth(s)],
      ...(s.gbif_mirror ? [[m.sysMirror, mirrorHealth(s.gbif_mirror)] as const] : []),
    ] as const;
    for (const [what, health] of checks) if (health !== "ok") attention.push({ tone: health, text: m.attSystem(what), to: go("system") });
  }
  attention.sort((a, b) => Number(a.tone === "warn") - Number(b.tone === "warn"));
  const checked = orgs.data && s && feedback.data;

  const latest = [...(act.data ?? [])].sort((a, b) => b.created_at.localeCompare(a.created_at)).slice(0, 6);

  return (
    <div className="space-y-5">
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-5">
        <Tile to={go("orgs")} label={m.kpiOrgs} value={orgs.data ? active.length : "…"} sub={disabledOrgs > 0 ? m.kpiOrgsDisabled(disabledOrgs) : undefined} />
        <Tile to={go("users")} label={m.kpiAccounts} value={users.data ? formatCount(users.data.pages[0]?.total ?? 0) : "…"} />
        <Tile to={go("usage")} label={m.kpiSpecies} value={orgs.data ? used : "…"} sub={orgs.data ? m.kpiOfQuota(quota) : undefined} />
        <Tile to={go("activity")} label={m.kpiInProgress} value={inProgress ?? "…"} />
        <Tile to={go("activity")} label={m.kpiFailed7d} value={s ? s.jobs.failed_7d : "…"} danger={(s?.jobs.failed_7d ?? 0) > 0} />
      </div>
      <InlineError error={orgs.error ?? sys.error ?? feedback.error ?? act.error ?? users.error} />
      <div className="grid gap-4 lg:grid-cols-2">
        <Panel title={m.attention}>
          {!checked ? (
            <Skeleton className="h-16 w-full" />
          ) : attention.length === 0 ? (
            <p className="flex items-start gap-2 text-sm">
              <CircleCheck className="mt-0.5 size-4 shrink-0 text-state-success" aria-hidden />
              <span>
                <span className="font-medium">{m.allClear}</span> <span className="text-muted-foreground">{m.allClearText}</span>
              </span>
            </p>
          ) : (
            <ul className="-mx-2 space-y-0.5">
              {attention.map((a) => {
                const Icon = a.tone === "bad" ? OctagonAlert : CircleAlert;
                return (
                  <li key={a.text}>
                    <Link to={a.to} className="flex items-start gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-muted">
                      <Icon className={cn("mt-0.5 size-4 shrink-0", a.tone === "bad" ? "text-destructive" : "text-state-warning")} aria-hidden />
                      <span>{a.text}</span>
                    </Link>
                  </li>
                );
              })}
            </ul>
          )}
        </Panel>
        <Panel
          title={m.latestStudies}
          action={
            <Link to={go("activity")} className="text-xs text-primary hover:underline">
              {m.seeAll}
            </Link>
          }
        >
          {act.isPending ? (
            <Skeleton className="h-16 w-full" />
          ) : latest.length === 0 ? (
            <p className="text-sm text-muted-foreground">{m.noStudy}</p>
          ) : (
            <ul className="-mx-2 space-y-0.5">
              {latest.map((r) => (
                <li key={r.id}>
                  <Link to={jobUrl(r)} className="flex items-center gap-3 rounded-md px-2 py-1.5 text-sm hover:bg-muted">
                    <span className="min-w-0 flex-1">
                      <span className="block truncate font-medium">{r.name}</span>
                      <span className="num block truncate text-xs text-muted-foreground">
                        {r.org_name ?? m.beforeAccounts} · {formatDateTime(r.created_at)}
                      </span>
                    </span>
                    <JobStatusBadge status={r.status} />
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>
    </div>
  );
}
