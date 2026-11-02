// Global search in the centre of the top bar (Ctrl/⌘ K or "/"): studies, projects, GBIF species
// (to start a study with one), help articles and actions, in one keyboard-driven list
// (WAI-ARIA combobox + listbox).

import { BookOpen, FolderOpen, Languages, Leaf, Map as MapIcon, Plus, Search, ShieldCheck, X } from "lucide-react";
import { useEffect, useId, useMemo, useRef, useState, type ReactNode } from "react";
import { useNavigate } from "react-router";

import { useChooseLang, useJobs, useMe, useProjects, useSpeciesSearch } from "@/api/hooks";
import type { Job } from "@/api/types";
import { SciName, SyntheticBadge } from "@/components/domain/common";
import { EvidenceBadge, JobStatusBadge } from "@/components/domain/status";
import { Button } from "@/components/ui/button";
import { fold, searchHelp } from "@/features/help/search";
import { preloadHelp, useHelp, useHelpContent } from "@/features/help/store";
import { LANGS, useLang } from "@/i18n/lang";
import { cn } from "@/lib/utils";
import { useDraft } from "@/stores/draft";

import { useSearchMessages } from "./messages";

interface Option {
  id: string;
  group: string;
  icon: ReactNode;
  label: ReactNode;
  meta?: ReactNode;
  run: () => void;
}

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const t = window.setTimeout(() => setV(value), ms);
    return () => window.clearTimeout(t);
  }, [value, ms]);
  return v;
}

const studyHref = (j: Job) => (j.status === "succeeded" ? `/jobs/${j.id}/results` : `/jobs/${j.id}`);

/** Every word of the query appears somewhere in the haystack (accent- and case-insensitive). */
function matches(words: string[], ...fields: (string | null | undefined)[]): boolean {
  const hay = fold(fields.filter(Boolean).join(" "));
  return words.every((w) => hay.includes(w));
}

