//! Administrative boundary packs (`.nrsb`): every administrative unit of the packaged countries
//! (country, region, department, intercommunality, municipality), searchable by name and by
//! point, each with a compact geometry decoded on demand. Self-hosted: no external service.
//!
//! Layout: `b"NRSB"`, format version (u8), index length (u32 LE), deflated JSON index, then one
//! deflated geometry blob per unit. A blob is a varint stream: polygon count; per polygon its
//! ring count; per ring its point count and zig-zag deltas of coordinates quantized to
//! [`QUANT`] per degree (1e-4° ≈ 11 m, far below the ~1 km modelling cell and the build
//! simplification tolerance).

use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use serde::{Deserialize, Serialize};

use crate::mask::Rings;

const MAGIC: &[u8; 4] = b"NRSB";
const FORMAT: u8 = 1;
/// Quantization steps per degree.
pub const QUANT: f64 = 1e4;

#[derive(Debug, thiserror::Error)]
pub enum BoundaryError {
    #[error("not a boundary pack: {0}")]
    Format(String),
    #[error("unknown place {0}")]
    UnknownPlace(String),
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Administrative level, from the largest to the smallest; then protected areas (parks,
/// reserves, Natura 2000 sites), which sit outside the administrative hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Country,
    Region,
    Department,
    Intercommunality,
    Municipality,
    Protected,
}

impl Level {
    /// Order on ties in search results: a protected area (often park- or department-sized)
    /// comes after departments and before intercommunalities and municipalities.
    fn rank(self) -> u8 {
        match self {
            Self::Country => 0,
            Self::Region => 1,
            Self::Department => 2,
            Self::Protected => 3,
            Self::Intercommunality => 4,
            Self::Municipality => 5,
        }
    }
}

/// One administrative unit in the index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Place {
    /// Stable code-based id, e.g. `FR-D33` (Gironde), `ES-3401` (Andalucía), `FR-C33063`.
    pub id: String,
    pub name: String,
    pub level: Level,
    /// ISO 3166-1 alpha-2.
    pub country: String,
    pub parent: Option<String>,
    /// West, south, east, north.
    pub bbox: [f64; 4],
    /// Approximate area (km², spherical), for display and ranking.
    pub area_km2: f64,
    /// Inhabitants when the source gives them (ranks homonymous municipalities).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population: Option<u64>,
    /// Other names the unit is searched by (e.g. a country in other languages).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Official designation of a protected area, in the source language (e.g. "Parc naturel
    /// régional", "Parque Natural", "Natura 2000 · ZPS").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designation: Option<String>,
    /// Index of the unit's source in the pack's sources, when a country has several.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<usize>,
    #[serde(default)]
    off: u64,
    #[serde(default)]
    len: u32,
}

/// Where the boundaries come from (licence and attribution are shown with every selection).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Source {
    pub country: String,
    pub name: String,
    pub licence: String,
    pub attribution: String,
    pub url: String,
    pub edition: String,
    /// SHA-256 of the downloaded source file.
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Index {
    format: u8,
    /// Pack identity, e.g. `fr-es-2026.1`; recorded with every study that uses it.
    pack: String,
    built_at: String,
    sources: Vec<Source>,
    places: Vec<Place>,
}

/// Input unit for [`build`] (one NDJSON line from `scripts/build_boundaries.py`).
#[derive(Debug, Clone, Deserialize)]
pub struct UnitIn {
    pub id: String,
    pub name: String,
    pub level: Level,
    pub country: String,
    pub parent: Option<String>,
    #[serde(default)]
    pub population: Option<u64>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub designation: Option<String>,
    #[serde(default)]
    pub source: Option<usize>,
    /// GeoJSON Polygon or MultiPolygon.
    pub geometry: serde_json::Value,
}

// ---- encoding ----------------------------------------------------------------------------------

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn zigzag(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

fn unzigzag(v: u64) -> i64 {
    ((v >> 1) as i64) ^ -((v & 1) as i64)
}

struct Cursor<'a> {
    b: &'a [u8],
    i: usize,
}

