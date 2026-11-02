// Health of the instance, read from /admin/system: one rule per check, shared by the System page
// and the overview so both flag the same things.

import type { MirrorStatus, SystemStatus } from "@/api/types";

export type Health = "ok" | "warn" | "bad";

const DAY_MS = 86_400_000;

export function storageHealth(s: SystemStatus): Health {
  if (!s.disk || s.disk.total_bytes === 0) return "ok";
  const free = s.disk.available_bytes / s.disk.total_bytes;
  return free < 0.05 ? "bad" : free < 0.15 ? "warn" : "ok";
}

export function backupHealth(s: SystemStatus, now: number): Health {
  const last = s.db_backups.last_at ? Date.parse(s.db_backups.last_at) : null;
  if (last === null) return "warn";
  return now - last > 2 * DAY_MS ? "bad" : now - last > 1.5 * DAY_MS ? "warn" : "ok";
}

export function gbifHealth(s: SystemStatus): Health {
  if (!s.gbif.configured) return "warn";
  return s.gbif.ok === true ? "ok" : s.gbif.ok === false ? "bad" : "warn";
}

export function mirrorHealth(mirror: MirrorStatus): Health {
  return mirror.state === "error" || mirror.state === "disabled" || (mirror.snapshot && !mirror.snapshot.usable) ? "warn" : "ok";
}
