// TanStack Query hooks: the only place components get server state from.

import {
  keepPreviousData,
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useEffect } from "react";

import { isLang, useLang, type Lang } from "@/i18n/lang";

import { api, ApiError } from "./client";
import type { AnalysisRequest, AuditQuery, UserQuery, DataPreviewRequest, FeedbackRequest, ImportRequest, Job, JobSpec, JobStatus, StudyArea } from "./types";

export const qk = {
  health: ["health"] as const,
  species: (q: string) => ["species", q] as const,
  layers: (demo: boolean) => ["layers", demo] as const,
  preview: (req: DataPreviewRequest) => ["preview", req] as const,
  areaCheck: (area: StudyArea) => ["area-check", area.geometry] as const,
  jobs: ["jobs"] as const,
  projects: ["projects"] as const,
  job: (id: string) => ["job", id] as const,
  results: (id: string) => ["results", id] as const,
  manifest: (id: string) => ["manifest", id] as const,
  sample: (id: string, key: number, lng: number, lat: number) =>
    ["sample", id, key, lng, lat] as const,
  me: ["me"] as const,
  account: {
    all: ["account"] as const,
    gbif: ["account", "gbif"] as const,
  },
  admin: {
    all: ["admin"] as const,
    orgs: ["admin", "orgs"] as const,
    invites: (org: string) => ["admin", "invites", org] as const,
    org: (id: string) => ["admin", "org", id] as const,
    users: (q: UserQuery) => ["admin", "users", q.org ?? "", q.q ?? "", q.limit ?? 0] as const,
    user: (id: string) => ["admin", "user", id] as const,
    usage: (month: string | null) => ["admin", "usage", month] as const,
    activity: (org: string | undefined) => ["admin", "activity", org ?? ""] as const,
    system: ["admin", "system"] as const,
    metrics: (month: string | null) => ["admin", "metrics", month] as const,
    feedback: ["admin", "feedback"] as const,
    audit: (q: AuditQuery) => ["admin", "audit", q.org ?? "", q.user ?? "", q.limit ?? 0] as const,
  },
  org: {
    all: ["org"] as const,
    profile: ["org", "profile"] as const,
    members: ["org", "members"] as const,
    invites: ["org", "invites"] as const,
    feedback: ["org", "feedback"] as const,
    audit: ["org", "audit"] as const,
  },
};

export const TERMINAL: readonly JobStatus[] = ["succeeded", "failed", "cancelled", "interrupted"];
export const isTerminal = (s: JobStatus) => TERMINAL.includes(s);

/** Do not hammer a server that is down; retry other transient errors once. */
function retry(count: number, err: unknown) {
  if (err instanceof ApiError && (err.unreachable || err.status < 500)) return false;
  return count < 1;
}

export function useHealth() {
  return useQuery({
    queryKey: qk.health,
    queryFn: ({ signal }) => api.health(signal),
    refetchInterval: 30_000,
    retry: false,
  });
}

/**
 * Signed-in account in hosted mode. `data === null` means local mode (no accounts; /auth/me is
 * not even requested); a 401 error means "not signed in".
 */
export function useMe() {
  return useQuery({
    queryKey: qk.me,
    queryFn: async ({ signal }) => {
      const health = await api.health(signal);
      return health.accounts ? api.me(signal) : null;
    },
    staleTime: 30_000,
    refetchInterval: 60_000,
    retry: false,
  });
}

/** Choose the interface language: applied at once, and stored with the account in hosted mode. */
export function useChooseLang() {
  const qc = useQueryClient();
  const me = useMe();
  return (lang: Lang) => {
    useLang.getState().setLang(lang);
    if (me.data) {
      void api.setLang(lang).then(() => qc.invalidateQueries({ queryKey: qk.me }));
    }
  };
}

/** Hosted mode: apply the account's language when the session is known (sign-in, reload). */
export function useAccountLang() {
  const lang = useMe().data?.lang;
  useEffect(() => {
    if (isLang(lang) && useLang.getState().lang !== lang) useLang.getState().setLang(lang);
  }, [lang]);
}

export function useLogout() {
  return useMutation({
    mutationFn: () => api.logout(),
    onSettled: () => window.location.assign("/login"),
  });
}

// ---- administration (ADR-0106) ------------------------------------------------------------

export function useAdminOrgs() {
  return useQuery({ queryKey: qk.admin.orgs, queryFn: ({ signal }) => api.admin.orgs(signal), retry, refetchInterval: 30_000 });
}

