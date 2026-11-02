// Slide-over help panel, reachable from the top bar on every workspace page and from the
// contextual "?" triggers. Right-hand sheet on wide screens, bottom sheet below 768 px (same
// breakpoint as the side panel). No dimming backdrop, so the map stays visible (design-principles
// anti-patterns: no modal over the map); focus is trapped while open, Esc closes and focus returns
// to the control that opened it.

import { CircleHelp, ExternalLink } from "lucide-react";
import { useSyncExternalStore } from "react";

import { useHealth } from "@/api/hooks";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "@/components/ui/sheet";

import { helpHref } from "./content";
import { HelpCentre } from "./HelpCentre";
import { useHelp, useHelpContent } from "./store";

const WIDE = "(min-width: 768px)";

function subscribe(cb: () => void) {
  if (typeof window.matchMedia !== "function") return () => {};
  const m = window.matchMedia(WIDE);
  m.addEventListener("change", cb);
  return () => m.removeEventListener("change", cb);
}
const wideNow = () => typeof window.matchMedia !== "function" || window.matchMedia(WIDE).matches;

function ServerLine() {
  const health = useHealth();
  return (
    <p className="num" lang="en">
      {health.data
        ? `Server ${health.data.version} (${health.data.build}) · remote sources ${health.data.online ? "reachable" : "unreachable"}`
        : health.isError
          ? "Server unreachable"
          : "Checking server…"}
    </p>
  );
}

export function HelpPanel() {
  const { open, article, section, nonce, setOpen, goTo } = useHelp();
  const wide = useSyncExternalStore(subscribe, wideNow, () => true);
  const ui = useHelpContent()?.ui;
  return (
    <Sheet open={open} onOpenChange={setOpen} modal="trap-focus">
      <SheetContent
        side={wide ? "right" : "bottom"}
        overlay={false}
        closeLabel={ui?.close ?? "Close"}
        initialFocus={false}
        className={wide ? "w-[440px] max-w-full border-l" : "h-[85dvh]"}
        data-help-panel
      >
        <div className="flex shrink-0 items-center gap-2 border-b py-2.5 pr-12 pl-4">
          <CircleHelp className="size-4 text-primary" aria-hidden />
          <SheetTitle className="mr-auto">{ui?.helpCentre ?? "Help"}</SheetTitle>
        </div>
        <SheetDescription className="sr-only">{ui?.reviewNote}</SheetDescription>
        <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-4 py-4" data-help-scroll>
          <HelpCentre
            scope="panel"
            article={article}
            section={section}
            nonce={nonce}
            onNavigate={goTo}
            hrefFor={helpHref}
            baseLevel={3}
            footerExtra={
              <>
                <a
                  href={helpHref(article, section)}
                  target="_blank"
                  rel="noopener"
                  className="inline-flex items-center gap-1 text-primary underline-offset-4 hover:underline"
                >
                  {ui?.openPage} <ExternalLink className="size-3" aria-hidden />
                </a>
                <ServerLine />
              </>
            }
          />
        </div>
      </SheetContent>
    </Sheet>
  );
}
