// "My account" sections (ADR-0110): profile and language, password, own GBIF account. Listed in
// @/features/settings/sections.tsx.

import { useId, useState } from "react";

import { api } from "@/api/client";
import { useAccountGbif, useAccountMutation, useChooseLang } from "@/api/hooks";
import { ErrorState } from "@/components/domain/common";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { InlineError, Loading } from "@/features/admin/shared";
import { Card } from "@/features/settings/Card";
import type { SectionProps } from "@/features/settings/sections";
import { LANGS, useLang, type Lang } from "@/i18n/lang";

import { GbifAccountEditor } from "./GbifAccountCard";
import { useAccountMessages } from "./messages";

export function ProfileSection({ me }: SectionProps) {
  const { name, email, organisation } = me;
  const m = useAccountMessages();
  const id = useId();
  const [value, setValue] = useState(name);
  const rename = useAccountMutation((n: string) => api.account.rename(n));
  const lang = useLang((s) => s.lang);
  const choose = useChooseLang();
  const changed = value.trim() !== name && value.trim() !== "";
  return (
    <Card className="max-w-xl">
      <form
        className="space-y-1.5"
        onSubmit={(e) => {
          e.preventDefault();
          if (changed) rename.mutate(value.trim());
        }}
      >
        <Label htmlFor={`${id}-name`}>{m.name}</Label>
        <div className="flex gap-2">
          <Input id={`${id}-name`} value={value} onChange={(e) => setValue(e.target.value)} maxLength={120} aria-describedby={`${id}-name-hint`} />
          <Button type="submit" variant="outline" disabled={!changed || rename.isPending}>
            {m.save}
          </Button>
        </div>
        <p id={`${id}-name-hint`} className="text-xs text-muted-foreground">
          {rename.isSuccess && !changed ? m.saved : m.nameHint}
        </p>
        <InlineError error={rename.error} />
      </form>
      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        <dt className="text-muted-foreground">{m.email}</dt>
        <dd className="min-w-0 truncate">{email}</dd>
        <dt className="text-muted-foreground">{m.organisation}</dt>
        <dd>{organisation}</dd>
      </dl>
      <div className="space-y-1.5">
        <Label htmlFor={`${id}-lang`}>{m.language}</Label>
        <select
          id={`${id}-lang`}
          value={lang}
          onChange={(e) => choose(e.target.value as Lang)}
          className="h-9 w-full max-w-xs rounded-md border border-input bg-transparent px-2 text-sm shadow-xs focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none"
        >
          {LANGS.map((l) => (
            <option key={l.id} value={l.id}>
              {l.label}
            </option>
          ))}
        </select>
      </div>
    </Card>
  );
}

export function SecuritySection() {
  const m = useAccountMessages();
  const id = useId();
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [repeat, setRepeat] = useState("");
  const change = useAccountMutation((b: { current: string; new: string }) => api.account.changePassword(b));
  const mismatch = repeat !== "" && next !== repeat;
  return (
    <Card className="max-w-xl">
      <form
        className="space-y-3"
        onSubmit={(e) => {
          e.preventDefault();
          if (mismatch) return;
          change.mutate(
            { current, new: next },
            {
              onSuccess: () => {
                setCurrent("");
                setNext("");
                setRepeat("");
              },
            },
          );
        }}
      >
        <div className="space-y-1.5">
          <Label htmlFor={`${id}-cur`}>{m.current}</Label>
          <Input id={`${id}-cur`} type="password" autoComplete="current-password" value={current} onChange={(e) => setCurrent(e.target.value)} required />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor={`${id}-new`}>{m.newPassword}</Label>
          <Input id={`${id}-new`} type="password" autoComplete="new-password" minLength={12} value={next} onChange={(e) => setNext(e.target.value)} required aria-describedby={`${id}-hint`} />
          <p id={`${id}-hint`} className="text-xs text-muted-foreground">
            {m.passwordHint}
          </p>
        </div>
        <div className="space-y-1.5">
          <Label htmlFor={`${id}-rep`}>{m.repeat}</Label>
          <Input id={`${id}-rep`} type="password" autoComplete="new-password" value={repeat} onChange={(e) => setRepeat(e.target.value)} required aria-invalid={mismatch} />
          {mismatch && <p className="text-sm text-destructive">{m.mismatch}</p>}
        </div>
        <Button type="submit" variant="outline" disabled={change.isPending || mismatch || !current || !next}>
          {m.changePassword}
        </Button>
        {change.isSuccess && (
          <p className="text-sm" role="status">
            {m.passwordChanged}
          </p>
        )}
        <InlineError error={change.error} />
      </form>
    </Card>
  );
}

export function GbifSection() {
  const m = useAccountMessages();
  const settings = useAccountGbif(true);
  const save = useAccountMutation((b: { username: string; password: string }) => api.account.setGbif(b));
  const check = useAccountMutation(() => api.account.checkGbif());
  const clear = useAccountMutation(() => api.account.clearGbif());
  const s = settings.data;
  return (
    <Card className="max-w-xl">
      <p className="text-sm text-muted-foreground">{m.gbifIntro}</p>
      {settings.isPending && <Loading />}
      {settings.isError && <ErrorState error={settings.error} onRetry={() => void settings.refetch()} />}
      {s && (
        <>
          <p className="rounded-md bg-muted px-3 py-2 text-sm" role="status">
            {m.effective(s.effective)}
          </p>
          <p className="text-xs text-muted-foreground">{m.order}</p>
          <h3 className="pt-1 text-sm font-medium">{m.yours}</h3>
          {!s.own && <p className="text-sm text-muted-foreground">{m.notSet}</p>}
          <GbifAccountEditor
            info={s.own}
            canStore={s.can_store}
            save={(b) => save.mutateAsync(b)}
            check={() => check.mutateAsync(undefined)}
            clear={() => clear.mutateAsync(undefined)}
          />
        </>
      )}
    </Card>
  );
}
