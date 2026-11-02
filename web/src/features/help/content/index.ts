// Help content registry. Each language is a separate lazily loaded chunk (≈ 45–50 kB of prose
// each), so the workspace bundle does not carry all three; only the type of `en` is imported here.

import type { EnContent } from "./en";
import type { HelpLang, TranslationOf } from "./types";

export type { Block, HelpLang } from "./types";

export type HelpContent = TranslationOf<EnContent>;
export type ArticleId = keyof EnContent["articles"];
export type SectionId<A extends ArticleId> =
  keyof EnContent["articles"][A]["sections"] & string;
export type UiKey = keyof EnContent["ui"];

/** A place in the help: an article and, optionally, one of its sections (type-checked). */
export type HelpTarget = {
  [A in ArticleId]: { article: A; section?: SectionId<A> };
}[ArticleId];

export const LOADERS: Record<HelpLang, () => Promise<HelpContent>> = {
  en: () => import("./en").then((m) => m.en),
  fr: () => import("./fr").then((m) => m.fr),
  es: () => import("./es").then((m) => m.es),
};

export const LANGS: { id: HelpLang; label: string; short: string }[] = [
  { id: "en", label: "English", short: "EN" },
  { id: "fr", label: "Français", short: "FR" },
  { id: "es", label: "Español", short: "ES" },
];

/** Reading order of the articles. */
export const ARTICLE_IDS = [
  "getting-started",
  "map",
  "evidence",
  "validation",
  "area",
  "uncertainty",
  "data",
  "variables",
  "predictors",
  "algorithms",
  "future",
  "field-data",
  "decision",
  "exports",
  "account",
  "glossary",
  "responsible",
] as const satisfies readonly ArticleId[];

// Compile-time check that ARTICLE_IDS lists every article of `en.ts`.
type Exhaustive<T extends never> = T;
export type ArticleIdsAreComplete = Exhaustive<
  Exclude<ArticleId, (typeof ARTICLE_IDS)[number]>
>;

export function isArticleId(v: unknown): v is ArticleId {
  return (
    typeof v === "string" && (ARTICLE_IDS as readonly string[]).includes(v)
  );
}

/** DOM id of a rendered section, unique per surface (panel vs page). */
export const sectionDomId = (scope: string, a: ArticleId, s: string) =>
  `help-${scope}-${a}-${s}`;

/** URL of the full-page help centre for an article/section. */
export const helpHref = (a: ArticleId | null, s?: string | null) =>
  a ? `/help/${a}${s ? `?s=${encodeURIComponent(s)}` : ""}` : "/help";
