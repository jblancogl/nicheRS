//! Comparison of two studies of one species (ADR-0018 §4), e.g. random vs target-group
//! background, MaxEnt vs random forest, climate only vs climate + land cover.
//!
//! The two suitability surfaces must be on the same grid (same study-area extent and
//! resolution); otherwise the comparison is refused rather than resampled. Over the cells with a
//! prediction in both: `difference.tif` = B − A, Schoener's D of the two surfaces each
//! normalised to sum 1 (Schoener 1968; Warren et al. 2008), the Spearman rank correlation, mean
//! (absolute) difference per cell, and the agreement of the two P10 maps in km².

use niche_geo::Raster;

use super::{cell_areas, mid_rank_percentiles, threshold};
use crate::AppError;
use crate::dto::{
    BinaryAgreement, ComparedStudy, ComparisonResult, JobSpec, SettingDiff, SpeciesResult,
};

pub struct Side<'a> {
    pub job_id: &'a str,
    pub spec: &'a JobSpec,
    pub result: &'a SpeciesResult,
    pub suitability: &'a Raster,
}

pub struct ComparisonOutput {
    pub result: ComparisonResult,
    pub difference: Raster,
}

/// Refusal when the grids differ (same wording for the API and the tests).
pub fn grid_mismatch(a: &niche_geo::GridSpec, b: &niche_geo::GridSpec) -> String {
    format!(
        "the two studies are on different grids (A: {}×{} cells of {:.5}° from {:.4}° E, {:.4}° N; B: {}×{} cells of {:.5}° from {:.4}° E, {:.4}° N). A difference map needs the same study area and resolution: run the second study with the same area and resolution as the first",
        a.width, a.height, a.cell, a.west, a.north, b.width, b.height, b.cell, b.west, b.north
    )
}

pub fn compute(a: &Side<'_>, b: &Side<'_>) -> Result<ComparisonOutput, AppError> {
    let (sa, sb) = (&a.suitability.spec, &b.suitability.spec);
    if !super::same_grid(sa, sb) {
        return Err(AppError::Invalid(grid_mismatch(sa, sb)));
    }
    let spec = *sa;
    let areas = cell_areas(&spec);
    let mut diff = Raster::new(spec, f32::NAN);
    let (mut va, mut vb, mut idx) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..spec.len() {
        let (x, y) = (
            f64::from(a.suitability.data[i]),
            f64::from(b.suitability.data[i]),
        );
        if x.is_finite() && y.is_finite() {
            diff.data[i] = (y - x) as f32;
            va.push(x);
            vb.push(y);
            idx.push(i);
        }
    }
    if idx.is_empty() {
        return Err(AppError::Invalid(
            "the two studies have no cell with a prediction in common".into(),
        ));
    }
    let n = idx.len() as f64;
    let (ta, tb) = (va.iter().sum::<f64>(), vb.iter().sum::<f64>());
    let d = if ta > 0.0 && tb > 0.0 {
        1.0 - 0.5
            * va.iter()
                .zip(&vb)
                .map(|(x, y)| (x / ta - y / tb).abs())
                .sum::<f64>()
    } else {
        0.0
    };
    let spearman = pearson(&mid_rank_percentiles(&va), &mid_rank_percentiles(&vb));
    let mean_diff = va.iter().zip(&vb).map(|(x, y)| y - x).sum::<f64>() / n;
    let mean_abs = va.iter().zip(&vb).map(|(x, y)| (y - x).abs()).sum::<f64>() / n;
    let (t_a, t_b) = (threshold(a.result, "p10"), threshold(b.result, "p10"));
    let mut agree = BinaryAgreement {
        both_km2: 0.0,
        only_a_km2: 0.0,
        only_b_km2: 0.0,
        neither_km2: 0.0,
    };
    let mut common_km2 = 0.0;
    for (k, &i) in idx.iter().enumerate() {
        let ar = areas[i];
        common_km2 += ar;
        let ina = t_a.is_some_and(|t| va[k] >= t);
        let inb = t_b.is_some_and(|t| vb[k] >= t);
        match (ina, inb) {
            (true, true) => agree.both_km2 += ar,
            (true, false) => agree.only_a_km2 += ar,
            (false, true) => agree.only_b_km2 += ar,
            (false, false) => agree.neither_km2 += ar,
        }
    }
    Ok(ComparisonOutput {
        result: ComparisonResult {
            species_key: a.result.species_key,
            scientific_name: a.result.scientific_name.clone(),
            a: side(a),
            b: side(b),
            settings: settings_diff(a.spec, b.spec, a.result.species_key),
            common_cells: idx.len(),
            common_km2,
            schoener_d: d.clamp(0.0, 1.0),
            spearman,
            mean_difference: mean_diff,
            mean_abs_difference: mean_abs,
            p10_agreement: agree,
        },
        difference: diff,
    })
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    if x.len() < 2 {
        return 0.0;
    }
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx).powi(2);
        syy += (b - my).powi(2);
    }
    if sxx <= 0.0 || syy <= 0.0 {
        return 0.0;
    }
    sxy / (sxx * syy).sqrt()
}

