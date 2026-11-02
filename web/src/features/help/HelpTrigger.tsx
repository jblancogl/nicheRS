// Contextual "?" help: opens the help panel at the article/section that explains the concept
// next to it. A real button (keyboard, screen readers); its accessible name says where it leads.

import { CircleHelp } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

import type { HelpContent, HelpTarget, UiKey } from "./content";
import { useHelp, useHelpContent } from "./store";

function targetTitle(c: HelpContent, t: HelpTarget): string {
  const art = c.articles[t.article];
  const sections = art.sections as Record<string, { title: string }>;
  return t.section ? (sections[t.section]?.title ?? art.title) : art.title;
}

/**
 * Icon-only "?" by default; with `text`, a link-style button showing that help UI string
 * (in the help language) next to the icon.
 */
export function HelpTrigger({ target, className, text }: { target: HelpTarget; className?: string; text?: UiKey }) {
  const lang = useHelp((s) => s.lang);
  const show = useHelp((s) => s.show);
  const c = useHelpContent();
  const label = c ? `${c.ui.helpOn} ${targetTitle(c, target)}` : "Help";
  if (text) {
    return (
      <Button
        type="button"
        variant="link"
        size="xs"
        className={cn("h-auto px-0", className)}
        aria-haspopup="dialog"
        onClick={() => show(target)}
        data-help-trigger={`${target.article}${target.section ? `/${target.section}` : ""}`}
      >
        <CircleHelp aria-hidden /> <span lang={lang}>{c?.ui[text] ?? "Help"}</span>
      </Button>
    );
  }
  return (
    <Button
      type="button"
      variant="ghost"
      size="icon-xs"
      className={cn("text-muted-foreground hover:text-foreground", className)}
      aria-label={label}
      title={label}
      aria-haspopup="dialog"
      onClick={() => show(target)}
      data-help-trigger={`${target.article}${target.section ? `/${target.section}` : ""}`}
    >
      <CircleHelp aria-hidden />
    </Button>
  );
}
