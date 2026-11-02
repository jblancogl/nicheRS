import { ArrowLeft, ChevronRight, CircleHelp, CircleUser, Languages, LogOut, Menu, Monitor, Moon, Plus, ShieldCheck, Sun, UserCog } from "lucide-react";
import { Link, useMatch, useNavigate } from "react-router";

import { useChooseLang, useHealth, useJob, useLogout, useMe, useProjects } from "@/api/hooks";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { HelpPanel } from "@/features/help/HelpPanel";
import { useHelp } from "@/features/help/store";
import { LANGS, isLang, useLang } from "@/i18n/lang";
import { FeedbackButton } from "@/features/feedback/FeedbackButton";
import { GlobalSearch } from "@/features/search/GlobalSearch";
import { useWorkspaceMessages } from "@/features/workspace/messages";
import { useDraft } from "@/stores/draft";
import { useTheme, type ThemePref } from "@/stores/theme";

import { useShellMessages } from "./messages";

/**
 * Where you are, in one place for every page below the workspace:
 * "← Organisation › Project › Study" (or "› New study" in the wizard). The first item is the way
 * back to the workspace.
 */
function Trail() {
  const m = useShellMessages();
  const w = useWorkspaceMessages();
  const me = useMe();
  const id = useMatch("/jobs/:id/*")?.params.id;
  const onSetup = useMatch("/new") !== null;
  const job = useJob(id, false);
  const draftProject = useDraft((s) => s.projectId);
  const projects = useProjects();
  if (!id && !onSetup) return null;
  const projectId = id ? job.data?.project_id : draftProject;
  const project = projects.data?.find((p) => p.id === projectId);
  const home = me.data?.organisation ?? w.myStudies;
  const sep = <ChevronRight className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />;
  return (
    <nav aria-label={w.location} className="flex min-w-0 items-center gap-1.5 text-sm">
      <Link
        to="/"
        className="inline-flex shrink-0 items-center gap-1 rounded-md px-1.5 py-1 font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
      >
        <ArrowLeft className="size-4" aria-hidden /> <span className="max-w-40 truncate">{home}</span>
      </Link>
      {project && (
        <>
          {sep}
          <Link
            to={`/?project=${encodeURIComponent(project.id)}`}
            className="hidden max-w-56 truncate rounded-md px-1.5 py-1 text-muted-foreground hover:bg-muted hover:text-foreground sm:inline"
          >
            {project.name}
          </Link>
        </>
      )}
      {sep}
      <span aria-current="page" className="min-w-0 truncate px-1 font-medium">
        {id ? (job.data?.name ?? "…") : m.studySetup}
      </span>
    </nav>
  );
}

/**
 * The one menu on the right: account and quota (hosted mode), interface language (stored with
 * the account), theme, help, administration and sign-out. In local mode, without accounts, the
 * same menu keeps language, theme and help.
 */
