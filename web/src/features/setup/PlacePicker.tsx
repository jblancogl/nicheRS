// Study area by place: search the administrative units of the installed boundary packs
// (municipality → country), pick one or several; the area is their union (a MultiPolygon),
// recorded with the place ids, pack and attribution for traceability.

import { Check, MapPin, Plus, Search, Trees, X } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useId, useState } from "react";

import { api, errorMessage } from "@/api/client";
import type { Geometry, PlaceHit, PlaceRef, StudyArea } from "@/api/types";
import { Button } from "@/components/ui/button";
import { formatArea } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useDraft } from "@/stores/draft";
import { useMapStore } from "@/stores/map";

import { usePlaceMessages } from "./placeMessages";

type Polygons = number[][][][];

/** Frequent study areas (official names; ids of the fr-es administrative pack). */
const SUGGESTIONS = [
  { id: "FR-METRO", name: "France métropolitaine" },
  { id: "ES-PENIN", name: "España peninsular y Baleares" },
  { id: "FR-R76", name: "Occitanie" },
  { id: "FR-R53", name: "Bretagne" },
  { id: "ES-3401", name: "Andalucía" },
];

function polygonsOf(g: Geometry): Polygons {
  if (g.type === "MultiPolygon") return g.coordinates;
  if (g.type === "Polygon") return [g.coordinates];
  return [];
}

/** The study area of the chosen places: their names, and the union of their polygons. */
function compose(refs: PlaceRef[], polys: Polygons): StudyArea | null {
  if (refs.length === 0) return null;
  const names = refs.map((r) => r.name);
  const name = names.length <= 3 ? names.join(" + ") : `${names.slice(0, 3).join(" + ")} + ${names.length - 3}`;
  return { name, geometry: { type: "MultiPolygon", coordinates: polys } as Geometry, places: refs };
}

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const t = window.setTimeout(() => setV(value), ms);
    return () => window.clearTimeout(t);
  }, [value, ms]);
  return v;
}

