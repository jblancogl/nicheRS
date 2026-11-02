// What the server is doing: unfinished studies of every organisation and the most recent finished
// ones, with GBIF waits and failure causes (ADR-0106). Opening a client's study is support access,
// recorded in the audit trail.

import { ExternalLink } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";

import { isTerminal, useAdminActivity } from "@/api/hooks";
import type { ActivityRow, FailureCause } from "@/api/types";
import { ErrorState } from "@/components/domain/common";
import { JobStatusBadge } from "@/components/domain/status";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useJobText } from "@/features/execution/jobText";
import { formatDateTime } from "@/lib/format";

import { CAUSES, useAdminMessages } from "./messages";
import { jobUrl } from "./links";
import { Loading, MobileItem, MobileList } from "./shared";

function Count({ label, value, tone }: { label: string; value: number; tone?: "danger" }) {
  return (
    <div className="rounded-lg border bg-card px-3 py-2">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className={`num text-xl font-semibold ${tone === "danger" && value > 0 ? "text-destructive" : ""}`}>{value}</p>
    </div>
  );
}

/** Every organisation's studies, or one organisation's on its page (`org`: its id; filtered on
 * the server, so its own 100 most recent finished studies). */
export function ActivityTable({ org: only }: { org?: string }) {
  const m = useAdminMessages();
  const jt = useJobText();
  const act = useAdminActivity(only);
  const [org, setOrg] = useState("");
  const [cause, setCause] = useState<FailureCause | null>(null);
  const now = act.dataUpdatedAt;
  const inOrg = (act.data ?? []).filter((r) => !org || (r.org_name ?? "—") === org);
  const rows = inOrg.filter((r) => !cause || r.cause === cause);
  const orgs = [...new Set((act.data ?? []).map((r) => r.org_name ?? "—"))].sort();
  const waiting = inOrg.filter((r) => r.status === "queued" && (r.data_state === "pending" || r.data_state === "preparing")).length;
  const queued = inOrg.filter((r) => r.status === "queued" && (r.data_state === "ready" || r.data_state === "nobatch")).length;
  const running = inOrg.filter((r) => r.status === "running").length;
  const failed = inOrg.filter((r) => r.status === "failed");
  const byCause = CAUSES.map((c) => [c, failed.filter((r) => r.cause === c).length] as const).filter(([, n]) => n > 0);
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end gap-3">
        <div className="grid w-full grid-cols-2 gap-2 sm:flex sm:w-auto sm:flex-wrap sm:gap-3">
          <Count label={m.preparing} value={waiting} />
          <Count label={m.queuedForComputing} value={queued} />
          <Count label={m.running} value={running} />
          <Count label={m.failedRecent} value={failed.length} tone="danger" />
        </div>
        {only === undefined && (
        <div className="ml-auto space-y-1.5">
          <Label htmlFor="activity-org">{m.colOrg}</Label>
          <select id="activity-org" value={org} onChange={(e) => setOrg(e.target.value)} className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm">
            <option value="">{m.all}</option>
            {orgs.map((o) => (
              <option key={o} value={o}>
                {o}
              </option>
            ))}
          </select>
        </div>
        )}
      </div>
      {byCause.length > 0 && (
        <div role="group" aria-label={m.failuresBy} className="flex flex-wrap items-center gap-1.5 text-sm">
          <span className="mr-1 text-muted-foreground">{m.failuresBy}</span>
          <Button size="xs" variant={cause === null ? "secondary" : "ghost"} aria-pressed={cause === null} onClick={() => setCause(null)}>
            {m.showAllCauses}
          </Button>
          {byCause.map(([c, n]) => (
            <Button key={c} size="xs" variant={cause === c ? "secondary" : "outline"} aria-pressed={cause === c} onClick={() => setCause(cause === c ? null : c)}>
              {m.cause(c)} · {n}
            </Button>
          ))}
        </div>
      )}
      {only === undefined && <p className="text-xs text-muted-foreground">{m.activityText}</p>}
      {act.isPending && <Loading />}
      {act.isError && <ErrorState error={act.error} onRetry={() => void act.refetch()} title={m.loadActivityFailed} />}
      {act.data && (
        <MobileList empty={rows.length === 0 ? m.noStudy : undefined}>
          {rows.map((r) => (
            <MobileItem
              key={r.id}
              to={jobUrl(r)}
              title={r.name}
              aside={<JobStatusBadge status={r.status} />}
              meta={
                <>
                  <span className="block">
                    {[only === undefined ? (r.org_name ?? m.beforeAccounts) : null, r.created_by].filter(Boolean).join(" · ")}
                  </span>
                  <span className="num block">
                    {formatDateTime(r.created_at)}
                    {!isTerminal(r.status) && ` · ${Math.round(r.progress * 100)} %`}
                  </span>
                  {r.error && (
                    <span className="block text-destructive">
                      {r.cause && `${m.cause(r.cause)} · `}
                      {r.error}
                    </span>
                  )}
                </>
              }
            />
          ))}
        </MobileList>
      )}
      {act.data && (
        <div className="hidden md:block">
        <Table>
          <TableHeader>
            <TableRow>
              {only === undefined && <TableHead>{m.colOrg}</TableHead>}
              <TableHead>{m.colStudy}</TableHead>
              <TableHead>{m.colStatus}</TableHead>
              <TableHead>{m.colStage}</TableHead>
              <TableHead className="text-right">{m.colSpecies}</TableHead>
              <TableHead>{m.colCreated}</TableHead>
              <TableHead>{m.colFinished}</TableHead>
              <TableHead />
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.length === 0 && (
              <TableRow>
                <TableCell colSpan={only === undefined ? 8 : 7} className="text-muted-foreground">
                  {m.noStudy}
                </TableCell>
              </TableRow>
            )}
            {rows.map((r: ActivityRow) => (
              <TableRow key={r.id}>
                {only === undefined && <TableCell>{r.org_name ?? <span className="text-muted-foreground">{m.beforeAccounts}</span>}</TableCell>}
                <TableCell>
                  <span className="font-medium">{r.name}</span>
                  {r.created_by && <span className="block text-xs text-muted-foreground">{r.created_by}</span>}
                </TableCell>
                <TableCell>
                  <JobStatusBadge status={r.status} />
                </TableCell>
                <TableCell className="max-w-96 text-xs">
                  {isTerminal(r.status) ? (
                    r.error ? (
                      <span className="space-y-0.5">
                        {r.cause && (
                          <Badge variant="outline" className="mr-1 font-normal">
                            {m.cause(r.cause)}
                          </Badge>
                        )}
                        <span className="text-destructive">{r.error}</span>
                      </span>
                    ) : (
                      <span className="text-muted-foreground">{jt.stage(r.stage)}</span>
                    )
                  ) : r.gbif_wait ? (
                    <>
                      <span className="block">
                        {m.gbifWaiting(
                          Math.floor((now - Date.parse(r.gbif_wait.since)) / 60_000),
                          Math.max(0, Math.round((now - Date.parse(r.gbif_wait.checked_at)) / 1000)),
                        )}
                      </span>
                      {r.gbif_wait.url && (
                        <a href={r.gbif_wait.url} target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-1 text-primary hover:underline">
                          {r.gbif_wait.key} {m.onGbif} <ExternalLink className="size-3" aria-hidden />
                        </a>
                      )}
                    </>
                  ) : (
                    <>
                      <span className="block truncate">{jt.stage(r.stage)}</span>
                      <span className="num text-muted-foreground">
                        {Math.round(r.progress * 100)} % · {m.dataState(r.data_state)}
                      </span>
                    </>
                  )}
                </TableCell>
                <TableCell className="num text-right">{r.species}</TableCell>
                <TableCell className="num whitespace-nowrap">{formatDateTime(r.created_at)}</TableCell>
                <TableCell className="num whitespace-nowrap">{r.finished_at ? formatDateTime(r.finished_at) : "—"}</TableCell>
                <TableCell className="text-right">
                  <Button variant="ghost" size="sm" render={<Link to={jobUrl(r)} />} nativeButton={false}>
                    <ExternalLink aria-hidden /> {m.open}
                  </Button>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        </div>
      )}
    </div>
  );
}

export function ActivitySection() {
  return <ActivityTable />;
}
