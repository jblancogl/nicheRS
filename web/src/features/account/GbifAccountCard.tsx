import { CircleAlert, CircleCheck, CircleHelp } from "lucide-react";
import { useId, useState } from "react";

import type { GbifAccountInfo, GbifSettings } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Confirm, InlineError } from "@/features/admin/shared";
import { formatDateTime } from "@/lib/format";

import { useAccountMessages } from "./messages";

/** GBIF's answer at the last check, in words. */
export function GbifStatus({ info }: { info: GbifAccountInfo }) {
  const m = useAccountMessages();
  const [Icon, text, tone] =
    info.ok === true
      ? [CircleCheck, m.ok, "text-state-success"]
      : info.ok === false
        ? [CircleAlert, m.refused, "text-destructive"]
        : [CircleHelp, m.unknown, "text-muted-foreground"];
  return (
    <p className="flex items-start gap-1.5 text-sm">
      <Icon className={`mt-0.5 size-4 shrink-0 ${tone}`} aria-hidden />
      <span>
        {text}
        {info.checked_at && <span className="num block text-xs text-muted-foreground">{m.checkedAt(formatDateTime(info.checked_at))}</span>}
      </span>
    </p>
  );
}

/**
 * A GBIF account (the user's own, or the organisation's for managers): state, then a form to set
 * or replace it. Saving stores the account sealed on the server and asks GBIF at once whether it
 * accepts it; the password is never shown again.
 */
export function GbifAccountEditor({
  info,
  canStore,
  save,
  check,
  clear,
}: {
  info: GbifAccountInfo | null;
  canStore: boolean;
  save: (b: { username: string; password: string }) => Promise<GbifSettings>;
  check: () => Promise<GbifAccountInfo>;
  clear: () => Promise<undefined>;
}) {
  const m = useAccountMessages();
  const id = useId();
  const [editing, setEditing] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const run = async (f: () => Promise<unknown>) => {
    setBusy(true);
    setError(null);
    try {
      await f();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  if (!canStore) return <p className="text-sm text-muted-foreground">{m.noKey}</p>;
  const form = (
    <form
      className="space-y-3"
      onSubmit={(e) => {
        e.preventDefault();
        void run(async () => {
          await save({ username: username.trim(), password });
          setPassword("");
          setEditing(false);
          await check();
        });
      }}
    >
      <div className="space-y-1.5">
        <Label htmlFor={`${id}-user`}>{m.username}</Label>
        <Input id={`${id}-user`} value={username} onChange={(e) => setUsername(e.target.value)} autoComplete="off" required aria-describedby={`${id}-user-hint`} />
        <p id={`${id}-user-hint`} className="text-xs text-muted-foreground">
          {m.usernameHint}
        </p>
      </div>
      <div className="space-y-1.5">
        <Label htmlFor={`${id}-pw`}>{m.gbifPassword}</Label>
        <Input id={`${id}-pw`} type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoComplete="new-password" required />
      </div>
      <div className="flex flex-wrap gap-2">
        <Button type="submit" disabled={busy || !username.trim() || !password}>
          {m.saveCheck}
        </Button>
        {info && (
          <Button type="button" variant="ghost" onClick={() => setEditing(false)}>
            {m.cancel}
          </Button>
        )}
      </div>
      <p className="text-xs text-muted-foreground">{m.privacy}</p>
    </form>
  );
  return (
    <div className="space-y-3">
      {info && !editing && (
        <div className="space-y-2 rounded-md border p-3">
          {info.username && <p className="font-mono text-sm">{info.username}</p>}
          <GbifStatus info={info} />
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setUsername(info.username ?? "");
                setEditing(true);
              }}
            >
              {m.replace}
            </Button>
            <Button variant="outline" size="sm" disabled={busy} onClick={() => void run(check)}>
              {m.checkAgain}
            </Button>
            <Confirm label={m.remove} title={m.removeTitle} description={m.removeText} action={m.removeAction} destructive onConfirm={() => void run(clear)} />
          </div>
        </div>
      )}
      {(!info || editing) && form}
      <InlineError error={error} />
      {!info && (
        <a href="https://www.gbif.org/user/profile" target="_blank" rel="noreferrer" className="inline-block text-sm text-primary underline-offset-4 hover:underline">
          {m.create}
        </a>
      )}
    </div>
  );
}
