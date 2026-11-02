// "My organisation" sections (ADR-0109, ADR-0110, ADR-0111): overview for every member; members
// and invitations, and feedback, for managers; the organisation's GBIF account (managers edit it,
// members see whether it exists). Listed in @/features/settings/sections.tsx.

import { ChevronRight, CircleHelp, KeyRound, Link2, UserPlus } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";

import { api } from "@/api/client";
import { useShellMessages } from "@/app/messages";
import { useAccountMutation, useOrgAudit, useOrgFeedback, useOrgInvites, useOrgMembers, useOrgMutation, useOrgProfile } from "@/api/hooks";
import type { LinkCreated } from "@/api/types";
import { ErrorState } from "@/components/domain/common";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { GbifAccountEditor, GbifStatus } from "@/features/account/GbifAccountCard";
import { useAccountMessages } from "@/features/account/messages";
import { AuditTable } from "@/features/admin/AuditList";
import { useAdminMessages } from "@/features/admin/messages";
import { Confirm, InlineError, LinkDialog, Loading, QuotaBar, RoleBadge, Status } from "@/features/admin/shared";
import { FeedbackList } from "@/features/feedback/FeedbackList";
import { Card } from "@/features/settings/Card";
import { useSettingsMessages } from "@/features/settings/messages";
import { sectionUrl, type SectionProps } from "@/features/settings/sections";
import { formatDateTime } from "@/lib/format";

