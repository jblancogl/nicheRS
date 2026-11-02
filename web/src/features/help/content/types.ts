// Help content model. English (`en.ts`) is the source of truth: its article and section ids
// define the `Translation` type that `fr.ts` and `es.ts` must satisfy, so a missing or extra
// article/section/UI string is a TypeScript error. Prose may differ in length per language;
// structure may not. See docs/design/help-content.md.

/**
 * Inline text supports a deliberately tiny markup, rendered without HTML injection:
 * `**strong**`, `*emphasis*`, `` `code` `` and links to other help sections
 * `[label](#article-id/section-id)` (section optional). External links are not supported on purpose.
 */
export type Inline = string;

export type Block =
  | { p: Inline }
  | { ul: Inline[] }
  | { ol: Inline[] }
  /** Callout. `warn` for caveats a user must not miss, `info` for tips. */
  | { note: Inline; tone?: "info" | "warn" }
  | { table: { head: Inline[]; rows: Inline[][]; caption: string } }
  /** Definition list (glossary). */
  | { dl: [Inline, Inline][] };

export interface SectionContent {
  title: string;
  blocks: Block[];
}

export interface ArticleContent {
  title: string;
  /** One sentence shown in the article list and used by search. */
  summary: string;
  sections: Record<string, SectionContent>;
}

export type HelpLang = "en" | "fr" | "es";

/** Shape-only view of an `en`-like dictionary: same article and section ids, any prose. */
export type TranslationOf<T extends { articles: Record<string, ArticleContent>; ui: Record<string, string> }> = {
  ui: { [K in keyof T["ui"]]: string };
  articles: {
    [A in keyof T["articles"]]: {
      title: string;
      summary: string;
      sections: { [S in keyof T["articles"][A]["sections"]]: SectionContent };
    };
  };
};
