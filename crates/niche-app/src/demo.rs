//! SYNTHETIC demonstration data: smooth artificial predictors over a fixed region of south-eastern
//! France and a virtual species with a known response, so the product works offline on first
//! run. Nothing here is real ecological data; every output derived from it is labelled synthetic.

use niche_data::gbif::Occurrence;
use niche_eval::rng::Rng;
use niche_geo::{GridSpec, Raster};

use crate::AppError;

pub const SPECIES_KEY: u64 = 0;
pub const SPECIES_NAME: &str = "Virtualis demonstrandum (SYNTHETIC)";
/// Second virtual species (SYNTHETIC), for multi-species demonstrations (decision-support
/// outputs, ADR-0018): warmer and drier conditions than the first, partly overlapping them. Only
/// this key gets its own records; every other key keeps the first species' records, so existing
/// demo studies are unchanged.
pub const SECOND_SPECIES_KEY: u64 = 1_000_002;
pub const SECOND_SPECIES_NAME: &str = "Virtualis secundus (SYNTHETIC)";
pub const LAYERS: &[(&str, &str, &str)] = &[
    ("demo_temp", "Synthetic temperature", "°C"),
    ("demo_prec", "Synthetic precipitation", "mm"),
    ("demo_rugged", "Synthetic ruggedness", "index"),
];

/// Demo region: 3.0–6.0 °E, 43.8–45.8 °N (inland, no sea) on the CHELSA 30″ lattice.
pub const REGION: GridSpec = GridSpec {
    west: 3.0,
    north: 45.8,
    cell: 1.0 / 120.0,
    width: 360,
    height: 240,
};

fn value(id: &str, lng: f64, lat: f64) -> f32 {
    let u = (lng - 3.0) / 3.0;
    let v = (lat - 43.8) / 2.0;
    let x = match id {
        "demo_temp" => 16.0 - 9.0 * v - 4.0 * (3.0 * u).sin() * v + 1.2 * (11.0 * u).cos(),
        "demo_prec" => 650.0 + 700.0 * v + 250.0 * (4.0 * u + 1.0).sin() + 60.0 * (17.0 * v).cos(),
        _ => (0.5 + 0.5 * (9.0 * u).sin() * (7.0 * v).cos()) * (0.4 + 0.6 * v),
    };
    x as f32
}

pub fn layers(
    ids: &[String],
    _bbox: (f64, f64, f64, f64),
) -> Result<(GridSpec, Vec<(String, String, Raster)>), AppError> {
    let g = REGION;
    let mut out = Vec::new();
    for id in ids {
        let (_, name, _) = LAYERS
            .iter()
            .find(|(i, _, _)| i == id)
            .ok_or_else(|| AppError::Invalid(format!("unknown demo layer {id}")))?;
        let mut r = Raster::new(g, f32::NAN);
        for row in 0..g.height {
            for col in 0..g.width {
                let (x, y) = g.center(row, col);
                r.data[g.index(row, col)] = value(id, x, y);
            }
        }
        out.push((id.clone(), format!("{name} (SYNTHETIC)"), r));
    }
    Ok((g, out))
}

/// Whether a demo layer plays the part of a climate predictor (it changes in the synthetic
/// future scenarios); the ruggedness layer is static, like real terrain (ADR-0015, ADR-0016).
pub fn is_climate(id: &str) -> bool {
    matches!(id, "demo_temp" | "demo_prec")
}

/// SYNTHETIC warming of a demo future scenario, °C: an invented, smooth function of the period
/// and pathway (larger later and under higher emissions), scaled per model so that the five
/// model names give five different futures. Not CMIP6 output.
pub fn synthetic_warming(period: &str, ssp: &str, gcm: &str) -> Option<f64> {
    let p = niche_data::chelsa::FUTURE_PERIODS
        .iter()
        .position(|x| *x == period)?;
    let s = niche_data::chelsa::FUTURE_SSPS
        .iter()
        .position(|x| *x == ssp)?;
    let g = niche_data::chelsa::FUTURE_GCMS
        .iter()
        .position(|x| *x == gcm)?;
    const BASE: [[f64; 3]; 3] = [[0.8, 1.0, 1.1], [1.1, 2.0, 2.4], [1.2, 3.1, 4.2]];
    const MODEL: [f64; 5] = [0.8, 1.15, 0.9, 0.85, 1.3];
    Some(BASE[p][s] * MODEL[g])
}

/// SYNTHETIC future layers of the demo: temperature raised by [`synthetic_warming`] (a little
/// more in the north), precipitation lowered by 4 % per °C; other layers unchanged. Same grid
/// as [`layers`]. Labelled synthetic in every output.
pub fn future_layers(
    ids: &[String],
    period: &str,
    ssp: &str,
    gcm: &str,
) -> Result<(GridSpec, Vec<(String, Raster)>), AppError> {
    let dt = synthetic_warming(period, ssp, gcm).ok_or_else(|| {
        AppError::Invalid(format!("unknown future scenario {period} {ssp} {gcm}"))
    })?;
    let g = REGION;
    let mut out = Vec::new();
    for id in ids {
        if !LAYERS.iter().any(|(i, _, _)| i == id) {
            return Err(AppError::Invalid(format!("unknown demo layer {id}")));
        }
        let mut r = Raster::new(g, f32::NAN);
        for row in 0..g.height {
            for col in 0..g.width {
                let (x, y) = g.center(row, col);
                let v = f64::from(value(id, x, y));
                let north = 1.0 + 0.15 * (y - 43.8) / 2.0;
                let f = match id.as_str() {
                    "demo_temp" => v + dt * north,
                    "demo_prec" => v * (1.0 - 0.04 * dt),
                    _ => v,
                };
                r.data[g.index(row, col)] = f as f32;
            }
        }
        out.push((id.clone(), r));
    }
    Ok((g, out))
}