export function useAdminInvites(org: string | null) {
  return useQuery({
    queryKey: qk.admin.invites(org ?? ""),
    queryFn: ({ signal }) => api.admin.invites(org ?? "", signal),
    enabled: org !== null,
    retry,
  });
}

export function useAdminOrg(id: string) {
  return useQuery({ queryKey: qk.admin.org(id), queryFn: ({ signal }) => api.admin.org(id, signal), retry, refetchInterval: 30_000 });
}

/** Accounts filtered on the server, one page at a time ("show more"). */
export function useAdminUsers(q: UserQuery) {
  return useInfiniteQuery({
    queryKey: qk.admin.users(q),
    queryFn: ({ signal, pageParam }) => api.admin.users({ ...q, offset: pageParam }, signal),
    initialPageParam: undefined as number | undefined,
    getNextPageParam: (last) => last.next_offset ?? undefined,
    placeholderData: keepPreviousData,
    retry,
  });
}

export function useAdminUser(id: string) {
  return useQuery({ queryKey: qk.admin.user(id), queryFn: ({ signal }) => api.admin.user(id, signal), retry });
}

export function useAdminUsage(month: string | null) {
  return useQuery({ queryKey: qk.admin.usage(month), queryFn: ({ signal }) => api.admin.usage(month, signal), retry });
}

/** Every organisation's studies, or one organisation's (`org`: its id). */
export function useAdminActivity(org?: string) {
  return useQuery({
    queryKey: qk.admin.activity(org),
    queryFn: ({ signal }) => api.admin.activity(org, signal),
    refetchInterval: (query) => (query.state.data?.some((j) => !isTerminal(j.status)) ? 3_000 : 15_000),
    retry,
  });
}

export function useAdminSystem() {
  return useQuery({ queryKey: qk.admin.system, queryFn: ({ signal }) => api.admin.system(signal), retry, refetchInterval: 60_000 });
}

export function useAdminMetrics(month: string | null) {
  return useQuery({ queryKey: qk.admin.metrics(month), queryFn: ({ signal }) => api.admin.metrics(month, signal), retry });
}

/** The audit trail (ADR-0113), newest first, one page at a time ("show older"). */
export function useAdminAudit(q: AuditQuery) {
  return useInfiniteQuery({
    queryKey: qk.admin.audit(q),
    queryFn: ({ signal, pageParam }) => api.admin.audit({ ...q, before: pageParam }, signal),
    initialPageParam: undefined as number | undefined,
    getNextPageParam: (last) => last.next ?? undefined,
    retry,
  });
}

// ---- the account's own properties and the organisation (ADR-0110) ---------------------------

export function useAccountGbif(enabled: boolean) {
  return useQuery({ queryKey: qk.account.gbif, queryFn: ({ signal }) => api.account.gbif(signal), enabled, retry });
}

export function useOrgProfile(enabled: boolean) {
  return useQuery({ queryKey: qk.org.profile, queryFn: ({ signal }) => api.org.profile(signal), enabled, retry });
}

/** A change to the account or its organisation: refresh both, and the signed-in summary. */
export function useAccountMutation<A, R>(fn: (a: A) => Promise<R>) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: qk.account.all });
      void qc.invalidateQueries({ queryKey: qk.org.all });
      void qc.invalidateQueries({ queryKey: qk.me });
    },
  });
}

// ---- feedback (ADR-0111) ---------------------------------------------------------------------

export function useAdminFeedback() {
  return useQuery({ queryKey: qk.admin.feedback, queryFn: ({ signal }) => api.admin.feedback(signal), retry, refetchInterval: 60_000 });
}

/** The organisation's history (managers, ADR-0113), newest first, older pages on demand. */
export function useOrgAudit(enabled: boolean) {
  return useInfiniteQuery({
    queryKey: qk.org.audit,
    queryFn: ({ signal, pageParam }) => api.org.audit(pageParam, signal),
    initialPageParam: undefined as number | undefined,
    getNextPageParam: (last) => last.next ?? undefined,
    enabled,
    retry,
  });
}

export function useOrgFeedback(enabled: boolean) {
  return useQuery({ queryKey: qk.org.feedback, queryFn: ({ signal }) => api.org.feedback(signal), enabled, retry });
}

export function useSendFeedback() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (b: FeedbackRequest) => api.sendFeedback(b),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: qk.admin.feedback });
      void qc.invalidateQueries({ queryKey: qk.org.feedback });
    },
  });
}

