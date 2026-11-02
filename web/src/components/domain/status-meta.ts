import type { EvidenceLevel, Severity } from "@/api/types";

import { useDomainMessages } from "./messages";

export type Tone = "success" | "warning" | "danger" | "info" | "running" | "neutral";

export const toneClass: Record<Tone, string> = {
  success: "text-state-success",
  warning: "text-state-warning",
  danger: "text-state-danger",
  info: "text-state-info",
  running: "text-state-running",
  neutral: "text-state-neutral",
};

export const toneBg: Record<Tone, string> = {
  success: "bg-state-success-subtle",
  warning: "bg-state-warning-subtle",
  danger: "bg-state-danger-subtle",
  info: "bg-state-info-subtle",
  running: "bg-state-info-subtle",
  neutral: "bg-muted",
};

/** In the current interface language; called during render of components that re-render on a language change. */
export const severityLabel = (s: Severity): string => useDomainMessages.get().severity(s);

export const evidenceMeaning = (l: EvidenceLevel): string => useDomainMessages.get().evidenceMeaning(l);
