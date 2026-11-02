//! Environmental stack on the modelling grid, study-area/land masks and occurrence cleaning.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use niche_data::{DataError, Http, chelsa, gbif, landmask, predictors};
use niche_eval::rng::Rng;
use niche_eval::thin_by_distance_reps;
use niche_geo::mask::{Rings, bbox, polygons_from_geojson, rasterize_polygons};
use niche_geo::{GridSpec, Raster, area};
use serde::Serialize;
use serde_json::json;

use crate::AppError;
use crate::dto::{
    ExclusionCount, GridInfo, LayerInfo, OccurrenceFilters, OccurrencePoint, Severity, StudyArea,
    Warning,
};

/// Hard limit on modelling-grid cells (memory and latency budget of a local instrument).
pub const MAX_CELLS: usize = 6_000_000;

pub struct EnvLayer {
    pub id: String,
    pub name: String,
    pub data: Vec<f32>,
}

pub struct EnvStack {
    pub spec: GridSpec,
    pub area_mask: Vec<bool>,
    pub land_mask: Vec<bool>,
    /// area ∧ land ∧ all layers finite: cells where a prediction is defined.
    pub valid: Vec<bool>,
    pub layers: Vec<EnvLayer>,
    pub row_area_km2: Vec<f64>,
    pub provenance: serde_json::Value,
}

impl EnvStack {
    pub fn cell_area(&self, idx: usize) -> f64 {
        self.row_area_km2[idx / self.spec.width]
    }

    pub fn valid_area_km2(&self) -> f64 {
        self.valid
            .iter()
            .enumerate()
            .filter(|(_, v)| **v)
            .map(|(i, _)| self.cell_area(i))
            .sum()
    }

    pub fn grid_info(&self) -> GridInfo {
        let s = self.spec;
        GridInfo {
            bounds: [s.west, s.south(), s.east(), s.north],
            width: s.width,
            height: s.height,
            cell_size_deg: s.cell,
            cells_in_area: self.valid.iter().filter(|v| **v).count(),
            area_km2: self.valid_area_km2(),
        }
    }

    pub fn values_at(&self, idx: usize) -> Vec<f64> {
        self.layers.iter().map(|l| f64::from(l.data[idx])).collect()
    }
}

pub fn area_polygons(area: &StudyArea) -> Result<Vec<Rings>, AppError> {
    let polys = polygons_from_geojson(&area.geometry)
        .map_err(|e| AppError::Invalid(format!("study area: {e}")))?;
    let vertices: usize = polys.iter().flatten().map(Vec::len).sum();
    if vertices > 200_000 {
        return Err(AppError::Invalid(
            "study area has more than 200 000 vertices; simplify it first".into(),
        ));
    }
    Ok(polys)
}

fn union_bbox(polys: &[Rings]) -> Option<(f64, f64, f64, f64)> {
    polys
        .iter()
        .filter_map(bbox)
        .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
}

/// Display group of a predictor (ADR-0015): `climate`, `terrain`, `land_cover` or `water`.
pub fn layer_group(id: &str) -> &'static str {
    if id.starts_with("dem_") || id == "demo_rugged" {
        "terrain"
    } else if id.starts_with("lc_") {
        "land_cover"
    } else if id == "dist_water" {
        "water"
    } else {
        "climate"
    }
}

/// Short licence name for display ("Copernicus WorldDEM-90 licence", "CC BY 4.0"); the full
/// text stays in the provenance.
fn licence_name(licence: &str) -> &str {
    licence.split(" (").next().unwrap_or(licence)
}

/// Catalogue of predictors offered by the server: the CHELSA climate layers (ADR-0009 default
/// selection), then terrain, land-cover and distance-to-water predictors (ADR-0015), none of
/// which is selected by default.
pub fn layer_catalogue(demo: bool) -> Vec<LayerInfo> {
    if demo {
        return crate::demo::LAYERS
            .iter()
            .map(|(id, name, unit)| LayerInfo {
                id: (*id).into(),
                name: (*name).into(),
                short_name: (*id).trim_start_matches("demo_").to_uppercase(),
                unit: (*unit).into(),
                source: "Synthetic demonstration layer (not real data)".into(),
                licence: "CC0 1.0".into(),
                default_selected: true,
                caution: None,
                group: layer_group(id).into(),
                attribution: None,
            })
            .collect();
    }
    let climate = chelsa::LAYERS.iter().map(|l| LayerInfo {
        id: l.id.into(),
        name: l.name.into(),
        short_name: l.short.into(),
        unit: l.unit.into(),
        source: chelsa::SOURCE.into(),
        licence: chelsa::LICENCE.into(),
        default_selected: l.default_selected,
        caution: l.caution.map(String::from),
        group: "climate".into(),
        attribution: None,
    });
    let other = predictors::LAYERS.iter().filter_map(|l| {
        let p = predictors::provenance(l.id)?;
        Some(LayerInfo {
            id: l.id.into(),
            name: l.name.into(),
            short_name: l.short.into(),
            unit: l.unit.into(),
            source: format!("{} ({})", p.source, p.vintage),
            licence: licence_name(p.licence).into(),
            default_selected: l.default_selected,
            caution: l.caution.map(String::from),
            group: layer_group(l.id).into(),
            attribution: Some(p.attribution.into()),
        })
    });
    climate.chain(other).collect()
}

