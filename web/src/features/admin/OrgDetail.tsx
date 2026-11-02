// One organisation (ADR-0113): quota, members and invitations, recent studies and its history.
// Disabling it signs its users out; the operator organisation and one's own cannot be disabled
// from here (ADR-0112).

import { ArrowLeft, Link2 } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";

import { ApiError, api, errorMessage } from "@/api/client";
import { useAdminInvites, useAdminMutation, useAdminOrg, useAdminUsers } from "@/api/hooks";
import type { OrgOverview, UserInfo } from "@/api/types";
import { EmptyState, ErrorState } from "@/components/domain/common";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Card } from "@/features/settings/Card";
import { formatDateTime } from "@/lib/format";
import { cn } from "@/lib/utils";

import { ActivityTable } from "./ActivitySection";
import { AuditList } from "./AuditList";
import { useRowLink } from "./links";
import { useAdminMessages } from "./messages";
import { Confirm, InlineError, LinkDialog, Loading, MobileItem, MobileList, QuotaBar, RoleBadge, RowChevron, Status, type Created } from "./shared";
import { adminUrl, type AdminSectionProps } from "./sections";

function QuotaEditor({ org }: { org: OrgOverview }) {
  const m = useAdminMessages();
  const [value, setValue] = useState(String(org.monthly_quota));
  const save = useAdminMutation((q: number) => api.admin.updateOrg(org.id, { monthly_quota: q }));
  const changed = value !== String(org.monthly_quota);
  const valid = Number.isInteger(Number(value)) && Number(value) >= 0 && value !== "";
  return (
    <form
      className="flex items-center gap-1"
      onSubmit={(e) => {
        e.preventDefault();
        if (changed && valid) save.mutate(Number(value));
      }}
    >
      <Input type="number" min={0} value={value} onChange={(e) => setValue(e.target.value)} className="h-7 w-20" aria-label={m.quotaOf(org.name)} />
      {changed && (
        <Button type="submit" size="sm" variant="outline" disabled={!valid || save.isPending}>
          {m.save}
        </Button>
      )}
      {save.isError && <span className="text-xs text-destructive">{errorMessage(save.error)}</span>}
    </form>
  );
}
function PendingInvites({ org }: { org: OrgOverview }) {
  const m = useAdminMessages();
  const invites = useAdminInvites(org.id);
  const revoke = useAdminMutation((id: string) => api.admin.revokeInvite(id));
  if (invites.isPending) return <Skeleton className="h-6 w-64" />;
  if (invites.isError) return <InlineError error={invites.error} />;
  if (invites.data.length === 0) return <p className="text-sm text-muted-foreground">{m.noPendingInvite}</p>;
  return (
    <ul className="space-y-1">
      {invites.data.map((i) => (
        <li key={i.id} className="flex items-center gap-3 text-sm">
          <Link2 className="size-3.5 text-muted-foreground" aria-hidden />
          <span className="num">{m.inviteLine(formatDateTime(i.created_at), formatDateTime(i.expires_at))}</span>
          <Button size="sm" variant="ghost" onClick={() => revoke.mutate(i.id)} disabled={revoke.isPending}>
            {m.revoke}
          </Button>
        </li>
      ))}
      <InlineError error={revoke.error} />
    </ul>
  );
}

