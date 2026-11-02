// Notice stack shown over the workspace (fed by useRunWatcher and other in-app events).

import { CircleCheck, CircleX, TriangleAlert, X } from "lucide-react";
import { useNavigate } from "react-router";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useNotices } from "@/stores/notices";

import { useShellMessages } from "./messages";

const ICON = { success: CircleCheck, warning: TriangleAlert, danger: CircleX, neutral: CircleCheck } as const;

export function Notices() {
  const notices = useNotices((s) => s.notices);
  const dismiss = useNotices((s) => s.dismiss);
  const navigate = useNavigate();
  const m = useShellMessages();
  return (
    <div
      className="pointer-events-none fixed top-14 right-3 z-50 flex w-[min(22rem,calc(100vw-1.5rem))] flex-col gap-2"
      role="status"
      aria-live="polite"
    >
      {notices.map((n) => {
        const Icon = ICON[n.tone];
        return (
          <div key={n.id} className="pointer-events-auto flex gap-3 rounded-lg border bg-card p-3 text-sm shadow-md">
            <Icon
              className={cn(
                "mt-0.5 size-4 shrink-0",
                n.tone === "success" && "text-state-success",
                n.tone === "warning" && "text-state-warning",
                n.tone === "danger" && "text-destructive",
                n.tone === "neutral" && "text-muted-foreground",
              )}
              aria-hidden
            />
            <div className="min-w-0 flex-1 space-y-1">
              <p className="font-medium">{n.title}</p>
              {n.body && <p className="text-muted-foreground">{n.body}</p>}
              {n.href && (
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    dismiss(n.id);
                    void navigate(n.href ?? "/");
                  }}
                >
                  {n.action}
                </Button>
              )}
            </div>
            <Button variant="ghost" size="icon-xs" aria-label={m.dismiss} onClick={() => dismiss(n.id)}>
              <X aria-hidden />
            </Button>
          </div>
        );
      })}
    </div>
  );
}
