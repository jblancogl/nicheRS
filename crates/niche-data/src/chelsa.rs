//! CHELSA v2.1 bioclimatic climatologies 1981–2010 (Karger et al. 2017, 2021; licence CC0 1.0),
//! and the CMIP6 future climatologies CHELSA derives from five ISIMIP3b models
//! ([`FUTURE_PERIODS`] × [`FUTURE_GCMS`] × [`FUTURE_SSPS`], see [`future_url`]).
//!
//! Files are global 30 arc-second GeoTIFFs with one row per strip, so a study-area window costs
//! a few MB of range requests. Windows are cached on disk as raw little-endian f32 per
//! (layer, window), keyed by the source URL and its ETag-independent file name.

use std::path::{Path, PathBuf};

use niche_geo::geotiff::GeoTiffInfo;
use niche_geo::{GridSpec, Raster};

use crate::DataError;
use crate::http::{Http, HttpSource};

pub const SOURCE: &str = "CHELSA v2.1 bioclim, 1981–2010 climatology";
pub const LICENCE: &str = "CC0 1.0";
pub const CITATION: &str = "Karger D.N. et al. (2017) Climatologies at high resolution for the earth's land surface areas. Scientific Data 4:170122. doi:10.1038/sdata.2017.122; CHELSA v2.1 (2021) doi:10.16904/envidat.228";
const ROOT: &str = "https://os.zhdk.cloud.switch.ch/chelsav2/GLOBAL/climatologies";
const BASE: &str = "https://os.zhdk.cloud.switch.ch/chelsav2/GLOBAL/climatologies/1981-2010/bio";

/// Future 30-year periods of CHELSA v2.1 CMIP6 climatologies.
pub const FUTURE_PERIODS: &[&str] = &["2011-2040", "2041-2070", "2071-2100"];
/// Shared socio-economic pathways (SSP-RCP combinations) CHELSA provides.
pub const FUTURE_SSPS: &[&str] = &["ssp126", "ssp370", "ssp585"];
/// The five ISIMIP3b general circulation models CHELSA downscales (CMIP6 `source_id`).
pub const FUTURE_GCMS: &[&str] = &[
    "GFDL-ESM4",
    "IPSL-CM6A-LR",
    "MPI-ESM1-2-HR",
    "MRI-ESM2-0",
    "UKESM1-0-LL",
];
pub const FUTURE_SOURCE: &str = "CHELSA v2.1 bioclim, CMIP6 future climatologies (ISIMIP3b models)";
/// As published by CHELSA for its climatologies (EnviDat record, checked 2026-10-20). The CMIP6
/// model output CHELSA starts from is under CC BY 4.0 (CMIP6 terms of use): cite and
/// acknowledge it, see [`FUTURE_CITATION`] and docs/architecture/data-sources-v1.md.
pub const FUTURE_LICENCE: &str = "CC0 1.0 (CHELSA); CMIP6 inputs CC BY 4.0, attribution required";
pub const FUTURE_CITATION: &str = "Karger D.N. et al. (2017) Climatologies at high resolution for the earth's land surface areas. Scientific Data 4:170122. doi:10.1038/sdata.2017.122; CHELSA v2.1 (2021) doi:10.16904/envidat.228; CMIP6: Eyring V. et al. (2016) Geosci. Model Dev. 9:1937–1958, doi:10.5194/gmd-9-1937-2016; bias-adjusted ISIMIP3b forcing: Lange S. (2019) Geosci. Model Dev. 12:3055–3070, doi:10.5194/gmd-12-3055-2019. We acknowledge the World Climate Research Programme, which coordinated CMIP6, and the modelling groups of GFDL-ESM4, IPSL-CM6A-LR, MPI-ESM1-2-HR, MRI-ESM2-0 and UKESM1-0-LL.";

#[derive(Debug, Clone, PartialEq)]
pub struct LayerDef {
    pub id: &'static str,
    pub short: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub default_selected: bool,
    pub caution: Option<&'static str>,
}

const ARTEFACT: &str = "Combines temperature and precipitation of specific quarters; known spatial discontinuities (Booth 2022, Austral Ecology 47:1506). Excluded by default.";

