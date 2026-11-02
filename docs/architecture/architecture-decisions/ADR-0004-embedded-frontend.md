# ADR-0004 — React frontend embedded at compile time, overridable in development

- Status: Accepted
- Evidence: `docs/research/10-frontend-and-wasm.md`

## Decision
Vite builds `web/dist` with content-hashed assets; the Rust build embeds it (`memory-serve`,
fallback `rust-embed`) with brotli precompression, immutable cache headers for hashed assets,
`no-cache` for `index.html`, SPA fallback, strict CSP (self-hosted MapLibre worker, no `blob:`
where avoidable). `NICHE_UI_DIR` serves assets from disk in development; Vite dev server proxies
`/api` to Axum. A build ID is injected into both and exposed at `/api/v1/health` so mismatches
are detectable. Mandated stack: React+TS strict, Vite, Tailwind v4, shadcn/ui (one primitive
library pinned), Lucide, TanStack Query, Zustand only for shared client state, MapLibre GL JS,
React Hook Form + Zod.

## Risks
shadcn moved its default primitives to Base UI (2026-07); pin one library app-wide.

## Implementation note (2026-10-15)

Implemented with `rust-embed` 8 (fallback option of this ADR) rather than `memory-serve`: fewer moving parts, assets served with immutable cache headers for `assets/*`, `no-cache` + CSP for `index.html`, SPA fallback, `NICHE_UI_DIR` override, build id in `/api/v1/health`. Precompression is not implemented yet (the UI is served from loopback in local mode).