/// A data source used by a stack and what any output must carry for it (reports, manifest).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceCredit {
    pub source: String,
    /// Reference period or year(s) of the data.
    pub vintage: String,
    pub licence: String,
    pub licence_url: String,
    pub citation: String,
    /// Notice required by the licence (`None` for CC0).
    pub attribution: Option<String>,
    /// Short names of the layers taken from this source, in selection order.
    pub layers: Vec<String>,
    /// How each layer's 30″ value is derived from the source (`SHORT: method`).
    pub methods: Vec<String>,
}

const CHELSA_LICENCE_URL: &str = "https://creativecommons.org/publicdomain/zero/1.0/";

/// Sources of the given layers, one entry per source in order of first use. Empty for the
/// synthetic demo layers.
pub fn sources_used(layer_ids: &[String]) -> Vec<SourceCredit> {
    let mut out: Vec<SourceCredit> = Vec::new();
    for id in layer_ids {
        let (credit, short, method) = if let Some(def) = chelsa::layer(id) {
            (
                SourceCredit {
                    source: chelsa::SOURCE.into(),
                    vintage: "1981–2010".into(),
                    licence: chelsa::LICENCE.into(),
                    licence_url: CHELSA_LICENCE_URL.into(),
                    citation: chelsa::CITATION.into(),
                    attribution: None,
                    layers: vec![],
                    methods: vec![],
                },
                def.short,
                None,
            )
        } else if let (Some(def), Some(p)) = (predictors::layer(id), predictors::provenance(id)) {
            (
                SourceCredit {
                    source: p.source.into(),
                    vintage: p.vintage.into(),
                    licence: p.licence.into(),
                    licence_url: p.licence_url.into(),
                    citation: p.citation.into(),
                    attribution: Some(p.attribution.into()),
                    layers: vec![],
                    methods: vec![],
                },
                def.short,
                Some(p.method),
            )
        } else {
            continue;
        };
        let i = match out.iter().position(|c| c.source == credit.source) {
            Some(i) => i,
            None => {
                out.push(credit);
                out.len() - 1
            }
        };
        out[i].layers.push(short.into());
        if let Some(m) = method {
            out[i].methods.push(format!("{short}: {m}"));
        }
    }
    out
}

/// Whether any selected layer is a terrain, land-cover or distance-to-water predictor
/// (ADR-0015). Reports and warnings change only then, so climate-only studies are unchanged.
pub fn uses_additional_predictors(layer_ids: &[String]) -> bool {
    layer_ids.iter().any(|id| predictors::layer(id).is_some())
}

/// Methods paragraph of the reports when additional predictors are selected (ADR-0015).
pub fn predictor_methods(factor: usize) -> String {
    format!(
        "Every predictor is read on the same window of the native CHELSA v2.1 30″ grid (about 1 km), without resampling (ADR-0010). Terrain comes from Copernicus DEM GLO-90: elevation is the mean of the 3″ (≈ 90 m) heights whose centre lies in the cell, slope the mean of 3″ slopes computed with Horn's (1981) method at 90 m before aggregation (never from the aggregated elevation), roughness the standard deviation of the 3″ heights. Land-cover fractions are the share of valid ESA WorldCover 2021 10 m pixels of each class group in the cell (eight groups that sum to 1). Distance to water is the distance from the cell centre to the nearest cell with at least 50 % permanent water, truncated at 25 km (exact up to 25 km, independent of the study-area extent). A cell is NoData when fewer than half of its source pixels are valid. {}A cell enters the model only where every predictor has a value.",
        if factor > 1 {
            format!(
                "The 30″ values are then aggregated by a factor of {factor} as the mean of valid sub-cells (NoData when more than half are missing). "
            )
        } else {
            String::new()
        }
    )
}

/// Interpretation limits of the additional predictors present in the selection (ADR-0015).
pub fn predictor_limits(layer_ids: &[String]) -> Vec<&'static str> {
    let any = |p: &str| layer_ids.iter().any(|id| id.starts_with(p));
    let mut out = Vec::new();
    if any("lc_") || layer_ids.iter().any(|id| id == "dist_water") {
        out.push("Land cover is that of 2021 (ESA WorldCover v200); the climate baseline is 1981–2010 and records may be older: where land use changed, older records are described by today's land cover. WorldCover's overall accuracy is about 77 %, and its classes do not separate broadleaved from coniferous forest or annual from permanent crops.");
    }
    if layer_ids.iter().any(|id| id == "lc_built") {
        out.push("Built-up fraction correlates with observer effort in opportunistic records: a positive response may reflect sampling bias rather than habitat preference.");
    }
    if layer_ids.iter().any(|id| id == "dist_water") {
        out.push("Distance to water only sees water bodies covering at least half of a 1 km cell (lakes, reservoirs, wide rivers, the sea); streams and small ponds are not resolved.");
    }
    if any("dem_") {
        out.push("Terrain comes from a surface model (Copernicus DEM, TanDEM-X 2011–2015): heights include forest canopy and buildings, so wooded and urban cells appear higher and rougher than the ground.");
    }
    out
}