export function PlacePicker() {
  const m = usePlaceMessages();
  const area = useDraft((s) => s.area);
  const source = useDraft((s) => s.areaSource);
  const setArea = useDraft((s) => s.setArea);
  const fitTo = useMapStore((s) => s.fitTo);
  const setPick = useMapStore((s) => s.setPick);
  const [point, setPoint] = useState<{ lng: number; lat: number } | null>(null);
  const listId = useId();
  const [q, setQ] = useState("");
  const [active, setActive] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const term = useDebounced(q.trim(), 200);

  const chosen: PlaceRef[] = source === "place" ? (area?.places ?? []) : [];

  const found = useQuery({
    queryKey: ["places", term],
    queryFn: ({ signal }) => api.places(term, signal),
    enabled: term.length >= 2,
    staleTime: 10 * 60_000,
  });
  // While this picker is shown, a click on the map lists the places containing that point.
  useEffect(() => {
    setPick((lng, lat) => setPoint({ lng, lat }));
    return () => setPick(null);
  }, [setPick]);
  const atPoint = useQuery({
    queryKey: ["places-at", point?.lng, point?.lat],
    queryFn: ({ signal }) => api.placesAt(point!.lng, point!.lat, signal),
    enabled: point !== null,
    staleTime: 10 * 60_000,
  });
  const kind = (h: PlaceHit) => h.designation ?? m.level(h.country, h.level);

  // Typing a new query hides the results of the previous one until they are replaced.
  const hits: PlaceHit[] = term.length >= 2 && q.trim() === term ? (found.data ?? []) : [];

  // Rebuild the union from the server (the draft keeps the places, not every boundary apart).
  const rebuild = async (refs: PlaceRef[]) => {
    const parts = await Promise.all(refs.map(async (ref) => polygonsOf((await api.place(ref.id)).geometry)));
    setArea(compose(refs, parts.flat()), refs.length ? "place" : null);
  };

  const add = async (h: PlaceHit) => {
    if (chosen.some((c) => c.id === h.id)) return;
    setBusy(true);
    setError(null);
    try {
      const g = await api.place(h.id);
      const ref: PlaceRef = { id: h.id, name: h.name, pack: h.pack, attribution: h.attribution };
      // The current geometry already holds every chosen place: append the new one to it.
      const current = chosen.length && area ? polygonsOf(area.geometry) : [];
      setArea(compose([...chosen, ref], [...current, ...polygonsOf(g.geometry)]), "place");
      fitTo(h.bbox);
      setQ("");
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  // Shortcuts to frequent study areas, with their official boundaries (ids of the installed packs).
  const addById = async (id: string) => {
    setBusy(true);
    setError(null);
    try {
      const g = await api.place(id);
      setBusy(false);
      await add(g.place);
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
    }
  };

  const remove = async (id: string) => {
    setBusy(true);
    try {
      await rebuild(chosen.filter((c) => c.id !== id));
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const onKey = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((i) => Math.min(i + 1, hits.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && hits[active]) {
      e.preventDefault();
      void add(hits[active]);
    } else if (e.key === "Escape") {
      setQ("");
    }
  };

  const credits = [...new Set(chosen.map((c) => c.attribution).filter(Boolean))];
  const open = hits.length > 0 || (found.isSuccess && term.length >= 2 && q.trim() === term);

  return (
    <div className="space-y-3 text-sm">
      <div className="space-y-1.5">
        <label htmlFor={`${listId}-input`} className="text-sm font-medium">
          {m.search}
        </label>
        <div className="relative">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden />
          <input
            id={`${listId}-input`}
            role="combobox"
            aria-expanded={open}
            aria-controls={listId}
            aria-autocomplete="list"
            aria-activedescendant={hits.length ? `${listId}-${active}` : undefined}
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              setActive(0);
            }}
            onKeyDown={onKey}
            placeholder={m.placeholder}
            autoComplete="off"
            className="h-9 w-full rounded-md border border-input bg-transparent pr-3 pl-8 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40"
          />
        </div>
        {open && (
          <ul id={listId} role="listbox" className="max-h-72 overflow-y-auto rounded-md border bg-popover p-1 shadow-md">
            {hits.length === 0 && <li className="px-2 py-1.5 text-muted-foreground">{m.none(term)}</li>}
            {hits.map((h, i) => (
              <li
                key={h.id}
                id={`${listId}-${i}`}
                role="option"
                aria-selected={i === active}
                onMouseEnter={() => setActive(i)}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => void add(h)}
                className={cn("flex cursor-pointer items-start gap-2 rounded px-2 py-1.5", i === active && "bg-accent")}
              >
                {h.level === "protected" ? (
                  <Trees className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />
                ) : (
                  <MapPin className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />
                )}
                <span className="min-w-0 flex-1">
                  <span className="block truncate font-medium">{h.name}</span>
                  <span className="block truncate text-xs text-muted-foreground">
                    {kind(h)} · {[...h.within.slice(0, 2), h.within.length ? null : m.countryName(h.country)].filter(Boolean).join(" › ")}
                  </span>
                </span>
                <span className="num shrink-0 text-xs text-muted-foreground">{formatArea(h.area_km2)}</span>
              </li>
            ))}
          </ul>
        )}
        <p className="text-xs text-muted-foreground">{m.hint}</p>
      </div>

      {chosen.length === 0 && q.trim() === "" && !point && (
        <div className="space-y-1.5">
          <p className="text-xs font-medium text-muted-foreground">{m.suggestions}</p>
          <ul className="flex flex-wrap gap-1.5">
            {SUGGESTIONS.map((s) => (
              <li key={s.id}>
                <Button variant="outline" size="xs" disabled={busy} onClick={() => void addById(s.id)}>
                  <Plus aria-hidden /> {s.name}
                </Button>
              </li>
            ))}
          </ul>
        </div>
      )}

      {point && (
        <section aria-labelledby={`${listId}-pt`} className="space-y-1.5 rounded-md border p-2">
          <div className="flex items-center justify-between gap-2">
            <p id={`${listId}-pt`} className="text-xs font-medium text-muted-foreground">
              {m.atPoint} <span className="num">({point.lat.toFixed(3)}, {point.lng.toFixed(3)})</span>
            </p>
            <Button variant="ghost" size="icon-xs" aria-label={m.closePoint} onClick={() => setPoint(null)}>
              <X aria-hidden />
            </Button>
          </div>
          {atPoint.isPending && <p className="text-xs text-muted-foreground">{m.loading}</p>}
          {atPoint.isError && <p className="text-xs text-destructive">{errorMessage(atPoint.error)}</p>}
          {atPoint.data?.length === 0 && <p className="text-xs text-muted-foreground">{m.atPointNone}</p>}
          {atPoint.data && atPoint.data.length > 0 && (
            <ul className="space-y-0.5">
              {atPoint.data.map((h, i, all) => {
                const has = chosen.some((c) => c.id === h.id);
                const firstProtected = h.level === "protected" && all[i - 1]?.level !== "protected";
                return (
                  <li key={h.id}>
                    {firstProtected && <p className="mt-1.5 px-1 text-[0.7rem] font-medium tracking-wide text-muted-foreground uppercase">{m.protectedAreas}</p>}
                    <button
                      type="button"
                      disabled={has || busy}
                      onClick={() => void add(h)}
                      aria-label={has ? `${h.name} — ${m.added}` : m.add(h.name)}
                      className="flex w-full items-start gap-2 rounded px-1.5 py-1 text-left hover:bg-accent disabled:cursor-default disabled:opacity-60 disabled:hover:bg-transparent"
                    >
                      {h.level === "protected" ? (
                        <Trees className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />
                      ) : (
                        <MapPin className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />
                      )}
                      <span className="min-w-0 flex-1">
                        <span className="block truncate">{h.name}</span>
                        <span className="block truncate text-xs text-muted-foreground">{kind(h)}</span>
                      </span>
                      <span className="num shrink-0 text-xs text-muted-foreground">{formatArea(h.area_km2)}</span>
                      {has ? <Check className="mt-0.5 size-4 shrink-0 text-primary" aria-hidden /> : <Plus className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />}
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </section>
      )}

      {busy && <p className="text-xs text-muted-foreground">{m.loading}</p>}
      {error && <p className="text-sm text-destructive">{error}</p>}

      {chosen.length > 0 && (
        <div className="space-y-2">
          <div className="flex items-center justify-between gap-2">
            <p className="text-xs font-medium text-muted-foreground">{m.selected(chosen.length)}</p>
            {chosen.length > 1 && (
              <Button variant="ghost" size="xs" onClick={() => setArea(null, null)}>
                {m.clear}
              </Button>
            )}
          </div>
          <ul className="flex flex-wrap gap-1.5">
            {chosen.map((c) => (
              <li key={c.id} className="inline-flex items-center gap-1 rounded-full border bg-muted/50 py-0.5 pr-1 pl-2.5 text-sm">
                {c.name}
                <Button variant="ghost" size="icon-xs" aria-label={m.remove(c.name)} disabled={busy} onClick={() => void remove(c.id)}>
                  <X aria-hidden />
                </Button>
              </li>
            ))}
          </ul>
          {credits.length > 0 && <p className="text-xs text-muted-foreground">{m.boundaries(credits.join(" · "))}</p>}
        </div>
      )}
    </div>
  );
}
