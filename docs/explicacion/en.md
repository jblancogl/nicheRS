# NicheRS explained from scratch

This document explains, assuming no prior knowledge, what scientific problem NicheRS solves, how it
solves it, how we know whether the result can be trusted, and how we got to where we are now
(20 October 2026, branch `feat/science-v1`, with the Science v1 programme complete, phases 1 to 9). If you want
the technical detail, each section points to the reference document.

Versions: [español](es.md) · [English](en.md) · [français](fr.md).

**How to read it.** If you have one minute: "On one page". If you have five: §1, the walkthrough in §2.4, the real results in §5.7 and the frequently asked
questions in §10. If you want to understand the science: §3 to §5. If you want to know where the
project stands: §7 and §8.

---

## On one page

- **What it does.** From the places where a species has been seen (GBIF) and the climate of every
  square kilometre (CHELSA), NicheRS maps where similar conditions occur.
- **What it does not do.** It does not say where the species *is*, nor give a probability: it gives
  a **relative suitability** (0 to 1) and an **area of suitable conditions**, as a band.
- **How to trust it.** Every result is tested by hiding whole regions and checking whether the
  model gets them right. The outcome is summarised as an **evidence level** (Insufficient,
  Exploratory, Indicative, Supported and, with field data, Validated) with its reasons, never as an invented percentage.
- **The engine.** MaxEnt, the reference method, rewritten in Rust and checked against the original:
  16 out of 16 cases identical. A study of one species across France takes seconds.
- **On real data.** Species with a clear climatic niche (holm oak) come out *Supported*; generalists
  (hedgehog, viper) come out *Exploratory* when only climate is used. That is the honest answer,
  not a failure.
- **Science v1 (finished on 20 October).** Added as expert options, pending review by ecologists:
  correction of observer bias, tests against chance, terrain, vegetation and water, a second method
  (Random Forest), future-climate projections, own field data with independent validation
  (*Validated* level) and decision outputs: where to survey, what a project affects, areas with many
  species and comparison of studies.
- **What is missing.** None of the new methods has yet been used end to end on a real species to
  draw a real conclusion. Before they go into a client report, they need the review by Natural
  Solutions ecologists and tests on species the ecologists know.

---

## 1. The question in one sentence

> **"Where in this area are the conditions this species likes, and how far can we trust the
> answer?"**

A real example: an ecological consultancy has to assess the impact of a wind farm in Brittany. It
needs to know where the western green lizard (*Lacerta bilineata*) might be, even though nobody has
searched for it in every plot. There is neither the time nor the money to walk every square
kilometre, so a **species distribution model** (SDM) is used: starting from the places where the
species *has* been seen and the climate of the whole area, the model estimates which other places
have similar conditions.

NicheRS is a tool (a web application) that does this end to end, for people who are not modelling
specialists.

---

## 2. The basic ideas

### 2.1 The ecological niche

Every species tolerates a range of conditions: temperature, rainfall, seasonality, altitude, type of
vegetation… A holm oak copes with hot, dry summers; a beech needs moisture. The set of conditions in
which a species can live is called its **ecological niche**. (Hence the name: *Niche* + *RS*, after
the Rust programming language.)

The central idea of SDMs is simple:

1. We look at **what conditions** exist in the places where the species has been observed.
2. We search the map for **other places with similar conditions**.

### 2.2 Why "just looking at the map of observations" is not enough

Because observations tell you where people have looked, not where the species lives:

- There are far more records near towns, roads and nature reserves than out in the countryside.
- An area with no observations may be an area without the species… or an area nobody has visited.
- There are errors: zoo animals, garden plants, fossils, misplaced coordinates.

The model tries to separate "where the conditions suit the species" from "where people have
looked". It never fully succeeds, which is why a large part of the work is **measuring and
honestly communicating** how well or how badly it does.

### 2.3 The map as a grid

Everything is done on a **grid of cells** about 1 km across (exactly 30 arc-seconds, the native
grid of the climate data). The whole of France is about 930,000 cells. For each cell we know its
conditions (mean temperature, rainfall…) and at the end the model gives it a score.

### 2.4 A study at a glance

```text
 1. Species + area          "Western green lizard, in Brittany"
        │
 2. Observations (GBIF)     thousands of records ──► cleaning ──► presence cells
        │                                            (reason for each exclusion, visible)
 3. Environment (CHELSA…)   climate of each cell (+ terrain, vegetation, water: optional)
        │                   land mask · redundant-variable filter
 4. Background              10,000 random cells to compare against
        │
 5. Model (MaxEnt)          25 configurations tested, exam by geographic blocks
        │                   ──► one chosen by fixed rules ──► final model
 6. Evaluation              exam scores, uncertainty, extrapolation
        │
 7. Result                  relative suitability map · area of suitable conditions (band)
                            evidence level + reasons · report · manifest
```

Every arrow leaves a trail: what was removed, what was chosen and why. That trail is what lets an
ecologist review the study and anyone repeat it.

---

## 3. The data we use

### 3.1 Where the species has been seen: GBIF

**GBIF** (*Global Biodiversity Information Facility*) is the world's major database of species
observations: museums, scientific programmes, citizen-science apps such as iNaturalist or
Observation.org… Each record says "this species, at these coordinates, on this date". These are
**presence-only** data: they say where the species was seen, never where it was looked for and not
found.

