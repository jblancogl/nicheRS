# ADR-0101: Configuration precedence and separation from experiment definitions

- Status: Proposed (2026-10-15)
- Deciders: architecture (research phase)
- Related: [configuration.md](../configuration.md), [twelve-factor-compliance.md](../twelve-factor-compliance.md) factor III, tension 4

## Context

NicheRS runs as a double-clicked desktop app, a systemd/container service, and a headless CLI. 12factor
says config is "everything that is likely to vary between deploys" and should live in env vars [2].
Desktop users need a file; servers need env; CLI users need flags. Scientific results must not depend
on deployment config, and experiment definitions must be reproducible and hashable.

## Decision

1. Precedence **CLI > `NICHE_*` env > single TOML config file > built-in defaults**, merged per field
   with provenance (`cli`/`env:VAR`/`file:path:line`/`default`).
2. Implementation: `clap` 4.6.7 derive with feature `env` and **no** `default_value` (gives `Option<T>`
   for CLI-or-env, CLI winning [10]); `serde` + `toml` for the file with `deny_unknown_fields`; a
   hand-written merge into a validated `Config` struct; validation before binding/opening anything;
   exit 78 on error listing all problems.
3. Experiment definitions are **not config**: separate schema-versioned files, never overridable by
   env, canonicalised with defaults materialised, SHA-256 hashed into the reproducibility manifest.
4. Secrets only via `*_FILE` (or discouraged plain env); never in the config file or CLI flags; redacted
   in `niche config show`.

## Alternatives

| Alternative | Why not |
|---|---|
| `figment` 0.10.19 (Serialized defaults → Toml → Env, provenance metadata) [8] | Semantics fit well, but last release 2024-05-17 (~2.4 years before access date); still needs clap integration for CLI layer. Acceptable fallback if the hand-written merge grows. |
| `config` 0.15.27 [9] | Active; key-string based access, provenance per value not first-class. |
| Env only | Hostile to desktop users. |
| Multiple cascading files (/etc + user + project) | Ambiguity about which file won; 12factor warns against config-group explosion [2]. |

## Evidence

clap `Arg::env` docs: CLI value beats env, env beats `default_value` [10]. figment docs: `merge`
overrides, `join` fills gaps; values carry `Metadata` [8]. 12factor config page [2].

## Trade-offs

+ Precise provenance in `config show`; no stale dependency; clear error messages.
− ~200 lines of merge code and tests we own.

## Risks

Drift between CLI flag names, env names and file keys → generate the reference table in
configuration.md from the `Config` struct (test asserts docs match) [REC].

## Reversibility

High: the `Config` struct is the contract; swapping the merge for figment is internal.

## Rationale

Respects factor III where it matters (servers configured purely by env) while giving desktop users a
file, and isolates science from deployment (priority: reproducibility).

## References

Ids as in [twelve-factor-compliance.md § References](../twelve-factor-compliance.md#references).
