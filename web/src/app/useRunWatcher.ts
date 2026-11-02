// Run watcher: when a run of this organisation finishes while the app is open, the user is told
// wherever they are (notice), and the tab title shows it while the tab is hidden.

import { useEffect, useRef } from "react";
import { useMatch } from "react-router";

import { isTerminal, useJobs } from "@/api/hooks";
import type { Job } from "@/api/types";
import { useNotices, type NoticeTone } from "@/stores/notices";

import { useShellMessages } from "./messages";

const BASE_TITLE = "NicheRS";

function outcome(j: Job): { tone: NoticeTone; ok: boolean } {
  if (j.status === "succeeded") {
    const fitted = j.species.filter((s) => s.status === "succeeded").length;
    return { tone: fitted === j.species.length ? "success" : "warning", ok: true };
  }
  return { tone: j.status === "failed" ? "danger" : "neutral", ok: false };
}

/** Watches the runs list (already polled every 5 s while a run is active). */
export function useRunWatcher() {
  const jobs = useJobs();
  const m = useShellMessages();
  const push = useNotices((s) => s.push);
  const onJob = useMatch("/jobs/:id/*")?.params.id;
  const seen = useRef<Map<string, Job["status"]> | null>(null);
  const finishedWhileHidden = useRef(0);

  useEffect(() => {
    const data = jobs.data;
    if (!data) return;
    const prev = seen.current;
    seen.current = new Map(data.map((j) => [j.id, j.status]));
    if (!prev) return; // first load: nothing has changed under the user's eyes yet
    for (const j of data) {
      const before = prev.get(j.id);
      if (!before || isTerminal(before) || !isTerminal(j.status) || j.status === "interrupted") continue;
      const { tone, ok } = outcome(j);
      if (document.hidden) finishedWhileHidden.current += 1;
      // On the run's own page the result is already on screen.
      if (onJob === j.id && !document.hidden) continue;
      push({
        id: `run-${j.id}`,
        tone,
        title: ok ? m.runDone(j.name) : j.status === "failed" ? m.runFailed(j.name) : m.runCancelled(j.name),
        body: ok ? m.runDoneBody(j.species.filter((s) => s.status === "succeeded").length, j.species.length) : undefined,
        href: j.status === "succeeded" ? `/jobs/${j.id}/results` : `/jobs/${j.id}`,
        action: ok ? m.openResults : m.seeWhatHappened,
      });
    }
    if (finishedWhileHidden.current > 0) document.title = `✓ ${BASE_TITLE}`;
  }, [jobs.data, m, push, onJob]);

  useEffect(() => {
    const reset = () => {
      if (!document.hidden) {
        finishedWhileHidden.current = 0;
        document.title = BASE_TITLE;
      }
    };
    document.addEventListener("visibilitychange", reset);
    return () => document.removeEventListener("visibilitychange", reset);
  }, []);
}