// ---- organisation managers (ADR-0109) -------------------------------------------------------

export function useOrgMembers(enabled: boolean) {
  return useQuery({ queryKey: qk.org.members, queryFn: ({ signal }) => api.org.members(signal), enabled, retry });
}

export function useOrgInvites(enabled: boolean) {
  return useQuery({ queryKey: qk.org.invites, queryFn: ({ signal }) => api.org.invites(signal), enabled, retry });
}

/** Any change by a manager: refresh the organisation views. */
export function useOrgMutation<A, R>(fn: (a: A) => Promise<R>) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: qk.org.all });
    },
  });
}

/** Any administration change: refresh every admin view (they are small). */
export function useAdminMutation<A, R>(fn: (a: A) => Promise<R>) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: qk.admin.all });
      void qc.invalidateQueries({ queryKey: qk.me });
    },
  });
}

export function useDeleteJob() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteJob(id),
    onSuccess: (_: undefined, id: string) => {
      qc.removeQueries({ queryKey: qk.job(id) });
      qc.removeQueries({ queryKey: qk.results(id) });
      void qc.invalidateQueries({ queryKey: qk.jobs });
    },
  });
}

export function useSpeciesSearch(q: string) {
  const term = q.trim();
  return useQuery({
    queryKey: qk.species(term),
    queryFn: ({ signal }) => api.searchSpecies(term, signal),
    enabled: term.length >= 3,
    staleTime: 5 * 60_000,
    placeholderData: keepPreviousData,
    retry,
  });
}

export function useLayers(demo: boolean) {
  return useQuery({
    queryKey: qk.layers(demo),
    queryFn: ({ signal }) => api.layers(demo, signal),
    staleTime: 10 * 60_000,
    retry,
  });
}

/** Grid cells of the study area per resolution (not for the offline demo, which has no limit). */
export function useAreaCheck(area: StudyArea | null, enabled = true) {
  return useQuery({
    queryKey: area ? qk.areaCheck(area) : ["area-check", "none"],
    queryFn: ({ signal }) => {
      if (!area) throw new Error("No study area");
      return api.checkArea(area, signal);
    },
    enabled: enabled && area !== null,
    staleTime: Infinity,
    retry,
  });
}

export function usePreview(req: DataPreviewRequest | null) {
  return useQuery({
    queryKey: req ? qk.preview(req) : ["preview", "none"],
    queryFn: ({ signal }) => {
      if (!req) throw new Error("No preview request");
      return api.previewOccurrences(req, signal);
    },
    enabled: req !== null,
    staleTime: 10 * 60_000,
    retry,
  });
}

export function useJobs() {
  return useQuery({
    queryKey: qk.jobs,
    queryFn: ({ signal }) => api.listJobs(signal),
    refetchInterval: (query) =>
      query.state.data?.some((j) => !isTerminal(j.status)) ? 5_000 : 30_000,
    retry,
  });
}

/**
 * A single job. `poll` enables the 2 s polling fallback used when the SSE stream is not
 * available; polling stops once the job reaches a terminal state.
 */
export function useJob(id: string | undefined, poll: boolean) {
  return useQuery({
    queryKey: qk.job(id ?? ""),
    queryFn: ({ signal }) => api.getJob(id ?? "", signal),
    enabled: Boolean(id),
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!poll) return false;
      if (data && isTerminal(data.status)) return false;
      return 2_000;
    },
    retry,
  });
}

export function useCreateJob() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ spec, projectId }: { spec: JobSpec; projectId?: string | null }) => api.createJob(spec, projectId),
    onSuccess: (job) => {
      qc.setQueryData(qk.job(job.id), job);
      void qc.invalidateQueries({ queryKey: qk.jobs });
      void qc.invalidateQueries({ queryKey: qk.projects });
      void qc.invalidateQueries({ queryKey: qk.me });
    },
  });
}

// ---- decision-support analyses of a finished study (ADR-0018) ----

const analysesKey = (id: string) => ["analyses", id] as const;

export function useAnalyses(id: string | undefined) {
  return useQuery({
    queryKey: analysesKey(id ?? ""),
    queryFn: ({ signal }) => api.analyses(id ?? "", signal),
    enabled: Boolean(id),
    retry,
  });
}

export function useCreateAnalysis(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: AnalysisRequest) => api.createAnalysis(id, body),
    onSuccess: () => void qc.invalidateQueries({ queryKey: analysesKey(id) }),
  });
}

