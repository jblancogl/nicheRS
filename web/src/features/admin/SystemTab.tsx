// State of the instance without kubectl: version, storage, queue, backups, GBIF account, packs.

import { RefreshCw } from "lucide-react";
import type { ReactNode } from "react";

import { useAdminSystem } from "@/api/hooks";
import { ErrorState } from "@/components/domain/common";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { formatBytes, formatCount, formatDate, formatDateTime } from "@/lib/format";
import { cn } from "@/lib/utils";

import { useAdminMessages } from "./messages";
import { backupHealth, gbifHealth, mirrorHealth, storageHealth, type Health } from "./health";
import { Loading } from "./shared";

function Card({ title, health, children }: { title: string; health?: Health; children: ReactNode }) {
  const m = useAdminMessages();
  return (
    <section className="space-y-2 rounded-lg border bg-card p-4" aria-label={title}>
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-sm font-semibold">{title}</h3>
        {health && (
          <Badge
            variant="outline"
            className={cn(
              health === "ok" && "border-state-success/40 text-state-success",
              health === "warn" && "border-state-warning/50 text-state-warning",
              health === "bad" && "border-destructive/50 text-destructive",
            )}
          >
            {health === "ok" ? m.stateOk : health === "warn" ? m.stateWarn : m.stateBad}
          </Badge>
        )}
      </div>
      <div className="space-y-1.5 text-sm">{children}</div>
    </section>
  );
}

function Row({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <span className="text-muted-foreground">{label}</span>
      <span className="num text-right">{value}</span>
    </div>
  );
}

