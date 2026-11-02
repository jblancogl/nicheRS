// Natural Solutions administration (ADR-0106, ADR-0113): sections grouped in a side navigation
// (./sections.tsx), one URL per section and per organisation or account (/admin/orgs/<id>,
// /admin/users/<id>). Full-width tables, outside the map workspace. Administrators give the role
// to members of the operator organisation after re-entering their password (ADR-0112); only the
// server command line removes it (`niche admin user demote`).

import { ShieldCheck } from "lucide-react";
import { useEffect } from "react";
import { NavLink, Navigate, useNavigate, useParams, useSearchParams } from "react-router";

import { ApiError, redirectToLogin } from "@/api/client";
import { useMe } from "@/api/hooks";
import { EmptyState, ErrorState } from "@/components/domain/common";
import { cn } from "@/lib/utils";

import { useAdminMessages } from "./messages";
import { PageHeader } from "./shared";
import { ADMIN_SECTIONS, GROUPS, adminUrl, type AdminSection } from "./sections";

function Nav() {
  const m = useAdminMessages();
  return (
    <nav aria-label={m.nav} className="hidden flex-col gap-5 md:flex">
      {GROUPS.map((g) => (
        <div key={g} className="flex flex-col gap-0.5">
          {m.group(g) && <p className="px-2.5 pb-1 text-xs font-medium text-muted-foreground">{m.group(g)}</p>}
          {ADMIN_SECTIONS.filter((s) => s.group === g).map((s) => {
            const Icon = s.icon;
            return (
              <NavLink
                key={s.id}
                to={adminUrl(s.id)}
                className={({ isActive }) =>
                  cn(
                    "flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm hover:bg-muted",
                    isActive ? "bg-muted font-medium text-foreground" : "text-muted-foreground",
                  )
                }
              >
                <Icon className="size-4 shrink-0" aria-hidden />
                <span>{m.section(s.id)}</span>
              </NavLink>
            );
          })}
        </div>
      ))}
    </nav>
  );
}

/** Narrow screens: every section in one select, grouped. */
function NavSelect({ current }: { current: AdminSection }) {
  const m = useAdminMessages();
  const navigate = useNavigate();
  return (
    <select
      aria-label={m.nav}
      value={current.id}
      onChange={(e) => void navigate(adminUrl(e.target.value as AdminSection["id"]))}
      className="h-9 w-full rounded-md border border-input bg-transparent px-2 text-sm md:hidden"
    >
      {GROUPS.map((g) => {
        const options = ADMIN_SECTIONS.filter((s) => s.group === g).map((s) => (
          <option key={s.id} value={s.id}>
            {m.section(s.id)}
          </option>
        ));
        return m.group(g) ? (
          <optgroup key={g} label={m.group(g)}>
            {options}
          </optgroup>
        ) : (
          options
        );
      })}
    </select>
  );
}

export function AdminPage() {
  const m = useAdminMessages();
  const me = useMe();
  const { section: path, id } = useParams();
  const [params] = useSearchParams();
  const signedOut = me.error instanceof ApiError && me.error.status === 401;
  useEffect(() => {
    if (signedOut) redirectToLogin();
  }, [signedOut]);
  if (me.isPending || signedOut) return null;
  const section = ADMIN_SECTIONS.find((s) => s.id === path);
  // /admin, the former ?tab= links, unknown sections and items of sections without detail pages.
  if (!section || (id && !section.Detail)) {
    const legacy = ADMIN_SECTIONS.find((s) => s.id === params.get("tab"));
    return <Navigate to={adminUrl(legacy?.id ?? "overview")} replace />;
  }
  const account = me.data;
  return (
    <div className="flex min-h-dvh flex-col bg-background">
      <PageHeader title={m.title} />
      <main className="mx-auto grid w-full max-w-7xl flex-1 content-start gap-5 p-4 md:grid-cols-[12rem_minmax(0,1fr)] md:gap-8 md:p-6">
        {me.isError && <ErrorState error={me.error} onRetry={() => void me.refetch()} />}
        {account === null && <EmptyState title={m.localMode}>{m.localModeText}</EmptyState>}
        {account && !account.is_admin && (
          <EmptyState icon={<ShieldCheck className="size-6" aria-hidden />} title={m.reserved}>
            {m.reservedText}
          </EmptyState>
        )}
        {account?.is_admin && (
          <>
            <Nav />
            <NavSelect current={section} />
            <div className="min-w-0 space-y-5">
              {id && section.Detail ? (
                <section.Detail me={account} id={id} />
              ) : (
                <>
                  <header className="space-y-1">
                    <h1 className="text-xl font-semibold">{m.section(section.id)}</h1>
                    <p className="text-sm text-muted-foreground">{m.sectionText(section.id)}</p>
                  </header>
                  <section.Component me={account} />
                </>
              )}
            </div>
          </>
        )}
      </main>
    </div>
  );
}