export function useDeleteAnalysis(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (aid: string) => api.deleteAnalysis(id, aid),
    onSuccess: () => void qc.invalidateQueries({ queryKey: analysesKey(id) }),
  });
}

// ---- the organisation's own datasets (ADR-0108) ----

const datasetsKey = ["datasets"] as const;

export function useDatasets() {
  return useQuery({ queryKey: datasetsKey, queryFn: ({ signal }) => api.datasets(signal), retry });
}

export function useCoordinateSystems() {
  return useQuery({
    queryKey: ["coordinate-systems"],
    queryFn: ({ signal }) => api.coordinateSystems(signal),
    staleTime: Infinity,
    retry,
  });
}

export function useImportDataset() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (b: ImportRequest) => api.importDataset(b),
    onSuccess: () => void qc.invalidateQueries({ queryKey: datasetsKey }),
  });
}

export function useDeleteDataset() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteDataset(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: datasetsKey }),
  });
}

/** GBIF matches of a dataset's taxon values (only the names leave the server). */
export function useTaxonMatches(names: string[] | null) {
  return useQuery({
    queryKey: ["taxon-match", names],
    queryFn: () => api.matchSpecies(names ?? []),
    enabled: names !== null && names.length > 0,
    staleTime: 60 * 60_000,
    retry,
  });
}

export function useProjects() {
  return useQuery({ queryKey: qk.projects, queryFn: ({ signal }) => api.projects(signal), retry });
}

/** Project and study edits refresh both lists (counts and memberships change together). */
function useWorkspaceMutation<A, R>(fn: (a: A) => Promise<R>) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: qk.projects });
      void qc.invalidateQueries({ queryKey: qk.jobs });
    },
  });
}

export const useCreateProject = () => useWorkspaceMutation((name: string) => api.createProject(name));
export const useRenameProject = () =>
  useWorkspaceMutation(({ id, name }: { id: string; name: string }) => api.renameProject(id, name));
export const useDeleteProject = () => useWorkspaceMutation((id: string) => api.deleteProject(id));
export const useUpdateJob = () =>
  useWorkspaceMutation(({ id, ...b }: { id: string; name?: string; project_id?: string | null }) => api.updateJob(id, b));

/** The GBIF download a queued study waits for (polled every 5 s while queued). */
export function useGbifWait(id: string, queued: boolean) {
  return useQuery({
    queryKey: ["gbif-wait", id],
    queryFn: ({ signal }) => api.gbifWait(id, signal),
    enabled: queued,
    refetchInterval: queued ? 5_000 : false,
  });
}

export function useSkipDownload() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.skipDownload(id),
    onSuccess: (_: unknown, id: string) => {
      void qc.invalidateQueries({ queryKey: ["gbif-wait", id] });
      void qc.invalidateQueries({ queryKey: qk.job(id) });
    },
  });
}

export function useCancelJob() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.cancelJob(id),
    onSuccess: (job: Job) => {
      qc.setQueryData(qk.job(job.id), job);
      void qc.invalidateQueries({ queryKey: qk.jobs });
    },
  });
}

export function useResults(id: string | undefined, enabled = true) {
  return useQuery({
    queryKey: qk.results(id ?? ""),
    queryFn: ({ signal }) => api.results(id ?? "", signal),
    enabled: Boolean(id) && enabled,
    staleTime: Infinity, // results of a finished job are immutable
    retry,
  });
}

export function useManifest(id: string, enabled: boolean) {
  return useQuery({
    queryKey: qk.manifest(id),
    queryFn: ({ signal }) => api.manifest(id, signal),
    enabled,
    staleTime: Infinity,
    retry,
  });
}

export function useJobOccurrences(id: string, key: number | undefined) {
  return useQuery({
    queryKey: ["occurrences", id, key ?? -1],
    queryFn: ({ signal }) => api.occurrences(id, key ?? -1, signal),
    enabled: key !== undefined,
    staleTime: Infinity,
    retry,
  });
}

export function useSample(
  id: string | undefined,
  key: number | undefined,
  at: { lng: number; lat: number } | null,
) {
  const lng = at ? Number(at.lng.toFixed(6)) : 0;
  const lat = at ? Number(at.lat.toFixed(6)) : 0;
  return useQuery({
    queryKey: qk.sample(id ?? "", key ?? -1, lng, lat),
    queryFn: ({ signal }) => api.sample(id ?? "", key ?? -1, lng, lat, signal),
    enabled: Boolean(id) && key !== undefined && at !== null,
    staleTime: Infinity,
    retry,
  });
}
