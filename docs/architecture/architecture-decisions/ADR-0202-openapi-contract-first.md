# ADR-0202: OpenAPI 3.1 contract, generated from Rust, committed and snapshot-tested

- Status: Proposed
- Date: 2026-10-15
- Related: `../engineering-principles.md` §3, ADR-0201

## Context

The React/TypeScript UI and the Rust Axum API ship together in one executable, but they are written in two languages. Server mode exposes the API to scripts and integrations. The product owner asked for contract-first API design with OpenAPI 3.1. The risks are type drift between Rust and TS, and silent breaking changes.

## Decision

1. **Spec source:** the spec is generated from the Rust handler and DTO types with `utoipa` + `utoipa-axum` (OpenAPI 3.1). DTOs live in `niche-api`, not in domain crates. Domain types are mapped to DTOs explicitly, so the domain never derives OpenAPI traits.
2. **Contract artifact:** the generated spec is committed as `api/openapi.json`. That file is the reviewed contract: any MR that changes the API shows the spec diff. API design discussions start by editing DTOs or the spec diff, before handler logic ("contract-first in review, code-first in mechanics").
3. **TS client:** `openapi-typescript` generates `web/src/api/schema.d.ts` from the committed spec, and `openapi-fetch` handles typed calls. Data-fetching hooks are thin hand-written TanStack Query wrappers [ASSUMPTION: TanStack Query chosen by the frontend research].
4. **CI drift gates:**
   - (a) Rust test `openapi_snapshot`: the generated spec must equal `api/openapi.json` byte-for-byte, after sorted-key pretty JSON. The fix is `cargo xtask openapi`.
   - (b) The frontend job regenerates TS types; `git diff --exit-code`; `tsc --noEmit`.
   - (c) The spec is validated as OpenAPI 3.1.
   - (d) Later: breaking-change detection against the last release tag [OPEN: tool choice].
5. **Versioning:** the API is under `/api/v1`. A breaking change needs `/api/v2` or an explicit deprecation once server mode has external consumers.

## Alternatives

| Option | Evidence | Why not chosen |
|---|---|---|
| `aide` 0.15.1 (OpenAPI 3.1.0 only, schemars-based) [E18] | One derive (`JsonSchema`) could serve both the API and the manifest JSON Schema | About 18× fewer recent downloads than utoipa [E17, E21]; smaller ecosystem. **Close second.** Re-evaluate if utoipa's 3.1 output proves problematic |
| Hand-written YAML spec, server types generated | Purest contract-first | Mature Rust/axum server generators are uncertain [UNVERIFIED]; two sources of truth need runtime conformance tests |
| `@hey-api/openapi-ts` 0.99.0 [E19] | Full SDK generation | Pre-1.0 and fast-moving; more generated code to review |
| `orval` 8.43.0 [E19] | React Query hooks plus mocks | Heavier config and output; couples the API layer to one data library |
| No spec; shared types via `ts-rs`/`specta` | Simple type sharing | No contract for third parties; no OGC/server-mode documentation |

## Evidence

- utoipa 6.0.0 / utoipa-axum 0.3.0, 2026-09-22, MIT OR Apache-2.0. The docs state OpenAPI 3.1 support; the module docs mention "V3.2", an inconsistency to verify during the spike [E17, FACT].
- aide 0.15.1: "Currently only Open API version 3.1.0 is supported" [E18, FACT].
- openapi-typescript 7.13.0 and openapi-fetch 0.17.0 (2026-02-11, MIT). 3.0/3.1 support per a secondary source [E19, E31].

## Trade-offs

- Code-first generation means annotations must be complete. Missing annotations produce an incomplete spec, and only review catches that. Mitigation: a test asserting that every registered route appears in the spec.
- The committed generated files add diff noise. That noise is the intended visibility.

## Risks

- utoipa's major-version churn (6.0 is recent). Pin it, and confine the macros to `niche-api`.
- The TS generator may mis-handle some 3.1 constructs (`oneOf` + `null`, `const`). Use a conservative DTO style: tagged enums, no untagged unions.

## Compliance

Gates (a)–(c) run in every pipeline. A test asserts that the route count in the Axum router equals the path-operation count in the spec.

## Reversibility

Medium-high. The committed spec is a portable artifact. Switching generators (utoipa to aide) changes annotations in one crate only, and the TS side is unaffected while the spec stays equivalent.

## Rationale

This is the lowest-cost way to get a reviewed, versioned contract with mechanical drift prevention for both languages. It adds no runtime dependency in the UI beyond a few-kB fetch wrapper.