/// Default selection: a parsimonious set covering energy, water and their seasonality and
/// extremes, before the automatic collinearity filter (|r| ≤ 0.7, Dormann et al. 2013) is applied.
pub const LAYERS: &[LayerDef] = &[
    LayerDef {
        id: "chelsa_bio1",
        short: "BIO1",
        name: "Annual mean temperature",
        unit: "°C",
        default_selected: true,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio2",
        short: "BIO2",
        name: "Mean diurnal range",
        unit: "°C",
        default_selected: false,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio3",
        short: "BIO3",
        name: "Isothermality",
        unit: "%",
        default_selected: false,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio4",
        short: "BIO4",
        name: "Temperature seasonality (SD of monthly means)",
        unit: "0.01 °C",
        default_selected: true,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio5",
        short: "BIO5",
        name: "Max temperature of warmest month",
        unit: "°C",
        default_selected: true,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio6",
        short: "BIO6",
        name: "Min temperature of coldest month",
        unit: "°C",
        default_selected: true,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio7",
        short: "BIO7",
        name: "Temperature annual range",
        unit: "°C",
        default_selected: false,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio8",
        short: "BIO8",
        name: "Mean temperature of wettest quarter",
        unit: "°C",
        default_selected: false,
        caution: Some(ARTEFACT),
    },
    LayerDef {
        id: "chelsa_bio9",
        short: "BIO9",
        name: "Mean temperature of driest quarter",
        unit: "°C",
        default_selected: false,
        caution: Some(ARTEFACT),
    },
    LayerDef {
        id: "chelsa_bio10",
        short: "BIO10",
        name: "Mean temperature of warmest quarter",
        unit: "°C",
        default_selected: false,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio11",
        short: "BIO11",
        name: "Mean temperature of coldest quarter",
        unit: "°C",
        default_selected: false,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio12",
        short: "BIO12",
        name: "Annual precipitation",
        unit: "kg m⁻² yr⁻¹",
        default_selected: true,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio13",
        short: "BIO13",
        name: "Precipitation of wettest month",
        unit: "kg m⁻² month⁻¹",
        default_selected: false,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio14",
        short: "BIO14",
        name: "Precipitation of driest month",
        unit: "kg m⁻² month⁻¹",
        default_selected: true,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio15",
        short: "BIO15",
        name: "Precipitation seasonality (CV)",
        unit: "%",
        default_selected: true,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio16",
        short: "BIO16",
        name: "Precipitation of wettest quarter",
        unit: "kg m⁻² quarter⁻¹",
        default_selected: false,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio17",
        short: "BIO17",
        name: "Precipitation of driest quarter",
        unit: "kg m⁻² quarter⁻¹",
        default_selected: false,
        caution: None,
    },
    LayerDef {
        id: "chelsa_bio18",
        short: "BIO18",
        name: "Precipitation of warmest quarter",
        unit: "kg m⁻² quarter⁻¹",
        default_selected: false,
        caution: Some(ARTEFACT),
    },
    LayerDef {
        id: "chelsa_bio19",
        short: "BIO19",
        name: "Precipitation of coldest quarter",
        unit: "kg m⁻² quarter⁻¹",
        default_selected: false,
        caution: Some(ARTEFACT),
    },
];

pub fn layer(id: &str) -> Option<&'static LayerDef> {
    LAYERS.iter().find(|l| l.id == id)
}

pub fn url(id: &str) -> Option<String> {
    let n = id.strip_prefix("chelsa_bio")?;
    Some(format!("{BASE}/CHELSA_bio{n}_1981-2010_V.2.1.tif"))
}

/// URL of a CHELSA v2.1 CMIP6 future climatology, e.g. `future_url("chelsa_bio1", "2041-2070",
/// "GFDL-ESM4", "ssp370")`. `None` unless every argument is in the catalogue (BIO1–BIO19,
/// [`FUTURE_PERIODS`], [`FUTURE_GCMS`], [`FUTURE_SSPS`]). Directory names keep the model's
/// CMIP6 casing; file names use it in lower case (all 855 URLs answered 200, 2026-10-20).
pub fn future_url(id: &str, period: &str, gcm: &str, ssp: &str) -> Option<String> {
    layer(id)?;
    let n = id.strip_prefix("chelsa_bio")?;
    if !(FUTURE_PERIODS.contains(&period)
        && FUTURE_GCMS.contains(&gcm)
        && FUTURE_SSPS.contains(&ssp))
    {
        return None;
    }
    let g = gcm.to_ascii_lowercase();
    Some(format!(
        "{ROOT}/{period}/{gcm}/{ssp}/bio/CHELSA_bio{n}_{period}_{g}_{ssp}_V.2.1.tif"
    ))
}

