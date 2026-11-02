// Pieces shared by the administration console and the organisation page (managers).

import { ArrowLeft, Check, ChevronRight, Copy, ShieldCheck, Trash2, UserCog } from "lucide-react";
import { useState, type ReactNode } from "react";
import { Link } from "react-router";

import { errorMessage, urls } from "@/api/client";
import type { LinkCreated, UserInfo } from "@/api/types";
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
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { FeedbackButton } from "@/features/feedback/FeedbackButton";
import { LANGS, useLang, type Lang } from "@/i18n/lang";
import { cn } from "@/lib/utils";

import { linkMessage, type LinkKind } from "./inviteMessage";
import { useAdminMessages } from "./messages";

export function Loading() {
  return (
    <div className="space-y-2" aria-busy="true">
      {[0, 1, 2].map((i) => (
        <Skeleton key={i} className="h-10 w-full" />
      ))}
    </div>
  );
}

export function InlineError({ error }: { error: unknown }) {
  if (!error) return null;
  return (
    <p role="alert" className="text-sm text-destructive">
      {errorMessage(error)}
    </p>
  );
}

/** A link to show once: invitation into an organisation or password reset of an account. */
export interface Created {
  link: LinkCreated;
  kind: "invite" | "reset";
  title: string;
  org: string;
}

/** Share of the quota from which the organisation is flagged (used + reserved). */
export const QUOTA_WARN = 0.8;

/** Quota usage of one month: used (solid) + reserved (light) over the total; flagged from 80 %. */
export function QuotaBar({ used, reserved, total }: { used: number; reserved: number; total: number }) {
  const m = useAdminMessages();
  const pct = (v: number) => (total > 0 ? Math.min(100, (v / total) * 100) : 0);
  const share = total > 0 ? (used + reserved) / total : 0;
  const full = share >= 1;
  const warn = !full && share >= QUOTA_WARN;
  return (
    <div className="flex min-w-40 items-center gap-2" title={warn || full ? m.quotaWarn(Math.round(share * 100)) : undefined}>
      <div className="relative h-1.5 flex-1 overflow-hidden rounded-full bg-muted" aria-hidden>
        <div className={cn("absolute inset-y-0 left-0", full ? "bg-destructive" : warn ? "bg-state-warning" : "bg-primary")} style={{ width: `${pct(used)}%` }} />
        <div className="absolute inset-y-0 bg-primary/35" style={{ left: `${pct(used)}%`, width: `${pct(reserved)}%` }} />
      </div>
      <span className={cn("num text-xs whitespace-nowrap", full ? "font-medium text-destructive" : warn ? "font-medium text-state-warning" : "text-muted-foreground")}>
        {used}
        {reserved > 0 && <span title={m.reservedHint}> +{reserved}</span>} / {total}
      </span>
      {(warn || full) && <span className="sr-only">{m.quotaWarn(Math.round(share * 100))}</span>}
    </div>
  );
}

async function copyText(text: string, fallbackId: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    (document.getElementById(fallbackId) as HTMLInputElement | HTMLTextAreaElement | null)?.select();
    return false;
  }
}

/**
 * A freshly created one-time link, shown once, with a ready-to-send message in the recipient's
 * language (NicheRS sends no e-mail).
 */
