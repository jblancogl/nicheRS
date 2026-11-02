import { ARTICLE_IDS, type ArticleId, type Block, type HelpContent } from "./content";

export interface SearchHit {
  article: ArticleId;
  section: string | null;
  articleTitle: string;
  sectionTitle: string | null;
}

function blockText(b: Block): string {
  if ("p" in b) return b.p;
  if ("ul" in b) return b.ul.join(" ");
  if ("ol" in b) return b.ol.join(" ");
  if ("note" in b) return b.note;
  if ("table" in b) return [...b.table.head, ...b.table.rows.flat()].join(" ");
  return b.dl.flat().join(" ");
}

/** Accent- and case-insensitive form, so "idoneite" finds "idonéité". */
export function fold(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();
}

/** Sections whose title or text contain every word of the query (max 30 hits). */
export function searchHelp(content: HelpContent, query: string): SearchHit[] {
  const words = fold(query).split(/\s+/).filter((w) => w.length > 1);
  if (words.length === 0) return [];
  const hits: SearchHit[] = [];
  for (const a of ARTICLE_IDS) {
    const art = content.articles[a];
    const head = fold(`${art.title} ${art.summary}`);
    const sections = Object.entries(art.sections) as [string, { title: string; blocks: Block[] }][];
    let any = false;
    for (const [id, s] of sections) {
      const text = fold(`${art.title} ${s.title} ${s.blocks.map(blockText).join(" ")}`);
      if (words.every((w) => text.includes(w))) {
        hits.push({ article: a, section: id, articleTitle: art.title, sectionTitle: s.title });
        any = true;
      }
    }
    if (!any && words.every((w) => head.includes(w))) {
      hits.push({ article: a, section: null, articleTitle: art.title, sectionTitle: null });
    }
  }
  return hits.slice(0, 30);
}