/// Cautions of the additional predictors actually used in a model (ids after the collinearity
/// filter): never hidden (AGENTS.md), worded in the interface from their code.
pub fn predictor_warnings(used_ids: &[&str]) -> Vec<Warning> {
    let mut w = Vec::new();
    let any = |p: &str| used_ids.iter().any(|id| id.starts_with(p));
    if any("lc_") || used_ids.contains(&"dist_water") {
        w.push(Warning::new(
            "land_cover_temporal",
            Severity::Info,
            "Land-cover predictors describe 2021 (ESA WorldCover); the climate baseline is 1981–2010 and records may be older. Where land use changed, older records are described by today's land cover.",
        ));
    }
    if used_ids.contains(&"lc_built") {
        w.push(Warning::new(
            "built_up_bias",
            Severity::Warning,
            "Built-up fraction is used: it is strongly correlated with observer effort in opportunistic records, so a positive response may reflect where people look rather than the species.",
        ));
    }
    let groups = used_ids.iter().filter(|id| id.starts_with("lc_")).count();
    if groups == niche_data::landcover::GROUPS.len() {
        w.push(Warning::new(
            "land_cover_composition",
            Severity::Warning,
            "All eight land-cover fractions are used: they sum to 1 in every cell, an exact linear dependency that the pairwise correlation filter does not detect. Leave out at least one group (usually the least relevant to the species).",
        ));
    }
    if any("dem_") {
        w.push(Warning::new(
            "surface_model",
            Severity::Info,
            "Terrain predictors come from Copernicus DEM, a surface model: heights include forest canopy and buildings.",
        ));
    }
    w
}

static LAND_CACHE: Mutex<Option<(GridSpec, Vec<bool>)>> = Mutex::new(None);

/// Window of the native CHELSA grid covering a bounding box, rounded to whole aggregated cells.
pub struct NativeWindow {
    pub grid: GridSpec,
    pub row: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
    pub factor: usize,
}

impl NativeWindow {
    /// Grid of the window at native resolution.
    pub fn spec(&self) -> GridSpec {
        GridSpec {
            west: self.grid.west + self.col as f64 * self.grid.cell,
            north: self.grid.north - self.row as f64 * self.grid.cell,
            cell: self.grid.cell,
            width: self.cols,
            height: self.rows,
        }
    }
    /// Cells of the modelling grid (after aggregation), land and sea.
    pub fn model_cells(&self) -> usize {
        (self.rows / self.factor) * (self.cols / self.factor)
    }
    /// (west, south, east, north) of the window: the extent used for GBIF requests.
    pub fn bbox(&self) -> (f64, f64, f64, f64) {
        let s = self.spec();
        (s.west, s.south(), s.west + s.cell * s.width as f64, s.north)
    }
}

pub fn native_window(
    http: &Http,
    cache: &Path,
    (w, s, e, n): (f64, f64, f64, f64),
    factor: usize,
) -> Result<NativeWindow, AppError> {
    let factor = factor.clamp(1, 20);
    let g = chelsa::native_grid(http, cache)?;
    let col = ((w - g.west) / g.cell).floor().max(0.0) as usize;
    let row = ((g.north - n) / g.cell).floor().max(0.0) as usize;
    let mut cols = (((e - g.west) / g.cell).ceil() as usize)
        .min(g.width)
        .saturating_sub(col);
    let mut rows = (((g.north - s) / g.cell).ceil() as usize)
        .min(g.height)
        .saturating_sub(row);
    cols = cols.div_ceil(factor) * factor;
    rows = rows.div_ceil(factor) * factor;
    cols = cols.min(g.width.saturating_sub(col));
    rows = rows.min(g.height.saturating_sub(row));
    if rows == 0 || cols == 0 {
        return Err(AppError::Invalid(
            "study area does not overlap the predictor grid".into(),
        ));
    }
    Ok(NativeWindow {
        grid: g,
        row,
        col,
        rows,
        cols,
        factor,
    })
}

/// Native window of a study area (real data only; the demo has its own synthetic grid).
pub fn area_window(
    http: &Http,
    cache: &Path,
    area: &StudyArea,
    factor: usize,
) -> Result<NativeWindow, AppError> {
    let polys = area_polygons(area)?;
    let bbox = union_bbox(&polys).ok_or_else(|| AppError::Invalid("empty study area".into()))?;
    native_window(http, cache, bbox, factor)
}

/// Build the environmental stack for a study area.
pub fn build_stack(
    http: &Http,
    cache: &Path,
    area: &StudyArea,
    layer_ids: &[String],
    factor: usize,
    demo: bool,
    progress: &dyn Fn(&str, f64),
) -> Result<EnvStack, AppError> {
    build_stack_cancellable(
        http,
        cache,
        area,
        layer_ids,
        factor,
        demo,
        progress,
        &|| false,
    )
}

