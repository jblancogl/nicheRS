import { useEffect, useRef, type ReactNode } from "react";

import { cn } from "@/lib/utils";

/**
 * Side-panel scaffold: header, scrollable body and a footer that holds the one primary action
 * of the current state (P2). The heading receives focus when `focusKey` changes (step changes,
 * route changes) so keyboard and screen-reader users land at the top of the new content.
 */
export function Panel({
  title,
  subtitle,
  crumbs,
  header,
  footer,
  children,
  focusKey,
  headingRef,
  className,
}: {
  title: ReactNode;
  subtitle?: ReactNode;
  /** Where this view sits (e.g. organisation › project), above the title. */
  crumbs?: ReactNode;
  header?: ReactNode;
  footer?: ReactNode;
  children: ReactNode;
  focusKey?: string | number;
  headingRef?: React.RefObject<HTMLHeadingElement | null>;
  className?: string;
}) {
  const own = useRef<HTMLHeadingElement>(null);
  const h = headingRef ?? own;
  const prevKey = useRef(focusKey);
  useEffect(() => {
    if (prevKey.current === focusKey) return; // initial mount (incl. StrictMode re-run)
    prevKey.current = focusKey;
    h.current?.focus({ preventScroll: false });
  }, [focusKey, h]);

  return (
    <div className={cn("flex h-full min-h-0 flex-col", className)}>
      <div className="shrink-0 space-y-2 border-b px-4 pt-3 pb-3">
        <div>
          {crumbs}
          <h2 ref={h} tabIndex={-1} className="text-lg font-semibold outline-none">
            {title}
          </h2>
          {subtitle && <div className="text-sm text-muted-foreground">{subtitle}</div>}
        </div>
        {header}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4" data-panel-body>
        {children}
      </div>
      {footer && <div className="flex shrink-0 items-center justify-end gap-2 border-t bg-card px-4 py-3">{footer}</div>}
    </div>
  );
}