function Members({ org }: { org: OrgOverview }) {
  const m = useAdminMessages();
  const rowLink = useRowLink();
  const users = useAdminUsers({ org: org.id, limit: 500 });
  if (users.isPending) return <Loading />;
  if (users.isError) return <InlineError error={users.error} />;
  const members = users.data.pages.flatMap((p) => p.users);
  if (members.length === 0) return <p className="text-sm text-muted-foreground">{m.noMember}</p>;
  const lastSignIn = (u: UserInfo) => (u.last_login_at ? formatDateTime(u.last_login_at) : <span className="text-muted-foreground">{m.never}</span>);
  const name = (u: UserInfo, link?: boolean) => (
    <span className="flex flex-wrap items-center gap-x-2 gap-y-1">
      {link ? (
        <Link to={adminUrl("users", u.id)} className="font-medium hover:underline">
          {u.name}
        </Link>
      ) : (
        <span className="font-medium">{u.name}</span>
      )}
      {u.disabled_at && <Status disabledAt={u.disabled_at} />}
    </span>
  );
  return (
    <>
      <div className="hidden md:block">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{m.colName}</TableHead>
              <TableHead>{m.colRole}</TableHead>
              <TableHead>{m.colLastSignIn}</TableHead>
              <TableHead className="w-8" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {members.map((u) => (
              <TableRow key={u.id} {...rowLink(adminUrl("users", u.id))} className={cn("cursor-pointer", u.disabled_at && "text-muted-foreground")}>
                <TableCell>
                  {name(u, true)}
                  <span className="block text-xs text-muted-foreground">{u.email}</span>
                </TableCell>
                <TableCell>
                  <RoleBadge u={u} />
                </TableCell>
                <TableCell className="num">{lastSignIn(u)}</TableCell>
                <TableCell>
                  <RowChevron />
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
      <MobileList>
        {members.map((u) => (
          <MobileItem
            key={u.id}
            to={adminUrl("users", u.id)}
            muted={u.disabled_at !== null}
            title={name(u)}
            aside={<RoleBadge u={u} />}
            meta={
              <>
                <span className="block truncate">{u.email}</span>
                <span className="block">
                  {m.colLastSignIn} {lastSignIn(u)}
                </span>
              </>
            }
          />
        ))}
      </MobileList>
    </>
  );
}

export function OrgDetail({ me, id }: AdminSectionProps & { id: string }) {
  const m = useAdminMessages();
  const query = useAdminOrg(id);
  const [created, setCreated] = useState<Created | null>(null);
  const invite = useAdminMutation((org: string) => api.admin.createInvite(org));
  const toggle = useAdminMutation((a: { id: string; disabled: boolean }) => api.admin.updateOrg(a.id, { disabled: a.disabled }));
  const back = (
    <Link to="/admin/orgs" className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
      <ArrowLeft className="size-3.5" aria-hidden /> {m.backOrgs}
    </Link>
  );
  if (query.isPending) return <Loading />;
  const missing = query.error instanceof ApiError && query.error.status === 404;
  if (query.isError && !missing) return <ErrorState error={query.error} onRetry={() => void query.refetch()} title={m.loadOrgsFailed} />;
  const org = query.data;
  if (!org)
    return (
      <div className="space-y-4">
        {back}
        <EmptyState title={m.orgNotFound}>{m.notFoundText}</EmptyState>
      </div>
    );
  const mine = org.name === me.organisation;
  const sender = `${me.name}\n${me.organisation}`;
  return (
    <div className="space-y-5">
      {back}
      <header className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <h1 className="text-xl font-semibold">{org.name}</h1>
        <Status disabledAt={org.disabled_at} />
        {org.is_operator && <span className="text-xs text-muted-foreground">{m.operator}</span>}
        {mine && <span className="text-xs text-muted-foreground">{m.yours}</span>}
        <span className="text-xs text-muted-foreground">{m.created(formatDateTime(org.created_at))}</span>
        <div className="ml-auto flex flex-wrap gap-1">
          <Button
            variant="outline"
            size="sm"
            disabled={org.disabled_at !== null || invite.isPending}
            onClick={() => invite.mutate(org.id, { onSuccess: (l) => setCreated({ link: l, kind: "invite", title: m.inviteInto(org.name), org: org.name }) })}
          >
            <Link2 aria-hidden /> {m.invite}
          </Button>
          {org.disabled_at ? (
            <Button variant="ghost" size="sm" onClick={() => toggle.mutate({ id: org.id, disabled: false })}>
              {m.enable}
            </Button>
          ) : (
            <Confirm
              label={m.disable}
              disabled={mine || org.is_operator}
              title={m.disableOrg(org.name)}
              description={m.disableOrgText}
              action={m.disableOrgAction}
              destructive
              onConfirm={() => toggle.mutate({ id: org.id, disabled: true })}
            />
          )}
        </div>
      </header>
      <InlineError error={invite.error ?? toggle.error} />
      <Card title={m.cardQuota}>
        <div className="flex flex-wrap items-center gap-4">
          {/* Keyed on the saved value: a new quota from the server resets the field. */}
          <QuotaEditor key={`${org.id}-${org.monthly_quota}`} org={org} />
          <div className="min-w-64 flex-1">
            <QuotaBar used={org.quota.used} reserved={org.quota.reserved} total={org.quota.total} />
          </div>
        </div>
      </Card>
      <Card title={m.members}>
        <Members org={org} />
      </Card>
      <Card title={m.pendingTitle}>
        <PendingInvites org={org} />
      </Card>
      <Card title={m.cardStudies}>
        <ActivityTable org={org.id} />
      </Card>
      <Card title={m.cardHistory}>
        <AuditList query={{ org: org.id }} />
      </Card>
      <LinkDialog
        link={created?.link ?? null}
        kind={created?.kind ?? "invite"}
        title={created?.title ?? ""}
        org={created?.org ?? ""}
        sender={sender}
        onClose={() => setCreated(null)}
      />
    </div>
  );
}