/// [`build_stack`], checking `cancel` between layers (a first study in a new region downloads
/// WorldCover and DEM tiles, several seconds per degree).
#[allow(clippy::too_many_arguments)]
pub fn build_stack_cancellable(
    http: &Http,
    cache: &Path,
    area: &StudyArea,
    layer_ids: &[String],
    factor: usize,
    demo: bool,
    progress: &dyn Fn(&str, f64),
    cancel: &dyn Fn() -> bool,
) -> Result<EnvStack, AppError> {
    let read = |grid: &GridSpec, id: &str, w: Window| {
        predictors::read_window_on_grid(http, cache, grid, id, w)
    };
    build_stack_with(
        http, cache, area, layer_ids, factor, demo, progress, cancel, &read,
    )
}

/// Same size and lattice to 1e-9 of a cell (floating-point noise of origin arithmetic only).
pub(crate) fn same_lattice(a: &GridSpec, b: &GridSpec) -> bool {
    let close = |x: f64, y: f64| (x - y).abs() <= 1e-9 * b.cell.abs();
    a.width == b.width
        && a.height == b.height
        && close(a.west, b.west)
        && close(a.north, b.north)
        && close(a.cell, b.cell)
}

/// Native CHELSA window: (row0, col0, rows, cols).
pub type Window = (usize, usize, usize, usize);

/// Reader of an additional predictor (terrain, land cover, distance to water) on a native
/// CHELSA window. Production uses `niche_data::predictors::read_window_on_grid`; tests pass
/// synthetic rasters so the stack assembly runs offline.
pub type PredictorReader<'a> = dyn Fn(&GridSpec, &str, Window) -> Result<Raster, DataError> + 'a;

/// [`build_stack_cancellable`] with an explicit reader for the additional predictors. CHELSA
/// layers, the native grid and the land mask are read through `http`/`cache` as usual.
///
/// Every layer is read on the same native window and aggregated by the same factor (mean of
/// valid sub-cells, NoData when more than half are missing). A cell is valid only when every
/// layer is finite there, so a predictor's NoData removes cells from the model: land-cover
/// fractions and distance to water are NoData where fewer than half of the 10 m pixels are
/// valid (outside WorldCover tiles), DEM layers where fewer than half of the 3″ pixels are
/// (absent sea tiles). Both coincide with open sea, which the land mask removes anyway; along
/// coasts a few land-mask cells can lose their DEM value and are then reported as "Missing
/// environmental data" for occurrences.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn build_stack_with(
    http: &Http,
    cache: &Path,
    area: &StudyArea,
    layer_ids: &[String],
    factor: usize,
    demo: bool,
    progress: &dyn Fn(&str, f64),
    cancel: &dyn Fn() -> bool,
    read_predictor: &PredictorReader<'_>,
) -> Result<EnvStack, AppError> {
    let factor = factor.clamp(1, 20);
    let polys = area_polygons(area)?;
    let (w, s, e, n) =
        union_bbox(&polys).ok_or_else(|| AppError::Invalid("empty study area".into()))?;
    if layer_ids.is_empty() {
        return Err(AppError::Invalid(
            "select at least one environmental variable".into(),
        ));
    }
    let (native, mut rasters, provenance) = if demo {
        let (g, r) = crate::demo::layers(layer_ids, (w, s, e, n))?;
        (
            g,
            r,
            json!({"source": "synthetic demonstration layers", "synthetic": true}),
        )
    } else {
        progress("Reading CHELSA grid description", 0.0);
        let win = native_window(http, cache, (w, s, e, n), factor)?;
        let (r0, c0, rows, cols) = (win.row, win.col, win.rows, win.cols);
        if win.model_cells() > MAX_CELLS {
            return Err(AppError::Invalid(format!(
                "study area needs {} cells at this resolution (limit {MAX_CELLS}); choose a coarser resolution",
                win.model_cells()
            )));
        }
        let mut out = Vec::new();
        let mut prov = Vec::new();
        let native = win.spec();
        let n = layer_ids.len();
        for (i, id) in layer_ids.iter().enumerate() {
            if cancel() {
                return Err(AppError::Cancelled);
            }
            let window = json!({"row": r0, "col": c0, "rows": rows, "cols": cols});
            if let Some(def) = chelsa::layer(id) {
                progress(
                    &format!("Reading {} ({}/{})", def.short, i + 1, n),
                    i as f64 / n as f64,
                );
                let r = chelsa::read_window(http, cache, id, (r0, c0, rows, cols))?;
                out.push((id.clone(), format!("{} — {}", def.short, def.name), r));
                prov.push(json!({"id": id, "url": chelsa::url(id), "window": window}));
            } else if let (Some(def), Some(p)) = (predictors::layer(id), predictors::provenance(id))
            {
                progress(
                    &format!(
                        "Reading {} from {} ({}/{})",
                        def.short,
                        p.source.split(',').next().unwrap_or(p.source),
                        i + 1,
                        n
                    ),
                    i as f64 / n as f64,
                );
                let mut r = read_predictor(&win.grid, id, (r0, c0, rows, cols))?;
                // Same lattice, no resampling (ADR-0010). Origins are compared to 1e-9 cell:
                // a window cropped from a padded one (distance to water) has its origin
                // recomputed by another sum of floats (measured difference 2e-14°).
                if !same_lattice(&r.spec, &native) || r.data.len() != native.len() {
                    return Err(AppError::Internal(format!(
                        "{id}: grid {:?} differs from the CHELSA window {native:?}",
                        r.spec
                    )));
                }
                r.spec = native;
                out.push((id.clone(), format!("{} — {}", def.short, def.name), r));
                prov.push(json!({"id": id, "source": p.source, "vintage": p.vintage, "licence": p.licence, "licence_url": p.licence_url, "citation": p.citation, "attribution": p.attribution, "method": p.method, "window": window}));
            } else {
                return Err(AppError::Invalid(format!("unknown layer {id}")));
            }
        }
        let mut provenance = json!({"layers": prov, "land_mask": landmask::SOURCE});
        // CHELSA block kept as before (manifests of climate-only studies keep their shape).
        if layer_ids.iter().any(|id| chelsa::layer(id).is_some()) {
            provenance["source"] = json!(chelsa::SOURCE);
            provenance["licence"] = json!(chelsa::LICENCE);
            provenance["citation"] = json!(chelsa::CITATION);
        }
        if uses_additional_predictors(layer_ids) {
            provenance["sources"] = json!(sources_used(layer_ids));
            provenance["aggregation"] = json!(format!(
                "every layer read on the same native CHELSA 30″ window; aggregated by a factor of {factor} as the mean of valid sub-cells (NoData when more than half are missing); a cell is used only where every layer has a value"
            ));
        }
        (native, out, provenance)
    };
    let spec = native.aggregated(factor);
    let layers: Vec<EnvLayer> = rasters
        .drain(..)
        .map(|(id, name, r)| EnvLayer {
            id,
            name,
            data: r.aggregate_mean(factor).data,
        })
        .collect();
    progress("Rasterizing study area", 0.9);
    let area_mask = rasterize_polygons(&polys, &spec);
    let land_mask = if demo {
        vec![true; spec.len()]
    } else {
        progress("Applying land mask", 0.95);
        let mut guard = LAND_CACHE
            .lock()
            .map_err(|_| AppError::Internal("land mask cache poisoned".into()))?;
        match guard.as_ref() {
            Some((cs, m)) if *cs == spec => m.clone(),
            _ => {
                let m = landmask::land_mask(http, cache, &spec)?;
                *guard = Some((spec, m.clone()));
                m
            }
        }
    };
    let valid: Vec<bool> = (0..spec.len())
        .map(|i| area_mask[i] && land_mask[i] && layers.iter().all(|l| l.data[i].is_finite()))
        .collect();
    if !valid.iter().any(|v| *v) {
        return Err(AppError::Invalid(
            "no land cells with environmental data inside the study area".into(),
        ));
    }
    Ok(EnvStack {
        spec,
        area_mask,
        land_mask,
        valid,
        layers,
        row_area_km2: area::row_areas_km2(&spec),
        provenance,
    })
}