export function LinkDialog({
  link,
  kind,
  title,
  org,
  sender,
  onClose,
}: {
  link: LinkCreated | null;
  kind: LinkKind;
  title: string;
  org: string;
  sender: string;
  onClose: () => void;
}) {
  const m = useAdminMessages();
  const uiLang = useLang((s) => s.lang);
  const [lang, setLang] = useState<Lang>(uiLang);
  const [copied, setCopied] = useState<"message" | "link" | null>(null);
  const url = link ? urls.link(link.path) : "";
  const days = link?.days_valid ?? 7;
  const message = link ? linkMessage(lang, { kind, url, days, org, sender }) : "";
  const done = (what: "message" | "link") => (ok: boolean) => {
    if (!ok) return;
    setCopied(what);
    window.setTimeout(() => setCopied(null), 2000);
  };
  return (
    <AlertDialog open={link !== null} onOpenChange={(open) => !open && onClose()}>
      <AlertDialogContent className="sm:max-w-2xl">
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{m.linkShownOnce(days)}</AlertDialogDescription>
        </AlertDialogHeader>
        <div className="space-y-2">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div role="group" aria-label={m.messageFor} className="flex items-center gap-1 text-xs">
              <span className="mr-1 text-muted-foreground">{m.messageFor}</span>
              {LANGS.map((l) => (
                <Button key={l.id} size="xs" variant={l.id === lang ? "secondary" : "ghost"} aria-pressed={l.id === lang} lang={l.id} onClick={() => setLang(l.id)}>
                  {l.label}
                </Button>
              ))}
            </div>
            <Button size="sm" onClick={() => void copyText(message, "link-message").then(done("message"))}>
              {copied === "message" ? <Check aria-hidden /> : <Copy aria-hidden />} {copied === "message" ? m.copied : m.copyMessage}
            </Button>
          </div>
          <textarea
            id="link-message"
            readOnly
            value={message}
            lang={lang}
            rows={12}
            className="w-full resize-y rounded-md border bg-muted/30 p-2 font-sans text-sm"
            onFocus={(e) => e.currentTarget.select()}
          />
          <div className="flex gap-2">
            <Input id="one-time-link" readOnly value={url} className="font-mono text-xs" onFocus={(e) => e.currentTarget.select()} aria-label={m.oneTimeLink} />
            <Button variant="outline" onClick={() => void copyText(url, "one-time-link").then(done("link"))}>
              {copied === "link" ? <Check aria-hidden /> : <Copy aria-hidden />} {copied === "link" ? m.copied : m.copyLink}
            </Button>
          </div>
        </div>
        <AlertDialogFooter>
          <AlertDialogCancel>{m.close}</AlertDialogCancel>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

/** A button that asks for confirmation naming the object. */
export function Confirm({
  label,
  title,
  description,
  action,
  destructive,
  disabled,
  onConfirm,
}: {
  label: ReactNode;
  title: string;
  description: ReactNode;
  action: string;
  destructive?: boolean;
  disabled?: boolean;
  onConfirm: () => void;
}) {
  const m = useAdminMessages();
  return (
    <AlertDialog>
      <AlertDialogTrigger render={<Button variant="ghost" size="sm" disabled={disabled} />}>{label}</AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{m.cancel}</AlertDialogCancel>
          <AlertDialogAction variant={destructive ? "destructive" : "default"} onClick={onConfirm}>
            {action}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

/** Irreversible erasure: the administrator types the account's e-mail to confirm. */
export function DeleteAccount({ email, disabled, onConfirm }: { email: string; disabled?: boolean; onConfirm: () => void }) {
  const m = useAdminMessages();
  const [typed, setTyped] = useState("");
  const ok = typed.trim().toLowerCase() === email.toLowerCase();
  return (
    <AlertDialog onOpenChange={(open) => !open && setTyped("")}>
      <AlertDialogTrigger render={<Button variant="ghost" size="sm" disabled={disabled} className="text-destructive" />}>
        <Trash2 aria-hidden /> {m.delete}
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{m.deleteTitle(email)}</AlertDialogTitle>
          <AlertDialogDescription>{m.deleteText}</AlertDialogDescription>
        </AlertDialogHeader>
        <div className="space-y-1.5">
          <Label htmlFor="confirm-delete">{m.typeEmail}</Label>
          <Input id="confirm-delete" value={typed} onChange={(e) => setTyped(e.target.value)} autoComplete="off" placeholder={email} />
        </div>
        <AlertDialogFooter>
          <AlertDialogCancel>{m.cancel}</AlertDialogCancel>
          <AlertDialogAction variant="destructive" disabled={!ok} onClick={onConfirm}>
            {m.deleteAction}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

export function Status({ disabledAt }: { disabledAt: string | null }) {
  const m = useAdminMessages();
  return disabledAt ? <Badge variant="destructive">{m.disabled}</Badge> : <Badge variant="secondary">{m.active}</Badge>;
}

export function RoleBadge({ u }: { u: UserInfo }) {
  const m = useAdminMessages();
  if (u.is_admin)
    return (
      <Badge>
        <ShieldCheck aria-hidden /> {m.roleAdmin}
      </Badge>
    );
  if (u.is_manager)
    return (
      <Badge variant="secondary">
        <UserCog aria-hidden /> {m.roleManager}
      </Badge>
    );
  return <span className="text-muted-foreground">{m.roleMember}</span>;
}

/** Header of the account, organisation and administration pages (outside the workspace shell). */
export function PageHeader({ title }: { title: string }) {
  const m = useAdminMessages();
  return (
    <header className="flex h-11 shrink-0 items-center gap-3 border-b bg-card px-3">
      <Link to="/" className="flex items-center gap-1.5 rounded-sm font-semibold tracking-tight">
        <svg viewBox="0 0 16 16" className="size-4 text-primary" aria-hidden>
          <path d="M8 1.5c3 3 4.5 5.4 4.5 7.3A4.5 4.5 0 0 1 3.5 8.8C3.5 6.9 5 4.5 8 1.5Z" fill="none" stroke="currentColor" strokeWidth="1.5" />
        </svg>
        NicheRS
      </Link>
      <span className="text-muted-foreground" aria-hidden>
        /
      </span>
      <span className="min-w-0 truncate font-medium">{title}</span>
      <div className="flex-1" />
      <FeedbackButton />
      <Button variant="ghost" size="sm" render={<Link to="/" />} nativeButton={false} aria-label={m.workspace}>
        <ArrowLeft aria-hidden /> <span className="hidden sm:inline">{m.workspace}</span>
      </Button>
    </header>
  );
}

/** Last cell of a clickable row. */
export function RowChevron() {
  return <ChevronRight className="ml-auto size-4 text-muted-foreground" aria-hidden />;
}

/**
 * Narrow screens: the rows of a list as stacked links, so nothing important hides in a
 * sideways-scrolling table (the table is shown from 768 px).
 */
export function MobileList({ children, empty }: { children: ReactNode; empty?: ReactNode }) {
  return (
    <ul className="divide-y overflow-hidden rounded-lg border bg-card md:hidden">
      {children}
      {empty && <li className="px-3 py-2.5 text-sm text-muted-foreground">{empty}</li>}
    </ul>
  );
}

export function MobileItem({ to, title, meta, aside, muted }: { to: string; title: ReactNode; meta?: ReactNode; aside?: ReactNode; muted?: boolean }) {
  return (
    <li>
      <Link to={to} className={cn("flex items-center gap-3 px-3 py-2.5 hover:bg-muted/50", muted && "text-muted-foreground")}>
        <div className="min-w-0 flex-1 space-y-1">
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm font-medium">{title}</div>
          {meta && <div className="space-y-1 text-xs text-muted-foreground">{meta}</div>}
        </div>
        {aside}
        <ChevronRight className="size-4 shrink-0 text-muted-foreground" aria-hidden />
      </Link>
    </li>
  );
}
