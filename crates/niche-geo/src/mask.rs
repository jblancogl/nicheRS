//! Polygon → grid mask by scanline at cell-centre latitude (even–odd rule; holes and
//! multipolygons supported). A cell belongs to the area when its centre is inside.

use crate::grid::GridSpec;

/// Rings in lon/lat; for each polygon the first ring is the exterior, the rest are holes. All
/// rings of all polygons are combined with the even–odd rule, which is correct for valid
/// (non-overlapping) multipolygons.
pub type Rings = Vec<Vec<(f64, f64)>>;

pub fn rasterize(rings: &Rings, spec: &GridSpec) -> Vec<bool> {
    let mut mask = vec![false; spec.len()];
    let mut xs: Vec<f64> = Vec::new();
    for row in 0..spec.height {
        let y = spec.north - (row as f64 + 0.5) * spec.cell;
        xs.clear();
        for ring in rings {
            let n = ring.len();
            for i in 0..n {
                let (x1, y1) = ring[i];
                let (x2, y2) = ring[(i + 1) % n];
                if (y1 > y) != (y2 > y) {
                    xs.push(x1 + (y - y1) * (x2 - x1) / (y2 - y1));
                }
            }
        }
        xs.sort_by(|a, b| a.total_cmp(b));
        for pair in xs.chunks(2) {
            if pair.len() < 2 {
                break;
            }
            // Columns whose centre x lies in [pair[0], pair[1]).
            let c0 = ((pair[0] - spec.west) / spec.cell - 0.5).ceil().max(0.0) as usize;
            let c1f = ((pair[1] - spec.west) / spec.cell - 0.5).ceil();
            let c1 = (c1f.max(0.0) as usize).min(spec.width);
            for c in c0..c1 {
                mask[spec.index(row, c)] = true;
            }
        }
    }
    mask
}

/// Bounding box (west, south, east, north) of the rings.
pub fn bbox(rings: &Rings) -> Option<(f64, f64, f64, f64)> {
    let mut b = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in rings.iter().flatten() {
        b = (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1));
    }
    if b.0.is_finite() { Some(b) } else { None }
}

/// Polygons (each: exterior + holes) from a GeoJSON Geometry / Feature / FeatureCollection.
/// Only Polygon and MultiPolygon are accepted; coordinates must be WGS84 lon/lat (RFC 7946).
pub fn polygons_from_geojson(v: &serde_json::Value) -> Result<Vec<Rings>, String> {
    let mut out = Vec::new();
    collect(v, &mut out, 0)?;
    if out.is_empty() {
        return Err("no Polygon or MultiPolygon geometry found".into());
    }
    Ok(out)
}

fn collect(v: &serde_json::Value, out: &mut Vec<Rings>, depth: usize) -> Result<(), String> {
    if depth > 8 {
        return Err("GeoJSON nesting too deep".into());
    }
    let t = v
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or("missing GeoJSON type")?;
    match t {
        "FeatureCollection" => {
            for f in v
                .get("features")
                .and_then(|f| f.as_array())
                .ok_or("features must be an array")?
            {
                collect(f, out, depth + 1)?;
            }
        }
        "Feature" => {
            if let Some(g) = v.get("geometry").filter(|g| !g.is_null()) {
                collect(g, out, depth + 1)?;
            }
        }
        "GeometryCollection" => {
            for g in v
                .get("geometries")
                .and_then(|f| f.as_array())
                .ok_or("geometries must be an array")?
            {
                collect(g, out, depth + 1)?;
            }
        }
        "Polygon" => out.push(rings(v.get("coordinates").ok_or("missing coordinates")?)?),
        "MultiPolygon" => {
            for p in v
                .get("coordinates")
                .and_then(|c| c.as_array())
                .ok_or("coordinates must be an array")?
            {
                out.push(rings(p)?);
            }
        }
        _ => {}
    }
    Ok(())
}

fn rings(v: &serde_json::Value) -> Result<Rings, String> {
    let arr = v.as_array().ok_or("polygon coordinates must be an array")?;
    let mut out = Vec::new();
    for ring in arr {
        let pts = ring.as_array().ok_or("ring must be an array")?;
        let mut r = Vec::with_capacity(pts.len());
        for p in pts {
            let x = p
                .get(0)
                .and_then(serde_json::Value::as_f64)
                .ok_or("bad coordinate")?;
            let y = p
                .get(1)
                .and_then(serde_json::Value::as_f64)
                .ok_or("bad coordinate")?;
            if !(-180.0..=180.0).contains(&x) || !(-90.0..=90.0).contains(&y) {
                return Err(format!(
                    "coordinate ({x}, {y}) is not WGS84 longitude/latitude"
                ));
            }
            r.push((x, y));
        }
        if r.len() >= 4 {
            out.push(r);
        }
    }
    if out.is_empty() {
        return Err("polygon without a valid ring".into());
    }
    Ok(out)
}

/// Rasterize several polygons (union).
pub fn rasterize_polygons(polys: &[Rings], spec: &GridSpec) -> Vec<bool> {
    let mut m = vec![false; spec.len()];
    for p in polys {
        let Some((w, s, e, n)) = bbox(p) else {
            continue;
        };
        if e < spec.west || w > spec.east() || n < spec.south() || s > spec.north {
            continue;
        }
        for (dst, src) in m.iter_mut().zip(rasterize(p, spec)) {
            *dst |= src;
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_with_hole() {
        let spec = GridSpec {
            west: 0.0,
            north: 10.0,
            cell: 1.0,
            width: 10,
            height: 10,
        };
        let outer = vec![(1.0, 1.0), (9.0, 1.0), (9.0, 9.0), (1.0, 9.0)];
        let hole = vec![(4.0, 4.0), (6.0, 4.0), (6.0, 6.0), (4.0, 6.0)];
        let m = rasterize(&vec![outer, hole], &spec);
        let count = m.iter().filter(|&&b| b).count();
        assert_eq!(count, 64 - 4);
        assert!(!m[spec.index(5, 5)]);
        assert!(m[spec.index(2, 2)]);
    }
}