Before using them, NicheRS **cleans** them with fixed rules (decided in ADR-0009), and always shows
how many records it removes and why:

| Excluded | Why |
|---|---|
| Fossils, captive animals, cultivated plants | They do not reflect where the species lives in the wild. |
| Records with a "non-commercial" licence (CC BY-NC) | They cannot be used in a paid consultancy report. |
| Records with no date or from before 1981 | The climate we use is 1981–2010; a record from 1900 may belong to a different climate. |
| Coordinates with uncertainty > 1 km | If the point could be 5 km away, we do not know which cell it falls in. |
| Coordinates "generalised" (blurred) by the publisher with no stated precision (new, ADR-0011) | Sensitive species (e.g. nesting birds of prey) are published with their position deliberately shifted to protect them. If we do not know by how much, the point misleads the model. |
| Points at sea or in large lakes | We work with land species. |
| Duplicates in the same cell | Ten photos of the same lizard in the same spot are not ten lizards. One per cell is kept. |

A note on taxonomic rank: if the user picks a **genus** instead of a **species** (e.g. "Quercus"
instead of "Quercus ilex"), several species with different niches would be mixed together. The tool
warns about this and caps the confidence level (we added this on 16 October after seeing it happen
in a real case).

### 3.2 What the environment is like: CHELSA climate

To describe the conditions in each cell we use **CHELSA v2.1**, a set of global climate maps at
1 km. By default seven "bioclimatic variables" are used:

- annual mean temperature (BIO1), its seasonality (BIO4), maximum of the warmest month (BIO5),
  minimum of the coldest month (BIO6);
- annual rainfall (BIO12), rainfall of the driest month (BIO14) and its seasonality (BIO15).

In other words: heat, water, how they change over the year, and their extremes.

Why CHELSA and not WorldClim, which is better known? Because **WorldClim forbids commercial use**
without permission, while CHELSA is in the public domain (CC0). Natural Solutions is a company: all
default data must be usable in paid work (ADR-0006).

Two details we discovered along the way:

- **CHELSA has values over the sea** (for example, 13.65 °C in the middle of the Bay of Biscay). If
  the sea is not masked out, the model would compare the species with marine conditions and look
  better than it is. That is why we apply a **land mask** (Natural Earth) — ADR-0010.
- Some variables are strongly **correlated** with each other (if summers are hot, the average tends
  to be hot too). Giving the model two variables that say the same thing confuses it. A filter is
  applied: if two variables are too similar (correlation > 0.7), the first one is kept.

### 3.3 Beyond climate: terrain, vegetation and water (optional)

At 1 km, climate tells you whether a region is suitable, but not whether it contains forest,
farmland or town. That is why many generalist species came out *Exploratory*. Since phase 4 of
Science v1 (ADR-0015), twelve more variables can be added in Expert mode, all commercially licensed:

| Group | Variables | Source |
|---|---|---|
| Terrain | elevation, slope, roughness | Copernicus DEM (90 m) |
| Land cover | fraction of each cell covered by trees, shrubland, grassland, cropland, built-up, bare ground, water, wetland | ESA WorldCover 2021 (10 m) |
| Water | distance to the nearest water | ESA WorldCover 2021 |

**Fractions** are used ("this cell is 60 % forest and 30 % cropland") rather than a single class
("this cell is forest"), because at 1 km almost every cell is a mixture. All of them are computed on
the same grid as the climate, without distorting it.

By default **none is selected**: a normal study is still climate only. And each one comes with its
warnings: land cover is a 2021 snapshot whereas the observations go back to 1981; "built-up" may
reflect where the observers live more than where the species lives; the elevation model measures
the surface (tree canopies, roofs), not the ground.

### 3.4 The "background": what we compare against

Since we only know where the species *is* and not where it *is not*, the model compares the cells
with observations against a sample of **10,000 random cells** from the study area, called the
**background**. The question the model asks is not "is the species here?" but "how do the places
where the species has been seen differ from the places available in general?".

---

## 4. The model: MaxEnt

### 4.1 The idea, without formulas

**MaxEnt** (*Maximum Entropy*, Phillips et al., 2006) is the most widely used method in the world
for this problem. Its logic:

> "Of all the ways of spreading the species across the map that are consistent with what we have
> observed (for example, that the mean temperature where it has been seen is about 12 °C), choose
> the most spread-out one, the one that assumes the least."

It is a cautious way of not inventing preferences the data do not show. To describe preferences,
MaxEnt can use simple shapes (linear: "the warmer, the better") or more complex ones ("it likes
between 10 and 14 °C, no more, no less"). The more complexity we allow, the more closely it fits the
data… and the greater the risk that it memorises the noise instead of learning the rule
(**overfitting**). A parameter called **regularisation** (β) penalises complexity.

### 4.2 Why we rebuilt MaxEnt in Rust

The reference program is `maxent.jar`, written in Java, free, and frozen since 2020. Ecologists
trust it. But it requires Java, is hard to integrate into a web service and is slow when hundreds of
fits are chained together.

We decided to **rewrite (port) MaxEnt in Rust**, but under a strict condition: it must give
**exactly the same results** as `maxent.jar`. Not "similar": the same. That meant copying even the
original program's internal quirks (the order in which it adds things up, how it breaks ties,
calculations it reuses even when they are slightly out of date), because MaxEnt stops before
reaching the mathematical optimum, and only by copying its exact path do you end up in the same
place (ADR-0001).