/// True relative suitability of the virtual species.
pub fn truth(lng: f64, lat: f64) -> f64 {
    let t = f64::from(value("demo_temp", lng, lat));
    let p = f64::from(value("demo_prec", lng, lat));
    let r = f64::from(value("demo_rugged", lng, lat));
    (-(t - 11.0).powi(2) / (2.0 * 2.5f64.powi(2))).exp() / (1.0 + (-(p - 850.0) / 90.0).exp())
        * (0.5 + 0.5 * r)
}

/// Synthetic "records": 160 presences sampled ∝ suitability plus deliberately problematic
/// records (old, imprecise, duplicate, non-commercial licence, outside region) to exercise and
/// demonstrate data cleaning.
pub fn occurrences(seed: u64) -> Vec<Occurrence> {
    sample(seed ^ 0xDE40, truth)
}

/// True relative suitability of the second virtual species: warmer (optimum 13.5 instead of
/// 11) and drier (declining above 900 instead of rising above 850), less tied to ruggedness.
pub fn truth_second(lng: f64, lat: f64) -> f64 {
    let t = f64::from(value("demo_temp", lng, lat));
    let p = f64::from(value("demo_prec", lng, lat));
    let r = f64::from(value("demo_rugged", lng, lat));
    (-(t - 13.5).powi(2) / (2.0 * 2.0f64.powi(2))).exp() / (1.0 + ((p - 900.0) / 120.0).exp())
        * (0.7 + 0.3 * r)
}

/// Synthetic records of a demo species: the second virtual species for
/// [`SECOND_SPECIES_KEY`], the first one for every other key.
pub fn occurrences_for(key: u64, seed: u64) -> Vec<Occurrence> {
    if key == SECOND_SPECIES_KEY {
        sample(seed ^ 0x5EC0, truth_second)
    } else {
        occurrences(seed)
    }
}

/// Name of a demo species by key.
pub fn species_name(key: u64) -> &'static str {
    if key == SECOND_SPECIES_KEY {
        SECOND_SPECIES_NAME
    } else {
        SPECIES_NAME
    }
}

fn sample(rng_seed: u64, truth: fn(f64, f64) -> f64) -> Vec<Occurrence> {
    let g = REGION;
    let mut rng = Rng::new(rng_seed);
    let cells: Vec<(f64, f64, f64)> = (0..g.height)
        .flat_map(|r| (0..g.width).map(move |c| (r, c)))
        .map(|(r, c)| {
            let (x, y) = g.center(r, c);
            (x, y, truth(x, y))
        })
        .collect();
    let max = cells.iter().map(|c| c.2).fold(0.0, f64::max);
    let mut out = Vec::new();
    let mut key = 1u64;
    let mk = |key: u64, lng: f64, lat: f64, year: i32, unc: f64, licence: &str| Occurrence {
        key,
        lng,
        lat,
        year: Some(year),
        uncertainty_m: Some(unc),
        licence: licence.into(),
        basis_of_record: "HUMAN_OBSERVATION".into(),
        dataset_key: "synthetic-demo".into(),
        generalised: None,
    };
    while out.len() < 160 {
        let i = rng.below(cells.len() as u64) as usize;
        let (x, y, s) = cells[i];
        if rng.uniform() < s / max {
            let jx = (rng.uniform() - 0.5) * g.cell * 0.8;
            let jy = (rng.uniform() - 0.5) * g.cell * 0.8;
            out.push(mk(
                key,
                x + jx,
                y + jy,
                1990 + (key % 34) as i32,
                25.0 + (key % 7) as f64 * 40.0,
                "CC0_1_0",
            ));
            key += 1;
        }
    }
    let base: Vec<(f64, f64)> = out.iter().take(12).map(|o| (o.lng, o.lat)).collect();
    for (i, (x, y)) in base.into_iter().enumerate() {
        let rec = match i % 4 {
            0 => mk(key, x, y, 1962, 30.0, "CC0_1_0"),
            1 => mk(key, x, y, 2015, 5000.0, "CC0_1_0"),
            2 => mk(key, x + 0.0005, y, 2016, 20.0, "CC0_1_0"),
            _ => mk(key, x, y, 2018, 20.0, "CC_BY_NC_4_0"),
        };
        out.push(rec);
        key += 1;
    }
    out.push(mk(key, 7.5, 44.0, 2019, 10.0, "CC0_1_0"));
    out
}

/// Demo study area: the full demo region as a GeoJSON polygon.
pub fn area_geojson() -> serde_json::Value {
    let g = REGION;
    let (w, n, e, s) = (
        g.west + 0.05,
        g.north - 0.05,
        g.east() - 0.05,
        g.south() + 0.05,
    );
    serde_json::json!({"type": "Polygon", "coordinates": [[[w, s], [e, s], [e, n], [w, n], [w, s]]]})
}
