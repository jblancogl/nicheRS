// Usage for invoicing (modelled species per organisation and month, CSV) and the month's cost and
// reliability metrics (ADR-0106).

import { Download } from "lucide-react";
import { useState } from "react";

import { urls } from "@/api/client";
import { useAdminMetrics, useAdminUsage } from "@/api/hooks";
import type { FailureCause } from "@/api/types";
import { ErrorState } from "@/components/domain/common";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { formatCount, formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";

import { CAUSES, useAdminMessages } from "./messages";
import { Loading, QuotaBar } from "./shared";

const thisMonth = () => new Date().toISOString().slice(0, 7);

function CauseChips({ failures }: { failures: Partial<Record<FailureCause, number>> }) {
  const m = useAdminMessages();
  const entries = CAUSES.filter((c) => (failures[c] ?? 0) > 0);
  if (entries.length === 0) return <span className="text-muted-foreground">—</span>;
  return (
    <span className="flex flex-wrap gap-1">
      {entries.map((c) => (
        <Badge key={c} variant="outline" className="font-normal">
          {m.cause(c)} · {failures[c]}
        </Badge>
      ))}
    </span>
  );
}

function MetricsTable({ month }: { month: string }) {
  const m = useAdminMessages();
  const metrics = useAdminMetrics(month || null);
  const secs = (s: number | null) => (s === null ? "—" : formatDuration(s * 1000));
  return (
    <section className="space-y-2" aria-labelledby="metrics-title">
      <h3 id="metrics-title" className="text-sm font-semibold">
        {m.metricsTitle}
      </h3>
      <p className="text-xs text-muted-foreground">{m.metricsText}</p>
      {metrics.isPending && <Loading />}
      {metrics.isError && <ErrorState error={metrics.error} onRetry={() => void metrics.refetch()} title={m.loadMetricsFailed} />}
      {metrics.data && (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{m.colOrg}</TableHead>
              <TableHead className="text-right">{m.colStudies}</TableHead>
              <TableHead className="text-right">{m.colRequested}</TableHead>
              <TableHead className="text-right">{m.colSucceeded}</TableHead>
              <TableHead className="text-right">{m.colFailed}</TableHead>
              <TableHead className="text-right">{m.colCompute}</TableHead>
              <TableHead className="text-right">{m.colPerSpecies}</TableHead>
              <TableHead className="text-right">{m.colGbifWait}</TableHead>
              <TableHead>{m.colCauses}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {metrics.data.length === 0 && (
              <TableRow>
                <TableCell colSpan={9} className="text-muted-foreground">
                  {m.noStudies}
                </TableCell>
              </TableRow>
            )}
            {metrics.data.map((r) => (
              <TableRow key={r.org_name}>
                <TableCell className="font-medium">{r.org_name === "—" ? m.beforeAccounts : r.org_name}</TableCell>
                <TableCell className="num text-right">{formatCount(r.studies)}</TableCell>
                <TableCell className="num text-right">{formatCount(r.species)}</TableCell>
                <TableCell className="num text-right">{formatCount(r.succeeded)}</TableCell>
                <TableCell className={cn("num text-right", r.failed > 0 && "text-destructive")}>{formatCount(r.failed)}</TableCell>
                <TableCell className="num text-right">{secs(r.compute_s)}</TableCell>
                <TableCell className="num text-right">{secs(r.compute_s_per_species)}</TableCell>
                <TableCell className="num text-right">
                  {secs(r.gbif_wait_s_mean)} / {secs(r.gbif_wait_s_max)}
                </TableCell>
                <TableCell>
                  <CauseChips failures={r.failures} />
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </section>
  );
}

export function UsageSection() {
  const m = useAdminMessages();
  const [month, setMonth] = useState(thisMonth);
  const usage = useAdminUsage(month || null);
  const total = usage.data?.reduce((a, r) => a + r.species, 0) ?? 0;
  return (
    <div className="space-y-6">
      <div className="space-y-4">
        <div className="flex flex-wrap items-end gap-3">
          <div className="space-y-1.5">
            <Label htmlFor="usage-month">{m.month}</Label>
            <Input id="usage-month" type="month" value={month} onChange={(e) => setMonth(e.target.value)} className="w-44" />
          </div>
          <Button variant="outline" render={<a href={urls.usageCsv(month || null)} download />} nativeButton={false}>
            <Download aria-hidden /> {m.csvMonth(month || m.allMonths)}
          </Button>
          <Button variant="ghost" render={<a href={urls.usageCsv(null)} download />} nativeButton={false}>
            <Download aria-hidden /> {m.csvAll}
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">{m.usageText}</p>
        {usage.isPending && <Loading />}
        {usage.isError && <ErrorState error={usage.error} onRetry={() => void usage.refetch()} title={m.loadUsageFailed} />}
        {usage.data && (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{m.colOrg}</TableHead>
                <TableHead className="text-right">{m.colModelled}</TableHead>
                <TableHead className="text-right">{m.colMonthlyQuota}</TableHead>
                <TableHead>{m.colUseOfQuota}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {usage.data.length === 0 && (
                <TableRow>
                  <TableCell colSpan={4} className="text-muted-foreground">
                    {m.noOrgShort}
                  </TableCell>
                </TableRow>
              )}
              {usage.data.map((r) => (
                <TableRow key={`${r.org_id}-${r.month}`}>
                  <TableCell className="font-medium">{r.org_name}</TableCell>
                  <TableCell className="num text-right">{r.species}</TableCell>
                  <TableCell className="num text-right">{r.monthly_quota}</TableCell>
                  <TableCell>
                    <QuotaBar used={r.species} reserved={0} total={r.monthly_quota} />
                  </TableCell>
                </TableRow>
              ))}
              {usage.data.length > 0 && (
                <TableRow className="font-medium">
                  <TableCell>{m.total}</TableCell>
                  <TableCell className="num text-right">{total}</TableCell>
                  <TableCell colSpan={2} />
                </TableRow>
              )}
            </TableBody>
          </Table>
        )}
      </div>
      {month && <MetricsTable month={month} />}
    </div>
  );
}