/// Result of occurrence cleaning.
pub struct Cleaned {
    pub presence_cells: Vec<usize>,
    pub points: Vec<OccurrencePoint>,
    pub exclusions: Vec<ExclusionCount>,
    pub licences: BTreeMap<String, usize>,
    /// Records flagged as generalised by their publisher (`generalised == Some(true)`).
    pub generalised_records: usize,
    /// Of those, excluded by the `generalised` rule (ADR-0011).
    pub generalised_excluded: usize,
    /// Of those, kept as presences.
    pub generalised_retained: usize,
    /// Records whose coordinates fall in the study area (denominator of the
    /// `generalised_records` warning threshold), and the generalised ones among them.
    pub records_in_area: usize,
    pub generalised_in_area: usize,
    /// Presence cells removed by distance thinning (0 when off).
    pub thinned: usize,
}

/// Distance thinning visits presences in random order: this many orders are tried and the
/// largest retained set kept (spThin `reps`; 100 is spThin's default), reduced for large sets
/// so that thinning stays below about 2·10⁶ point visits.
pub const THIN_REPS: usize = 100;
const THIN_WORK: usize = 2_000_000;

/// Seed of a species' distance thinning, derived from the study seed and the species key (so
/// that a species gives the same thinning in a preview, a run and a resumed run).
pub fn thin_seed(seed: u64, species_key: u64) -> u64 {
    seed ^ species_key.wrapping_mul(0x9E37_79B9) ^ 0x7417_4E00_D15A_0001
}

