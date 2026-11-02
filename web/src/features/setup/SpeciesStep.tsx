import { FlaskConical, Search, X } from "lucide-react";
import { useId, useState } from "react";

import { useSpeciesSearch } from "@/api/hooks";
import type { SpeciesHit } from "@/api/types";
import { ErrorState, SciName, Section } from "@/components/domain/common";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { DEMO_SECOND_SPECIES } from "@/lib/geo";
import { useDebounced } from "@/lib/useDebounced";
import { cn } from "@/lib/utils";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { useDraft } from "@/stores/draft";

import { useSpeciesMessages } from "./messages";
import { usePathMessages } from "./pathMessages";
import { WizardFrame } from "./WizardFrame";

const isAccepted = (h: { status?: string }) => (h.status ?? "").toUpperCase() === "ACCEPTED";

function HitMeta({ hit }: { hit: SpeciesHit }) {
  return (
    <span className="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-xs text-muted-foreground">
      <span className="lowercase">{hit.rank}</span>
      {hit.family && <span>{hit.family}</span>}
      {hit.vernacular_name && <span className="not-italic">“{hit.vernacular_name}”</span>}
      <span className="font-mono">GBIF {hit.key}</span>
    </span>
  );
}

function StatusTag({ status }: { status?: string }) {
  const m = useSpeciesMessages();
  if (!status) return null;
  const s = status.toUpperCase();
  if (s === "ACCEPTED") return <Badge variant="outline">{m.accepted}</Badge>;
  return (
    <Badge variant="outline" className="border-state-warning text-foreground">
      {s.toLowerCase().replace(/_/g, " ")}
    </Badge>
  );
}