/** Name, members, quota, and a door to every other section this member may open. */
export function OverviewSection({ me, siblings }: SectionProps) {
  const a = useAccountMessages();
  const s = useSettingsMessages();
  const shell = useShellMessages();
  const profile = useOrgProfile(true);
  return (
    <div className="space-y-5">
      <Card className="max-w-xl">
        <p className="text-lg font-semibold">{me.organisation}</p>
        {profile.data && <p className="num text-sm text-muted-foreground">{a.members(profile.data.members)}</p>}
        <div className="max-w-sm space-y-1">
          <p className="text-xs text-muted-foreground">{a.quota}</p>
          <QuotaBar used={me.quota.used} reserved={me.quota.reserved} total={me.quota.total} />
        </div>
        <p className="text-xs text-muted-foreground">{shell.quotaNote}</p>
        <Link to="/help/account?s=quota" className="inline-flex items-center gap-1 text-sm text-primary hover:underline">
          <CircleHelp className="size-3.5" aria-hidden /> {shell.quotaHelp}
        </Link>
      </Card>
      {siblings.length > 0 && (
        <ul className="grid gap-3 sm:grid-cols-2">
          {siblings.map((x) => {
            const Icon = x.icon;
            return (
              <li key={x.id}>
                <Link to={sectionUrl(x)} className="flex h-full items-start gap-3 rounded-lg border bg-card p-4 hover:bg-muted/50">
                  <Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />
                  <span className="min-w-0 flex-1 space-y-0.5">
                    <span className="block text-sm font-medium">{s.title(x.id)}</span>
                    <span className="block text-sm text-muted-foreground">{s.text(x.id)}</span>
                  </span>
                  <ChevronRight className="size-4 shrink-0 text-muted-foreground" aria-hidden />
                </Link>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

export function MembersSection({ me }: SectionProps) {
  const m = useAdminMessages();
  const members = useOrgMembers(true);
  const invites = useOrgInvites(true);
  const [created, setCreated] = useState<{ link: LinkCreated; kind: "invite" | "reset"; title: string } | null>(null);
  const invite = useOrgMutation(() => api.org.invite());
  const revoke = useOrgMutation((id: string) => api.org.revokeInvite(id));
  const reset = useOrgMutation((email: string) => api.org.resetLink(email));
  const toggle = useOrgMutation((a: { email: string; disabled: boolean }) => api.org.setDisabled(a.email, a.disabled));
  const org = me.organisation;
  return (
    <div className="space-y-6">
      <div>
        <Button
          onClick={() => invite.mutate(undefined, { onSuccess: (l) => setCreated({ link: l, kind: "invite", title: m.inviteInto(org) }) })}
          disabled={invite.isPending}
        >
          <UserPlus aria-hidden /> {m.inviteColleague}
        </Button>
      </div>
      <InlineError error={invite.error ?? revoke.error ?? reset.error ?? toggle.error} />

      <section className="space-y-2" aria-label={m.members}>
        {members.isPending && <Loading />}
        {members.isError && <ErrorState error={members.error} onRetry={() => void members.refetch()} title={m.loadMembersFailed} />}
        {members.data && (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{m.colName}</TableHead>
                <TableHead>{m.colEmail}</TableHead>
                <TableHead>{m.colRole}</TableHead>
                <TableHead>{m.colLastSignIn}</TableHead>
                <TableHead>{m.colStatus}</TableHead>
                <TableHead className="text-right">{m.colActions}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {members.data.map((u) => {
                const self = u.email === me.email;
                const locked = u.is_admin && !me.is_admin;
                return (
                  <TableRow key={u.id}>
                    <TableCell className="font-medium">
                      {u.name}
                      {self && <span className="ml-2 text-xs text-muted-foreground">{m.you}</span>}
                    </TableCell>
                    <TableCell>{u.email}</TableCell>
                    <TableCell>
                      <RoleBadge u={u} />
                    </TableCell>
                    <TableCell className="num">{u.last_login_at ? formatDateTime(u.last_login_at) : <span className="text-muted-foreground">{m.never}</span>}</TableCell>
                    <TableCell>
                      <Status disabledAt={u.disabled_at} />
                    </TableCell>
                    <TableCell className="text-right whitespace-nowrap">
                      {self ? null : locked ? (
                        <span className="text-xs text-muted-foreground">{m.adminManaged}</span>
                      ) : (
                        <>
                          <Button
                            variant="outline"
                            size="sm"
                            disabled={reset.isPending}
                            onClick={() => reset.mutate(u.email, { onSuccess: (l) => setCreated({ link: l, kind: "reset", title: m.resetFor(u.email) }) })}
                          >
                            <KeyRound aria-hidden /> {m.resetLink}
                          </Button>
                          {u.disabled_at ? (
                            <Button variant="ghost" size="sm" onClick={() => toggle.mutate({ email: u.email, disabled: false })}>
                              {m.enable}
                            </Button>
                          ) : (
                            <Confirm
                              label={m.disable}
                              title={m.disableUser(u.email)}
                              description={m.disableUserText}
                              action={m.disableUserAction}
                              destructive
                              onConfirm={() => toggle.mutate({ email: u.email, disabled: true })}
                            />
                          )}
                        </>
                      )}
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        )}
      </section>

      <section className="space-y-2" aria-labelledby="pending-title">
        <h2 id="pending-title" className="text-sm font-semibold">
          {m.pendingTitle}
        </h2>
        {invites.data?.length === 0 && <p className="text-sm text-muted-foreground">{m.noPendingInvite}</p>}
        {invites.data && invites.data.length > 0 && (
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
          </ul>
        )}
      </section>
      <LinkDialog
        link={created?.link ?? null}
        kind={created?.kind ?? "invite"}
        title={created?.title ?? ""}
        org={org}
        sender={`${me.name}\n${me.organisation}`}
        onClose={() => setCreated(null)}
      />
    </div>
  );
}

/** The organisation's GBIF account: managers edit it, other members see whether it exists. */
export function OrgGbifSection() {
  const a = useAccountMessages();
  const profile = useOrgProfile(true);
  const save = useAccountMutation((b: { username: string; password: string }) => api.org.setGbif(b));
  const check = useAccountMutation(() => api.org.checkGbif());
  const clear = useAccountMutation(() => api.org.clearGbif());
  const p = profile.data;
  return (
    <Card className="max-w-xl">
      {profile.isPending && <Loading />}
      {profile.isError && <ErrorState error={profile.error} onRetry={() => void profile.refetch()} />}
      {p &&
        (p.can_manage ? (
          <GbifAccountEditor
            info={p.gbif}
            canStore={p.can_store}
            save={(b) => save.mutateAsync(b)}
            check={() => check.mutateAsync(undefined)}
            clear={() => clear.mutateAsync(undefined)}
          />
        ) : (
          <div className="space-y-2">
            <p className="text-sm">{p.gbif ? a.orgHas : a.orgHasNot}</p>
            {p.gbif && <GbifStatus info={p.gbif} />}
            <p className="text-xs text-muted-foreground">{a.managersOnly}</p>
          </div>
        ))}
    </Card>
  );
}

export function OrgFeedbackSection() {
  const feedback = useOrgFeedback(true);
  return (
    <div className="max-w-4xl">
      <FeedbackList query={feedback} />
    </div>
  );
}

/** What changed in the organisation, by whom (managers; ADR-0113). */
export function OrgHistorySection() {
  const audit = useOrgAudit(true);
  // Every entry is about this organisation: not named again in each row.
  const org = audit.data?.pages[0]?.entries[0]?.org_id ?? undefined;
  return <AuditTable audit={audit} query={{ org }} links={false} />;
}
