//! Multi-species aggregation (ADR-0018 §3) of a study's species models (a stacked SDM).
//!
//! Over the species whose evidence level is at least the chosen minimum (default Indicative):
//!
//! - `richness.tif`: number of species with suitable conditions (relative suitability at or
//!   above each species' own P10 threshold) — "number of species with suitable conditions", not
//!   a species count on the ground: stacked binary maps overpredict richness (Calabrese et al.
//!   2014; Guisan & Rahbek 2011; D'Amen et al. 2017);
//! - `suitability_sum.tif`: sum of the species' relative suitability, an index (not an expected
//!   number of species: the surfaces are not probabilities);
//! - `contributing.tif`: species with a prediction in the cell;
//! - `hotspot.tif`: the top 10 % of the study-area cells ranked by stacked count, ties broken by
//!   the summed suitability, then by cell order (1 = hotspot, 0 = not).

use niche_geo::Raster;

use super::{HOTSPOT_SHARE, SpeciesLayers, cell_areas, threshold};
use crate::AppError;
use crate::dto::{
    EvidenceLevel, HotspotSummary, RichnessBin, RichnessResult, RichnessSpecies, Severity,
    SpeciesResult, SpeciesRunStatus, Warning,
};

pub struct RichnessOutput {
    pub result: RichnessResult,
    pub richness: Raster,
    pub sum: Raster,
    pub contributing: Raster,
    pub hotspot: Raster,
    pub warnings: Vec<Warning>,
}

fn level_name(l: EvidenceLevel) -> &'static str {
    match l {
        EvidenceLevel::Insufficient => "Insufficient",
        EvidenceLevel::Exploratory => "Exploratory",
        EvidenceLevel::Indicative => "Indicative",
        EvidenceLevel::Supported => "Supported",
        EvidenceLevel::Validated => "Validated",
    }
}

