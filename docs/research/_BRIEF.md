# NicheRS research brief (shared by all research agents)

NicheRS: standalone species distribution modelling (SDM) platform for Natural Solutions (French
ecological consultancy). Users: ecology professionals (not data scientists) at consultancies in
France/Europe. Must: select 1..N species, define study area, retrieve occurrences + environmental
rasters, train/validate SDMs (MaxEnt first), produce habitat suitability maps, suitable-area
estimates, uncertainty/reliability indicators, compare and export, scale to hundreds of species.
Preferred direction (to be challenged, not assumed): Rust (Axum/Tokio/Rayon) single executable
serving API + embedded React/Vite/TS/Tailwind/shadcn-ui/MapLibre frontend; optional WASM.
Priority: scientific correctness > reproducibility > measurable performance > reliable geospatial
> usable UX > deployment simplicity.

## Rules for every research document
- Required sections: Scope, Findings, Evidence and references, Alternatives, Trade-offs, Risks,
  Recommendations, Open questions.
- Every significant technical claim gets a traceable reference (URL, paper DOI, software version,
  publication date) and an access date (today is 2026-10-15).
- Tag claims as [FACT] (verified from a source you actually read), [ASSUMPTION], [OPEN] (open
  research question) or [REC] (engineering recommendation). If you could not verify something
  online, mark it [UNVERIFIED – prior knowledge].
- Never invent benchmark numbers, equivalences, licences, or crate capabilities. Verify crates on
  crates.io / docs.rs (version, last release, licence, downloads) before recommending them.
- Write in English, concise, dense, tables where useful. Markdown. No marketing tone.
- At the end of each document, add a "References" list with [n] ids; also append these entries
  to docs/research/refs/<your-doc-number>.md (one line each) so they can be merged into
  14-references.md.