fn side(s: &Side<'_>) -> ComparedStudy {
    let r = s.result;
    ComparedStudy {
        job_id: s.job_id.to_string(),
        name: s.spec.name.clone(),
        algorithm: r.selected.algorithm().to_string(),
        evidence_level: r.evidence.level,
        presences: r.data.presences,
        cbi_mean: r.validation.cbi_mean,
        or10_mean: r.validation.or10_mean,
        auc_mean: r.validation.auc_mean,
        validation_method: r.validation.method.clone(),
        background: r
            .background_sampling
            .as_ref()
            .map_or_else(|| "random".to_string(), |b| b.method.clone()),
        thresholds: super::rules_in_order(r).into_iter().cloned().collect(),
        extrapolated_share: r.extrapolation.novel_fraction,
        outside_aoa_share: r.aoa.as_ref().map(|a| a.fraction_outside),
        variables: r.variables_used.clone(),
    }
}

/// Settings that differ between the two specifications, as dotted JSON paths. The study name
/// and the other species are left out; the area geometry is compared as a whole.
pub fn settings_diff(a: &JobSpec, b: &JobSpec, key: u64) -> Vec<SettingDiff> {
    let to_json = |s: &JobSpec| -> serde_json::Value {
        let mut v = serde_json::to_value(s).unwrap_or_default();
        if let Some(o) = v.as_object_mut() {
            o.remove("name");
            let sp = s
                .species
                .iter()
                .find(|x| x.key == key)
                .and_then(|x| serde_json::to_value(x).ok())
                .unwrap_or(serde_json::Value::Null);
            o.insert("species".into(), sp);
            if let Some(area) = o.get_mut("area").and_then(|a| a.as_object_mut())
                && let Some(g) = area.remove("geometry")
            {
                let h = super::super::artifacts::sha256(g.to_string().as_bytes());
                area.insert(
                    "geometry_sha256".into(),
                    serde_json::Value::String(h[..16].to_string()),
                );
            }
        }
        v
    };
    let mut out = Vec::new();
    diff("", &to_json(a), &to_json(b), &mut out);
    out
}

