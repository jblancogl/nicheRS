# In-app help: structure, translations and review status

Date: 2026-10-16. Audience: ~25 consultants of French *bureaux d'études* using the hosted service
without hand-holding. Code: `web/src/features/help/`.

## Review status

**Pending review by Natural Solutions ecologists.** Every article footer says so, in each
language. The content was written from the code and ADRs, not from a separate specification:
numbers, rules and wording were checked against ADR-0007, ADR-0009, ADR-0010, ADR-0105, ADR-0106
and `crates/niche-app/src/{fit.rs,env.rs,engine.rs,artifacts.rs,dto.rs}`. When those change, the help
must change in the same merge request (it is not generated from them).

Points the reviewers should look at first:

- Evidence-level table and the "what to do with an *Exploratory* / *Insufficient* species" advice
  (`evidence` article).
- "What the map does not replace" and "Limits & responsible use" (professional responsibility,
  citation in a client report).
- French and Spanish terminology (see below); level names stay in English because the UI shows them
  in English.
- The statement that a synthetic demo run counts as one modelled species in hosted mode: this is
  what the code does today (`engine.rs` records usage for every `Succeeded` species), and may not be
  the intended product rule.

## Where help appears

| Surface | What | Code |
|---|---|---|
| Top bar, every workspace page (incl. results) | "?" button opens the help panel (consistent position, WCAG 3.2.6) | `HelpPanel.tsx` (`HelpButton`, `HelpPanel`) |
| Help panel | Right-hand sheet (440 px) on wide screens, bottom sheet (85 % height) below 768 px; no dimming backdrop so the map stays visible; focus trapped while open; Esc closes and focus returns to the opener | `HelpPanel.tsx`, `components/ui/sheet.tsx` |
| `/help`, `/help/:article?s=section` | Same content as a page, outside the workspace shell (no session needed; linkable, printable) | `HelpPage.tsx` |
| Contextual "?" | Next to step subtitles in setup, licence and exclusion lists (Data review), validation method (Model), evidence / area / validation / map-summary cards (Summary), each metric (Validation), tuning, the synthetic-demo card and banner, the quota on My organisation → Overview | `HelpTrigger.tsx` |
| Welcome screen | Replaces the former first-run hint: every visit starts on `/` with what NicheRS does, the saved draft, the demo and a link to *Getting started* | `features/setup/Home.tsx` |

Each "?" is a `<button>` whose accessible name is "Help: <section title>" in the help language, with
`aria-haspopup="dialog"`. Opening focuses the target section heading (or the article title, or the
search box for the index).

## Languages

English (`en.ts`) is the source of truth. French (`fr.ts`) and Spanish (`es.ts`) are typed as
`TranslationOf<EnContent>`: a missing or extra article, section or UI string is a TypeScript error.
Prose length may differ; structure may not. Only the help and its own UI strings are translated;
the rest of the interface stays English. No i18n library: a typed dictionary covers the need.

- Selection: the EN/FR/ES switch in the panel and on the page. Default from `navigator.languages`
  (`fr*` → French, `es*` → Spanish, otherwise English). The choice is stored in
  `localStorage["nichers.help.lang"]` (wrapped in `try/catch`; private modes fall back to the
  default).
- Each language is a lazily loaded chunk (≈ 35–42 kB minified, ≈ 13–15 kB gzip); the current one is
  fetched at start-up because the "?" labels need its section titles.
- Vocabulary (binding, consistent with `design-principles.md` §3):

| English | Français | Español |
|---|---|---|
| relative habitat suitability | idonéité relative de l'habitat | idoneidad relativa del hábitat |
| evidence level | niveau de preuve | nivel de evidencia |
| consultancy | bureau d'études | consultora |
| study area | zone d'étude | zona de estudio |
| area of suitable conditions | surface de conditions favorables | superficie de condiciones favorables |
| spatial block cross-validation | validation croisée par blocs spatiaux | validación cruzada por bloques espaciales |
| fold | pli | pliegue |
| background | fond | fondo |
| study (one job: species + study area + settings; never "run" in the UI) | étude (f.) | estudio (m.) |

Never: "probability of presence / probabilité de présence / probabilidad de presencia" except in an
explicit negation; no confidence percentage; never "range / aire de répartition / área de
distribución" or "occupied area" for a thresholded map. `scripts/ui_smoke.py` checks every article
in every language for the presence-probability phrases outside a negation.

## Content model

```ts
type Block =
  | { p: Inline } | { ul: Inline[] } | { ol: Inline[] }
  | { note: Inline; tone?: "info" | "warn" }
  | { table: { caption: string; head: Inline[]; rows: Inline[][] } }
  | { dl: [Inline, Inline][] };
```

`Inline` is a string with a tiny markup rendered without HTML injection: `**strong**`,
`*emphasis*`, `` `code` `` and internal links `[label](#article)` or `[label](#article/section)`.
External links are deliberately not supported.

## How to add or change content

1. **Edit `en.ts` first.** To add a section, add a key under the article's `sections`; to add an
   article, add it to `articles` *and* to `ARTICLE_IDS` in `content/index.ts` (a compile-time check
   fails if you forget).
2. Run `npm run typecheck`: it lists every place in `fr.ts` and `es.ts` that now lacks the new key.
   Add the translations there. Do not leave English text in a translation without saying so in the
   merge request.
3. To link a UI concept to it, add `<HelpTrigger target={{ article: "…", section: "…" }} />` next to
   the concept; `target` is type-checked against the English ids.
4. If the text states a number or rule, cite where it comes from in the merge request (ADR section or
   source file) and check all three languages say the same thing.
5. Run `npm run typecheck && npm run lint && npm run build`, then `scripts/ui_smoke.py`.

To add a language: add it to `HelpLang` (`types.ts`), create `xx.ts` typed `TranslationOf<EnContent>`,
register it in `LOADERS` and `LANGS` (`content/index.ts`) and in `detectLang` (`store.ts`).