impl Cursor<'_> {
    fn varint(&mut self) -> Result<u64, BoundaryError> {
        let mut v = 0u64;
        let mut shift = 0;
        loop {
            let byte = *self
                .b
                .get(self.i)
                .ok_or_else(|| BoundaryError::Format("truncated geometry".into()))?;
            self.i += 1;
            v |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(v);
            }
            shift += 7;
            if shift > 63 {
                return Err(BoundaryError::Format("bad varint".into()));
            }
        }
    }
}

/// Polygons (each a list of rings, exterior first) from GeoJSON Polygon / MultiPolygon.
fn polygons(geometry: &serde_json::Value) -> Result<Vec<Vec<Vec<(f64, f64)>>>, BoundaryError> {
    let ring = |r: &serde_json::Value| -> Vec<(f64, f64)> {
        r.as_array()
            .map(|pts| {
                pts.iter()
                    .filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?)))
                    .collect()
            })
            .unwrap_or_default()
    };
    let poly = |p: &serde_json::Value| -> Vec<Vec<(f64, f64)>> {
        p.as_array()
            .map(|rs| rs.iter().map(ring).collect())
            .unwrap_or_default()
    };
    let coords = &geometry["coordinates"];
    match geometry["type"].as_str() {
        Some("Polygon") => Ok(vec![poly(coords)]),
        Some("MultiPolygon") => Ok(coords
            .as_array()
            .map(|ps| ps.iter().map(poly).collect())
            .unwrap_or_default()),
        other => Err(BoundaryError::Format(format!(
            "unsupported geometry {other:?}"
        ))),
    }
}

fn encode_geometry(polys: &[Vec<Vec<(f64, f64)>>]) -> Vec<u8> {
    let mut raw = Vec::new();
    put_varint(&mut raw, polys.len() as u64);
    let (mut px, mut py) = (0i64, 0i64);
    for p in polys {
        put_varint(&mut raw, p.len() as u64);
        for r in p {
            // Quantize, then drop consecutive duplicates created by quantization.
            let mut q: Vec<(i64, i64)> = r
                .iter()
                .map(|&(x, y)| ((x * QUANT).round() as i64, (y * QUANT).round() as i64))
                .collect();
            q.dedup();
            put_varint(&mut raw, q.len() as u64);
            for (x, y) in q {
                put_varint(&mut raw, zigzag(x - px));
                put_varint(&mut raw, zigzag(y - py));
                (px, py) = (x, y);
            }
        }
    }
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
    // Writing to a Vec cannot fail.
    let _ = enc.write_all(&raw);
    enc.finish().unwrap_or_default()
}

fn decode_geometry(blob: &[u8]) -> Result<Vec<Vec<Vec<(f64, f64)>>>, BoundaryError> {
    let mut raw = Vec::new();
    DeflateDecoder::new(blob).read_to_end(&mut raw)?;
    let mut c = Cursor { b: &raw, i: 0 };
    let (mut px, mut py) = (0i64, 0i64);
    let n = c.varint()?;
    let mut out = Vec::with_capacity(n.min(1 << 16) as usize);
    for _ in 0..n {
        let nr = c.varint()?;
        let mut rings = Vec::with_capacity(nr.min(1 << 16) as usize);
        for _ in 0..nr {
            let np = c.varint()?;
            let mut ring = Vec::with_capacity(np.min(1 << 20) as usize);
            for _ in 0..np {
                px += unzigzag(c.varint()?);
                py += unzigzag(c.varint()?);
                ring.push((px as f64 / QUANT, py as f64 / QUANT));
            }
            rings.push(ring);
        }
        out.push(rings);
    }
    Ok(out)
}

/// Spherical polygon area (km²) of exterior minus holes; good to ~0.5 % at these scales.
fn area_km2(polys: &[Vec<Vec<(f64, f64)>>]) -> f64 {
    const R: f64 = 6_371.008_8;
    let ring_area = |r: &[(f64, f64)]| -> f64 {
        let n = r.len();
        if n < 3 {
            return 0.0;
        }
        let mut s = 0.0;
        for i in 0..n {
            let (x1, y1) = r[i];
            let (x2, y2) = r[(i + 1) % n];
            s += (x2 - x1).to_radians() * (2.0 + y1.to_radians().sin() + y2.to_radians().sin());
        }
        (s * R * R / 2.0).abs()
    };
    polys
        .iter()
        .map(|p| {
            let ext = p.first().map_or(0.0, |r| ring_area(r));
            let holes: f64 = p.iter().skip(1).map(|r| ring_area(r)).sum();
            (ext - holes).max(0.0)
        })
        .sum()
}