export function SystemTab() {
  const m = useAdminMessages();
  const sys = useAdminSystem();
  if (sys.isPending) return <Loading />;
  if (sys.isError) return <ErrorState error={sys.error} onRetry={() => void sys.refetch()} title={m.loadSystemFailed} />;
  const s = sys.data;
  const now = sys.dataUpdatedAt;
  const used = s.disk ? s.disk.total_bytes - s.disk.available_bytes : 0;
  const usedPct = s.disk && s.disk.total_bytes > 0 ? (used / s.disk.total_bytes) * 100 : 0;
  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Button variant="ghost" size="sm" onClick={() => void sys.refetch()} disabled={sys.isFetching}>
          <RefreshCw aria-hidden className={cn(sys.isFetching && "motion-safe:animate-spin")} /> {m.refresh}
        </Button>
      </div>
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        <Card title={m.sysVersion}>
          <Row label={m.sysVersion} value={s.version} />
          <Row label={m.sysBuild} value={<code className="font-mono text-xs">{s.build.slice(0, 12)}</code>} />
          <Row label={m.sysStarted} value={formatDateTime(s.started_at)} />
          <p className="text-xs text-muted-foreground">{m.sysRetentionDays(s.retention_days)}</p>
        </Card>

        <Card title={m.sysStorage} health={storageHealth(s)}>
          {s.disk ? (
            <>
              <div className="h-2 overflow-hidden rounded-full bg-muted" aria-hidden>
                <div className={cn("h-full", storageHealth(s) === "ok" ? "bg-primary" : storageHealth(s) === "warn" ? "bg-state-warning" : "bg-destructive")} style={{ width: `${usedPct}%` }} />
              </div>
              <p className="text-xs">{m.sysDisk(formatBytes(used), formatBytes(s.disk.total_bytes), formatBytes(s.disk.available_bytes))}</p>
            </>
          ) : (
            <p className="text-xs text-muted-foreground">{m.sysDiskUnknown}</p>
          )}
          <Row label={m.sysDatabase} value={formatBytes(s.storage.database_bytes)} />
          <Row label={m.sysStudies} value={formatBytes(s.storage.jobs_bytes)} />
          <Row label={m.sysCache} value={formatBytes(s.storage.cache_bytes)} />
          <Row label={m.sysBoundaries} value={formatBytes(s.storage.boundaries_bytes)} />
          <Row label={m.sysBackupsSize} value={formatBytes(s.storage.backups_bytes)} />
        </Card>

        <Card title={m.sysJobs} health={s.jobs.failed_7d > 0 ? "warn" : "ok"}>
          <Row label={m.sysTotal} value={formatCount(s.jobs.total)} />
          <Row label={m.sysQueued} value={formatCount(s.jobs.queued)} />
          <Row label={m.sysPreparing} value={formatCount(s.jobs.preparing)} />
          <Row label={m.sysRunning} value={formatCount(s.jobs.running)} />
          <Row label={m.sysFailed7} value={<span className={cn(s.jobs.failed_7d > 0 && "text-state-warning")}>{formatCount(s.jobs.failed_7d)}</span>} />
        </Card>

        <Card title={m.sysBackups} health={backupHealth(s, now)}>
          <Row label={m.sysLastCopy} value={s.db_backups.last_at ? formatDateTime(s.db_backups.last_at) : m.sysNoCopy} />
          <p className="text-xs text-muted-foreground">{m.sysDbCopies(s.db_backups.count, s.db_backups.keep)}</p>
          <Row label={m.sysVolume} value={s.volume_backup ?? <span className="text-muted-foreground">{m.sysVolumeUnknown}</span>} />
        </Card>

        <Card title={m.sysGbif} health={gbifHealth(s)}>
          {!s.gbif.configured ? (
            <p>{m.sysGbifOff}</p>
          ) : (
            <>
              <p>
                {s.gbif.ok === true
                  ? m.sysGbifOk(s.gbif.user ?? "")
                  : s.gbif.ok === false
                    ? m.sysGbifRefused(s.gbif.user ?? "")
                    : m.sysGbifUnknown(s.gbif.user ?? "", s.gbif.detail ?? "")}
              </p>
              {s.gbif.checked_at && <p className="text-xs text-muted-foreground">{m.sysChecked(formatDateTime(s.gbif.checked_at))}</p>}
            </>
          )}
        </Card>

        {s.gbif_mirror && (
          <Card
            title={`${m.sysMirror} · ${s.gbif_mirror.name}`}
            health={mirrorHealth(s.gbif_mirror)}
          >
            <p>{m.sysMirrorState(s.gbif_mirror.state)}</p>
            {s.gbif_mirror.detail && <p className="text-xs text-muted-foreground">{s.gbif_mirror.detail}</p>}
            {s.gbif_mirror.snapshot ? (
              <>
                <p className="font-medium">{m.sysMirrorData(formatDate(s.gbif_mirror.snapshot.created))}</p>
                <Row label={m.sysMirrorRecords} value={formatCount(s.gbif_mirror.snapshot.records)} />
                <Row label={m.sysMirrorTaxa} value={formatCount(s.gbif_mirror.snapshot.taxa)} />
                <Row label={m.sysMirrorSize} value={formatBytes(s.gbif_mirror.snapshot.bytes)} />
                {s.gbif_mirror.snapshot.doi && (
                  <a href={s.gbif_mirror.snapshot.doi} target="_blank" rel="noreferrer" className="text-xs text-primary underline-offset-4 hover:underline">
                    {s.gbif_mirror.snapshot.doi}
                  </a>
                )}
                {!s.gbif_mirror.snapshot.usable && <p className="text-xs text-state-warning">{m.sysMirrorStale(s.gbif_mirror.max_age_days)}</p>}
              </>
            ) : (
              <p className="text-xs text-muted-foreground">{m.sysMirrorNone}</p>
            )}
            <p className="text-xs text-muted-foreground">{m.sysMirrorRefresh(s.gbif_mirror.refresh_days)}</p>
          </Card>
        )}

        <Card title={m.sysPacks} health={s.boundaries.length > 0 ? "ok" : "warn"}>
          {s.boundaries.length === 0 ? (
            <p>{m.sysNoPack}</p>
          ) : (
            s.boundaries.map((p) => <Row key={p.name} label={p.name} value={m.sysUnits(formatCount(p.units))} />)
          )}
        </Card>
      </div>
    </div>
  );
}
