// Hosted mode entry points (ADR-0105). There is no sign-up: the invitation link handed over by
// Natural Solutions is the only way in. The link token travels in the URL fragment (#…), which
// browsers never send to the server, and is posted in a request body.

import { useQuery } from "@tanstack/react-query";
import { Globe } from "lucide-react";
import { Fragment, useState, type FormEvent, type ReactNode } from "react";
import { useSearchParams } from "react-router";

import { api, errorMessage } from "@/api/client";
import { ErrorState } from "@/components/domain/common";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { LANGS, useLang } from "@/i18n/lang";
import { cn } from "@/lib/utils";

import { useAuthMessages } from "./messages";

const MIN_PASSWORD = 12;

/** Only same-origin paths are accepted as a return target (no open redirect; backslashes refused). */
function safeNext(next: string | null): string {
  if (!next || !next.startsWith("/") || next.includes("\\")) return "/";
  try {
    const url = new URL(next, window.location.origin);
    return url.origin === window.location.origin ? url.pathname + url.search + url.hash : "/";
  } catch {
    return "/";
  }
}

/** Interface language before sign-in (remembered in this browser; the account keeps its own). */
function LangSwitch() {
  const m = useAuthMessages();
  const { lang, setLang } = useLang();
  return (
    <div role="group" aria-label={m.language} className="flex items-center justify-center gap-1 text-xs text-muted-foreground">
      <Globe className="mr-0.5 size-3.5" aria-hidden />
      {LANGS.map((l, i) => (
        <Fragment key={l.id}>
          {i > 0 && <span aria-hidden>·</span>}
          <button
            type="button"
            lang={l.id}
            aria-pressed={l.id === lang}
            onClick={() => setLang(l.id)}
            className={cn(
              "rounded px-1.5 py-1 underline-offset-4 hover:text-foreground hover:underline focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none",
              l.id === lang && "font-medium text-foreground",
            )}
          >
            {l.label}
          </button>
        </Fragment>
      ))}
    </div>
  );
}

function AuthCard({ title, subtitle, children }: { title: string; subtitle?: ReactNode; children: ReactNode }) {
  return (
    <main className="flex min-h-dvh items-center justify-center bg-background p-4">
      <div className="w-full max-w-sm space-y-4">
        <p className="flex items-center justify-center gap-2 text-lg font-semibold tracking-tight">
          <svg viewBox="0 0 16 16" className="size-5 text-primary" aria-hidden>
            <path d="M8 1.5c3 3 4.5 5.4 4.5 7.3A4.5 4.5 0 0 1 3.5 8.8C3.5 6.9 5 4.5 8 1.5Z" fill="none" stroke="currentColor" strokeWidth="1.5" />
          </svg>
          NicheRS
        </p>
        <div className="space-y-5 rounded-xl border bg-card p-6 shadow-sm">
          <div className="space-y-1">
            <h1 className="text-lg font-semibold">{title}</h1>
            {subtitle && <p className="text-sm text-muted-foreground">{subtitle}</p>}
          </div>
          {children}
        </div>
        <LangSwitch />
      </div>
    </main>
  );
}

function FieldRow({ id, label, hint, ...input }: { id: string; label: string; hint?: string } & React.ComponentProps<"input">) {
  return (
    <div className="space-y-1.5">
      <Label htmlFor={id}>{label}</Label>
      <Input id={id} name={id} aria-describedby={hint ? `${id}-hint` : undefined} {...input} />
      {hint && (
        <p id={`${id}-hint`} className="text-xs text-muted-foreground">
          {hint}
        </p>
      )}
    </div>
  );
}

function FormError({ error }: { error: unknown }) {
  if (!error) return null;
  return (
    <Alert variant="destructive" role="alert">
      <AlertDescription>{errorMessage(error)}</AlertDescription>
    </Alert>
  );
}

/** New password + confirmation, checked before submitting. */
function usePasswordPair() {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const m = useAuthMessages();
  const problem =
    password.length > 0 && [...password].length < MIN_PASSWORD
      ? m.atLeast(MIN_PASSWORD)
      : confirm.length > 0 && confirm !== password
        ? m.differ
        : null;
  const ready = [...password].length >= MIN_PASSWORD && confirm === password;
  const fields = (
    <>
      <FieldRow
        id="password"
        label={m.newPassword}
        type="password"
        autoComplete="new-password"
        required
        value={password}
        onChange={(e) => setPassword(e.target.value)}
        hint={m.newPasswordHint(MIN_PASSWORD)}
      />
      <FieldRow
        id="confirm"
        label={m.repeatPassword}
        type="password"
        autoComplete="new-password"
        required
        value={confirm}
        onChange={(e) => setConfirm(e.target.value)}
        aria-invalid={problem ? true : undefined}
      />
      {problem && <p className="text-sm text-destructive">{problem}</p>}
    </>
  );
  return { password, ready, fields };
}

