// Where the administration's rows lead (kept apart from components for fast refresh).

import type { MouseEvent } from "react";
import { useNavigate } from "react-router";

import type { ActivityRow } from "@/api/types";

/** Finished studies open on their results, the others on their progress. */
export const jobUrl = (r: ActivityRow) => (r.status === "succeeded" ? `/jobs/${r.id}/results` : `/jobs/${r.id}`);

/**
 * A table row that opens its item: the whole row is the target with a mouse, while its first
 * link stays the keyboard and screen-reader target (no nested interactive roles).
 */
export function useRowLink() {
  const navigate = useNavigate();
  return (to: string) => ({
    onClick: (e: MouseEvent<HTMLTableRowElement>) => {
      if ((e.target as HTMLElement).closest("a, button, input, select, textarea")) return;
      void navigate(to);
    },
  });
}
