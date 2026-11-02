import { Compass, SlidersHorizontal } from "lucide-react";
import type { ReactNode } from "react";

import { Panel } from "@/components/domain/Panel";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { useDraft, type StudyPath } from "@/stores/draft";

import { usePathMessages } from "./pathMessages";

function Choice({ path, icon, title, lead, steps }: { path: StudyPath; icon: ReactNode; title: string; lead: string; steps: string }) {
  const setPath = useDraft((s) => s.setPath);
  return (
    <button
      type="button"
      onClick={() => setPath(path)}
      className="flex w-full items-start gap-3 rounded-lg border p-4 text-left transition-colors hover:border-primary hover:bg-primary/5 focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none"
    >
      <span className="mt-0.5 shrink-0 text-primary">{icon}</span>
      <span className="space-y-1">
        <span className="block font-semibold">{title}</span>
        <span className="block text-sm">{lead}</span>
        <span className="block text-xs text-muted-foreground">{steps}</span>
      </span>
    </button>
  );
}

/** First step of a new study until a path is chosen (then remembered with the draft store). */
export function PathChooser() {
  const p = usePathMessages();
  return (
    <Panel title={p.chooserTitle} subtitle={p.chooserSubtitle}>
      <div className="space-y-3">
        <Choice path="quick" icon={<Compass aria-hidden />} title={p.quick} lead={p.quickLead} steps={p.quickSteps} />
        <Choice path="expert" icon={<SlidersHorizontal aria-hidden />} title={p.expert} lead={p.expertLead} steps={p.expertSteps} />
        <p className="text-sm text-muted-foreground">{p.sameModel}</p>
        <p className="text-xs text-muted-foreground">{p.remembered}</p>
        <HelpTrigger target={{ article: "getting-started", section: "paths" }} text="pathHelp" />
      </div>
    </Panel>
  );
}
