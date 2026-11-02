// Full-page help centre at /help and /help/:article?s=section. Outside the workspace shell, so it
// needs no map and no session (it can be opened from the sign-in page or shared as a link).

import { ArrowLeft } from "lucide-react";
import { useEffect } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router";

import { helpHref, isArticleId } from "./content";
import { HelpCentre, HelpLangSwitch } from "./HelpCentre";
import { useHelp, useHelpContent } from "./store";

export function HelpPage() {
  const params = useParams();
  const [sp] = useSearchParams();
  const navigate = useNavigate();
  const lang = useHelp((s) => s.lang);
  const c = useHelpContent();
  const centre = c?.ui.helpCentre ?? "Help";
  const article = isArticleId(params.article) ? params.article : null;
  const s = sp.get("s");
  const section = article && s && c && s in c.articles[article].sections ? s : null;
  const title = article && c ? c.articles[article].title : null;

  useEffect(() => {
    document.title = title ? `${title} — ${centre} — NicheRS` : `${centre} — NicheRS`;
  }, [title, centre]);

  return (
    <div className="min-h-dvh bg-background">
      <header className="sticky top-0 z-10 flex h-11 items-center gap-3 border-b bg-card px-3">
        <Link to="/" className="flex items-center gap-1.5 rounded-sm font-semibold tracking-tight">
          <svg viewBox="0 0 16 16" className="size-4 text-primary" aria-hidden>
            <path d="M8 1.5c3 3 4.5 5.4 4.5 7.3A4.5 4.5 0 0 1 3.5 8.8C3.5 6.9 5 4.5 8 1.5Z" fill="none" stroke="currentColor" strokeWidth="1.5" />
          </svg>
          NicheRS
        </Link>
        <span className="text-sm text-muted-foreground" lang={lang}>
          {centre}
        </span>
        <div className="flex-1" />
        <HelpLangSwitch />
      </header>
      <main className="mx-auto max-w-3xl px-4 py-6" data-help-scroll>
        <Link to="/" className="mb-4 inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground" lang={lang}>
          <ArrowLeft className="size-4" aria-hidden /> {c?.ui.backToApp ?? "Back"}
        </Link>
        <h1 className="mb-4 text-2xl font-semibold" lang={lang}>
          {centre}
        </h1>
        <HelpCentre
          scope="page"
          article={article}
          section={section}
          nonce={0}
          onNavigate={(a, sec) => void navigate(helpHref(a, sec))}
          hrefFor={helpHref}
          baseLevel={2}
        />
      </main>
    </div>
  );
}