fn diff(path: &str, a: &serde_json::Value, b: &serde_json::Value, out: &mut Vec<SettingDiff>) {
    use serde_json::Value;
    if a == b {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                diff(
                    &p,
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        (Value::Object(_), Value::Null) => {
            diff(path, a, &Value::Object(serde_json::Map::new()), out);
        }
        (Value::Null, Value::Object(_)) => {
            diff(path, &Value::Object(serde_json::Map::new()), b, out);
        }
        _ => out.push(SettingDiff {
            path: path.to_string(),
            a: a.clone(),
            b: b.clone(),
        }),
    }
}

/// `comparison.csv`: the metrics of both studies side by side, then the comparison statistics.
pub fn csv(c: &ComparisonResult) -> String {
    use super::{csv_field, num, opt_num};
    let mut s = String::from("measure,study_a,study_b\n");
    let mut row = |k: &str, a: String, b: String| {
        s.push_str(&format!(
            "{},{},{}\n",
            csv_field(k),
            csv_field(&a),
            csv_field(&b)
        ));
    };
    row("job_id", c.a.job_id.clone(), c.b.job_id.clone());
    row("study", c.a.name.clone(), c.b.name.clone());
    row("algorithm", c.a.algorithm.clone(), c.b.algorithm.clone());
    row(
        "evidence_level",
        format!("{:?}", c.a.evidence_level).to_lowercase(),
        format!("{:?}", c.b.evidence_level).to_lowercase(),
    );
    row(
        "presences",
        c.a.presences.to_string(),
        c.b.presences.to_string(),
    );
    row("cbi_mean", opt_num(c.a.cbi_mean), opt_num(c.b.cbi_mean));
    row("or10_mean", num(c.a.or10_mean), num(c.b.or10_mean));
    row("auc_mean", num(c.a.auc_mean), num(c.b.auc_mean));
    row(
        "validation_method",
        c.a.validation_method.clone(),
        c.b.validation_method.clone(),
    );
    row("background", c.a.background.clone(), c.b.background.clone());
    for rule in ["p10", "mtp", "max_sss"] {
        let f = |x: &ComparedStudy| x.thresholds.iter().find(|t| t.rule == rule).cloned();
        if let (Some(ta), Some(tb)) = (f(&c.a), f(&c.b)) {
            row(
                &format!("{rule}_threshold"),
                num(ta.threshold),
                num(tb.threshold),
            );
            row(
                &format!("{rule}_area_km2"),
                num(ta.area_km2),
                num(tb.area_km2),
            );
        }
    }
    row(
        "extrapolated_share",
        num(c.a.extrapolated_share),
        num(c.b.extrapolated_share),
    );
    row(
        "outside_aoa_share",
        opt_num(c.a.outside_aoa_share),
        opt_num(c.b.outside_aoa_share),
    );
    row(
        "variables",
        c.a.variables.join(" "),
        c.b.variables.join(" "),
    );
    for d in &c.settings {
        row(
            &format!("setting:{}", d.path),
            d.a.to_string(),
            d.b.to_string(),
        );
    }
    s.push_str(&format!(
        "common_cells,{},\ncommon_km2,{},\nschoener_d,{},\nspearman,{},\nmean_difference_b_minus_a,{},\nmean_abs_difference,{},\np10_both_km2,{},\np10_only_a_km2,{},\np10_only_b_km2,{},\np10_neither_km2,{},\n",
        c.common_cells,
        num(c.common_km2),
        num(c.schoener_d),
        num(c.spearman),
        num(c.mean_difference),
        num(c.mean_abs_difference),
        num(c.p10_agreement.both_km2),
        num(c.p10_agreement.only_a_km2),
        num(c.p10_agreement.only_b_km2),
        num(c.p10_agreement.neither_km2),
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{ModelSettings, OccurrenceFilters, SpeciesRef, StudyArea, ValidationSettings};

    fn spec(algorithm: Option<&str>) -> JobSpec {
        JobSpec {
            name: "x".into(),
            species: vec![SpeciesRef {
                key: 1,
                scientific_name: "A".into(),
                own: None,
            }],
            area: StudyArea {
                name: "a".into(),
                geometry: serde_json::json!({"type": "Point"}),
                places: vec![],
            },
            layers: vec!["bio1".into()],
            resolution_factor: 1,
            filters: OccurrenceFilters::default(),
            model: ModelSettings {
                algorithm: algorithm.unwrap_or("maxent").into(),
                ..ModelSettings::default()
            },
            validation: ValidationSettings::default(),
            seed: 1,
            demo: false,
            own_dataset: None,
            projections: vec![],
        }
    }

    #[test]
    fn settings_differences_are_dotted_paths() {
        let d = settings_diff(&spec(None), &spec(Some("rf")), 1);
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0].path, "model.algorithm");
        assert_eq!(d[0].a, serde_json::Value::Null);
        assert_eq!(d[0].b, serde_json::json!("rf"));
        assert!(settings_diff(&spec(None), &spec(None), 1).is_empty());
    }

    #[test]
    fn identical_surfaces_have_d_one_and_rho_one() {
        assert!((pearson(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]) - 1.0).abs() < 1e-12);
        assert!((pearson(&[1.0, 2.0, 3.0], &[3.0, 2.0, 1.0]) + 1.0).abs() < 1e-12);
    }
}