/// Species of the study, with layers for those that have a model.
pub fn compute(
    species: &[(SpeciesResult, Option<SpeciesLayers>)],
    min_evidence: EvidenceLevel,
) -> Result<RichnessOutput, AppError> {
    if species.len() < 2 {
        return Err(AppError::Invalid(
            "the multi-species aggregation needs a study with at least two species".into(),
        ));
    }
    let mut included: Vec<(&SpeciesLayers, f64)> = Vec::new();
    let mut inc_rows = Vec::new();
    let mut excluded = Vec::new();
    let mut below_level = 0usize;
    for (r, l) in species {
        let row = |reason: Option<String>| RichnessSpecies {
            species_key: r.species_key,
            scientific_name: r.scientific_name.clone(),
            evidence_level: (r.status == SpeciesRunStatus::Succeeded).then_some(r.evidence.level),
            threshold: None,
            suitable_km2: None,
            reason,
        };
        let (Some(l), true) = (l.as_ref(), r.status == SpeciesRunStatus::Succeeded) else {
            excluded.push(row(Some("no model for this species".into())));
            continue;
        };
        if r.evidence.level.rank() < min_evidence.rank() {
            below_level += 1;
            excluded.push(row(Some(format!(
                "evidence level {} is below the minimum {}",
                level_name(r.evidence.level),
                level_name(min_evidence)
            ))));
            continue;
        }
        let Some(t) = threshold(r, "p10") else {
            excluded.push(row(Some("no P10 threshold".into())));
            continue;
        };
        included.push((l, t));
        inc_rows.push(RichnessSpecies {
            threshold: Some(t),
            suitable_km2: r
                .thresholds
                .iter()
                .find(|x| x.rule == "p10")
                .map(|x| x.area_km2),
            ..row(None)
        });
    }
    if included.len() < 2 {
        return Err(AppError::Invalid(format!(
            "the multi-species aggregation needs at least two species at or above the {} level ({} found); lower the minimum evidence level to include more species, knowing their maps are less reliable",
            level_name(min_evidence),
            included.len()
        )));
    }
    let spec = included[0].0.suitability.spec;
    if included
        .iter()
        .any(|(l, _)| !super::same_grid(&l.suitability.spec, &spec))
    {
        return Err(AppError::Internal(
            "species rasters of one study on different grids".into(),
        ));
    }
    let areas = cell_areas(&spec);
    let n = spec.len();
    let mut richness = Raster::new(spec, f32::NAN);
    let mut sum = Raster::new(spec, f32::NAN);
    let mut contributing = Raster::new(spec, f32::NAN);
    let mut hotspot = Raster::new(spec, f32::NAN);
    let mut cells = Vec::new();
    let mut max_species = 0usize;
    let mut max_sum = 0.0f64;
    let mut bins: Vec<(usize, f64)> = vec![(0, 0.0); included.len() + 1];
    let mut study_km2 = 0.0;
    for (i, &area) in areas.iter().enumerate().take(n) {
        let (mut c, mut k, mut s) = (0usize, 0usize, 0.0f64);
        for (l, t) in &included {
            let v = f64::from(l.suitability.data[i]);
            if v.is_finite() {
                c += 1;
                s += v;
                if v >= *t {
                    k += 1;
                }
            }
        }
        if c == 0 {
            continue;
        }
        richness.data[i] = k as f32;
        sum.data[i] = s as f32;
        contributing.data[i] = c as f32;
        hotspot.data[i] = 0.0;
        cells.push((i, k, s));
        max_species = max_species.max(k);
        max_sum = max_sum.max(s);
        bins[k].0 += 1;
        bins[k].1 += area;
        study_km2 += area;
    }
    // Hotspots: top 10 % of cells by (count desc, summed suitability desc, cell order).
    let mut order = cells.clone();
    order.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.total_cmp(&a.2)).then(a.0.cmp(&b.0)));
    let k = ((cells.len() as f64 * HOTSPOT_SHARE).ceil() as usize).min(order.len());
    let mut hot_area = 0.0;
    let mut min_species = usize::MAX;
    for &(i, c, _) in order.iter().take(k) {
        hotspot.data[i] = 1.0;
        hot_area += areas[i];
        min_species = min_species.min(c);
    }
    let mut warnings = vec![Warning::new(
        "stacking_overprediction",
        Severity::Info,
        "Stacked species maps overpredict the number of species present in a place: each map shows where conditions are suitable, not where the species occurs, and the species are not assumed to co-occur. Read the count as \"species with suitable conditions\", not as species richness on the ground (Calabrese et al. 2014; Guisan & Rahbek 2011).",
    )];
    if below_level > 0 {
        warnings.push(
            Warning::new(
                "species_below_evidence_level",
                Severity::Info,
                format!(
                    "{below_level} species left out because their evidence level is below {} (Exploratory models are excluded by default).",
                    level_name(min_evidence)
                ),
            )
            .with("count", below_level)
            .with("level", level_name(min_evidence)),
        );
    }
    let result = RichnessResult {
        min_evidence,
        threshold_rule: "p10".into(),
        included: inc_rows,
        excluded,
        study_area_km2: study_km2,
        max_species,
        max_suitability_sum: max_sum,
        histogram: bins
            .iter()
            .enumerate()
            .map(|(s, (c, a))| RichnessBin {
                species: s,
                cells: *c,
                area_km2: *a,
            })
            .collect(),
        hotspot: HotspotSummary {
            target_share: HOTSPOT_SHARE,
            cells: k,
            area_km2: hot_area,
            fraction_of_study_area: if study_km2 > 0.0 {
                hot_area / study_km2
            } else {
                0.0
            },
            min_species: if k == 0 { 0 } else { min_species },
        },
    };
    Ok(RichnessOutput {
        result,
        richness,
        sum,
        contributing,
        hotspot,
        warnings,
    })
}

/// `richness_species.csv`: species included and left out.
pub fn species_csv(r: &RichnessResult) -> String {
    use super::{csv_field, opt_num};
    let mut s = String::from(
        "species_key,scientific_name,evidence_level,included,p10_threshold,p10_suitable_km2,reason\n",
    );
    for (inc, rows) in [(true, &r.included), (false, &r.excluded)] {
        for x in rows {
            s.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                x.species_key,
                csv_field(&x.scientific_name),
                x.evidence_level
                    .map(|l| format!("{l:?}").to_lowercase())
                    .unwrap_or_default(),
                inc,
                opt_num(x.threshold),
                opt_num(x.suitable_km2),
                csv_field(x.reason.as_deref().unwrap_or("")),
            ));
        }
    }
    s
}
