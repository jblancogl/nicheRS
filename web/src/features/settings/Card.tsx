import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

/** One block of a settings section; `title` when a section has several. */
export function Card({ title, className, children }: { title?: string; className?: string; children: ReactNode }) {
  return (
    <section className={cn("space-y-3 rounded-lg border bg-card p-4", className)} aria-label={title}>
      {title && <h2 className="text-sm font-semibold">{title}</h2>}
      {children}
    </section>
  );
}
