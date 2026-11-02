// One account (ADR-0113): organisation, role and access, and what was done to or by it. Roles:
// manager (ADR-0109) and administrator (members of the operator organisation, with the
// administrator's password, ADR-0112); removing the administrator role stays on the server.

import { ArrowLeft, KeyRound, ShieldCheck } from "lucide-react";
import { useState, type ReactNode } from "react";
import { Link, useNavigate } from "react-router";

import { ApiError, api } from "@/api/client";
import { useAdminMutation, useAdminUser } from "@/api/hooks";
import type { GrantAdminRequest } from "@/api/types";
import { EmptyState, ErrorState } from "@/components/domain/common";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card } from "@/features/settings/Card";
import { formatDateTime } from "@/lib/format";

import { AuditList } from "./AuditList";
import { useAdminMessages } from "./messages";
import { Confirm, DeleteAccount, InlineError, LinkDialog, Loading, RoleBadge, Status, type Created } from "./shared";
import type { AdminSectionProps } from "./sections";

/** The administrator re-enters their password: a session alone cannot create administrators. */
function GrantAdmin({ email, disabled, onConfirm }: { email: string; disabled?: boolean; onConfirm: (password: string) => void }) {
  const m = useAdminMessages();
  const [password, setPassword] = useState("");
  return (
    <AlertDialog onOpenChange={(open) => !open && setPassword("")}>
      <AlertDialogTrigger render={<Button variant="ghost" size="sm" disabled={disabled} />}>
        <ShieldCheck aria-hidden /> {m.makeAdmin}
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{m.adminTitle(email)}</AlertDialogTitle>
          <AlertDialogDescription>{m.adminText}</AlertDialogDescription>
        </AlertDialogHeader>
        <div className="space-y-1.5">
          <Label htmlFor="confirm-admin">{m.yourPassword}</Label>
          <Input id="confirm-admin" type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoComplete="current-password" />
        </div>
        <AlertDialogFooter>
          <AlertDialogCancel>{m.cancel}</AlertDialogCancel>
          <AlertDialogAction disabled={!password} onClick={() => onConfirm(password)}>
            {m.adminAction}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="space-y-0.5">
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="text-sm">{children}</dd>
    </div>
  );
}

export function UserDetail({ me, id }: AdminSectionProps & { id: string }) {
  const m = useAdminMessages();
  const navigate = useNavigate();
  const user = useAdminUser(id);
  const [created, setCreated] = useState<Created | null>(null);
  const reset = useAdminMutation((email: string) => api.admin.resetLink(email));
  const toggle = useAdminMutation((a: { email: string; disabled: boolean }) => api.admin.setUserDisabled(a.email, a.disabled));
  const manager = useAdminMutation((a: { email: string; manager: boolean }) => api.admin.setManager(a.email, a.manager));
  const grant = useAdminMutation((b: GrantAdminRequest) => api.admin.grantAdmin(b));
  const erase = useAdminMutation((email: string) => api.admin.deleteUser(email, email));
  const back = (
    <Link to="/admin/users" className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
      <ArrowLeft className="size-3.5" aria-hidden /> {m.backUsers}
    </Link>
  );
  if (user.isPending) return <Loading />;
  const missing = user.error instanceof ApiError && user.error.status === 404;
  if (user.isError && !missing) return <ErrorState error={user.error} onRetry={() => void user.refetch()} title={m.loadUsersFailed} />;
  const u = user.data;
  if (!u)
    return (
      <div className="space-y-4">
        {back}
        <EmptyState title={m.userNotFound}>{m.notFoundText}</EmptyState>
      </div>
    );
  const self = u.email === me.email;
  const sender = `${me.name}\n${me.organisation}`;
  return (
    <div className="space-y-5">
      {back}
      <header className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <h1 className="text-xl font-semibold">{u.name}</h1>
        <RoleBadge u={u} />
        <Status disabledAt={u.disabled_at} />
        {self && <span className="text-xs text-muted-foreground">{m.you}</span>}
      </header>
      <Card>
        <dl className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <Field label={m.colEmail}>{u.email}</Field>
          <Field label={m.colOrg}>
            {/* Organisation pages are reached by id or name. */}
            <Link to={`/admin/orgs/${encodeURIComponent(u.org_name)}`} className="hover:underline">
              {u.org_name}
            </Link>
          </Field>
          <Field label={m.colLastSignIn}>{u.last_login_at ? formatDateTime(u.last_login_at) : <span className="text-muted-foreground">{m.never}</span>}</Field>
          <Field label={m.colCreated}>{formatDateTime(u.created_at)}</Field>
        </dl>
        {u.is_admin && !u.in_operator_org && <p className="text-xs text-muted-foreground">{m.outsideOperator}</p>}
      </Card>
      <Card title={m.cardAccess}>
        <div className="flex flex-wrap items-center gap-1">
          <Button
            variant="outline"
            size="sm"
            disabled={reset.isPending}
            onClick={() => reset.mutate(u.email, { onSuccess: (l) => setCreated({ link: l, kind: "reset", title: m.resetFor(u.email), org: u.org_name }) })}
          >
            <KeyRound aria-hidden /> {m.resetLink}
          </Button>
          {!u.is_admin &&
            (u.is_manager ? (
              <Button variant="ghost" size="sm" disabled={manager.isPending} onClick={() => manager.mutate({ email: u.email, manager: false })}>
                {m.removeManager}
              </Button>
            ) : (
              <Confirm
                label={m.makeManager}
                title={m.managerTitle(u.email)}
                description={m.managerText}
                action={m.managerAction}
                disabled={u.disabled_at !== null}
                onConfirm={() => manager.mutate({ email: u.email, manager: true })}
              />
            ))}
          {!u.is_admin && u.in_operator_org && (
            <GrantAdmin email={u.email} disabled={u.disabled_at !== null || grant.isPending} onConfirm={(password) => grant.mutate({ email: u.email, password })} />
          )}
        </div>
        <InlineError error={reset.error ?? manager.error ?? grant.error} />
      </Card>
      <section aria-label={m.dangerZone} className="space-y-2 rounded-lg border border-destructive/30 bg-card p-4">
        <h2 className="text-sm font-semibold">{m.dangerZone}</h2>
        <p className="text-xs text-muted-foreground">{m.dangerText}</p>
        <div className="flex flex-wrap items-center gap-1">
          {u.disabled_at ? (
            <Button variant="outline" size="sm" onClick={() => toggle.mutate({ email: u.email, disabled: false })}>
              {m.enable}
            </Button>
          ) : (
            <Confirm
              label={m.disable}
              disabled={self}
              title={m.disableUser(u.email)}
              description={m.disableUserText}
              action={m.disableUserAction}
              destructive
              onConfirm={() => toggle.mutate({ email: u.email, disabled: true })}
            />
          )}
          <DeleteAccount email={u.email} disabled={self} onConfirm={() => erase.mutate(u.email, { onSuccess: () => void navigate("/admin/users") })} />
        </div>
        <InlineError error={toggle.error ?? erase.error} />
      </section>
      <Card title={m.cardHistory}>
        <AuditList query={{ user: u.id }} />
      </Card>
      <LinkDialog
        link={created?.link ?? null}
        kind={created?.kind ?? "reset"}
        title={created?.title ?? ""}
        org={created?.org ?? ""}
        sender={sender}
        onClose={() => setCreated(null)}
      />
    </div>
  );
}
