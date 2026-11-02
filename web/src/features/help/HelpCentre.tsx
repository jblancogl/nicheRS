// Help centre body, shared by the slide-over panel and the /help page: language switch, search,
// article index, article view with table of contents, previous/next, review footer.

import { ChevronLeft, ChevronRight, Info, Search, TriangleAlert } from "lucide-react";
import { Fragment, useEffect, useRef, useState, type ReactNode } from "react";

import { useChooseLang } from "@/api/hooks";
import { Input } from "@/components/ui/input";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { cn } from "@/lib/utils";

import { ARTICLE_IDS, LANGS, isArticleId, sectionDomId, type ArticleId, type Block, type HelpLang } from "./content";
import { searchHelp } from "./search";
import { useHelp, useHelpContent } from "./store";

type Navigate = (article: ArticleId | null, section?: string | null) => void;
type HrefFor = (article: ArticleId | null, section?: string | null) => string;

const TOKEN = /(\*\*[^*]+\*\*|\*[^*\s][^*]*\*|`[^`]+`|\[[^\]]+\]\(#[^)\s]+\))/g;

function Rich({ text, onNavigate, hrefFor }: { text: string; onNavigate: Navigate; hrefFor: HrefFor }) {
  const parts = text.split(TOKEN);
  return (
    <>
      {parts.map((part, i) => {
        if (i % 2 === 0) return <Fragment key={i}>{part}</Fragment>;
        if (part.startsWith("**")) return <strong key={i} className="font-semibold">{part.slice(2, -2)}</strong>;
        if (part.startsWith("`")) return <code key={i} className="rounded bg-muted px-1 font-mono text-[0.85em]">{part.slice(1, -1)}</code>;
        if (part.startsWith("[")) {
          const m = /^\[([^\]]+)\]\(#([^/)]+)(?:\/([^)]+))?\)$/.exec(part);
          const label = m?.[1] ?? part;
          const art = m?.[2];
          if (!m || !isArticleId(art)) return <Fragment key={i}>{label}</Fragment>;
          const sec = m[3] ?? null;
          return (
            <a
              key={i}
              href={hrefFor(art, sec)}
              onClick={(e) => {
                e.preventDefault();
                onNavigate(art, sec);
              }}
              className="text-primary underline underline-offset-4 hover:no-underline"
            >
              {label}
            </a>
          );
        }
        return <em key={i}>{part.slice(1, -1)}</em>;
      })}
    </>
  );
}

