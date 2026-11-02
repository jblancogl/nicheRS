// Feedback from every organisation (ADR-0111): administrators close and delete items.

import { useAdminFeedback, useAdminMutation } from "@/api/hooks";
import type { FeedbackStatus } from "@/api/types";
import { api } from "@/api/client";
import { FeedbackList } from "@/features/feedback/FeedbackList";
import { useFeedbackMessages } from "@/features/feedback/messages";

export function FeedbackSection() {
  const f = useFeedbackMessages();
  const query = useAdminFeedback();
  const status = useAdminMutation((a: { id: string; status: FeedbackStatus }) => api.admin.setFeedbackStatus(a.id, a.status));
  const remove = useAdminMutation((id: string) => api.admin.deleteFeedback(id));
  return (
    <div className="max-w-4xl space-y-4">
      <p className="text-sm text-muted-foreground">{f.listIntro}</p>
      <FeedbackList
        query={query}
        actions={{
          setStatus: (id, s) => status.mutate({ id, status: s }),
          remove: (id) => remove.mutate(id),
          pending: status.isPending || remove.isPending,
          error: status.error ?? remove.error,
        }}
      />
    </div>
  );
}