function UserMenu() {
  const m = useShellMessages();
  const me = useMe();
  const logout = useLogout();
  const navigate = useNavigate();
  const lang = useLang((s) => s.lang);
  const choose = useChooseLang();
  const pref = useTheme((s) => s.pref);
  const setPref = useTheme((s) => s.setPref);
  const showHelp = useHelp((s) => s.show);
  const account = me.data;
  const current = LANGS.find((l) => l.id === lang)?.label ?? lang;
  const used = account ? account.quota.used + account.quota.reserved : 0;
  const low = account ? account.quota.remaining === 0 : false;
  const ThemeIcon = { light: Sun, dark: Moon, system: Monitor }[pref];
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            size="sm"
            data-user-menu
            aria-label={
              account
                ? m.accountLabel(account.name, account.organisation, account.quota.remaining, account.quota.total)
                : m.menuLabel
            }
          />
        }
      >
        {account ? (
          <>
            <CircleUser aria-hidden />
            <span className="hidden max-w-36 truncate md:inline">{account.organisation}</span>
            <span
              className={`num rounded-full border px-1.5 text-[0.7rem] leading-4 ${low ? "border-destructive text-destructive" : "text-muted-foreground"}`}
            >
              {m.quotaShort(used, account.quota.total)}
            </span>
          </>
        ) : (
          <>
            <Menu aria-hidden /> <span className="hidden sm:inline">{m.menu}</span>
          </>
        )}
        <span className="text-[0.7rem] font-medium text-muted-foreground uppercase" aria-hidden>
          {lang}
        </span>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        {account && (
          <>
            <DropdownMenuGroup>
              <DropdownMenuLabel className="flex flex-col gap-0.5">
                <span className="truncate font-medium text-foreground">{account.name}</span>
                <span className="truncate text-xs font-normal">
                  {account.email} · {account.organisation}
                </span>
                {/* The month's quota at a glance; details on the organisation overview. */}
                <span className="mt-1.5 h-1 overflow-hidden rounded-full bg-muted" aria-hidden>
                  <span
                    className={`block h-full ${low ? "bg-destructive" : "bg-primary"}`}
                    style={{ width: `${account.quota.total ? Math.min(100, (used / account.quota.total) * 100) : 0}%` }}
                  />
                </span>
                <span className="num text-xs font-normal">{m.quotaLeft(account.quota.remaining, account.quota.total)}</span>
              </DropdownMenuLabel>
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuGroup>
              <DropdownMenuItem onClick={() => void navigate("/account")}>
                <CircleUser aria-hidden /> {m.myAccount}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => void navigate("/org")}>
                <UserCog aria-hidden /> {m.myOrganisation}
              </DropdownMenuItem>
              {account.is_admin && (
                <DropdownMenuItem onClick={() => void navigate("/admin")}>
                  <ShieldCheck aria-hidden /> {m.administration}
                </DropdownMenuItem>
              )}
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
          </>
        )}
        <DropdownMenuGroup>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger data-language-menu>
              <Languages aria-hidden /> {m.language}
              <span className="ml-auto text-xs text-muted-foreground">{current}</span>
            </DropdownMenuSubTrigger>
            <DropdownMenuSubContent>
              <DropdownMenuRadioGroup value={lang} onValueChange={(v) => isLang(v) && choose(v)}>
                {LANGS.map((l) => (
                  <DropdownMenuRadioItem key={l.id} value={l.id} lang={l.id}>
                    {l.label}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuSubContent>
          </DropdownMenuSub>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>
              <ThemeIcon aria-hidden /> {m.theme}
              <span className="ml-auto text-xs text-muted-foreground">{m[pref]}</span>
            </DropdownMenuSubTrigger>
            <DropdownMenuSubContent>
              <DropdownMenuRadioGroup value={pref} onValueChange={(v) => setPref(v as ThemePref)}>
                <DropdownMenuRadioItem value="light">{m.light}</DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="dark">{m.dark}</DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="system">{m.system}</DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuSubContent>
          </DropdownMenuSub>
          <DropdownMenuItem onClick={() => showHelp()} data-help-button>
            <CircleHelp aria-hidden /> {m.helpCentre}
          </DropdownMenuItem>
        </DropdownMenuGroup>
        {account && (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={() => logout.mutate()}>
              <LogOut aria-hidden /> {m.signOut}
            </DropdownMenuItem>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Shown only when something is wrong: a healthy connection needs no permanent label. */
function HealthAlert() {
  const m = useShellMessages();
  const health = useHealth();
  if (health.isPending) return null;
  const ok = health.data !== undefined;
  const offline = ok && !health.data.online;
  if (ok && !offline) return null;
  return (
    <span className="inline-flex items-center gap-1.5 text-xs" role="status">
      <span className={`size-2 rounded-full ${!ok ? "bg-state-danger" : "bg-state-warning"}`} aria-hidden />
      <span className={!ok ? "text-destructive" : "text-state-warning"}>{!ok ? m.serverDown : m.sourcesDown}</span>
    </span>
  );
}

export function TopBar() {
  const m = useShellMessages();
  const navigate = useNavigate();
  // The workspace has its own "New study", and the wizard is the new study: no button there.
  const onWorkspace = useMatch("/") !== null;
  const onSetup = useMatch("/new") !== null;
  const noNewStudy = onWorkspace || onSetup;
  return (
    <header className="grid h-11 shrink-0 grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-2 border-b bg-card px-3 md:gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(14rem,36rem)_minmax(0,1fr)]">
      {/* Left: where you are. Centre: global search. Right: account. */}
      <div className="flex min-w-0 items-center gap-2">
        <Link to="/" aria-label={m.home} className="flex shrink-0 items-center gap-1.5 rounded-sm font-semibold tracking-tight">
          <svg viewBox="0 0 16 16" className="size-4 text-primary" aria-hidden>
            <path d="M8 1.5c3 3 4.5 5.4 4.5 7.3A4.5 4.5 0 0 1 3.5 8.8C3.5 6.9 5 4.5 8 1.5Z" fill="none" stroke="currentColor" strokeWidth="1.5" />
          </svg>
          <span className="hidden sm:inline">NicheRS</span>
        </Link>
        <Trail />
      </div>
      <div className="flex justify-center">
        <GlobalSearch />
      </div>
      <div className="flex min-w-0 items-center justify-end gap-2">
        <HealthAlert />
        {!noNewStudy && (
          <Button variant="ghost" size="sm" onClick={() => void navigate("/new")}>
            <Plus aria-hidden /> <span className="hidden lg:inline">{m.studySetup}</span>
          </Button>
        )}
        {!noNewStudy && <div className="mx-1 h-5 w-px bg-border" aria-hidden />}
        <FeedbackButton />
        <UserMenu />
        <HelpPanel />
      </div>
    </header>
  );
}
