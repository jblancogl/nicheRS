// Feedback as Natural Solutions administrators (all organisations, can close and delete) and
// organisation managers (their organisation, read-only) see it: message, screenshots and
// recordings inline, the page it was sent from, and whether it has been dealt with.

import { Lightbulb, MessageCircle, MessageSquare, TriangleAlert } from "lucide-react";
import { useState } from "react";

import { urls } from "@/api/client";
import type { Feedback, FeedbackKind, FeedbackStatus } from "@/api/types";
import { EmptyState, ErrorState } from "@/components/domain/common";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Confirm, InlineError, Loading } from "@/features/admin/shared";
import { formatDateTime } from "@/lib/format";

import { useFeedbackMessages } from "./messages";

const ICON: Record<FeedbackKind, typeof Lightbulb> = { problem: TriangleAlert, idea: Lightbulb, other: MessageCircle };

interface Actions {
  setStatus: (id: string, status: FeedbackStatus) => void;
  remove: (id: string) => void;
  pending: boolean;
  error: unknown;
}

interface Props {
  query: { data?: Feedback[]; isPending: boolean; isError: boolean; error: unknown; refetch: () => unknown };
  /** Administrators only: the organisation of each item, close/reopen and delete. */
  actions?: Actions;
}

function Item({ f, actions }: { f: Feedback; actions?: Actions }) {
  const m = useFeedbackMessages();
  const Icon = ICON[f.kind];
  const closed = f.status === "closed";
  return (
    <li className={`space-y-3 rounded-lg border bg-card p-4 ${closed ? "opacity-70" : ""}`}>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
        <span className="inline-flex items-center gap-1.5 font-medium">
          <Icon className="size-4 text-muted-foreground" aria-hidden /> {m[f.kind]}
        </span>
        <Badge variant={closed ? "secondary" : "outline"}>{closed ? m.closed : m.open}</Badge>
        <span className="text-muted-foreground">
          {f.author ?? m.erased}
          {f.author_email && <> · {f.author_email}</>}
          {actions && f.org_name && <> · {f.org_name}</>}
        </span>
        <span className="num ml-auto text-xs text-muted-foreground">{formatDateTime(f.created_at)}</span>
      </div>
      {f.message ? <p className="text-sm whitespace-pre-wrap">{f.message}</p> : <p className="text-sm text-muted-foreground italic">{m.noText}</p>}
      {f.files.length > 0 && (
        <div className="flex flex-wrap gap-3">
          {f.files.map((file) => {
            const src = urls.feedbackFile(f.id, file.id);
            return file.mime.startsWith("image/") ? (
              <a key={file.id} href={src} target="_blank" rel="noopener noreferrer" className="block">
                <img src={src} alt={file.name} loading="lazy" className="h-28 max-w-56 rounded border object-cover object-top" />
              </a>
            ) : (
              <audio key={file.id} src={src} controls preload="none" className="h-10 w-72 max-w-full" aria-label={`${m.voice}: ${file.name}`} />
            );
          })}
        </div>
      )}
      <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
        {f.page && (
          <span>
            {m.page}: <code className="rounded bg-muted px-1">{f.page}</code>
          </span>
        )}
        {actions && (
          <span className="ml-auto flex gap-1">
            <Button size="sm" variant="outline" disabled={actions.pending} onClick={() => actions.setStatus(f.id, closed ? "open" : "closed")}>
              {closed ? m.reopen : m.close}
            </Button>
            <Confirm label={m.delete} title={m.deleteTitle} description={m.deleteText} action={m.delete} destructive onConfirm={() => actions.remove(f.id)} />
          </span>
        )}
      </div>
    </li>
  );
}

export function FeedbackList({ query, actions }: Props) {
  const m = useFeedbackMessages();
  const [showClosed, setShowClosed] = useState(false);
  if (query.isPending) return <Loading />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => void query.refetch()} />;
  const all = query.data ?? [];
  if (all.length === 0) return <EmptyState icon={<MessageSquare className="size-6" aria-hidden />} title={m.empty} />;
  const open = all.filter((f) => f.status === "open").length;
  const shown = showClosed ? all : all.filter((f) => f.status === "open");
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-4 text-sm">
        <span className="num text-muted-foreground">{m.count(open, all.length)}</span>
        <label className="ml-auto inline-flex items-center gap-2">
          <Switch checked={showClosed} onCheckedChange={setShowClosed} /> {m.showClosed}
        </label>
      </div>
      {actions && <InlineError error={actions.error} />}
      <ul className="space-y-3">
        {shown.map((f) => (
          <Item key={f.id} f={f} actions={actions} />
        ))}
      </ul>
    </div>
  );
}
