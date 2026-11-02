// Client organisations: find one, see each one's quota, members and studies at a glance, and
// open it (OrgDetail) to act on it. Creating one is a folded form: it is rare next to reading.

import { Building2, Plus, Search } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link } from "react-router";

import { api } from "@/api/client";
import { useAdminMutation, useAdminOrgs } from "@/api/hooks";
import type { OrgOverview } from "@/api/types";
import { EmptyState, ErrorState } from "@/components/domain/common";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { cn } from "@/lib/utils";

import { useRowLink } from "./links";
import { useAdminMessages } from "./messages";
import { InlineError, Loading, MobileItem, MobileList, QuotaBar, RowChevron, Status } from "./shared";
import { adminUrl, type AdminSectionProps } from "./sections";

type StatusFilter = "all" | "active" | "disabled";

function CreateOrg({ onDone }: { onDone: () => void }) {
  const m = useAdminMessages();
  const [name, setName] = useState("");
  const [quota, setQuota] = useState("20");
  const create = useAdminMutation((b: { name: string; monthly_quota: number }) => api.admin.createOrg(b));
  const submit = (e: FormEvent) => {
    e.preventDefault();
    create.mutate({ name: name.trim(), monthly_quota: Number(quota) }, { onSuccess: onDone });
  };
  const valid = name.trim().length > 0 && Number.isInteger(Number(quota)) && Number(quota) >= 0;
  return (
    <form onSubmit={submit} className="flex flex-wrap items-end gap-3 rounded-lg border bg-card p-3">
      <div className="min-w-0 flex-1 basis-56 space-y-1.5">
        <Label htmlFor="new-org">{m.newOrg}</Label>
        <Input id="new-org" value={name} onChange={(e) => setName(e.target.value)} placeholder={m.newOrgPlaceholder} autoFocus />
      </div>
      <div className="space-y-1.5">
        <Label htmlFor="new-quota">{m.speciesMonth}</Label>
        <Input id="new-quota" type="number" min={0} value={quota} onChange={(e) => setQuota(e.target.value)} className="w-28" />
      </div>
      <div className="flex gap-1">
        <Button type="submit" disabled={!valid || create.isPending}>
          <Plus aria-hidden /> {m.create}
        </Button>
        <Button type="button" variant="ghost" onClick={onDone}>
          {m.cancel}
        </Button>
      </div>
      <InlineError error={create.error} />
    </form>
  );
}

/** Name with the marks that matter: yours, operator, invitations waiting, disabled. */
function OrgName({ o, mine, link }: { o: OrgOverview; mine: boolean; link?: boolean }) {
  const m = useAdminMessages();
  return (
    <>
      {link ? (
        <Link to={adminUrl("orgs", o.id)} className="hover:underline">
          {o.name}
        </Link>
      ) : (
        <span>{o.name}</span>
      )}
      {o.disabled_at && <Status disabledAt={o.disabled_at} />}
      {mine && <span className="text-xs font-normal text-muted-foreground">{m.yours}</span>}
      {o.is_operator && <span className="text-xs font-normal text-muted-foreground">{m.operator}</span>}
      {o.open_invites > 0 && <span className="text-xs font-normal text-muted-foreground">{m.pendingInvites(o.open_invites)}</span>}
    </>
  );
}

export function OrgsSection({ me }: AdminSectionProps) {
  const m = useAdminMessages();
  const orgs = useAdminOrgs();
  const rowLink = useRowLink();
  const [creating, setCreating] = useState(false);
  const [q, setQ] = useState("");
  const [status, setStatus] = useState<StatusFilter>("all");
  const term = q.trim().toLocaleLowerCase();
  const shown = (orgs.data ?? [])
    .filter((o) => !term || o.name.toLocaleLowerCase().includes(term))
    .filter((o) => status === "all" || (status === "disabled") === (o.disabled_at !== null))
    // Active organisations first; disabled ones are history.
    .sort((a, b) => Number(a.disabled_at !== null) - Number(b.disabled_at !== null) || a.name.localeCompare(b.name));
  const disabledCount = (orgs.data ?? []).filter((o) => o.disabled_at).length;
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <div className="relative min-w-0 flex-1 basis-56 sm:max-w-80">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden />
          <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder={m.searchOrg} aria-label={m.searchOrg} className="pl-8" type="search" />
        </div>
        {disabledCount > 0 && (
          <div role="group" aria-label={m.statusFilter} className="flex gap-1">
            {(["all", "active", "disabled"] as const).map((s) => (
              <Button key={s} size="sm" variant={status === s ? "secondary" : "ghost"} aria-pressed={status === s} onClick={() => setStatus(s)}>
                {s === "all" ? m.statusAll : s === "active" ? m.active : `${m.disabled} · ${disabledCount}`}
              </Button>
            ))}
          </div>
        )}
        {!creating && (
          <Button className="ml-auto" onClick={() => setCreating(true)}>
            <Plus aria-hidden /> {m.newOrg}
          </Button>
        )}
      </div>
      {creating && <CreateOrg onDone={() => setCreating(false)} />}
      {orgs.isPending && <Loading />}
      {orgs.isError && <ErrorState error={orgs.error} onRetry={() => void orgs.refetch()} title={m.loadOrgsFailed} />}
      {orgs.data && orgs.data.length === 0 && (
        <EmptyState icon={<Building2 className="size-6" aria-hidden />} title={m.noOrg}>
          {m.noOrgText}
        </EmptyState>
      )}
      {orgs.data && orgs.data.length > 0 && (
        <>
          <div className="hidden md:block">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{m.colOrg}</TableHead>
                  <TableHead>{m.colThisMonth(orgs.data[0]?.quota.month ?? "")}</TableHead>
                  <TableHead className="text-right">{m.colUsers}</TableHead>
                  <TableHead className="text-right">{m.colStudies}</TableHead>
                  <TableHead className="w-8" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {shown.length === 0 && (
                  <TableRow>
                    <TableCell colSpan={5} className="text-muted-foreground">
                      {m.noOrgMatch}
                    </TableCell>
                  </TableRow>
                )}
                {shown.map((o) => (
                  <TableRow key={o.id} {...rowLink(adminUrl("orgs", o.id))} className={cn("cursor-pointer", o.disabled_at && "text-muted-foreground")}>
                    <TableCell className="font-medium">
                      <span className="flex flex-wrap items-center gap-x-2 gap-y-1">
                        <OrgName o={o} mine={o.name === me.organisation} link />
                      </span>
                    </TableCell>
                    <TableCell>
                      <QuotaBar used={o.quota.used} reserved={o.quota.reserved} total={o.quota.total} />
                    </TableCell>
                    <TableCell className="num text-right">{o.users}</TableCell>
                    <TableCell className="num text-right">{o.runs}</TableCell>
                    <TableCell>
                      <RowChevron />
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
          <MobileList empty={shown.length === 0 ? m.noOrgMatch : undefined}>
            {shown.map((o) => (
              <MobileItem
                key={o.id}
                to={adminUrl("orgs", o.id)}
                muted={o.disabled_at !== null}
                title={<OrgName o={o} mine={o.name === me.organisation} />}
                meta={
                  <>
                    <QuotaBar used={o.quota.used} reserved={o.quota.reserved} total={o.quota.total} />
                    <span className="num block">
                      {m.colUsers} {o.users} · {m.colStudies} {o.runs}
                    </span>
                  </>
                }
              />
            ))}
          </MobileList>
        </>
      )}
    </div>
  );
}