fn bbox(polys: &[Vec<Vec<(f64, f64)>>]) -> [f64; 4] {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for &(x, y) in polys.iter().flat_map(|p| p.first()).flatten() {
        b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
    }
    b
}

/// Encode units into a pack. `pack` names it (recorded in study manifests).
pub fn build(
    units: impl IntoIterator<Item = UnitIn>,
    sources: Vec<Source>,
    pack: &str,
    built_at: &str,
) -> Result<Vec<u8>, BoundaryError> {
    let mut places = Vec::new();
    let mut blobs = Vec::new();
    for u in units {
        let polys: Vec<_> = polygons(&u.geometry)?
            .into_iter()
            .filter(|p| p.first().is_some_and(|r| r.len() >= 4))
            .collect();
        if polys.is_empty() {
            continue;
        }
        let blob = encode_geometry(&polys);
        places.push(Place {
            id: u.id,
            name: u.name,
            level: u.level,
            country: u.country,
            parent: u.parent,
            bbox: bbox(&polys),
            area_km2: area_km2(&polys),
            population: u.population,
            aliases: u.aliases,
            designation: u.designation,
            source: u.source,
            off: blobs.len() as u64,
            len: blob.len() as u32,
        });
        blobs.extend_from_slice(&blob);
    }
    let index = Index {
        format: FORMAT,
        pack: pack.to_string(),
        built_at: built_at.to_string(),
        sources,
        places,
    };
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
    enc.write_all(&serde_json::to_vec(&index)?)?;
    let idx = enc.finish()?;
    let mut out = Vec::with_capacity(9 + idx.len() + blobs.len());
    out.extend_from_slice(MAGIC);
    out.push(FORMAT);
    out.extend_from_slice(&(idx.len() as u32).to_le_bytes());
    out.extend_from_slice(&idx);
    out.extend_from_slice(&blobs);
    Ok(out)
}

// ---- reading -----------------------------------------------------------------------------------

/// Lower-case, accents removed, punctuation as spaces: "Saint-Étienne" → "saint etienne".
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase) {
        let r = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a",
            'ç' => "c",
            'è' | 'é' | 'ê' | 'ë' => "e",
            'ì' | 'í' | 'î' | 'ï' => "i",
            'ñ' => "n",
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => "o",
            'ù' | 'ú' | 'û' | 'ü' => "u",
            'ý' | 'ÿ' => "y",
            'œ' => "oe",
            'æ' => "ae",
            'ß' => "ss",
            '-' | '\'' | '’' | '_' | '/' | '(' | ')' | ',' | '.' => " ",
            _ => {
                out.push(c);
                continue;
            }
        };
        out.push_str(r);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A loaded pack: the index in memory, geometries decoded on request.
pub struct BoundaryPack {
    index: Index,
    blobs: Vec<u8>,
    /// Folded name of each place, then its folded aliases.
    folded: Vec<Vec<String>>,
    by_id: std::collections::HashMap<String, usize>,
}

