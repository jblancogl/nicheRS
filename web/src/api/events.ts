// Live job progress over Server-Sent Events, with reconnection and a polling fallback.

import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import { urls } from "./client";
import { isTerminal, qk } from "./hooks";
import type { Job, JobEvent } from "./types";

export type StreamState = "connecting" | "live" | "reconnecting" | "polling" | "closed";

const MAX_EVENTS = 500;
const MAX_FAILURES = 4;

function parseEvent(data: string): JobEvent | null {
  try {
    const v: unknown = JSON.parse(data);
    if (typeof v === "object" && v !== null && "seq" in v && "status" in v) return v as JobEvent;
  } catch {
    /* ignore malformed frames */
  }
  return null;
}

/**
 * Subscribes to /jobs/{id}/events while `active`. Events are merged into the ["job", id] query
 * cache so every view (top bar, jobs menu, execution panel) sees the same state. When the stream
 * fails repeatedly, the hook reports "polling" and the caller should enable useJob polling.
 * Mount it under a component keyed by job id so the event log resets per job.
 */
export function useJobEvents(id: string | undefined, active: boolean) {
  const qc = useQueryClient();
  const [events, setEvents] = useState<JobEvent[]>([]);
  const [state, setState] = useState<StreamState>("connecting");
  const lastSeq = useRef(-1);

  const unsupported = typeof EventSource === "undefined";

  useEffect(() => {
    if (!id || !active || unsupported) return;
    let es: EventSource | null = null;
    let failures = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let disposed = false;

    const applyToCache = (ev: JobEvent) => {
      const before = qc.getQueryData<Job>(qk.job(id));
      // a status transition changes fields not carried by events (started_at, species): refetch
      if (before && before.status !== ev.status) void qc.invalidateQueries({ queryKey: qk.job(id) });
      qc.setQueryData<Job>(qk.job(id), (job) =>
        job
          ? {
              ...job,
              status: ev.status,
              stage: ev.stage || job.stage,
              // progress never moves backwards (interaction-specifications §5)
              progress: Math.max(job.progress, ev.progress),
            }
          : job,
      );
      if (isTerminal(ev.status)) {
        es?.close();
        setState("closed");
        void qc.invalidateQueries({ queryKey: qk.job(id) });
        void qc.invalidateQueries({ queryKey: qk.jobs });
      }
    };

    // "progress": a log-worthy event with a sequence number.
    const onProgress = (msg: MessageEvent<string>) => {
      const ev = parseEvent(msg.data);
      if (!ev) return;
      failures = 0;
      if (ev.seq <= lastSeq.current) return; // replay after reconnect
      lastSeq.current = ev.seq;
      setEvents((prev) => {
        const next = prev.length >= MAX_EVENTS ? prev.slice(prev.length - MAX_EVENTS + 1) : prev;
        return [...next, ev];
      });
      applyToCache(ev);
      if (!isTerminal(ev.status) && ev.species_key !== null && ev.species_key !== undefined) {
        // per-species status lives on the Job document; refresh it lazily
        void qc.invalidateQueries({ queryKey: qk.job(id) });
      }
    };

    // "status": heartbeat snapshot (same shape, empty message) — updates state, not the log.
    const onStatus = (msg: MessageEvent<string>) => {
      const ev = parseEvent(msg.data);
      if (!ev) return;
      failures = 0;
      applyToCache(ev);
    };

    const connect = () => {
      if (disposed) return;
      setState(failures === 0 ? "connecting" : "reconnecting");
      es = new EventSource(urls.events(id));
      es.addEventListener("progress", onProgress as EventListener);
      es.addEventListener("status", onStatus as EventListener);
      es.onmessage = onProgress; // tolerate unnamed events
      es.onopen = () => setState("live");
      es.onerror = () => {
        if (disposed) return;
        failures += 1;
        es?.close();
        if (failures >= MAX_FAILURES) {
          setState("polling");
          return;
        }
        setState("reconnecting");
        timer = setTimeout(connect, Math.min(1000 * 2 ** failures, 8000));
      };
    };
    timer = setTimeout(connect, 0);

    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
      es?.close();
    };
  }, [id, active, unsupported, qc]);

  const effective: StreamState = unsupported
    ? "polling"
    : active
      ? state
      : state === "polling"
        ? "polling"
        : "closed";
  return { events, state: effective };
}