/// Exclusion reasons, in the order they are applied (first failing rule is recorded). Distance
/// thinning (`f.thin_km`, opt-in) then removes retained presence cells closer than `thin_km` to
/// another retained one, seeded by `thin_seed`.
pub fn clean(
    records: &[gbif::Occurrence],
    stack: &EnvStack,
    f: &OccurrenceFilters,
    thin_seed: u64,
) -> Cleaned {
    let mut counts: BTreeMap<usize, (String, usize)> = BTreeMap::new();
    let mut add = |order: usize, reason: String| {
        counts.entry(order).or_insert((reason, 0)).1 += 1;
    };
    let mut points = Vec::with_capacity(records.len());
    let mut licences = BTreeMap::new();
    let mut seen = std::collections::HashSet::new();
    let mut cells = Vec::new();
    // Index in `points` of the record kept for each presence cell (for thinning).
    let mut kept_points = Vec::new();
    let (mut gen_records, mut gen_excluded, mut gen_retained) = (0, 0, 0);
    let (mut in_area, mut gen_in_area) = (0, 0);
    for o in records {
        *licences.entry(o.licence.clone()).or_insert(0) += 1;
        let generalised = o.generalised == Some(true);
        let cell = stack
            .spec
            .cell_of(o.lng, o.lat)
            .map(|(r, c)| stack.spec.index(r, c))
            .filter(|&i| stack.area_mask[i]);
        if generalised {
            gen_records += 1;
        }
        if cell.is_some() {
            in_area += 1;
            if generalised {
                gen_in_area += 1;
            }
        }
        let status: Result<usize, (usize, String, &str)> = (|| {
            if matches!(
                o.basis_of_record.as_str(),
                "FOSSIL_SPECIMEN" | "LIVING_SPECIMEN"
            ) {
                return Err((
                    0,
                    "Fossil or captive/cultivated specimen".to_string(),
                    "basis_of_record",
                ));
            }
            // The organisation's own records are always accepted (ADR-0108): it holds the
            // rights to them. No GBIF record carries this code.
            if o.licence != crate::owndata::ORG_OWNED && !f.licences.iter().any(|l| l == &o.licence)
            {
                return Err((
                    1,
                    format!("Licence not allowed for this project ({})", o.licence),
                    "licence",
                ));
            }
            match o.year {
                None => return Err((2, "Undated record".to_string(), "undated")),
                Some(y) if y < f.year_min => {
                    return Err((
                        3,
                        format!("Recorded before {} (climate baseline)", f.year_min),
                        "too_old",
                    ));
                }
                _ => {}
            }
            // ADR-0011: a generalised record is used only when its stated precision is within
            // the uncertainty limit; a missing uncertainty is not a guarantee of precision.
            if generalised && o.uncertainty_m.is_none_or(|u| u > f.max_uncertainty_m) {
                return Err((
                    4,
                    format!(
                        "Coordinates generalised by the publisher; precision not stated or > {} m",
                        f.max_uncertainty_m
                    ),
                    "generalised",
                ));
            }
            if o.uncertainty_m.is_some_and(|u| u > f.max_uncertainty_m) {
                return Err((
                    5,
                    format!("Coordinate uncertainty > {} m", f.max_uncertainty_m),
                    "uncertain",
                ));
            }
            let Some(idx) = cell else {
                return Err((6, "Outside study area".to_string(), "outside_area"));
            };
            if !stack.land_mask[idx] {
                return Err((
                    7,
                    "In sea or large lake (land mask)".to_string(),
                    "not_land",
                ));
            }
            if !stack.valid[idx] {
                return Err((8, "Missing environmental data".to_string(), "no_env"));
            }
            if !seen.insert(idx) {
                return Err((
                    9,
                    "Duplicate within one grid cell".to_string(),
                    "duplicate_cell",
                ));
            }
            Ok(idx)
        })();
        let code = match status {
            Ok(idx) => {
                cells.push(idx);
                kept_points.push(points.len());
                "retained".to_string()
            }
            Err((order, reason, code)) => {
                if code == "generalised" {
                    gen_excluded += 1;
                }
                add(order, reason);
                code.to_string()
            }
        };
        points.push(OccurrencePoint {
            lng: o.lng,
            lat: o.lat,
            year: o.year,
            status: code,
            source: None,
        });
    }
    // Distance thinning (opt-in) on the cell centres of the retained presences.
    let mut thinned = 0;
    if let Some(km) = f.thin_km.filter(|k| k.is_finite() && *k > 0.0)
        && cells.len() > 1
    {
        let spec = stack.spec;
        let centres: Vec<(f64, f64)> = cells
            .iter()
            .map(|&c| spec.center(c / spec.width, c % spec.width))
            .collect();
        let reps = (THIN_WORK / centres.len()).clamp(1, THIN_REPS);
        let keep = thin_by_distance_reps(&centres, km, reps, &mut Rng::new(thin_seed));
        let mut keep_mask = vec![false; cells.len()];
        for &k in &keep {
            keep_mask[k] = true;
        }
        let reason = format!("Thinned (closer than {km} km to a retained record)");
        for (i, &kept) in keep_mask.iter().enumerate() {
            if !kept {
                points[kept_points[i]].status = "thinned".into();
                add(10, reason.clone());
                thinned += 1;
            }
        }
        cells = keep.iter().map(|&k| cells[k]).collect();
        kept_points = keep.iter().map(|&k| kept_points[k]).collect();
    }
    for &p in &kept_points {
        if records[p].generalised == Some(true) {
            gen_retained += 1;
        }
    }
    Cleaned {
        presence_cells: cells,
        points,
        exclusions: counts
            .into_values()
            .map(|(reason, count)| ExclusionCount { reason, count })
            .collect(),
        licences,
        generalised_records: gen_records,
        generalised_excluded: gen_excluded,
        generalised_retained: gen_retained,
        records_in_area: in_area,
        generalised_in_area: gen_in_area,
        thinned,
    }
}