impl BoundaryPack {
    pub fn open(bytes: &[u8]) -> Result<Self, BoundaryError> {
        if bytes.len() < 9 || &bytes[..4] != MAGIC {
            return Err(BoundaryError::Format("missing NRSB header".into()));
        }
        if bytes[4] != FORMAT {
            return Err(BoundaryError::Format(format!(
                "format {} (this NicheRS reads {FORMAT})",
                bytes[4]
            )));
        }
        let n = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]) as usize;
        let idx = bytes
            .get(9..9 + n)
            .ok_or_else(|| BoundaryError::Format("truncated index".into()))?;
        let mut json = Vec::new();
        DeflateDecoder::new(idx).read_to_end(&mut json)?;
        let index: Index = serde_json::from_slice(&json)?;
        let blobs = bytes[9 + n..].to_vec();
        if let Some(p) = index
            .places
            .iter()
            .find(|p| p.off as usize + p.len as usize > blobs.len())
        {
            return Err(BoundaryError::Format(format!(
                "geometry of {} out of range",
                p.id
            )));
        }
        let folded = index
            .places
            .iter()
            .map(|p| {
                std::iter::once(&p.name)
                    .chain(&p.aliases)
                    .map(|n| fold(n))
                    .collect()
            })
            .collect();
        let by_id = index
            .places
            .iter()
            .enumerate()
            .map(|(i, p)| (p.id.clone(), i))
            .collect();
        Ok(Self {
            index,
            blobs,
            folded,
            by_id,
        })
    }

    pub fn pack(&self) -> &str {
        &self.index.pack
    }
    pub fn sources(&self) -> &[Source] {
        &self.index.sources
    }
    pub fn places(&self) -> &[Place] {
        &self.index.places
    }
    pub fn place(&self, id: &str) -> Option<&Place> {
        self.by_id.get(id).map(|&i| &self.index.places[i])
    }

    /// Names from the unit up to its country: "Valence", "Drôme", "Auvergne-Rhône-Alpes", "France".
    pub fn lineage(&self, id: &str) -> Vec<&Place> {
        let mut out = Vec::new();
        let mut cur = self.place(id);
        while let Some(p) = cur {
            out.push(p);
            if out.len() > 8 {
                break;
            }
            cur = p.parent.as_deref().and_then(|q| self.place(q));
        }
        out
    }

    pub fn geometry(&self, id: &str) -> Result<Rings, BoundaryError> {
        let p = self
            .place(id)
            .ok_or_else(|| BoundaryError::UnknownPlace(id.into()))?;
        let blob = &self.blobs[p.off as usize..p.off as usize + p.len as usize];
        Ok(decode_geometry(blob)?.into_iter().flatten().collect())
    }

    /// GeoJSON MultiPolygon of a unit.
    pub fn geojson(&self, id: &str) -> Result<serde_json::Value, BoundaryError> {
        let p = self
            .place(id)
            .ok_or_else(|| BoundaryError::UnknownPlace(id.into()))?;
        let blob = &self.blobs[p.off as usize..p.off as usize + p.len as usize];
        let polys = decode_geometry(blob)?;
        let coords: Vec<Vec<Vec<[f64; 2]>>> = polys
            .iter()
            .map(|rings| {
                rings
                    .iter()
                    .map(|r| {
                        let mut ring: Vec<[f64; 2]> = r.iter().map(|&(x, y)| [x, y]).collect();
                        if ring.first() != ring.last()
                            && let Some(&f) = ring.first()
                        {
                            ring.push(f);
                        }
                        ring
                    })
                    .collect()
            })
            .collect();
        Ok(serde_json::json!({"type": "MultiPolygon", "coordinates": coords}))
    }

    /// Units whose name (or, for later words, an ancestor's name) matches every word of the
    /// query, best first: exact name, then name prefix, then word prefix; larger units first
    /// on ties, so "Gironde" finds the department before "Gironde-sur-Dropt".
    pub fn search(&self, query: &str, country: Option<&str>, limit: usize) -> Vec<&Place> {
        let mut hits = self.matches(query, country);
        hits.sort_by(better);
        hits.into_iter().take(limit).map(|(_, p)| p).collect()
    }

    /// Unsorted matches with their rank (0 exact name, 1 prefix).
    fn matches(&self, query: &str, country: Option<&str>) -> Vec<(u8, &Place)> {
        let q = fold(query);
        let words: Vec<&str> = q.split(' ').filter(|w| !w.is_empty()).collect();
        let Some(first) = words.first() else {
            return Vec::new();
        };
        let mut hits: Vec<(u8, &Place)> = Vec::new();
        for (i, p) in self.index.places.iter().enumerate() {
            if country.is_some_and(|c| !p.country.eq_ignore_ascii_case(c)) {
                continue;
            }
            // The best match among the name and the aliases.
            let Some((rank, name)) = self.folded[i]
                .iter()
                .filter_map(|name| {
                    if *name == q {
                        Some((0u8, name))
                    } else if name.starts_with(&q) || name.split(' ').any(|t| t.starts_with(first))
                    {
                        // A name starting with the query, or with a word starting with it: then
                        // larger units first ("Comunidad de Madrid" before "Madridejos").
                        Some((1, name))
                    } else {
                        None
                    }
                })
                .min_by_key(|(r, _)| *r)
            else {
                continue;
            };
            if words.len() > 1 {
                // Remaining words may name the place itself or one of its ancestors.
                let hay: String = std::iter::once(name.clone())
                    .chain(self.lineage(&p.id).iter().skip(1).map(|a| fold(&a.name)))
                    .collect::<Vec<_>>()
                    .join(" ");
                if !words[1..]
                    .iter()
                    .all(|w| hay.split(' ').any(|t| t.starts_with(w)))
                {
                    continue;
                }
            }
            hits.push((rank, p));
        }
        hits
    }

    /// Every unit containing the point, from the largest to the smallest.
    pub fn at(&self, lng: f64, lat: f64) -> Vec<&Place> {
        let mut hits: Vec<&Place> = self
            .index
            .places
            .iter()
            .filter(|p| {
                lng >= p.bbox[0] && lng <= p.bbox[2] && lat >= p.bbox[1] && lat <= p.bbox[3]
            })
            .filter(|p| {
                self.geometry(&p.id)
                    .is_ok_and(|rings| contains(&rings, lng, lat))
            })
            .collect();
        hits.sort_by_key(|p| p.level);
        hits
    }
}

