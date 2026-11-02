//! Land mask from Natural Earth 1:10m land, minor islands and lakes (public domain).
//!
//! CHELSA v2.1 has values over the ocean, so terrestrial models need an explicit land mask;
//! otherwise background points fall in the sea and inflate discrimination metrics (ADR-0010).

use std::path::Path;

use niche_geo::GridSpec;
use niche_geo::mask::{polygons_from_geojson, rasterize_polygons};

use crate::DataError;
use crate::http::Http;

pub const SOURCE: &str = "Natural Earth 1:10m land + minor islands − lakes (v5.x, public domain)";
const BASE: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson";

fn layer(http: &Http, cache: &Path, name: &str) -> Result<serde_json::Value, DataError> {
    let file = cache.join("naturalearth").join(format!("{name}.geojson"));
    if let Ok(t) = std::fs::read_to_string(&file)
        && let Ok(v) = serde_json::from_str(&t)
    {
        return Ok(v);
    }
    let url = format!("{BASE}/{name}.geojson");
    let v = http.get_json(&url)?;
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d).map_err(|e| DataError::Cache(e.to_string()))?;
    }
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, v.to_string())
        .and_then(|()| std::fs::rename(&tmp, &file))
        .map_err(|e| DataError::Cache(e.to_string()))?;
    Ok(v)
}

/// `true` where the cell centre is on land and not in a large lake.
pub fn land_mask(http: &Http, cache: &Path, spec: &GridSpec) -> Result<Vec<bool>, DataError> {
    let fmt = |e: String| DataError::Format {
        url: "natural earth".into(),
        msg: e,
    };
    let land = polygons_from_geojson(&layer(http, cache, "ne_10m_land")?).map_err(fmt)?;
    let islands =
        polygons_from_geojson(&layer(http, cache, "ne_10m_minor_islands")?).map_err(fmt)?;
    let lakes = polygons_from_geojson(&layer(http, cache, "ne_10m_lakes")?).map_err(fmt)?;
    let mut m = rasterize_polygons(&land, spec);
    for (a, b) in m.iter_mut().zip(rasterize_polygons(&islands, spec)) {
        *a |= b;
    }
    for (a, b) in m.iter_mut().zip(rasterize_polygons(&lakes, spec)) {
        *a &= !b;
    }
    Ok(m)
}