/// Native CHELSA grid (read once from the bio1 header and cached).
pub fn native_grid(http: &Http, cache: &Path) -> Result<GridSpec, DataError> {
    let f = cache.join("chelsa_grid.json");
    if let Ok(t) = std::fs::read_to_string(&f)
        && let Ok(v) = serde_json::from_str::<[f64; 5]>(&t)
    {
        return Ok(GridSpec {
            west: v[0],
            north: v[1],
            cell: v[2],
            width: v[3] as usize,
            height: v[4] as usize,
        });
    }
    let src = HttpSource::new(http.clone(), url("chelsa_bio1").unwrap_or_default());
    let info = GeoTiffInfo::read(&src)?;
    let s = info.spec;
    std::fs::create_dir_all(cache).map_err(|e| DataError::Cache(e.to_string()))?;
    let _ = std::fs::write(
        &f,
        serde_json::json!([s.west, s.north, s.cell, s.width, s.height]).to_string(),
    );
    Ok(s)
}

/// Read a pixel window of a CHELSA layer (native resolution), using the on-disk cache.
pub fn read_window(
    http: &Http,
    cache: &Path,
    id: &str,
    window: (usize, usize, usize, usize),
) -> Result<Raster, DataError> {
    let u = url(id).ok_or_else(|| DataError::Format {
        url: id.into(),
        msg: "unknown CHELSA layer".into(),
    })?;
    read_window_at(
        http,
        cache,
        &u,
        &cache.join("chelsa").join(id),
        window,
        false,
    )
}

/// [`read_window`] for a CMIP6 future climatology ([`future_url`]): same native grid, same
/// on-disk window cache (under `chelsa-future/<period>/<gcm>/<ssp>/<id>/`). Values are in the
/// same physical units as the 1981–2010 layer (the file's own scale/offset are applied). The
/// file's grid is checked against the 1981–2010 grid when the window is first read.
#[allow(clippy::too_many_arguments)]
pub fn read_window_future(
    http: &Http,
    cache: &Path,
    id: &str,
    period: &str,
    gcm: &str,
    ssp: &str,
    window: (usize, usize, usize, usize),
) -> Result<Raster, DataError> {
    let u = future_url(id, period, gcm, ssp).ok_or_else(|| DataError::Format {
        url: format!("{id} {period} {gcm} {ssp}"),
        msg: "unknown CHELSA future layer".into(),
    })?;
    let dir = cache
        .join("chelsa-future")
        .join(period)
        .join(gcm)
        .join(ssp)
        .join(id);
    read_window_at(http, cache, &u, &dir, window, true)
}

fn same_grid(a: &GridSpec, b: &GridSpec) -> bool {
    let close = |x: f64, y: f64| (x - y).abs() <= 1e-9 * b.cell.abs().max(1e-12);
    a.width == b.width
        && a.height == b.height
        && close(a.west, b.west)
        && close(a.north, b.north)
        && close(a.cell, b.cell)
}