function useLinkToken() {
  // Read once: the fragment is cleared from the address bar so it does not linger in history.
  const [token] = useState(() => {
    const t = window.location.hash.replace(/^#/, "");
    if (t) window.history.replaceState(null, "", window.location.pathname);
    return t;
  });
  const info = useQuery({
    queryKey: ["link", token],
    queryFn: () => api.linkInfo(token),
    enabled: token.length > 0,
    retry: false,
    staleTime: Infinity,
  });
  return { token, info };
}

function LinkProblem({ error }: { error: unknown }) {
  const m = useAuthMessages();
  return (
    <ErrorState
      error={error}
      title={m.linkProblem}
    />
  );
}

function WrongLink({ kind }: { kind: "invite" | "reset" }) {
  const other = kind === "invite" ? "/invite" : "/reset";
  const m = useAuthMessages();
  return (
    <Alert>
      <AlertTitle>{kind === "invite" ? m.isInvite : m.isReset}</AlertTitle>
      <AlertDescription>
        {m.openExactly(other)}
      </AlertDescription>
    </Alert>
  );
}

function NoToken() {
  const m = useAuthMessages();
  return (
    <Alert>
      <AlertTitle>{m.incomplete}</AlertTitle>
      <AlertDescription>{m.incompleteText}</AlertDescription>
    </Alert>
  );
}

export function LoginPage() {
  const [params] = useSearchParams();
  const m = useAuthMessages();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api.login({ email, password });
      window.location.assign(safeNext(params.get("next")));
    } catch (err) {
      setError(err);
      setBusy(false);
    }
  };
  return (
    <AuthCard title={m.signIn} subtitle={m.signInSubtitle}>
      <form className="space-y-4" onSubmit={(e) => void submit(e)}>
        <FieldRow id="email" label={m.email} type="email" autoComplete="username" required value={email} onChange={(e) => setEmail(e.target.value)} />
        <FieldRow id="current-password" label={m.password} type="password" autoComplete="current-password" required value={password} onChange={(e) => setPassword(e.target.value)} />
        <FormError error={error} />
        <Button type="submit" className="w-full" disabled={busy}>
          {busy ? m.signingIn : m.signIn}
        </Button>
      </form>
      <p className="text-xs text-muted-foreground">
        {m.forgot}
      </p>
    </AuthCard>
  );
}

export function InvitePage() {
  const { token, info } = useLinkToken();
  const m = useAuthMessages();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const pw = usePasswordPair();
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!pw.ready) return;
    setBusy(true);
    setError(null);
    try {
      await api.acceptInvite({ token, name, email, password: pw.password });
      window.location.assign("/");
    } catch (err) {
      setError(err);
      setBusy(false);
    }
  };
  return (
    <AuthCard
      title={m.createTitle}
      subtitle={info.data ? <>{m.invitedTo} <strong className="text-foreground">{info.data.org_name}</strong>.</> : undefined}
    >
      {!token && <NoToken />}
      {info.isPending && token && <p className="text-sm text-muted-foreground">{m.checkingInvite}</p>}
      {info.isError && <LinkProblem error={info.error} />}
      {info.data?.kind === "reset" && <WrongLink kind="reset" />}
      {info.data?.kind === "invite" && (
        <form className="space-y-4" onSubmit={(e) => void submit(e)}>
          <FieldRow id="name" label={m.yourName} autoComplete="name" required value={name} onChange={(e) => setName(e.target.value)} />
          <FieldRow
            id="email"
            label={m.email}
            type="email"
            autoComplete="username"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            hint={m.emailHint}
          />
          {pw.fields}
          <FormError error={error} />
          <Button type="submit" className="w-full" disabled={busy || !pw.ready}>
            {busy ? m.creating : m.create}
          </Button>
        </form>
      )}
    </AuthCard>
  );
}

export function ResetPage() {
  const { token, info } = useLinkToken();
  const m = useAuthMessages();
  const pw = usePasswordPair();
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!pw.ready) return;
    setBusy(true);
    setError(null);
    try {
      await api.resetPassword({ token, password: pw.password });
      window.location.assign("/");
    } catch (err) {
      setError(err);
      setBusy(false);
    }
  };
  return (
    <AuthCard
      title={m.resetTitle}
      subtitle={info.data?.email ? <>{m.resetFor} <strong className="text-foreground">{info.data.email}</strong> ({info.data.org_name}). {m.resetSignOut}</> : undefined}
    >
      {!token && <NoToken />}
      {info.isPending && token && <p className="text-sm text-muted-foreground">{m.checkingLink}</p>}
      {info.isError && <LinkProblem error={info.error} />}
      {info.data?.kind === "invite" && <WrongLink kind="invite" />}
      {info.data?.kind === "reset" && (
        <form className="space-y-4" onSubmit={(e) => void submit(e)}>
          {/* Lets password managers attach the new password to the right account. */}
          <input type="hidden" name="username" autoComplete="username" value={info.data.email ?? ""} />
          {pw.fields}
          <FormError error={error} />
          <Button type="submit" className="w-full" disabled={busy || !pw.ready}>
            {busy ? m.saving : m.save}
          </Button>
        </form>
      )}
    </AuthCard>
  );
}
