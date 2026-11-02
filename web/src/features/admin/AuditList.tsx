// The audit trail (ADR-0113): the History section, the history of one organisation or account on
// its page, and an organisation's history for its managers (settings, without administration
// links). Newest first, older entries on demand.

import { Link } from "react-router";

import type { InfiniteData, UseInfiniteQueryResult } from "@tanstack/react-query";
import type { ReactNode } from "react";

import { useAdminAudit } from "@/api/hooks";
import type { AuditEntry, AuditPage, AuditQuery } from "@/api/types";
import { ErrorState } from "@/components/domain/common";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { formatDateTime } from "@/lib/format";

import { useAdminMessages } from "./messages";
import { Loading } from "./shared";

/** A link in the administration; plain text for managers. */
function To({ to, links, children }: { to: string; links: boolean; children: ReactNode }) {
  return links ? (
    <Link to={to} className="hover:underline">
      {children}
    </Link>
  ) : (
    <span>{children}</span>
  );
}

function Who({ e, links }: { e: AuditEntry; links: boolean }) {
  const m = useAdminMessages();
  if (e.via === "cli") return <span className="text-muted-foreground">{m.byServer}</span>;
  if (!e.actor_id) return <span className="text-muted-foreground">—</span>;
  if (!e.actor_name) return <span className="text-muted-foreground">{m.deletedAccount}</span>;
  return (
    <To to={`/admin/users/${e.actor_id}`} links={links}>
      {e.actor_name}
    </To>
  );
}

function Concerning({ e, query, links }: { e: AuditEntry; query: AuditQuery; links: boolean }) {
  const m = useAdminMessages();
  const parts = [];
  // On an organisation's or account's page, not that organisation or account again.
  if (e.org_id && e.org_id !== query.org)
    parts.push(
      e.org_name ? (
        <To key="org" to={`/admin/orgs/${e.org_id}`} links={links}>
          {e.org_name}
        </To>
      ) : (
        <span key="org" className="text-muted-foreground">
          {m.deletedOrg}
        </span>
      ),
    );
  if (e.user_id && e.user_id !== query.user)
    parts.push(
      e.user_email ? (
        <To key="user" to={`/admin/users/${e.user_id}`} links={links}>
          {e.user_name} · {e.user_email}
        </To>
      ) : (
        <span key="user" className="text-muted-foreground">
          {m.deletedAccount}
        </span>
      ),
    );
  if (e.job_id)
    parts.push(
      e.job_name ? (
        <Link key="job" to={`/jobs/${e.job_id}`} className="hover:underline">
          {e.job_name}
        </Link>
      ) : (
        <span key="job" className="text-muted-foreground">
          {m.deletedStudy}
        </span>
      ),
    );
  if (parts.length === 0) return <span className="text-muted-foreground">—</span>;
  return <span className="flex flex-col gap-0.5">{parts}</span>;
}

/** The entries of an audit query, with "show older". `links`: to the administration pages. */
export function AuditTable({
  audit,
  query = {},
  links = true,
}: {
  audit: UseInfiniteQueryResult<InfiniteData<AuditPage>, unknown>;
  query?: AuditQuery;
  links?: boolean;
}) {
  const m = useAdminMessages();
  if (audit.isPending) return <Loading />;
  if (audit.isError) return <ErrorState error={audit.error} onRetry={() => void audit.refetch()} title={m.loadAuditFailed} />;
  const entries = audit.data.pages.flatMap((p) => p.entries);
  if (entries.length === 0) return <p className="text-sm text-muted-foreground">{m.noAudit}</p>;
  return (
    <div className="space-y-3">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{m.colWhen}</TableHead>
            <TableHead>{m.colWho}</TableHead>
            <TableHead>{m.colWhat}</TableHead>
            <TableHead>{m.colConcerning}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {entries.map((e) => (
            <TableRow key={e.id}>
              <TableCell className="num whitespace-nowrap">{formatDateTime(e.at)}</TableCell>
              <TableCell>
                <Who e={e} links={links} />
              </TableCell>
              <TableCell>{m.auditAction(e.action, e.detail)}</TableCell>
              <TableCell>
                <Concerning e={e} query={query} links={links} />
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {audit.hasNextPage && (
        <Button variant="outline" size="sm" disabled={audit.isFetchingNextPage} onClick={() => void audit.fetchNextPage()}>
          {m.showOlder}
        </Button>
      )}
    </div>
  );
}

export function AuditList({ query }: { query: AuditQuery }) {
  return <AuditTable audit={useAdminAudit(query)} query={query} />;
}

export function AuditSection() {
  return <AuditList query={{}} />;
}