fn read_window_at(
    http: &Http,
    cache: &Path,
    u: &str,
    dir: &Path,
    window: (usize, usize, usize, usize),
    future: bool,
) -> Result<Raster, DataError> {
    let (r0, c0, rows, cols) = window;
    let file: PathBuf = dir.join(format!("{r0}_{c0}_{rows}x{cols}.f32"));
    let grid = native_grid(http, cache)?;
    let spec = GridSpec {
        west: grid.west + c0 as f64 * grid.cell,
        north: grid.north - r0 as f64 * grid.cell,
        cell: grid.cell,
        width: cols,
        height: rows,
    };
    if let Ok(bytes) = std::fs::read(&file)
        && bytes.len() == rows * cols * 4
    {
        let data = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        return Ok(Raster { spec, data });
    }
    let src = HttpSource::new(http.clone(), u);
    let mut info = GeoTiffInfo::read(&src)?;
    if future {
        // The CMIP6 files declare no-data 0 for BIO1/2/4–7/12–14, whose baseline files declare
        // −99999, a value their integer data cannot hold (no cell is masked). A raw 0 is real
        // data: 0 mm of precipitation in arid cells (BIO14 in the Atacama is 0 in both files,
        // measured 2026-10-20); for temperatures it would be 0 K and never occurs. Ignoring it
        // keeps future layers masked exactly like the baseline. Other no-data values (65535,
        // f32::MAX) are the same in both and kept.
        if info.nodata == Some(0.0) {
            info.nodata = None;
        }
        if !same_grid(&info.spec, &grid) {
            return Err(DataError::Format {
                url: u.into(),
                msg: format!(
                    "grid differs from the 1981-2010 CHELSA grid ({:?} vs {:?})",
                    info.spec, grid
                ),
            });
        }
    }
    let mut r = info.read_window(&src, r0, c0, rows, cols)?;
    r.spec = spec;
    std::fs::create_dir_all(dir).map_err(|e| DataError::Cache(e.to_string()))?;
    let bytes: Vec<u8> = r.data.iter().flat_map(|v| v.to_le_bytes()).collect();
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, bytes)
        .and_then(|()| std::fs::rename(&tmp, &file))
        .map_err(|e| DataError::Cache(e.to_string()))?;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_urls_follow_the_verified_pattern() {
        assert_eq!(
            future_url("chelsa_bio1", "2041-2070", "GFDL-ESM4", "ssp370").as_deref(),
            Some(
                "https://os.zhdk.cloud.switch.ch/chelsav2/GLOBAL/climatologies/2041-2070/GFDL-ESM4/ssp370/bio/CHELSA_bio1_2041-2070_gfdl-esm4_ssp370_V.2.1.tif"
            )
        );
        assert_eq!(
            future_url("chelsa_bio19", "2071-2100", "UKESM1-0-LL", "ssp585").as_deref(),
            Some(
                "https://os.zhdk.cloud.switch.ch/chelsav2/GLOBAL/climatologies/2071-2100/UKESM1-0-LL/ssp585/bio/CHELSA_bio19_2071-2100_ukesm1-0-ll_ssp585_V.2.1.tif"
            )
        );
        let mut n = 0;
        for l in LAYERS {
            for p in FUTURE_PERIODS {
                for g in FUTURE_GCMS {
                    for s in FUTURE_SSPS {
                        assert!(future_url(l.id, p, g, s).is_some());
                        n += 1;
                    }
                }
            }
        }
        assert_eq!(n, 19 * 3 * 5 * 3);
        // Outside the catalogue.
        assert!(future_url("chelsa_bio20", "2041-2070", "GFDL-ESM4", "ssp370").is_none());
        assert!(future_url("chelsa_bio1", "1981-2010", "GFDL-ESM4", "ssp370").is_none());
        assert!(future_url("chelsa_bio1", "2041-2070", "gfdl-esm4", "ssp370").is_none());
        assert!(future_url("chelsa_bio1", "2041-2070", "GFDL-ESM4", "ssp245").is_none());
    }

    /// A future window comes from the cache like a 1981–2010 one, in its own directory.
    #[test]
    fn future_windows_use_their_own_cache_entries() {
        let dir = std::env::temp_dir().join(format!("niche-chelsa-future-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(
            dir.join("chelsa_grid.json"),
            "[-180.0001388888,84.0001388888,0.0083333333,43200,20880]",
        )
        .expect("grid");
        let entry = dir.join("chelsa-future/2041-2070/MRI-ESM2-0/ssp126/chelsa_bio12");
        std::fs::create_dir_all(&entry).expect("dir");
        let vals: Vec<u8> = [1.5f32, 2.5].iter().flat_map(|v| v.to_le_bytes()).collect();
        std::fs::write(entry.join("10_20_1x2.f32"), vals).expect("cache");
        let http = Http::new();
        let r = read_window_future(
            &http,
            &dir,
            "chelsa_bio12",
            "2041-2070",
            "MRI-ESM2-0",
            "ssp126",
            (10, 20, 1, 2),
        )
        .expect("cached window");
        assert_eq!(r.data, vec![1.5, 2.5]);
        assert!((r.spec.west - (-180.0001388888 + 20.0 * 0.0083333333)).abs() < 1e-9);
        assert!(
            read_window_future(
                &http,
                &dir,
                "chelsa_bio12",
                "2041-2070",
                "X",
                "ssp126",
                (10, 20, 1, 2)
            )
            .is_err()
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn grid_comparison() {
        let g = GridSpec {
            west: -180.000138,
            north: 84.000138,
            cell: 1.0 / 120.0,
            width: 43200,
            height: 20880,
        };
        assert!(same_grid(&g, &g));
        assert!(!same_grid(&GridSpec { width: 43201, ..g }, &g));
        assert!(!same_grid(&GridSpec { west: -180.0, ..g }, &g));
    }
}