export function SpeciesStep() {
  const species = useDraft((s) => s.species);
  const demo = useDraft((s) => s.demo);
  const add = useDraft((s) => s.addSpecies);
  const remove = useDraft((s) => s.removeSpecies);
  const startDemo = useDraft((s) => s.startDemo);
  const addSecondDemo = useDraft((s) => s.addSecondDemoSpecies);
  const reset = useDraft((s) => s.reset);
  const setStep = useDraft((s) => s.setStep);
  const quick = useDraft((s) => s.path === "quick");
  const m = useSpeciesMessages();
  const p = usePathMessages();

  const [q, setQ] = useState("");
  const [active, setActive] = useState(0);
  const [open, setOpen] = useState(false);
  const term = useDebounced(q, 350);
  const search = useSpeciesSearch(term);
  const listId = useId();
  const hits = search.data ?? [];
  const showList = open && term.trim().length >= 3;

  const choose = (h: SpeciesHit) => {
    add(h);
    setQ("");
    setOpen(false);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!showList || hits.length === 0) {
      if (e.key === "ArrowDown") setOpen(true);
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((a) => Math.min(a + 1, hits.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((a) => Math.max(a - 1, 0));
    } else if (e.key === "Enter") {
      const h = hits[active];
      if (h) {
        e.preventDefault();
        choose(h);
      }
    } else if (e.key === "Escape") {
      setOpen(false);
    }
  };

  return (
    <WizardFrame
      title={m.title}
      subtitle={m.subtitle}
      onNext={() => setStep(1)}
      nextDisabled={species.length === 0}
      nextReason={m.nextReason}
    >
      <div className="space-y-6">
        <div className="space-y-1.5">
          <Label htmlFor="species-search">{m.searchLabel}</Label>
          <div className="relative">
            <Search className="pointer-events-none absolute top-2 left-2.5 size-4 text-muted-foreground" aria-hidden />
            <Input
              id="species-search"
              className="h-9 pl-8"
              placeholder={m.placeholder}
              value={q}
              autoComplete="off"
              role="combobox"
              aria-expanded={showList}
              aria-controls={listId}
              aria-autocomplete="list"
              aria-activedescendant={showList && hits[active] ? `${listId}-${active}` : undefined}
              onChange={(e) => {
                setQ(e.target.value);
                setActive(0);
                setOpen(true);
              }}
              onFocus={() => setOpen(true)}
              onKeyDown={onKeyDown}
            />
          </div>
          <p className="text-xs text-muted-foreground">{m.searchHint}</p>
          {quick && <p className="text-xs text-muted-foreground">{p.oneSpecies}</p>}
          <div aria-live="polite" className="sr-only">
            {showList && search.data ? m.results(hits.length) : ""}
          </div>
          {showList && (
            <div className="rounded-lg border">
              {search.isFetching && hits.length === 0 && <p className="px-3 py-2 text-sm text-muted-foreground">{m.searching}</p>}
              {search.isError && <ErrorState error={search.error} onRetry={() => void search.refetch()} title={m.searchUnavailable} className="border-0" />}
              {search.isSuccess && hits.length === 0 && !search.isFetching && (
                <p className="px-3 py-2 text-sm text-muted-foreground">{m.noMatch(term)}</p>
              )}
              <ul id={listId} role="listbox" aria-label={m.matches} className="max-h-72 overflow-y-auto">
                {hits.map((h, i) => {
                  const selected = species.some((s) => s.key === h.key);
                  return (
                    <li
                      key={h.key}
                      id={`${listId}-${i}`}
                      role="option"
                      aria-selected={i === active}
                      aria-disabled={selected}
                      className={cn(
                        "flex cursor-pointer flex-col gap-0.5 border-b px-3 py-2 last:border-b-0",
                        i === active && "bg-accent",
                        selected && "cursor-default opacity-60",
                      )}
                      onMouseEnter={() => setActive(i)}
                      onMouseDown={(e) => e.preventDefault()}
                      onClick={() => !selected && choose(h)}
                    >
                      <span className="flex items-center justify-between gap-2">
                        <SciName name={h.scientific_name} className="text-sm" />
                        <StatusTag status={h.status} />
                      </span>
                      <HitMeta hit={h} />
                      {!isAccepted(h) && (
                        <span className="text-xs text-state-warning">
                          {m.notAccepted}
                        </span>
                      )}
                      {selected && <span className="text-xs">{m.alreadySelected}</span>}
                    </li>
                  );
                })}
              </ul>
            </div>
          )}
        </div>

        <Section title={m.selected(species.length)} id="selected-species">
          {species.length === 0 ? (
            <p className="text-sm text-muted-foreground">{m.noneSelected}</p>
          ) : (
            <ul className="divide-y rounded-lg border">
              {species.map((s) => (
                <li key={s.key} className="flex items-start justify-between gap-2 px-3 py-2">
                  <div className="min-w-0 space-y-0.5">
                    <SciName name={s.scientific_name} className="text-sm" />
                    <div className="flex flex-wrap gap-x-2 text-xs text-muted-foreground">
                      {s.rank && <span className="lowercase">{s.rank}</span>}
                      {s.family && <span>{s.family}</span>}
                      {s.status && !demo && <span className="lowercase">{s.status}</span>}
                      {demo ? <span>{m.syntheticSpecies}</span> : <span className="font-mono">GBIF {s.key}</span>}
                    </div>
                  </div>
                  <Button variant="ghost" size="icon-sm" onClick={() => remove(s.key)} aria-label={m.remove(s.scientific_name)}>
                    <X aria-hidden />
                  </Button>
                </li>
              ))}
            </ul>
          )}
          {species.length > 1 && (
            <p className="text-sm text-muted-foreground">
              <span className="num">{species.length}</span>
              {m.batch}
            </p>
          )}
        </Section>

        {demo ? (
          <div className="rounded-lg border border-state-warning/40 bg-state-warning/5 p-3">
            <div className="flex items-start gap-3">
              <FlaskConical className="mt-0.5 size-4 shrink-0 text-state-warning" aria-hidden />
              <div className="space-y-2">
                <p className="text-sm font-medium">{m.inDemo}</p>
                <p className="text-sm text-muted-foreground">
                  <i lang="la">Virtualis demonstrandum</i>
                  {m.inDemoText}
                </p>
                <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
                  {!quick && !species.some((s) => s.key === DEMO_SECOND_SPECIES.key) && (
                    <Button variant="outline" size="sm" onClick={addSecondDemo}>
                      {m.addSecondDemo}
                    </Button>
                  )}
                  <Button variant="outline" size="sm" onClick={() => reset(useDraft.getState().projectId)}>
                    {m.leaveDemo}
                  </Button>
                  <HelpTrigger target={{ article: "getting-started", section: "demo" }} text="demoHelp" />
                </div>
              </div>
            </div>
          </div>
        ) : (
          <div className="rounded-lg border border-dashed p-3">
            <div className="flex items-start gap-3">
              <FlaskConical className="mt-0.5 size-4 shrink-0 text-state-warning" aria-hidden />
              <div className="space-y-2">
                <p className="text-sm font-medium">{m.tryTitle}</p>
                <p className="text-sm text-muted-foreground">
                  {m.tryA}
                  <i lang="la">Virtualis demonstrandum</i>
                  {m.tryB}
                  <strong>{m.tryStrong}</strong>
                  {m.tryC}
                </p>
                <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
                  <Button variant="outline" size="sm" onClick={startDemo}>
                    {m.tryButton}
                  </Button>
                  <HelpTrigger target={{ article: "getting-started", section: "demo" }} text="demoHelp" />
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </WizardFrame>
  );
}