export function GlobalSearch() {
  const m = useSearchMessages();
  const navigate = useNavigate();
  const jobs = useJobs();
  const projects = useProjects();
  const me = useMe();
  const lang = useLang((s) => s.lang);
  const choose = useChooseLang();
  const help = useHelpContent();
  const showHelp = useHelp((s) => s.show);
  const reset = useDraft((s) => s.reset);
  const addSpecies = useDraft((s) => s.addSpecies);
  const setStep = useDraft((s) => s.setStep);

  const [q, setQ] = useState("");
  const [open, setOpen] = useState(false);
  // Phones: the search is an icon that expands over the whole top bar.
  const [expanded, setExpanded] = useState(false);
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const root = useRef<HTMLDivElement>(null);
  const listId = useId();
  const term = q.trim();
  const words = useMemo(() => fold(term).split(/\s+/).filter(Boolean), [term]);
  const species = useSpeciesSearch(useDebounced(term, 250));

  // Ctrl/⌘ K anywhere, or "/" when not typing in a field, opens the search.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLElement && e.target.closest("input, textarea, select, [contenteditable]");
      if ((e.key === "k" && (e.metaKey || e.ctrlKey)) || (e.key === "/" && !typing)) {
        e.preventDefault();
        setExpanded(true);
        setOpen(true);
        window.setTimeout(() => input.current?.focus(), 0);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    if (open) preloadHelp(lang);
  }, [open, lang]);

  const close = () => {
    setOpen(false);
    setExpanded(false);
    setQ("");
    input.current?.blur();
  };
  const execute = (o: Option) => {
    o.run();
    close();
  };

  const options = useMemo<Option[]>(() => {
    const out: Option[] = [];
    const all = [...(jobs.data ?? [])].sort((a, b) => b.created_at.localeCompare(a.created_at));
    const plist = projects.data ?? [];
    const projectName = (id?: string | null) => plist.find((p) => p.id === id)?.name;
    const studies = words.length
      ? all.filter((j) =>
          matches(words, j.name, j.spec.area.name, projectName(j.project_id), j.created_by_name, ...j.spec.species.map((s) => s.scientific_name)),
        )
      : all;
    for (const j of studies.slice(0, words.length ? 6 : 4)) {
      const fitted = j.species.find((s) => s.status === "succeeded" && s.evidence_level);
      const first = j.spec.species[0];
      out.push({
        id: `job-${j.id}`,
        group: words.length ? m.studies : m.recent,
        icon: <MapIcon className="size-4" aria-hidden />,
        label: j.name,
        meta: (
          <span className="flex min-w-0 items-center gap-1.5">
            {j.status === "succeeded" && fitted?.evidence_level ? <EvidenceBadge level={fitted.evidence_level} /> : <JobStatusBadge status={j.status} />}
            {/* The SYNTHETIC badge stays outside the truncated part: it must always be visible. */}
            {j.spec.demo && <SyntheticBadge className="shrink-0" />}
            {first && (
              <i lang="la" className="min-w-0 truncate">
                {first.scientific_name.replace(/\s*\(SYNTHETIC\)\s*/, "")}
              </i>
            )}
            <span className="truncate">· {j.spec.area.name}</span>
            {projectName(j.project_id) && <span className="truncate">· {projectName(j.project_id)}</span>}
          </span>
        ),
        run: () => void navigate(studyHref(j)),
      });
    }
    if (words.length) {
      for (const p of plist.filter((p) => matches(words, p.name)).slice(0, 4)) {
        out.push({
          id: `project-${p.id}`,
          group: m.projects,
          icon: <FolderOpen className="size-4" aria-hidden />,
          label: p.name,
          meta: m.projectStudies(p.studies),
          run: () => void navigate(`/?project=${encodeURIComponent(p.id)}`),
        });
      }
      // Species first: a genus or family can be modelled, but with a warning (pooled taxa).
      const taxa = [...(species.data ?? [])].sort((a, b) => Number(b.rank === "SPECIES") - Number(a.rank === "SPECIES"));
      for (const h of taxa.slice(0, 5)) {
        out.push({
          id: `species-${h.key}`,
          group: m.species,
          icon: <Leaf className="size-4" aria-hidden />,
          label: <SciName name={h.scientific_name} />,
          meta: [h.vernacular_name, h.family, h.rank.toLowerCase()].filter(Boolean).join(" · "),
          run: () => {
            reset(null);
            addSpecies(h);
            setStep(1);
            void navigate("/new");
          },
        });
      }
      if (help) {
        // Sections whose title holds the query first, then those that only mention it.
        const inTitle = (h: { sectionTitle: string | null; articleTitle: string }) => matches(words, h.sectionTitle ?? h.articleTitle);
        const helpHits = searchHelp(help, term).sort((a, b) => Number(inTitle(b)) - Number(inTitle(a)));
        for (const h of helpHits.slice(0, 4)) {
          out.push({
            id: `help-${h.article}-${h.section ?? ""}`,
            group: m.help,
            icon: <BookOpen className="size-4" aria-hidden />,
            label: h.sectionTitle ?? h.articleTitle,
            meta: h.sectionTitle ? h.articleTitle : undefined,
            run: () => {
              showHelp();
              useHelp.getState().goTo(h.article, h.section);
            },
          });
        }
      }
    }
    const actions: Option[] = [
      {
        id: "a-new-study",
        group: m.actions,
        icon: <Plus className="size-4" aria-hidden />,
        label: m.newStudy,
        run: () => {
          reset(null);
          void navigate("/new");
        },
      },
      {
        id: "a-new-project",
        group: m.actions,
        icon: <Plus className="size-4" aria-hidden />,
        label: m.newProject,
        run: () => void navigate("/?newProject=1"),
      },
      {
        id: "a-help",
        group: m.actions,
        icon: <BookOpen className="size-4" aria-hidden />,
        label: m.helpCentre,
        run: () => showHelp(),
      },
      ...LANGS.filter((l) => l.id !== lang).map((l) => ({
        id: `a-lang-${l.id}`,
        group: m.actions,
        icon: <Languages className="size-4" aria-hidden />,
        label: m.language(l.label),
        run: () => choose(l.id),
      })),
      ...(me.data?.is_admin
        ? [
            {
              id: "a-admin",
              group: m.actions,
              icon: <ShieldCheck className="size-4" aria-hidden />,
              label: m.administration,
              run: () => void navigate("/admin"),
            },
          ]
        : []),
    ];
    out.push(...(words.length ? actions.filter((a) => matches(words, typeof a.label === "string" ? a.label : "")) : actions));
    return out;
  }, [jobs.data, projects.data, words, term, species.data, help, lang, me.data, m, navigate, reset, addSpecies, setStep, showHelp, choose]);

  const current = Math.min(active, Math.max(0, options.length - 1));
  const optionId = (i: number) => `${listId}-${i}`;

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setOpen(true);
      setActive((i) => Math.min(i + 1, options.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (options[current]) execute(options[current]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  };

  // Close when focus or a click leaves the search.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (root.current && !root.current.contains(e.target as Node)) {
        setOpen(false);
        setExpanded(false);
      }
    };
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
  }, [open]);

  const groups: [string, { o: Option; i: number }[]][] = [];
  options.forEach((o, i) => {
    const g = groups.find(([name]) => name === o.group);
    if (g) g[1].push({ o, i });
    else groups.push([o.group, [{ o, i }]]);
  });
  const waitingGbif = term.length >= 3 && species.isFetching;

  return (
    <div ref={root} className="relative flex w-full max-w-xl justify-end lg:block" data-global-search>
      <Button
        variant="ghost"
        size="icon-sm"
        className={cn("lg:hidden", expanded && "hidden")}
        aria-label={m.label}
        onClick={() => {
          setExpanded(true);
          setOpen(true);
          window.setTimeout(() => input.current?.focus(), 0);
        }}
      >
        <Search aria-hidden />
      </Button>
      <div
        className={cn(
          "lg:relative lg:block",
          expanded ? "fixed inset-x-0 top-0 z-50 flex h-11 items-center border-b bg-card px-3 lg:static lg:h-auto lg:border-0 lg:p-0" : "hidden",
        )}
      >
        <div className="relative w-full">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden />
          <input
            ref={input}
            type="search"
            role="combobox"
            aria-label={m.label}
            aria-expanded={open}
            aria-controls={listId}
            aria-autocomplete="list"
            aria-activedescendant={open && options.length ? optionId(current) : undefined}
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              setActive(0);
              setOpen(true);
            }}
            onFocus={() => setOpen(true)}
            onKeyDown={onKeyDown}
            placeholder={m.placeholder}
            className="h-8 w-full rounded-md border border-input bg-muted/40 pr-16 pl-8 text-sm outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:bg-background focus-visible:ring-2 focus-visible:ring-ring/40 [&::-webkit-search-cancel-button]:hidden"
          />
          {q ? (
            <Button variant="ghost" size="icon-xs" className="absolute top-1/2 right-1.5 -translate-y-1/2" aria-label={m.clear} onClick={() => setQ("")}>
              <X aria-hidden />
            </Button>
          ) : (
            <kbd className="pointer-events-none absolute top-1/2 right-2 hidden -translate-y-1/2 rounded border bg-background px-1.5 font-mono text-[0.65rem] text-muted-foreground sm:block">
              Ctrl K
            </kbd>
          )}
        </div>
        {open && (
          <div className="absolute top-full right-0 left-0 z-50 mt-1.5 min-w-[min(36rem,calc(100vw-1.5rem))] overflow-hidden rounded-lg border bg-popover text-popover-foreground shadow-lg lg:left-1/2 lg:-translate-x-1/2">
            <ul id={listId} role="listbox" aria-label={m.label} className="max-h-[min(70vh,34rem)] overflow-y-auto p-1">
              {groups.map(([name, items]) => (
                <li key={name} role="presentation">
                  <p className="px-2 pt-2 pb-1 text-[0.7rem] font-semibold tracking-wide text-muted-foreground uppercase" aria-hidden>
                    {name}
                  </p>
                  <ul role="group" aria-label={name}>
                    {items.map(({ o, i }) => (
                      <li
                        key={o.id}
                        id={optionId(i)}
                        role="option"
                        aria-selected={i === current}
                        onMouseEnter={() => setActive(i)}
                        onMouseDown={(e) => e.preventDefault()}
                        onClick={() => execute(o)}
                        className={cn(
                          "flex cursor-pointer items-center gap-2.5 rounded-md px-2 py-1.5 text-sm",
                          i === current ? "bg-accent text-accent-foreground" : "",
                        )}
                      >
                        <span className="shrink-0 text-muted-foreground">{o.icon}</span>
                        <span className="min-w-0 flex-1">
                          <span className="block truncate">{o.label}</span>
                          {o.meta && <span className="block truncate text-xs text-muted-foreground">{o.meta}</span>}
                        </span>
                      </li>
                    ))}
                  </ul>
                </li>
              ))}
            </ul>
            <div className="flex items-center justify-between gap-2 border-t px-3 py-1.5 text-[0.7rem] text-muted-foreground">
              <span>
                {waitingGbif
                  ? m.searchingGbif
                  : term && options.every((o) => o.group === m.actions)
                    ? m.noResults(term)
                    : term.length > 0 && term.length < 3
                      ? m.typeMore
                      : null}
              </span>
              <span className="hidden sm:inline">{m.hint}</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
