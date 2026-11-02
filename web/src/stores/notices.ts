// In-app notices (e.g. "run finished"): a short stack shown over the workspace, announced to
// screen readers, dismissed by the user or after a delay.

import { create } from "zustand";

export type NoticeTone = "success" | "warning" | "danger" | "neutral";

export interface Notice {
  id: string;
  tone: NoticeTone;
  title: string;
  body?: string;
  /** In-app link opened by the notice's action. */
  href?: string;
  action?: string;
}

interface NoticeState {
  notices: Notice[];
  push: (n: Notice) => void;
  dismiss: (id: string) => void;
}

const DELAY_MS = 15_000;

export const useNotices = create<NoticeState>()((set, get) => ({
  notices: [],
  push: (n) => {
    // Same id replaces the previous notice (a job finishing twice in quick polls).
    set((s) => ({ notices: [...s.notices.filter((x) => x.id !== n.id), n].slice(-4) }));
    window.setTimeout(() => get().dismiss(n.id), DELAY_MS);
  },
  dismiss: (id) => set((s) => ({ notices: s.notices.filter((x) => x.id !== id) })),
}));
