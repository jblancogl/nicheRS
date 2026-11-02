// "My account" and "My organisation" as one settings area (ADR-0110, ADR-0111): a side navigation
// grouped by area (a scrolling row on narrow screens), one URL per section, and the section's
// title and purpose above its content. Sections come from ./sections.tsx.

import { ShieldCheck } from "lucide-react";
import { useEffect } from "react";
import { Link, NavLink, Navigate, useNavigate, useParams } from "react-router";

import { ApiError, redirectToLogin } from "@/api/client";
import { useMe } from "@/api/hooks";
import type { Me } from "@/api/types";
import { EmptyState, ErrorState } from "@/components/domain/common";
import { PageHeader } from "@/features/admin/shared";
import { cn } from "@/lib/utils";

import { useSettingsMessages } from "./messages";
import { sectionUrl, visibleSections, type Area, type Section } from "./sections";

const AREAS: Area[] = ["account", "org"];

function NavItem({ s }: { s: Section }) {
  const m = useSettingsMessages();
  const Icon = s.icon;
  return (
    <NavLink
      to={sectionUrl(s)}
      end
      className={({ isActive }) =>
        cn(
          "flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm hover:bg-muted",
          isActive ? "bg-muted font-medium text-foreground" : "text-muted-foreground",
        )
      }
    >
      <Icon className="size-4 shrink-0" aria-hidden />
      <span>{m.title(s.id)}</span>
      {s.managers && <span className="ml-auto text-[0.65rem] tracking-wide text-muted-foreground uppercase">{m.managersOnly}</span>}
    </NavLink>
  );
}

/** Narrow screens: every section in one select, grouped by area. */
function NavSelect({ me, current }: { me: Me; current: Section }) {
  const m = useSettingsMessages();
  const navigate = useNavigate();
  return (
    <select
      aria-label={m.nav}
      value={sectionUrl(current)}
      onChange={(e) => void navigate(e.target.value)}
      className="h-9 w-full rounded-md border border-input bg-transparent px-2 text-sm md:hidden"
    >
      {AREAS.map((area) => (
        <optgroup key={area} label={area === "org" ? me.organisation : m[area]}>
          {visibleSections(me, area).map((s) => (
            <option key={s.id} value={sectionUrl(s)}>
              {m.title(s.id)}
            </option>
          ))}
        </optgroup>
      ))}
      {me.is_admin && (
        <optgroup label={m.ns}>
          <option value="/admin">{m.administration}</option>
        </optgroup>
      )}
    </select>
  );
}

function Nav({ me }: { me: Me }) {
  const m = useSettingsMessages();
  return (
    <nav aria-label={m.nav} className="hidden flex-col gap-5 md:flex">
      {AREAS.map((area) => (
        <div key={area} className="flex flex-col gap-0.5">
          <p className="truncate px-2.5 pb-1 text-xs font-medium text-muted-foreground">{area === "org" ? me.organisation : m[area]}</p>
          {visibleSections(me, area).map((s) => (
            <NavItem key={s.id} s={s} />
          ))}
        </div>
      ))}
      {me.is_admin && (
        <div className="flex flex-col gap-0.5">
          <p className="px-2.5 pb-1 text-xs font-medium text-muted-foreground">{m.ns}</p>
          <Link to="/admin" className="flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm text-muted-foreground hover:bg-muted">
            <ShieldCheck className="size-4" aria-hidden /> {m.administration}
          </Link>
        </div>
      )}
    </nav>
  );
}

function SettingsPage({ area }: { area: Area }) {
  const m = useSettingsMessages();
  const me = useMe();
  const { section: path = "" } = useParams();
  const signedOut = me.error instanceof ApiError && me.error.status === 401;
  useEffect(() => {
    if (signedOut) redirectToLogin();
  }, [signedOut]);
  if (me.isPending || signedOut) return null;
  const account = me.data;
  const siblings = account ? visibleSections(account, area) : [];
  const section = siblings.find((s) => s.path === path);
  // Unknown or not allowed (e.g. a member opening a managers' page): the area's first page.
  if (account && !section) return <Navigate to={`/${area}`} replace />;
  return (
    <div className="flex min-h-dvh flex-col bg-background">
      <PageHeader title={m[area]} />
      <main className="mx-auto grid w-full max-w-6xl flex-1 content-start gap-5 p-4 md:grid-cols-[14rem_minmax(0,1fr)] md:gap-10 md:p-6">
        {me.isError && <ErrorState error={me.error} onRetry={() => void me.refetch()} />}
        {/* Local mode: no accounts, nothing to set here. */}
        {account === null && <EmptyState title={m[area]} />}
        {account && section && (
          <>
            <Nav me={account} />
            <NavSelect me={account} current={section} />
            <div className="min-w-0 space-y-5">
              <header className="space-y-1">
                <h1 className="text-xl font-semibold">{m.title(section.id)}</h1>
                <p className="text-sm text-muted-foreground">{m.text(section.id)}</p>
              </header>
              <section.Component me={account} siblings={siblings.filter((s) => s !== section)} />
            </div>
          </>
        )}
      </main>
    </div>
  );
}

export const AccountSettings = () => <SettingsPage area="account" />;
export const OrgSettings = () => <SettingsPage area="org" />;