Measured result: **16 out of 16 test cases identical** to `maxent.jar` 3.4.4, including 5 real
species, with differences of the order of 0.00000000005. Every change to the engine is checked
again against the original (`scripts/compare_maxent.py`); if it does not reach 100 %, it does not
go in.

### 4.3 Automatic tuning: choosing the best configuration

There is no MaxEnt configuration that suits every species. NicheRS tries **25 configurations**
(5 complexity levels × 5 regularisation levels) and chooses one using fixed, transparent criteria:

1. the one that **makes the fewest mistakes on data it has not seen** (omission rate, see §5);
2. if tied, the one that best **ranks** the places (Boyce index);
3. if still tied, the **simplest**.

This criterion favours cautious, "smooth" models, which work better when applied to new areas, at
the cost of looking somewhat less accurate on their own data. It is a deliberate choice.

### 4.4 What the number on the map means

The model gives each cell a value between 0 and 1. **It is not a probability of presence.** It is a
**relative suitability**: 0.8 means "conditions more similar to those at the observations than a
cell scoring 0.3", not "an 80 % chance of finding the species". With presence-only data it is
impossible to calculate a real probability, because we do not know how many times people searched
without success.

This rule is written down (ADR-0007) and is non-negotiable: no NicheRS text, map or report ever
shows "probability" or an invented "confidence percentage".

---

## 5. How do we know whether the model is good?

This is the most delicate part scientifically, and where we have put the most effort.

### 5.1 Testing with questions it has not seen: cross-validation

If you test a student with the same questions they studied, they will get full marks even if they
understood nothing. Models are the same. So **part of the data is hidden**, the model is trained on
the rest, and we check whether it gets the hidden part right. This is repeated several times,
changing the hidden part each time: that is **cross-validation**.

### 5.2 Hiding by area, not at random: spatial blocks

If we hide points at random, each hidden point has an almost identical neighbour in the training
data (500 m away, with the same climate), and the test is too easy. So the territory is split into
**4 geographic blocks** (cutting at the median latitude and longitude) and a whole block is hidden
each time. This checks whether the model can **transfer to another area**, which is exactly what it
is asked to do in practice.

### 5.3 The test scores

- **10 % omission rate (OR10).** A threshold is set that leaves out the 10 % of training
  observations with the lowest scores, and we look at what share of the *hidden* observations falls
  below it. If the model is not overfitted, this should be ≈ 0.10. If it is 0.40, the model fails to
  recognise new places where the species actually is.
- **Boyce index.** Checks whether the areas the model scores highly really contain more
  observations than chance would give. It ranges from −1 to 1: 0 is "like tossing a coin", close to
  1 is "very consistent".
- **AUC.** This is the most famous metric, but with presence-only data it mainly measures **how big
  the study area is**, not the quality of the model. We show it in the details, but we do not use it
  to decide anything.

### 5.4 Uncertainty and extrapolation

- **Uncertainty map:** how much a cell's prediction changes between the cross-validation models. If
  it changes a lot, that result depends on which data were used.
- **Extrapolation (MESS):** areas whose climate is **more extreme than any of the data used** (e.g.
  summits colder than any observed point). There the model is guessing. They are marked with
  hatching on the map, on by default, and the percentage of area affected is reported.

### 5.5 Thresholds and area of suitable conditions

To say "there are X km² of suitable conditions" you have to set a cut-off: above it, "suitable".
There is no single correct cut-off, so we give a **band from three well-known rules** (the most
inclusive one, the 10 % one — used by default — and the one that balances hits and errors). Areas
are calculated with the exact formula for each cell's area on the Earth's ellipsoid. We always say
"area of suitable conditions", never "area occupied by the species".

### 5.6 The evidence level: the honest summary

Instead of a confidence percentage, every result carries an **evidence level** computed from
written rules (ADR-0009), together with the list of reasons behind it:

| Level | What it means | Conditions (all of them) |
|---|---|---|
| **Insufficient** | No model is fitted. | Fewer than 10 presence cells. |
| **Exploratory** | Good for orientation, not for decisions. | There is a model, but it fails at least one "Indicative" criterion. |
| **Indicative** | Usable with caution. | ≥ 25 cells; Boyce ≥ 0.40; OR10 ≤ 0.25; ≤ 25 % of the area extrapolated. |
| **Supported** | The best that can be said with these data. | ≥ 50 cells; Boyce ≥ 0.70; OR10 ≤ 0.15; ≤ 10 % extrapolated; validation by spatial blocks. |
| **Validated** | The map is right on field data it never saw. | Everything in "Supported", plus the organisation's own field data **held out** from training: ≥ 30 presence and ≥ 30 absence sites; presence–absence AUC ≥ 0.75; TSS ≥ 0.40. |

