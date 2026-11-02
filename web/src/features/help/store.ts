// Help panel state: open/closed, current article and section, and the help language, which is
// the interface language (`@/i18n/lang`).

import { create } from "zustand";

import { useLang } from "@/i18n/lang";

import { LOADERS, type ArticleId, type HelpContent, type HelpLang, type HelpTarget } from "./content";

const HINT_KEY = "nichers.help.hint-dismissed";

function read(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string) {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // storage unavailable: the choice lasts for this page only
  }
}

export { detectLang } from "@/i18n/lang";

interface HelpState {
  open: boolean;
  article: ArticleId | null;
  section: string | null;
  /** Bumped on every `show` so the same target re-scrolls when opened twice. */
  nonce: number;
  lang: HelpLang;
  /** Loaded languages (each is a lazy chunk). */
  content: Partial<Record<HelpLang, HelpContent>>;
  hintDismissed: boolean;
  show: (target?: HelpTarget) => void;
  goTo: (article: ArticleId | null, section?: string | null) => void;
  setOpen: (open: boolean) => void;
  setLang: (lang: HelpLang) => void;
  dismissHint: () => void;
}

const pending = new Set<HelpLang>();

/** Load the help content of a language without opening the panel (e.g. for global search). */
export function preloadHelp(lang: HelpLang) {
  load(lang);
}

function load(lang: HelpLang) {
  if (pending.has(lang)) return;
  pending.add(lang);
  LOADERS[lang]()
    .then((c) => useHelp.setState((s) => ({ content: { ...s.content, [lang]: c } })))
    .catch(() => pending.delete(lang)); // retried on the next language change or open
}

export const useHelp = create<HelpState>((set, get) => ({
  open: false,
  article: null,
  section: null,
  nonce: 0,
  lang: useLang.getState().lang,
  content: {},
  hintDismissed: read(HINT_KEY) === "1",
  show: (target) => {
    load(get().lang);
    set((s) => ({
      open: true,
      article: target ? target.article : s.article,
      section: target ? (target.section ?? null) : s.section,
      nonce: s.nonce + 1,
    }));
  },
  goTo: (article, section = null) => set((s) => ({ article, section, nonce: s.nonce + 1 })),
  setOpen: (open) => set({ open }),
  // The help follows the interface language; choosing it here changes the whole UI.
  setLang: (lang) => useLang.getState().setLang(lang),
  dismissHint: () => {
    write(HINT_KEY, "1");
    set({ hintDismissed: true });
  },
}));

// Start loading the current language right away (small chunk, needed by the "?" labels).
load(useHelp.getState().lang);

/** Content in the current language, or null while its chunk loads. */
export const useHelpContent = (): HelpContent | null => useHelp((s) => s.content[s.lang] ?? null);

// Load the new language whatever shows the help: the panel, the /help page or the "?" labels.
useLang.subscribe((s) => {
  if (useHelp.getState().lang !== s.lang) {
    load(s.lang);
    useHelp.setState({ lang: s.lang });
  }
});