/// Random background cells among valid cells (Phillips & Dudík 2008: 10 000 by default).
pub fn sample_background(stack: &EnvStack, n: usize, rng: &mut Rng) -> Vec<usize> {
    let valid: Vec<usize> = (0..stack.valid.len()).filter(|&i| stack.valid[i]).collect();
    let mut picks: Vec<usize> = rng
        .sample_indices(valid.len(), n)
        .into_iter()
        .map(|i| valid[i])
        .collect();
    picks.sort_unstable();
    picks
}

/// Target-group background (Phillips et al. 2009; ADR-0013): up to `n` cells drawn uniformly,
/// without replacement, among the valid cells with at least one target-group record
/// (`effort[i] > 0`), i.e. the target group's record sites de-duplicated per cell.
pub fn sample_background_target_group(
    stack: &EnvStack,
    effort: &[f64],
    n: usize,
    rng: &mut Rng,
) -> Vec<usize> {
    let eligible: Vec<usize> = (0..stack.valid.len())
        .filter(|&i| stack.valid[i] && effort.get(i).is_some_and(|&e| e > 0.0))
        .collect();
    let mut picks: Vec<usize> = rng
        .sample_indices(eligible.len(), n)
        .into_iter()
        .map(|i| eligible[i])
        .collect();
    picks.sort_unstable();
    picks
}

