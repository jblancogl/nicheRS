import { Check, ChevronLeft } from "lucide-react";
import { useEffect, useRef, type ReactNode } from "react";

import { Panel } from "@/components/domain/Panel";
import { Button } from "@/components/ui/button";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import type { HelpTarget } from "@/features/help/content";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { cn } from "@/lib/utils";
import { STEPS, pathSteps, stepAfter, useDraft, type StudyPath } from "@/stores/draft";

import { useFrameMessages, useStepLabels, useStepShortLabels } from "./messages";
import { usePathMessages } from "./pathMessages";
import { useReachable } from "./reachable";

/** Help section explaining each setup step (contextual "?" next to the step title). */
const STEP_HELP: HelpTarget[] = [
  { article: "getting-started", section: "workflow" },
  { article: "getting-started", section: "workflow" },
  { article: "data", section: "rules" },
  { article: "variables", section: "collinearity" },
  { article: "validation", section: "selection" },
  { article: "evidence", section: "levels" },
];

/** The quick path's last step explains what the quick mode decides. */
const QUICK_REVIEW_HELP: HelpTarget = { article: "getting-started", section: "paths" };

/** Last step whose panel was shown; used to move focus to the heading on step changes only. */
let lastShownStep: number | null = null;

/** Quick / Expert switch: both paths edit the same draft, so nothing chosen is lost. */
function PathSwitch() {
  const path = useDraft((s) => s.path);
  const setPath = useDraft((s) => s.setPath);
  const p = usePathMessages();
  return (
    <ToggleGroup
      aria-label={p.switchLabel}
      value={[path ?? "expert"]}
      onValueChange={(v) => {
        const next = v[0] as StudyPath | undefined;
        if (next) setPath(next);
      }}
      variant="outline"
      size="sm"
      className="mb-3 w-full"
    >
      <ToggleGroupItem value="quick" className="flex-1">
        {p.quick}
      </ToggleGroupItem>
      <ToggleGroupItem value="expert" className="flex-1">
        {p.expert}
      </ToggleGroupItem>
    </ToggleGroup>
  );
}

function Stepper() {
  const step = useDraft((s) => s.step);
  const setStep = useDraft((s) => s.setStep);
  const path = useDraft((s) => s.path);
  const reachable = useReachable();
  const m = useFrameMessages();
  const labels = useStepLabels();
  const short = useStepShortLabels();
  const p = usePathMessages();
  const steps = pathSteps(path);
  const quick = path === "quick";
  return (
    <nav aria-label={m.progress}>
      <PathSwitch />
      <p className="num mb-1.5 text-xs text-muted-foreground">
        {m.stepOf(steps.indexOf(step) + 1, steps.length)}
      </p>
      <ol className={cn("grid gap-1", quick ? "grid-cols-3" : "grid-cols-6")}>
        {steps.map((i) => {
          const s = STEPS[i];
          const done = i < step && i <= reachable;
          const current = i === step;
          const enabled = i <= reachable;
          return (
            <li key={s.id} className="min-w-0">
              <button
                type="button"
                disabled={!enabled}
                onClick={() => setStep(i)}
                aria-current={current ? "step" : undefined}
                title={quick && i === 5 ? p.reviewTitle : labels[s.id]}
                className={cn(
                  "group flex w-full flex-col items-stretch gap-1 rounded-sm text-left disabled:cursor-not-allowed",
                )}
              >
                <span
                  className={cn(
                    "h-1 rounded-full",
                    current ? "bg-primary" : done ? "bg-primary/50" : "bg-muted",
                  )}
                  aria-hidden
                />
                <span
                  className={cn(
                    "flex min-w-0 items-center gap-0.5 text-[0.7rem] leading-tight",
                    current ? "font-semibold text-foreground" : enabled ? "text-muted-foreground group-hover:text-foreground" : "text-muted-foreground/60",
                  )}
                >
                  {done && <Check className="size-3 shrink-0" aria-hidden />}
                  <span className="min-w-0 truncate">{quick && i === 5 ? p.reviewTitle : short[s.id]}</span>
                  {done && <span className="sr-only">{m.completed}</span>}
                </span>
              </button>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

export function WizardFrame({
  title,
  subtitle,
  children,
  nextLabel,
  onNext,
  nextDisabled,
  nextReason,
  nextPending,
  secondary,
  nextType = "button",
  formId,
}: {
  title: string;
  subtitle?: ReactNode;
  children: ReactNode;
  nextLabel?: string;
  onNext?: () => void;
  nextDisabled?: boolean;
  /** Visible explanation when the primary action is disabled. */
  nextReason?: string;
  nextPending?: boolean;
  secondary?: ReactNode;
  nextType?: "button" | "submit";
  formId?: string;
}) {
  const step = useDraft((s) => s.step);
  const setStep = useDraft((s) => s.setStep);
  const path = useDraft((s) => s.path);
  const m = useFrameMessages();
  const heading = useRef<HTMLHeadingElement>(null);
  const help = path === "quick" && step === 5 ? QUICK_REVIEW_HELP : STEP_HELP[step];
  useEffect(() => {
    if (lastShownStep !== null && lastShownStep !== step) heading.current?.focus();
    lastShownStep = step;
  }, [step]);
  return (
    <Panel
      title={title}
      subtitle={
        <span className="flex items-start justify-between gap-2">
          <span>{subtitle}</span>
          {help && <HelpTrigger target={help} className="-mt-0.5 shrink-0" />}
        </span>
      }
      header={<Stepper />}
      headingRef={heading}
      footer={
        <div className="flex w-full flex-col gap-2">
          {nextDisabled && nextReason && (
            <p className="text-right text-xs text-muted-foreground" id="next-reason">
              {nextReason}
            </p>
          )}
          {/* Wraps on phones when the labels are long (ES: Atrás · Empezar de nuevo · Lanzar el estudio). */}
          <div className="flex flex-wrap items-center gap-2">
            {step > 0 && (
              <Button variant="ghost" onClick={() => setStep(stepAfter(path, step, -1))}>
                <ChevronLeft aria-hidden /> {m.back}
              </Button>
            )}
            <div className="flex-1" />
            {secondary}
            <Button
              className="ml-auto"
              type={nextType}
              form={formId}
              onClick={onNext}
              disabled={nextDisabled || nextPending}
              aria-describedby={nextDisabled && nextReason ? "next-reason" : undefined}
            >
              {nextPending ? m.working : (nextLabel ?? m.continue)}
            </Button>
          </div>
        </div>
      }
    >
      {children}
    </Panel>
  );
}
