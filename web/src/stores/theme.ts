// Theme preference (light / dark / system). Shared by the top-bar toggle and the map basemap.

import { create } from "zustand";

export type ThemePref = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

const KEY = "nichers.theme";

function readPref(): ThemePref {
  try {
    const v = window.localStorage.getItem(KEY);
    return v === "light" || v === "dark" || v === "system" ? v : "system";
  } catch {
    return "system";
  }
}

const media = () =>
  typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-color-scheme: dark)")
    : null;

function resolve(pref: ThemePref): ResolvedTheme {
  if (pref !== "system") return pref;
  return media()?.matches ? "dark" : "light";
}

function apply(t: ResolvedTheme) {
  const root = document.documentElement;
  root.classList.toggle("dark", t === "dark");
  root.style.colorScheme = t;
}

interface ThemeState {
  pref: ThemePref;
  resolved: ResolvedTheme;
  setPref: (p: ThemePref) => void;
}

export const useTheme = create<ThemeState>()((set) => {
  const pref = readPref();
  const resolved = resolve(pref);
  apply(resolved);
  media()?.addEventListener("change", () =>
    set((s) => {
      if (s.pref !== "system") return s;
      const r = resolve("system");
      apply(r);
      return { resolved: r };
    }),
  );
  return {
    pref,
    resolved,
    setPref: (p) => {
      try {
        window.localStorage.setItem(KEY, p);
      } catch {
        /* ignore */
      }
      const r = resolve(p);
      apply(r);
      set({ pref: p, resolved: r });
    },
  };
});