The **Validated** level (phase 7, ADR-0108) is the only one that does not depend solely on the
records the model was trained on: it requires comparing the map with the organisation's
**independent** field data (sites where the species was looked for and either found or not found),
explained in §8. When no such data are provided, the result says so explicitly: "no independent
validation data: the evidence comes from cross-validation on the same, possibly biased, records".

Expected consequence: many species with opportunistic data will come out *Exploratory* or
*Indicative*. For example, the holm oak (*Quercus ilex*) in France comes out *Supported*; the asp
viper (*Vipera aspis*), a generalist species, comes out *Exploratory*. That is not a failure of the
tool: it is the honest answer.

### 5.7 What ten real species taught us

On 15 October ten species were modelled across mainland France at 1 km, with climate only and the
default settings (`docs/benchmarks/g6-france-batch.md`). At most 20 000 records per species were
used, sampled evenly across the territory:

| Species | Level | Boyce | OR10 | Area of suitable conditions (10 % threshold) |
|---|---|---|---|---|
| Holm oak (*Quercus ilex*) | Supported | 0.74 | 0.12 | 110 166 km² |
| Red squirrel (*Sciurus vulgaris*) | Supported | 0.84 | 0.11 | 372 868 km² |
| Stag beetle (*Lucanus cervus*) | Supported | 0.73 | 0.14 | 378 774 km² |
| European beech (*Fagus sylvatica*) | Indicative | 0.90 | 0.19 | 330 204 km² |
| Spiny toad (*Bufo spinosus*) | Indicative | 0.62 | 0.06 | 289 513 km² |
| Agile frog (*Rana dalmatina*) | Indicative | 0.60 | 0.11 | 290 522 km² |
| Fire salamander (*Salamandra salamandra*) | Exploratory | 0.32 | 0.09 | 428 067 km² |
| European hedgehog (*Erinaceus europaeus*) | Exploratory | 0.27 | 0.06 | 408 759 km² |
| Asp viper (*Vipera aspis*) | Exploratory | 0.23 | 0.10 | 469 518 km² |
| Western green lizard (*Lacerta bilineata*) | Exploratory | 0.17 | 0.05 | 431 716 km² |

Three lessons:

1. **Species with a clear climatic niche do well; generalists do not.** The holm oak is
   Mediterranean and climate explains it. The hedgehog or the viper live almost everywhere in
   France: climate at 1 km cannot tell where they are, and the tool does not hide it. They need
   other variables (land cover, §3.3) or field data.
2. **One good score is not enough.** The beech has the best Boyce index (0.90), but its OR10 (0.19)
   shows that when a whole region is hidden, the model misses too many observations: it struggles
   to transfer to climates it has not seen. That is why it stops at *Indicative*.
3. **The amount of data changes the result.** Going from 20 000 records to every available record
   (with GBIF bulk downloads) raised the hedgehog's Boyce index from 0.27 to 0.80 and the western
   green lizard's from 0.17 to 0.70 (`docs/benchmarks/gbif-batch-downloads.md`). More records also
   carry more observer bias, which is why the bias warnings stay.

The areas are areas of **suitable conditions**, not of presence: the western green lizard having
431 716 km² of suitable conditions does not mean it occupies all of that area.

---

## 6. What NicheRS delivers

At the end of a study the user gets:

- the **relative suitability map**, with uncertainty and extrapolation layers;
- the **area of suitable conditions, as a band**;
- the **evidence level** with its reasons and all the warnings (warnings can never be hidden);
- the **importance of each variable** and the **response curves** ("above what temperature it
  likes it less");
- a printable **methods report**, with bibliographic references;
- exports (GeoTIFF, GeoJSON, the model in MaxEnt's `.lambdas` format);
- an **execution manifest**: a file recording exactly which data (with their checksums and the DOI
  of the GBIF download), which versions, which random seed and which settings were used. With it,
  anyone can repeat the study and get the same result, bit for bit.

---

## 7. How we got here

### Phase 0 — Research before coding (15 Oct 2026)

Before writing any code, a review of the state of the art was carried out (14 documents in
`docs/research/`): which methods work, which tools exist (MaxEnt, biomod2, ENMeval, Wallace…), which
data can legally be used, which language to choose. Main conclusions:

- MaxEnt remains a solid reference; the second engine should be a balanced *Random Forest* (see
  phase 1b).
- The main risks are **not technical**: they are data bias, people reading suitability as
  probability, silent extrapolation, and data licences.
- Measurable "gates" were set: if the Rust engine did not reproduce `maxent.jar`, plan B was to use
  `maxent.jar` itself.

The MaxEnt port was demonstrated the same day (11/11 synthetic cases identical; later 16/16 with
real species).

### MVP — The complete minimum product (15–16 Oct)

The whole pipeline was built: GBIF and CHELSA downloads, cleaning, model, validation, maps, report,
batches of many species, jobs that resume if the power goes out. The default scientific decisions
(cleaning, selection, evidence levels) were fixed in ADR-0009. All 14 of the MVP's "done" criteria
were met (`docs/mvp-status.md`).

Real measurements: a complete study of one species across the whole of France at 1 km takes
**5–12 s** on an 8-core laptop. A batch of 10 species went from **22.5 min** (GBIF search API) to
**3 min 10 s** on the server once the GBIF data were prepared (the first download depends on the
GBIF queue). For 100 species there is only an extrapolation (~25 min of modelling), **not a
measurement**.

### A product others can use (16–19 Oct)

What turned the prototype into something a team can use, without changing the science:

- accounts, organisations, projects and quotas; an administration console;
- interface and help in **English, French and Spanish**;
- two ways to create a study: **Quick** (three steps, no modelling knowledge needed) and
  **Expert**; a simplified reading of the results for non-specialists;
- study areas by name (municipalities, departments, official protected areas);
- a **regional GBIF mirror** for France and Spain: records arrive in seconds instead of waiting for
  the GBIF queue (ADR-0114). This mirror also stores how many observations of each species group
  there are in each cell — something later used to correct bias (§8);
- privacy (no observer names in the cache), zero-downtime deployments, error reports.

### Scientific review (20 Oct)

On 20 October the code was reviewed and it was found that **the scientific core had not changed
since the MVP**, while its weaknesses for professional use were precisely scientific:

1. **sampling bias** is not corrected (it partly models where people look);
2. only **climate** is used (no terrain, vegetation or water);
3. there is no **independent validation**;
4. the output is a map per species, not an answer to the **client's decision** (where to sample?
   does this project have an impact?).

This gave rise to the **Science v1** programme (`docs/science-v1-plan.md`), which is where we are
now.

---

## 8. Science v1: what has been added

### Programme rules

- **Defaults do not change.** Every new method is **optional** (Expert mode). A study with the
  default settings gives results **bit-for-bit identical** to before; we have checked this by
  comparing files.
- **The MaxEnt engine is not touched.** Equivalence with `maxent.jar` remains at 100 %.
- Every new method has an ADR with the status **"Proposed — pending review by Natural Solutions
  ecologists"**. Making it a default will be a later decision, after that review.

### Phases

| Phase | What it is | Status |
|---|---|---|
| 1a | Evaluation building blocks: distance thinning, grid blocks, spatial autocorrelation, area of applicability, null models, presence–absence metrics | Done |
| 1b | Second engine: balanced *Random Forest* | Done |
| 1c | Data: generalised-coordinate flags, sampling-effort grids, taxonomic group, CHELSA CMIP6 future climate | Done |
| 1d | Predictors: terrain (Copernicus DEM), land cover (ESA WorldCover), distance to water, projection changes | Done |
| 2 | **Bias and cleaning**: generalised coordinates, target-group background, effort layer, thinning | Done (ADR-0011, ADR-0013) |
| 3 | **Evaluation**: null models, autocorrelation-based blocks, area of applicability | Done (ADR-0014) |
| 4 | **Predictors**: terrain, land cover and water as selectable variables (§3.3) | Done (ADR-0015) |
| 5 | **Future climate**: projections, change maps and agreement between climate models | Done (ADR-0016) |
| 6 | **Second engine in the application**: Random Forest and MaxEnt + RF ensemble | Done (ADR-0017) |
| 7 | **Own field data**: import, independent validation and the *Validated* level | Done (ADR-0108) |
| 8 | **Decision outputs**: survey priority, project footprint, several species, comparison of studies | Done (ADR-0018) |
| 9 | Documentation: MVP status, executive summary, roadmap, French note for the ecologists | Done |

### The new methods, explained

**Null models (phase 3).** Is the model better than chance? The same model, with the same settings,
is trained on points placed **at random** (20, 50 or 100 times) and compared: if the real model does
not beat the random ones in at least 95 % of cases, the evidence level drops to *Exploratory*. It is
like checking that a fortune-teller does better than someone guessing.

**Autocorrelation-based blocks (phase 3).** The 4 median blocks have an arbitrary size. The correct
approach is for blocks to be at least as large as the distance over which the climate "resembles
itself" (**spatial autocorrelation**). The size can now be calculated from the climate data
themselves (a variogram), and the territory is divided into a grid of blocks of that size.

**Area of applicability (AOA, phase 3).** MESS extrapolation looks at each variable separately. The
AOA also looks at **new combinations**: a place can have a normal temperature and normal rainfall,
but a combination of the two that does not appear in any of the data (for example, heat with very
heavy rain). It is shown as another map layer.

**Generalised coordinates (phase 2, ADR-0011).** Sensitive species are published with their
position deliberately shifted (iNaturalist up to ~27 km, PatriNat 5 km). Previously, if a record
did not state its uncertainty, it was used as if it were exact. Now, if the publisher indicates that
it blurred the position and does not say by how much (or says more than 1 km), the record is
excluded. Limitation: only the GBIF search API provides this information; bulk downloads and the
regional mirror do not, and the tool warns about this. Tested with real data: in a study of
Hermann's tortoise (*Testudo hermanni*) in Provence, 339 of 1,509 records were flagged as
generalised; the tool counted them, warned and explained the reason for each exclusion, without
changing the number of presences or the evidence level in that case.

**Target-group background (phase 2, ADR-0013).** This is the sampling-bias correction most
recommended in the literature (Phillips et al., 2009). Instead of comparing the observations with
random cells, they are compared with cells **where other species from the same group have been
observed** (e.g. other birds if we are studying a bird). That way, presences and background share
the same bias ("people walk near towns") and that bias partly cancels out. The data come from the
regional mirror's effort grids. If there are not enough data, the study falls back to a random
background **with a warning**: a study never fails because the correction is unavailable.

**Distance thinning (phase 2).** If there are 200 observations in a single park and 5 in the rest
of the region, the model learns "the park". Thinning keeps only observations that are at least a
given distance apart (e.g. 5 km).

**Balanced Random Forest (phases 1b and 6, ADR-0017).** A second method, very different from MaxEnt:
thousands of "decision trees" ("if temperature > 12 °C and rainfall < 800 mm, then…") that vote. The
standard version performs poorly with presence-only data; the **balanced** version (each tree sees
as many background points as presence points) is among the best methods in published comparisons
(Valavi et al., 2022). As there is no reference program that can be copied bit for bit, it is
validated with synthetic species whose true niche we know: the model recovers the important
variables and the correct optima. Having two different methods also shows **where they disagree**,
which is useful information.

Since phase 6, Expert mode lets you choose the algorithm: **MaxEnt** (recommended, the default),
**Random Forest**, or an **ensemble** of the two that averages their rankings of the cells. All
three go through exactly the same exam (same blocks, same scores, null models, extrapolation,
evidence level), so they can be compared. Instead of response curves, the Random Forest shows
*partial dependence*, which reads the same way ("how suitability changes as temperature varies").
On the synthetic demo, with spatial blocks, the Boyce index was 0.53 for MaxEnt, 0.47 for Random
Forest and 0.52 for the ensemble; the study took 5.0 s, 2.7 s and 7.9 s. This is only a test on
synthetic data: **it has not yet been compared on real species**. Trees are known to extrapolate
worse than MaxEnt outside the observed conditions, and the Random Forest result varies somewhat with
the random seed (Boyce 0.51 vs 0.47 with two seeds on the demo), which the tool does not yet show
to users.

**Terrain, vegetation and water (phase 4, ADR-0015).** Explained in §3.3. Tested on a real clip of
Provence (about 1,000 km²): preparing the layers takes about 12 s the first time and hundredths of
a second afterwards, thanks to the cache. What **has not yet been measured** is how much models of
real species improve when land cover is added; that is the next pending check.

**Future climate (phase 5, ADR-0016).** The fitted model is applied to the future climate of
CHELSA CMIP6: three periods (2011–2040, 2041–2070, 2071–2100), three emission scenarios (from low to
very high) and up to five different climate models; using all five is recommended, because the
spread between them is part of the answer. Only climate changes: terrain, vegetation and water stay
as they are today, and the tool says so. It works with all three algorithms. For each scenario you
get:

- a **change map** with four classes: suitable now and later, **newly suitable**, **no longer
  suitable**, and unsuitable in both, using the same threshold as today (the 10 % one) so the
  comparison is fair. On the map, areas where most climate models do not agree are shown pale;
- **how many climate models agree** in each cell, and the area on which **all** of them agree: if
  all five say an area is lost, the conclusion is stronger than if only one says so;
- the **spread between models** and the **future extrapolation** (hatched on the map): areas whose
  future climate resembles nothing the model has seen. Above 10 % of the area there is a warning;
- the area of each class, with the range across climate models, in a "Future" tab of the results
  and in the report.

Measured cost: on the demo, projecting one scenario with five climate models adds 0.2 s to a study
with MaxEnt and 2.2 s with the ensemble. Reading CHELSA's future layers for a real clip of Provence
took 1.8 s. What **has not been done yet**: projecting a real species end to end (the real-data
test used a virtual species on real layers), a national-scale study, and confirming with CHELSA the
licence of its future layers (attribution is given anyway). The evidence level refers to the
present model; **it does not cover the projections**.

**Own field data and independent validation (phase 7, ADR-0108).** Consultancies hold their own
surveys, often in Lambert-93 and often with **absences**: sites visited where the species did not
turn up. They can now be imported as a CSV file. The browser reads it and recognises separators,
decimal commas, dates and a closed list of coordinate systems (WGS84, Lambert-93, LAEA Europe and
UTM). The data belong to the organisation: no other organisation can see them, and neither the
manifest nor the reports contain their coordinates.

For each species you choose how to use them:

- **GBIF + own presences** for training;
- **own presences only**, without querying GBIF;
- **GBIF for training and own data held out for checking**. This is the option that gives access to
  the *Validated* level.

The check compares the map with the field sites: does it score higher where the species was found
than where it was not? It computes the **presence–absence AUC** (meaningful here, because there are
real absences), **sensitivity** (the share of presences it recognises), **specificity** (the share
of absences it recognises) and **TSS**, which combines the two. Thresholds are set from the training
data, never tuned on the checking data. One rule prevents cheating: a checking site in a cell already
used for training is discarded, because the model already knows that answer (this happens, for
example, with SINP observations that are also published in GBIF).

Tested with a **synthetic** survey of the demo species, in Lambert-93: 105 presence sites and 193
absence sites remained, and 2 were discarded by the rule above. The result was AUC 0.84 and TSS 0.53,
but the level stayed at *Indicative*, because the model did not meet every *Supported* criterion:
providing field data does not hand out the level. **It has not yet been tested with a real client
survey** (none may be included in the repository). Nor does it model that an absence may be a
species that was there and was not seen.

**Decision support (phase 8, ADR-0018).** A map per species does not answer the client's
questions. A new results tab, "Decision support", computes four outputs on demand from a finished
study, without refitting any model or touching its files:

- **Survey priority**: areas worth visiting because they are suitable and little sampled (according
  to the target-group effort or, failing that, the distance to the nearest known record). It
  proposes ranked candidate areas. It does not say where the species is.
- **Project footprint**: upload or draw the outline of a project (a wind farm, a road) and get, with
  exact areas, how much suitable area it touches for each species, what share of the footprint that
  is, and whether that area is extrapolated.
- **Several species**: how many species find suitable conditions in each cell and where they
  concentrate ("hotspots", the 10 % of cells with most species). Only species at *Indicative* level
  or above are included, with a warning that adding maps up tends to overestimate.
- **Compare with…**: two studies of the same species and area (for example, with and without bias
  correction), with a difference map and similarity measures.

All of them carry the warning that they **do not replace a field survey or an impact assessment**,
and that low suitability is not evidence of absence. Each analysis keeps its own traceability
record, linked to the study's. Tested with two virtual demo species; **not tested on real species**.

### What comes after Science v1

The programme is finished, but **no new option has been validated**. The next step is not more
programming, but:

1. **The review by Natural Solutions ecologists** of each proposed ADR (and of ADR-0009, which has
   not been reviewed either): which species group to use as the bias reference, the evidence-level
   thresholds, the *Validated* criteria, how survey priority is computed… The French note
   the design notes gathers these questions.
2. **Testing on real species** the ecologists know: every Science v1 figure comes from the synthetic
   demo, synthetic tests or small clips of Provence.
3. **Confirming two licences**: CHELSA's future layers and the Copernicus elevation model.
4. Only then, making what is approved a default, through a new ADR.

---

## 9. How we work to avoid getting it wrong

A tool like this can produce beautiful, wrong maps. These are the rules that prevent it
(`AGENTS.md`):

- **Scientific correctness comes before everything**, including speed and ease of use.
- **Every scientific decision is written down** in an ADR, with its bibliographic justification and
  its known cost. No default changes "without saying so".
- **New things stay optional until ecologists review them.** And we check that switching nothing new
  on leaves the results identical, file by file.
- **The engine is compared with the original** on every change; a tolerance is never loosened to
  make a test pass.
- **No warning is hidden** to simplify the interface (data quality, extrapolation, too few samples,
  bias).
- **Demo data are synthetic and say so**; they are never presented as real.
- **No speeds are claimed without measuring them**, and every phase ends with a list of what was
  verified and what was **not**.

---

## 10. Frequently asked questions

**Does the map show where the species is?** No. It shows where conditions resemble those of the
places where it has been seen. The species may be absent from a suitable area (it has not got there,
hunting, competition) and occasionally turn up in an unsuitable one.

**Why doesn't it give a reliability percentage?** Because any percentage would be made up: with
presence-only data there is no honest way to calculate it. Instead, the evidence level says which
criteria are met and which are not.

**My species comes out "Exploratory". Is the study wrong?** Not necessarily. It usually means few
data, a generalist species that climate does not explain well, or a large extrapolated area. The
reasons appear next to the level. Adding land cover, correcting bias or providing field data can
improve it; forcing the result cannot.

**Why not just use `maxent.jar` directly?** Because it requires Java, does not integrate well into
a web service and is slower when hundreds of fits are chained together. We still use it as the
**judge**: the NicheRS engine must give the same numbers.

**MaxEnt or Random Forest?** MaxEnt by default: it is the method checked against the reference and
the one that behaves best in new areas. Random Forest and the ensemble are expert options, useful to
see whether two very different methods reach the same conclusion; if they disagree, that is a sign
to be careful.

**Does the future map say where the species will live in 2070?** No. It says where climatic
conditions similar to today's (where the species is seen) will occur, according to each climate
model. It does not know whether the species can get there, nor how land use or other species will
change. And where the future climate resembles nothing known (hatched areas), the model is guessing.

**What do I gain by providing my own field data?** Two things. More presences for training, often
of better quality than GBIF's. And above all, if they are held out for checking, an independent test
of the map, which is the only thing that can take a result to the *Validated* level. Absences (sites
visited where the species was not found) are the most valuable part.

**Can I already use the new options in a client report?** Not yet. They are implemented and
technically checked, but not reviewed by ecologists or tested on real species. The default settings
(those of the MVP) are the ones that can be used today.

**Can I use the results in a report for a client?** The default data are commercially licensed and
"non-commercial" records are excluded. The report includes methods, sources and references. For
sensitive species, the regional reference list must be checked before publishing points (the report
reminds you).

**Why are so many options "pending review"?** Because they are methods recognised in the
literature, but their specific settings (thresholds, reference species group, distances) must be
validated by a Natural Solutions ecologist before becoming a default.

---

## 11. What NicheRS does not do (yet) — honest limits

- **Land** species only; the climate is the 1981–2010 average.
- By default models use **climate only**; terrain, vegetation and water are optional and their
  effect on real species has not yet been measured.
- The target-group bias correction has only been tested offline (test data and synthetic demo), not
  with the real regional mirror.
- Random Forest and the ensemble have only been tested on the synthetic demo, not on real species.
- Future-climate projections have not yet been tested end to end on a real species, do not include dispersal or land-use change, and the licence of CHELSA's future layers is still to be confirmed.
- **No new Science v1 method has been used end to end on a real species.** The only exception is the generalised-coordinates rule.
- Decision outputs have only been tested on virtual species, and their warnings appear in English only in the interface.
- Independent validation and the *Validated* level have only been tested with synthetic field data; import accepts CSV only (not GeoNature exports, Excel, GeoPackage or Shapefile).
- The bias corrections are optional and **pending review** by Natural Solutions ecologists, as are
  the evidence-level cut-offs (0.40 / 0.70 for Boyce, etc.), which are reasoned but debatable
  decisions.
- The speeds quoted are from specific machines and are not extrapolated without measuring.
- The result is always **relative suitability**: it does not say where the species is, but where
  conditions resemble those of the places where it has been seen.

---

## 12. Glossary

| Term | Meaning |
|---|---|
| **SDM** | Species distribution model: estimates where there are suitable conditions for a species. |
| **Ecological niche** | The set of environmental conditions in which a species can live. |
| **GBIF** | Global database of species observations. |
| **Presence / presence-only** | An observation of the species at a place. "Presence-only": we do not know where it was looked for without success. |
| **Presence–absence** | Field data that also record whether the species was *not* there. They allow independent validation. |
| **Sensitivity / specificity** | The share of field presences / absences the map recognises. |
| **TSS** | Sensitivity + specificity − 1. 0 is chance, 1 is perfect. |
| **Validated** | Highest evidence level: the map passes a check against independent field data. |
| **Survey priority** | Suitable, little-sampled areas where it is worth looking first. |
| **Project footprint** | The outline of a project (wind farm, road…) overlaid on the maps. |
| **Hotspot** | An area where many species find suitable conditions at the same time. |
| **CHELSA** | Global 1 km climate maps, in the public domain. |
| **Emission scenario (SSP)** | An assumption about how much greenhouse gas will be emitted; from low (SSP1-2.6) to very high (SSP5-8.5). |
| **Climate model (GCM)** | A simulation of the Earth's climate; different models give somewhat different futures, and that difference is part of the uncertainty. |
| **Copernicus DEM / ESA WorldCover** | European maps of terrain (90 m) and land cover (10 m), usable commercially. |
| **Fractional cover** | The share of each cell occupied by each type of cover (forest, cropland…). |
| **Bioclimatic variable (BIO1…)** | A climate summary useful for biology (mean temperature, rainfall of the driest month…). |
| **Background** | A sample of cells from the area against which the presences are compared. |
| **MaxEnt** | The reference modelling method for presence-only data. |
| **Regularisation (β)** | A penalty on model complexity to avoid overfitting. |
| **Overfitting** | The model memorises the data (including the noise) instead of learning the general rule. |
| **Relative suitability** | A 0–1 score of how favourable conditions are. It is not a probability. |
| **Cross-validation** | Hiding part of the data, training on the rest and checking whether the model gets the hidden part right. |
| **Spatial blocks** | Hiding whole geographic areas, so that the test is not too easy. |
| **OR10** | The share of hidden observations the model leaves out at the 10 % threshold. Ideally ≈ 0.10. |
| **Boyce index** | Measures whether highly scored areas have more observations than expected by chance. From −1 to 1. |
| **AUC** | A classic metric; with presence-only data it depends mainly on the size of the area. Informative only. |
| **MESS** | Detects areas where a variable's values fall outside the range of the data (extrapolation). |
| **AOA** | Area of applicability: also detects combinations of variables never seen before. |
| **Null model** | A model trained on random points, to check that the real model is better than chance. |
| **Spatial autocorrelation** | Nearby things are alike: two places 1 km apart have almost the same climate. |
| **Sampling bias** | There are more data where people look more, not where there is more of the species. |
| **Target-group background** | Background taken from where similar species have been seen, so that it has the same bias as the presences. |
| **Distance thinning** | Removing observations that are too close to each other. |
| **Random Forest** | A method based on many decision trees that vote; NicheRS's second engine. |
| **Evidence level** | A rule-based summary (Insufficient / Exploratory / Indicative / Supported) of how far to trust the result. |
| **Execution manifest** | A complete record of data, versions and settings so the study can be repeated. |
| **ADR** | *Architecture Decision Record*: a document that puts a decision, its rationale and its cost in writing. |

## Further reading

- Overview: `docs/executive-summary.md`
- Default scientific decisions: `docs/architecture/architecture-decisions/ADR-0009-ecological-defaults-and-evidence-levels.md`
- How confidence is communicated: `ADR-0007-output-semantics-and-confidence.md`
- Science v1 programme: `docs/science-v1-plan.md` (summary at the top) and ADRs 0011, 0013, 0014, 0015, 0016, 0017, 0018 and 0108
- Note for Natural Solutions ecologists (in French): the design notes
- State of the art and bibliography: `docs/research/01-sdm-state-of-the-art.md`, `docs/research/14-references.md`
- MaxEnt equivalence: `docs/benchmarks/phase3-results.md`
