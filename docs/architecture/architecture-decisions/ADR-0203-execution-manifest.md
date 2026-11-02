# ADR-0203: Versioned JSON execution manifest per run; RO-Crate export later

- Status: Proposed
- Date: 2026-10-15
- Related: `../execution-manifest.md` (schema v1), `../engineering-principles.md` §2, §4, §5

## Context

Reproducibility is priority #2. Consultancy deliverables must be traceable to data versions, settings, software version and randomness. The product owner asked for an execution manifest that covers dataset versions, provenance, model config, software version, seeds, validation strategy and artifact checksums. The candidate standards are Workflow Run RO-Crate, ODMAP and W3C PROV.

## Decision

1. Every run writes `manifest.json` conforming to `nichers.run-manifest/1`, defined in `../execution-manifest.md`. It is a plain JSON document whose JSON Schema is generated from Rust types (`schemars`), committed, and validated in tests (`jsonschema`).
2. Content: software build (version, commit, target, lockfile hashes, golden suite version), inputs (role, source incl. DOI, licence, `synthetic` flag, SHA-256), preprocessing steps, fully resolved model config with `config_sha256`, randomness (master seed, PRNG, derivation, stage seeds), validation strategy and metrics, outputs (role, format profile, SHA-256, licence), per-stage timings and memory, warnings and errors, and a self-integrity hash.
3. No raw coordinates, usernames, hostnames or absolute paths.
4. Field names follow PROV concepts (used, generated, derived-from) to make a later mapping mechanical.
5. MVP commands: `niche rerun <manifest>` and `niche models verify <run_dir>`.
6. **Later milestone:** export to RO-Crate 1.3 following the Process Run Crate profile, and an ODMAP-structured methods report pre-filled from the manifest. Trigger: a user need (deposit to Zenodo or data repository, client requirement) or server-mode institutional integration.

## Alternatives

| Option | Why not (for MVP) |
|---|---|
| RO-Crate JSON-LD as the native format | JSON-LD context handling and profile conformance add authoring and validation cost. Consultancy users gain nothing until they deposit. Export later is lossless if the manifest is designed for the mapping |
| W3C PROV-JSON/PROV-O graph | General-purpose graph and verbose; we need a fixed, schema-validated record |
| ODMAP as the record | A reporting protocol for humans, with free text, and no checksums or seeds. Good as a *derived* report |
| Provenance only in SQLite | Does not travel with exported results; breaks when files are moved |
| No seeds (OS randomness) | Irreproducible; unacceptable given priority #2 |

## Evidence

- RO-Crate 1.3 is the current released spec [E5, FACT].
- Workflow Run RO-Crate defines Process Run Crate, Workflow Run Crate and Provenance Run Crate; see Leo et al. 2024, doi:10.1371/journal.pone.0309210 [E4, FACT].
- ODMAP v1.0, sections Overview/Data/Model/Assessment/Prediction, Zurell et al. 2020, doi:10.1111/ecog.04960 [E15, FACT].
- PROV-O W3C Recommendation, 2013-04-30 [E16, FACT].
- `sha2` 0.11.1, `schemars` 1.2.3 and `jsonschema` 0.58.6 are current on crates.io, MIT/Apache [E21, FACT].

## Trade-offs

- A custom schema means NicheRS owns its evolution and documentation. In exchange, it is simpler, validated and fast.
- Hashing every input and output costs I/O time. A content-addressed cache amortizes it, and it is measured as part of `execution.stages`.
- Deterministic reductions (needed for "rerun gives identical hashes") may reduce parallel speed-up. This is accepted per the priority order and measured.

## Risks

- Bit-identical reruns are guaranteed only on the same build and target. Across CPUs and OSes only tolerance equivalence holds [ASSUMPTION]. The manifest records `target_triple`, `cpu_features` and `threads` so the scope of the guarantee is explicit.
- Licence propagation to outputs may be legally ambiguous [OPEN].
- Schema churn before 1.0. Mitigation: additive-only minor changes, and a major bump with a migration in `niche migrate`.

## Compliance

- Every test run validates emitted manifests against the committed schema.
- CI rerun test: demo run, then `niche rerun`, then equal output hashes on the same runner.
- Negative test: an altered artifact makes `niche models verify` fail.
- Snapshot (insta) of the manifest *shape*, with timestamps, ids and hashes redacted.

## Reversibility

High. Additive schema evolution is easy. RO-Crate can be added as an export at any time. Abandoning the custom manifest in favour of native RO-Crate would mean writing a converter for old runs, which is feasible.

## Rationale

This is the minimum that makes every map traceable and re-runnable, which directly serves the top two priorities. It does not pay JSON-LD and standards-conformance costs before a user needs them.