/// Raster view of a data vector on the stack grid.
pub fn raster(stack: &EnvStack, data: Vec<f32>) -> Raster {
    Raster {
        spec: stack.spec,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use niche_geo::GridSpec;

    /// 10 × 10 one-degree cells, all valid, one layer.
    fn stack() -> EnvStack {
        let spec = GridSpec {
            west: 0.0,
            north: 10.0,
            cell: 1.0,
            width: 10,
            height: 10,
        };
        EnvStack {
            spec,
            area_mask: vec![true; spec.len()],
            land_mask: vec![true; spec.len()],
            valid: vec![true; spec.len()],
            layers: vec![EnvLayer {
                id: "x".into(),
                name: "x".into(),
                data: vec![1.0; spec.len()],
            }],
            row_area_km2: vec![1.0; 10],
            provenance: serde_json::Value::Null,
        }
    }

    fn rec(
        key: u64,
        lng: f64,
        lat: f64,
        unc: Option<f64>,
        generalised: Option<bool>,
    ) -> gbif::Occurrence {
        gbif::Occurrence {
            key,
            lng,
            lat,
            year: Some(2015),
            uncertainty_m: unc,
            licence: "CC0_1_0".into(),
            basis_of_record: "HUMAN_OBSERVATION".into(),
            dataset_key: "d".into(),
            generalised,
        }
    }

    fn reasons(c: &Cleaned) -> Vec<(String, usize)> {
        c.exclusions
            .iter()
            .map(|e| (e.reason.clone(), e.count))
            .collect()
    }

    // the design notes §7, test 1.
    #[test]
    fn generalised_without_uncertainty_is_excluded_and_counted() {
        let s = stack();
        let c = clean(
            &[
                rec(1, 1.5, 1.5, None, Some(true)),
                rec(2, 2.5, 2.5, None, Some(false)),
            ],
            &s,
            &OccurrenceFilters::default(),
            1,
        );
        assert_eq!(c.points[0].status, "generalised");
        assert_eq!(c.points[1].status, "retained");
        assert_eq!(
            reasons(&c),
            vec![(
                "Coordinates generalised by the publisher; precision not stated or > 1000 m"
                    .to_string(),
                1
            )]
        );
        assert_eq!(
            (
                c.generalised_records,
                c.generalised_excluded,
                c.generalised_retained
            ),
            (1, 1, 0)
        );
    }

    // Test 2: blurred to 27 505 m (iNaturalist) moves from `uncertain` to `generalised`; the total
    // number of exclusions does not change.
    #[test]
    fn generalised_with_large_uncertainty_changes_reason_not_total() {
        let s = stack();
        let f = OccurrenceFilters::default();
        let flagged = clean(&[rec(1, 1.5, 1.5, Some(27_505.0), Some(true))], &s, &f, 1);
        let unflagged = clean(&[rec(1, 1.5, 1.5, Some(27_505.0), None)], &s, &f, 1);
        assert_eq!(flagged.points[0].status, "generalised");
        assert_eq!(unflagged.points[0].status, "uncertain");
        let total = |c: &Cleaned| c.exclusions.iter().map(|e| e.count).sum::<usize>();
        assert_eq!(total(&flagged), total(&unflagged));
        assert_eq!(flagged.generalised_excluded, 1);
    }

    // Test 3: generalised to a 400 m grid with a stated 283 m uncertainty is kept and counted.
    #[test]
    fn generalised_with_stated_small_uncertainty_is_retained_and_counted() {
        let s = stack();
        let c = clean(
            &[rec(1, 1.5, 1.5, Some(283.0), Some(true))],
            &s,
            &OccurrenceFilters::default(),
            1,
        );
        assert_eq!(c.points[0].status, "retained");
        assert_eq!(c.presence_cells.len(), 1);
        assert!(c.exclusions.is_empty());
        assert_eq!(
            (
                c.generalised_records,
                c.generalised_excluded,
                c.generalised_retained
            ),
            (1, 0, 1)
        );
        assert_eq!((c.records_in_area, c.generalised_in_area), (1, 1));
    }

    // Test 4: without generalised records (flag false or unknown) cleaning is unchanged.
    #[test]
    fn records_without_the_flag_are_cleaned_as_before() {
        let s = stack();
        let f = OccurrenceFilters::default();
        let base = [
            (1.5, 1.5, None),
            (2.5, 2.5, Some(5000.0)),
            (3.5, 3.5, Some(10.0)),
            (3.6, 3.6, Some(10.0)),
            (20.0, 3.5, Some(10.0)),
        ];
        let make = |g: Option<bool>| -> Vec<gbif::Occurrence> {
            base.iter()
                .enumerate()
                .map(|(i, &(x, y, u))| rec(i as u64, x, y, u, g))
                .collect()
        };
        let none = clean(&make(None), &s, &f, 1);
        let no = clean(&make(Some(false)), &s, &f, 1);
        for c in [&none, &no] {
            assert_eq!(
                c.points
                    .iter()
                    .map(|p| p.status.as_str())
                    .collect::<Vec<_>>(),
                [
                    "retained",
                    "uncertain",
                    "retained",
                    "duplicate_cell",
                    "outside_area"
                ]
            );
            assert_eq!(c.generalised_records, 0);
            assert_eq!(c.thinned, 0);
        }
        assert_eq!(reasons(&none), reasons(&no));
        assert_eq!(none.presence_cells, no.presence_cells);
    }

    #[test]
    fn thinning_is_deterministic_and_respects_the_distance() {
        let s = stack();
        // One record in each of the 100 one-degree cells (≈ 111 km apart in latitude).
        let recs: Vec<gbif::Occurrence> = (0..100)
            .map(|i| {
                rec(
                    i,
                    (i % 10) as f64 + 0.5,
                    (i / 10) as f64 + 0.5,
                    Some(10.0),
                    None,
                )
            })
            .collect();
        let f = OccurrenceFilters {
            thin_km: Some(250.0),
            ..OccurrenceFilters::default()
        };
        let a = clean(&recs, &s, &f, 42);
        let b = clean(&recs, &s, &f, 42);
        assert_eq!(a.presence_cells, b.presence_cells);
        assert_eq!(
            a.points.iter().map(|p| &p.status).collect::<Vec<_>>(),
            b.points.iter().map(|p| &p.status).collect::<Vec<_>>()
        );
        assert!(a.thinned > 0 && a.presence_cells.len() + a.thinned == 100);
        let thinned = a.points.iter().filter(|p| p.status == "thinned").count();
        assert_eq!(thinned, a.thinned);
        assert_eq!(
            reasons(&a),
            vec![(
                "Thinned (closer than 250 km to a retained record)".to_string(),
                a.thinned
            )]
        );
        // Every pair of retained cells is at least 250 km apart.
        let centres: Vec<(f64, f64)> = a
            .presence_cells
            .iter()
            .map(|&c| s.spec.center(c / 10, c % 10))
            .collect();
        for (i, p) in centres.iter().enumerate() {
            for q in &centres[i + 1..] {
                assert!(niche_eval::haversine_km(*p, *q) >= 250.0);
            }
        }
        // Off (None) or the same records without thinning: nothing removed.
        let off = clean(&recs, &s, &OccurrenceFilters::default(), 42);
        assert_eq!(off.presence_cells.len(), 100);
        // The seed comes from the study seed and the species key.
        assert_ne!(thin_seed(1, 2), thin_seed(1, 3));
        assert_ne!(thin_seed(1, 2), thin_seed(2, 2));
    }

    #[test]
    fn target_group_background_only_uses_cells_with_records() {
        let s = stack();
        let mut effort = vec![0.0; 100];
        for c in [3, 14, 15, 77, 99] {
            effort[c] = 2.0;
        }
        let mut rng = Rng::new(1);
        let bg = sample_background_target_group(&s, &effort, 3, &mut rng);
        assert_eq!(bg.len(), 3);
        assert!(bg.iter().all(|&c| effort[c] > 0.0));
        assert!(bg.windows(2).all(|w| w[0] < w[1]));
        let mut rng = Rng::new(1);
        assert_eq!(
            sample_background_target_group(&s, &effort, 50, &mut rng),
            vec![3, 14, 15, 77, 99]
        );
    }
}