function Blocks({ blocks, onNavigate, hrefFor }: { blocks: Block[]; onNavigate: Navigate; hrefFor: HrefFor }) {
  const r = (t: string) => <Rich text={t} onNavigate={onNavigate} hrefFor={hrefFor} />;
  return (
    <>
      {blocks.map((b, i) => {
        if ("p" in b) return <p key={i}>{r(b.p)}</p>;
        if ("ul" in b)
          return (
            <ul key={i} className="list-disc space-y-1 pl-5">
              {b.ul.map((x, j) => (
                <li key={j}>{r(x)}</li>
              ))}
            </ul>
          );
        if ("ol" in b)
          return (
            <ol key={i} className="list-decimal space-y-1.5 pl-5">
              {b.ol.map((x, j) => (
                <li key={j}>{r(x)}</li>
              ))}
            </ol>
          );
        if ("note" in b) {
          const warn = b.tone === "warn";
          return (
            <div key={i} className={cn("flex gap-2 rounded-md px-3 py-2", warn ? "bg-state-warning-subtle" : "bg-state-info-subtle")}>
              {warn ? (
                <TriangleAlert className="mt-0.5 size-4 shrink-0 text-state-warning" aria-hidden />
              ) : (
                <Info className="mt-0.5 size-4 shrink-0 text-state-info" aria-hidden />
              )}
              <p>{r(b.note)}</p>
            </div>
          );
        }
        if ("table" in b)
          return (
            <div key={i} className="overflow-x-auto rounded-md border" tabIndex={0} role="region" aria-label={b.table.caption}>
              <table className="w-full text-left text-[0.8rem]">
                <caption className="sr-only">{b.table.caption}</caption>
                <thead className="bg-muted/50">
                  <tr>
                    {b.table.head.map((h, j) => (
                      <th key={j} scope="col" className="px-2 py-1.5 align-bottom font-semibold">
                        {r(h)}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="divide-y">
                  {b.table.rows.map((row, j) => (
                    <tr key={j}>
                      {row.map((c, k) => (
                        <td key={k} className="px-2 py-1.5 align-top">
                          {r(c)}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          );
        return (
          <dl key={i} className="divide-y rounded-md border">
            {b.dl.map(([t, d], j) => (
              <div key={j} className="px-3 py-2">
                <dt className="font-semibold">{r(t)}</dt>
                <dd className="text-muted-foreground">{r(d)}</dd>
              </div>
            ))}
          </dl>
        );
      })}
    </>
  );
}

export function HelpLangSwitch({ className }: { className?: string }) {
  const lang = useHelp((s) => s.lang);
  // Same as the user menu: the whole interface changes, and the choice is saved with the account.
  const setLang = useChooseLang();
  const label = useHelpContent()?.ui.language ?? "Help language";
  return (
    <ToggleGroup
      aria-label={label}
      value={[lang]}
      onValueChange={(v) => {
        const next = v[0] as HelpLang | undefined;
        if (next) setLang(next);
      }}
      variant="outline"
      size="sm"
      className={className}
    >
      {LANGS.map((l) => (
        <ToggleGroupItem key={l.id} value={l.id} aria-label={l.label} lang={l.id} title={l.label}>
          {l.short}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
}

function Heading({ level, children, ...rest }: { level: 2 | 3 | 4; children: ReactNode } & React.HTMLAttributes<HTMLHeadingElement>) {
  const Tag = `h${level}` as const;
  return <Tag {...rest}>{children}</Tag>;
}

export function HelpCentre({
  scope,
  article,
  section,
  nonce,
  onNavigate,
  hrefFor,
  baseLevel = 2,
  footerExtra,
}: {
  scope: "panel" | "page";
  article: ArticleId | null;
  section: string | null;
  nonce: number;
  onNavigate: Navigate;
  hrefFor: HrefFor;
  /** Heading level of an article title (sections are one below). */
  baseLevel?: 2 | 3;
  footerExtra?: ReactNode;
}) {
  const lang = useHelp((s) => s.lang);
  const c = useHelpContent();
  const [query, setQuery] = useState("");
  const root = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  // Move focus (and scroll) to what was asked for: a section, an article, or the search box.
  useEffect(() => {
    const id = requestAnimationFrame(() => {
      const el =
        article && section
          ? document.getElementById(sectionDomId(scope, article, section))
          : article
            ? root.current?.querySelector<HTMLElement>("[data-help-article-title]")
            : null;
      if (el) {
        el.scrollIntoView({ block: "start" });
        el.focus({ preventScroll: true });
      } else {
        if (scope === "page") window.scrollTo({ top: 0 });
        else root.current?.closest("[data-help-scroll]")?.scrollTo({ top: 0 });
        if (scope === "panel") searchRef.current?.focus();
      }
    });
    return () => cancelAnimationFrame(id);
  }, [article, section, nonce, scope, c]);

  if (!c) {
    return (
      <p className="text-sm text-muted-foreground" role="status">
        …
      </p>
    );
  }

  const go: Navigate = (a, s = null) => {
    setQuery("");
    onNavigate(a, s);
  };
  const hits = query.trim() ? searchHelp(c, query) : null;
  const idx = article ? ARTICLE_IDS.indexOf(article) : -1;
  const prev = idx > 0 ? ARTICLE_IDS[idx - 1] : undefined;
  const next = idx >= 0 && idx < ARTICLE_IDS.length - 1 ? ARTICLE_IDS[idx + 1] : undefined;
  const art = article ? c.articles[article] : null;
  const hl = baseLevel;
  const sl = (baseLevel + 1) as 3 | 4;

  return (
    <div ref={root} lang={lang} className="space-y-4 text-sm leading-relaxed">
      <div className="relative">
        <label htmlFor={`help-search-${scope}`} className="sr-only">
          {c.ui.search}
        </label>
        <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden />
        <Input
          ref={searchRef}
          id={`help-search-${scope}`}
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={c.ui.searchPlaceholder}
          className="pl-8"
          autoComplete="off"
        />
      </div>

      {hits ? (
        <section aria-label={c.ui.search}>
          <p className="sr-only" role="status">
            {hits.length === 0 ? c.ui.noResults : `${hits.length}`}
          </p>
          {hits.length === 0 ? (
            <p className="text-muted-foreground">{c.ui.noResults}</p>
          ) : (
            <ul className="divide-y rounded-md border">
              {hits.map((h) => (
                <li key={`${h.article}-${h.section ?? ""}`}>
                  <a
                    href={hrefFor(h.article, h.section)}
                    onClick={(e) => {
                      e.preventDefault();
                      go(h.article, h.section);
                    }}
                    className="block px-3 py-2 hover:bg-accent focus-visible:bg-accent focus-visible:outline-none"
                  >
                    <span className="font-medium">{h.sectionTitle ?? h.articleTitle}</span>
                    {h.sectionTitle && <span className="block text-xs text-muted-foreground">{h.articleTitle}</span>}
                  </a>
                </li>
              ))}
            </ul>
          )}
        </section>
      ) : art && article ? (
        <article aria-labelledby={`help-${scope}-${article}-title`} className="space-y-4">
          <a
            href={hrefFor(null)}
            onClick={(e) => {
              e.preventDefault();
              go(null);
            }}
            className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
            data-help-back
          >
            <ChevronLeft className="size-3.5" aria-hidden /> {c.ui.back}
          </a>
          <header className="space-y-1">
            <Heading level={hl} id={`help-${scope}-${article}-title`} tabIndex={-1} data-help-article-title className="text-lg font-semibold outline-none">
              {art.title}
            </Heading>
            <p className="text-muted-foreground">{art.summary}</p>
          </header>
          {Object.keys(art.sections).length > 1 && (
            <nav aria-label={c.ui.onThisPage} className="rounded-md border bg-muted/30 px-3 py-2">
              <p className="mb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{c.ui.onThisPage}</p>
              <ul className="space-y-0.5">
                {Object.entries(art.sections).map(([id, s]) => (
                  <li key={id}>
                    <a
                      href={hrefFor(article, id)}
                      onClick={(e) => {
                        e.preventDefault();
                        go(article, id);
                      }}
                      className={cn("text-primary underline-offset-4 hover:underline", section === id && "font-semibold")}
                    >
                      {s.title}
                    </a>
                  </li>
                ))}
              </ul>
            </nav>
          )}
          {Object.entries(art.sections).map(([id, s]) => (
            <section key={id} aria-labelledby={sectionDomId(scope, article, id)} className="scroll-mt-2 space-y-2">
              <Heading level={sl} id={sectionDomId(scope, article, id)} tabIndex={-1} className="text-base font-semibold outline-none focus-visible:underline">
                {s.title}
              </Heading>
              <Blocks blocks={s.blocks} onNavigate={go} hrefFor={hrefFor} />
            </section>
          ))}
          <nav aria-label={c.ui.articles} className="flex items-stretch justify-between gap-2 border-t pt-3">
            {prev ? (
              <a href={hrefFor(prev)} onClick={(e) => { e.preventDefault(); go(prev); }} className="flex min-w-0 flex-col rounded-md px-2 py-1 hover:bg-accent">
                <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                  <ChevronLeft className="size-3.5" aria-hidden /> {c.ui.previous}
                </span>
                <span className="truncate font-medium">{c.articles[prev].title}</span>
              </a>
            ) : (
              <span />
            )}
            {next && (
              <a href={hrefFor(next)} onClick={(e) => { e.preventDefault(); go(next); }} className="flex min-w-0 flex-col items-end rounded-md px-2 py-1 text-right hover:bg-accent">
                <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                  {c.ui.next} <ChevronRight className="size-3.5" aria-hidden />
                </span>
                <span className="truncate font-medium">{c.articles[next].title}</span>
              </a>
            )}
          </nav>
        </article>
      ) : (
        <nav aria-label={c.ui.articles}>
          <ul className="divide-y rounded-md border">
            {ARTICLE_IDS.map((a) => (
              <li key={a}>
                <a
                  href={hrefFor(a)}
                  onClick={(e) => {
                    e.preventDefault();
                    go(a);
                  }}
                  className="block px-3 py-2 hover:bg-accent focus-visible:bg-accent focus-visible:outline-none"
                >
                  <span className="font-medium">{c.articles[a].title}</span>
                  <span className="block text-xs text-muted-foreground">{c.articles[a].summary}</span>
                </a>
              </li>
            ))}
          </ul>
        </nav>
      )}

      <footer className="space-y-1 border-t pt-3 text-xs text-muted-foreground">
        <p className="flex gap-1.5">
          <TriangleAlert className="mt-0.5 size-3.5 shrink-0 text-state-warning" aria-hidden />
          <span>{c.ui.reviewNote}</span>
        </p>
        <p>{c.ui.languageNote}</p>
        {footerExtra}
      </footer>
    </div>
  );
}
