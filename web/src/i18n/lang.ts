// Interface language (English, French, Spanish) for the whole UI, help included. Hosted mode:
// the choice is stored with the account (`users.lang`, POST /auth/lang) and applied at sign-in;
// before sign-in and in local mode it is remembered per browser.

import { create } from "zustand";

export type Lang = "en" | "fr" | "es";

export const LANGS: readonly { id: Lang; label: string }[] = [
  { id: "en", label: "English" },
  { id: "fr", label: "Français" },
  { id: "es", label: "Español" },
];

const KEY = "nichers.lang";
/** Language picked in the help panel before the whole UI was translated. */
const LEGACY_KEY = "nichers.help.lang";

export function isLang(v: unknown): v is Lang {
  return v === "en" || v === "fr" || v === "es";
}

/** fr* → French, es* → Spanish, anything else → English. */
export function detectLang(languages: readonly string[] | undefined): Lang {
  for (const l of languages ?? []) {
    const p = l.toLowerCase();
    if (p.startsWith("fr")) return "fr";
    if (p.startsWith("es")) return "es";
    if (p.startsWith("en")) return "en";
  }
  return "en";
}

function read(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function initial(): Lang {
  const saved = read(KEY) ?? read(LEGACY_KEY);
  if (isLang(saved)) return saved;
  if (typeof navigator === "undefined") return "en";
  return detectLang(navigator.languages?.length ? navigator.languages : [navigator.language]);
}

function apply(lang: Lang) {
  try {
    window.localStorage.setItem(KEY, lang);
  } catch {
    // storage unavailable: the choice lasts for this page only
  }
  if (typeof document !== "undefined") document.documentElement.lang = lang;
}

export const useLang = create<{ lang: Lang; setLang: (lang: Lang) => void }>()((set) => ({
  lang: initial(),
  setLang: (lang) => {
    apply(lang);
    set({ lang });
  },
}));

if (typeof document !== "undefined") document.documentElement.lang = useLang.getState().lang;