/// Even–odd point in polygon over all rings (holes and multipolygons included).
pub fn contains(rings: &Rings, x: f64, y: f64) -> bool {
    let mut inside = false;
    for r in rings {
        let n = r.len();
        if n < 3 {
            continue;
        }
        let mut j = n - 1;
        for i in 0..n {
            let (xi, yi) = r[i];
            let (xj, yj) = r[j];
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                inside = !inside;
            }
            j = i;
        }
    }
    inside
}

/// Search order: match rank, then level, then population and area (larger first).
fn better(a: &(u8, &Place), b: &(u8, &Place)) -> std::cmp::Ordering {
    let ((ra, a), (rb, b)) = (a, b);
    ra.cmp(rb)
        .then(a.level.rank().cmp(&b.level.rank()))
        .then(b.population.unwrap_or(0).cmp(&a.population.unwrap_or(0)))
        .then(b.area_km2.total_cmp(&a.area_km2))
}

/// [`BoundaryPack::search`] across several packs (e.g. administrative units and protected
/// areas), ranked together so that one pack cannot crowd out the other.
pub fn search_packs<'a>(
    packs: &'a [BoundaryPack],
    query: &str,
    country: Option<&str>,
    limit: usize,
) -> Vec<(&'a BoundaryPack, &'a Place)> {
    let mut hits: Vec<(&BoundaryPack, (u8, &Place))> = packs
        .iter()
        .flat_map(|pack| {
            pack.matches(query, country)
                .into_iter()
                .map(move |h| (pack, h))
        })
        .collect();
    hits.sort_by(|(_, a), (_, b)| better(a, b));
    hits.into_iter()
        .take(limit)
        .map(|(pack, (_, p))| (pack, p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(
        id: &str,
        name: &str,
        level: Level,
        parent: Option<&str>,
        x0: f64,
        y0: f64,
        s: f64,
    ) -> UnitIn {
        UnitIn {
            id: id.into(),
            name: name.into(),
            level,
            country: "FR".into(),
            parent: parent.map(String::from),
            population: None,
            aliases: vec![],
            designation: None,
            source: None,
            geometry: serde_json::json!({"type": "Polygon", "coordinates": [[[x0, y0], [x0 + s, y0], [x0 + s, y0 + s], [x0, y0 + s], [x0, y0]]]}),
        }
    }

    fn pack() -> BoundaryPack {
        let units = vec![
            square("FR", "France", Level::Country, None, 0.0, 40.0, 10.0),
            square(
                "FR-R1",
                "Nouvelle-Aquitaine",
                Level::Region,
                Some("FR"),
                0.0,
                40.0,
                5.0,
            ),
            square(
                "FR-D33",
                "Gironde",
                Level::Department,
                Some("FR-R1"),
                0.0,
                40.0,
                2.0,
            ),
            square(
                "FR-C33500",
                "Gironde-sur-Dropt",
                Level::Municipality,
                Some("FR-D33"),
                0.0,
                40.0,
                0.1,
            ),
            square(
                "FR-C42218",
                "Saint-Étienne",
                Level::Municipality,
                Some("FR-D33"),
                1.0,
                41.0,
                0.2,
            ),
        ];
        let mut units = units;
        units[0].aliases = vec!["Francia".into(), "Frankreich".into()];
        let bytes = build(units, vec![], "test-1", "2026-10-16").expect("build");
        BoundaryPack::open(&bytes).expect("open")
    }

    #[test]
    fn geometry_round_trips_within_quantization() {
        let p = pack();
        let rings = p.geometry("FR-C42218").expect("geometry");
        let r = &rings[0];
        assert!(
            (r[0].0 - 1.0).abs() <= 0.5 / QUANT && (r[2].1 - 41.2).abs() <= 0.5 / QUANT,
            "{r:?}"
        );
        let km2 = p.place("FR-D33").expect("place").area_km2;
        // 2° × 2° at 40–42° N ≈ 222 km × ~168 km.
        assert!((35_000.0..40_000.0).contains(&km2), "{km2}");
    }

    #[test]
    fn search_ranks_exact_names_and_larger_units_first() {
        let p = pack();
        let ids: Vec<&str> = p
            .search("gironde", None, 5)
            .iter()
            .map(|x| x.id.as_str())
            .collect();
        assert_eq!(ids, ["FR-D33", "FR-C33500"]);
        // Accents and hyphens do not matter; extra words may name an ancestor.
        assert_eq!(p.search("saint etienne", None, 5)[0].id, "FR-C42218");
        assert_eq!(
            p.search("st-étienne gironde", None, 5).len(),
            0,
            "word prefixes only"
        );
        assert_eq!(p.search("saint gironde", None, 5)[0].id, "FR-C42218");
        assert!(p.search("gironde", Some("ES"), 5).is_empty());
        // Aliases find the unit, which keeps its own name.
        let hit = p.search("francia", None, 5);
        assert_eq!((hit[0].id.as_str(), hit[0].name.as_str()), ("FR", "France"));
    }

    #[test]
    fn protected_areas_rank_with_administrative_units_across_packs() {
        let admin = pack();
        let communes: Vec<UnitIn> = (0..4)
            .map(|i| {
                square(
                    &format!("FR-C3800{i}"),
                    &format!("Vercors {i}"),
                    Level::Municipality,
                    Some("FR-D33"),
                    0.0,
                    40.0,
                    0.1,
                )
            })
            .collect();
        let admin2 =
            BoundaryPack::open(&build(communes, vec![], "b", "x").expect("build")).expect("open");
        let mut pnr = square(
            "FR-PFR8000032",
            "Vercors",
            Level::Protected,
            None,
            0.0,
            40.0,
            1.0,
        );
        pnr.designation = Some("Parc naturel régional".into());
        pnr.source = Some(0);
        let protected =
            BoundaryPack::open(&build(vec![pnr], vec![], "p", "x").expect("build")).expect("open");
        let packs = [admin, admin2, protected];
        let hits = search_packs(&packs, "vercors", None, 2);
        assert_eq!(hits[0].1.id, "FR-PFR8000032", "exact name first");
        assert_eq!(
            hits[0].1.designation.as_deref(),
            Some("Parc naturel régional")
        );
        assert_eq!(hits[0].0.pack(), "p");
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn point_lookup_returns_the_whole_hierarchy() {
        let p = pack();
        let ids: Vec<&str> = p.at(1.1, 41.1).iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids, ["FR", "FR-R1", "FR-D33", "FR-C42218"]);
        assert_eq!(p.lineage("FR-C42218").len(), 4);
    }

    #[test]
    fn corrupt_packs_are_refused() {
        assert!(BoundaryPack::open(b"NOPE").is_err());
        let mut bytes = build(vec![], vec![], "x", "y").expect("build");
        bytes[4] = 9;
        assert!(BoundaryPack::open(&bytes).is_err());
    }
}
