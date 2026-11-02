// Typed UI messages without a dependency: each feature defines its strings once per language,
// and TypeScript requires French and Spanish to have exactly the keys (and function signatures)
// of English. Messages are plain strings or functions for interpolation and plurals.
//
//   const useM = defineMessages({ en: { hello: (n: string) => `Hello ${n}` }, fr: {...}, es: {...} });
//   const m = useM(); m.hello(name);
//
// Scientific wording rules apply in every language (AGENTS.md): "relative suitability" /
// "idonéité relative" / "idoneidad relativa", never a probability.

import { useLang, type Lang } from "./lang";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Msg = string | ((...args: any[]) => string);
type Same<T> = { [K in keyof T]: T[K] extends string ? string : T[K] };

export interface MessageTable<T> {
  (): T;
  /** Outside React (e.g. callbacks, non-component helpers). */
  get: (lang?: Lang) => T;
}

export function defineMessages<T extends Record<string, Msg>>(m: { en: T; fr: Same<T>; es: Same<T> }): MessageTable<T> {
  const table = m as Record<Lang, T>;
  const useTable = () => table[useLang((s) => s.lang)];
  return Object.assign(useTable, { get: (lang?: Lang) => table[lang ?? useLang.getState().lang] });
}
