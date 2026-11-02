//! Self-contained HTML methods-and-results report (`report.html`), laid out for A4 printing so
//! that a consultancy can attach it, or print it to PDF from any browser. No external resources:
//! maps are embedded PNG data URIs, response curves inline SVG.
//!
//! Wording follows ADR-0007: relative suitability, never a probability; every warning is shown.

use std::fmt::Write;

use niche_geo::{colormap, tiles};

use crate::dto::{JobSpec, OccurrencePoint, ResponseCurve, SpeciesResult, SpeciesRunStatus};
use crate::env::{self, EnvStack};

/// Longest side of the embedded maps, in pixels (nearest-neighbour from the model grid, up- or
/// down-sampled).
const MAP_MAX_PX: usize = 1400;

pub struct Maps<'a> {
    pub stack: &'a EnvStack,
    pub suitability: &'a [f32],
    pub uncertainty: &'a [f32],
    pub mess: &'a [f32],
    /// Dissimilarity index of the area of applicability (only when computed).
    pub aoa: Option<&'a [f32]>,
    /// Target-group records per cell (only with a target-group effort surface, ADR-0013).
    pub effort: Option<&'a [f32]>,
    pub points: &'a [OccurrencePoint],
    /// Change maps of the future-climate projections (ADR-0016), in the result's order.
    pub future: &'a [FutureMap],
}

/// Change classes of one projection, with the GCMs agreeing and the lowest future MESS.
pub struct FutureMap {
    pub period: String,
    pub ssp: String,
    pub gcms: usize,
    pub change: Vec<f32>,
    pub agreement: Vec<f32>,
    pub mess: Vec<f32>,
}

pub(crate) fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(T[((n >> shift) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Render one grid layer to an RGBA image no larger than [`MAP_MAX_PX`] on its longest side.
fn render<F: Fn(f32, usize) -> Option<[u8; 4]>>(
    stack: &EnvStack,
    values: &[f32],
    style: F,
) -> (u32, u32, Vec<u8>) {
    let s = stack.spec;
    // Cells per pixel: below 1 for small grids, so presence dots stay small once printed.
    let scale = s.width.max(s.height) as f64 / MAP_MAX_PX as f64;
    let w = ((s.width as f64 / scale).floor() as usize).max(1);
    let h = ((s.height as f64 / scale).floor() as usize).max(1);
    let mut rgba = vec![0u8; w * h * 4];
    for py in 0..h {
        let r = ((py as f64 + 0.5) * scale) as usize;
        for px in 0..w {
            let c = ((px as f64 + 0.5) * scale) as usize;
            let i = r.min(s.height - 1) * s.width + c.min(s.width - 1);
            let v = values[i];
            let colour = if v.is_nan() { None } else { style(v, i) };
            if let Some(col) = colour {
                rgba[(py * w + px) * 4..(py * w + px) * 4 + 4].copy_from_slice(&col);
            }
        }
    }
    (w as u32, h as u32, rgba)
}

/// Filled dot with a white ring; radius scales with the image so it prints at ~1.5 mm.
fn dot(rgba: &mut [u8], w: u32, h: u32, x: f64, y: f64, fill: [u8; 3]) {
    let (cx, cy) = (x.round() as i64, y.round() as i64);
    let r = (i64::from(w.max(h)) / 180).max(3);
    let inner = (r - (r / 3).max(1)).pow(2);
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = dx * dx + dy * dy;
            if d2 > r * r {
                continue;
            }
            let (px, py) = (cx + dx, cy + dy);
            if px < 0 || py < 0 || px >= i64::from(w) || py >= i64::from(h) {
                continue;
            }
            let o = ((py as u32 * w + px as u32) * 4) as usize;
            let c = if d2 > inner { [255, 255, 255] } else { fill };
            rgba[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
}

fn img(w: u32, h: u32, rgba: &[u8], alt: &str) -> String {
    let png = tiles::encode_image(w, h, rgba);
    format!(
        "<img class=\"map\" alt=\"{}\" src=\"data:image/png;base64,{}\">",
        esc(alt),
        base64(&png)
    )
}

fn map_figures(m: &Maps<'_>, aoa_threshold: Option<f64>, algorithm: &str) -> String {
    let s = m.stack.spec;
    let mut out = String::new();
    // Suitability with retained presences.
    let (w, h, mut rgba) = render(m.stack, m.suitability, |v, _| {
        let c = colormap::viridis(v);
        Some([c[0], c[1], c[2], 255])
    });
    let (sx, sy) = (
        f64::from(w) / (s.width as f64 * s.cell),
        f64::from(h) / (s.height as f64 * s.cell),
    );
    let retained: Vec<&OccurrencePoint> =
        m.points.iter().filter(|p| p.status == "retained").collect();
    for p in &retained {
        dot(
            &mut rgba,
            w,
            h,
            (p.lng - s.west) * sx,
            (s.north - p.lat) * sy,
            [20, 20, 20],
        );
    }
    let _ = write!(
        out,
        "<figure>{}<figcaption><b>Relative habitat suitability</b> ({}, 0–1; not a probability of presence), final model. Dots: the {} presence records used for training (one per grid cell). <span class=\"legend\"><span class=\"ramp\"></span> 0 &nbsp;→&nbsp; 1</span><br>Geographic coordinates (WGS84 longitude/latitude, plate carrée), extent {:.2}° to {:.2}° E, {:.2}° to {:.2}° N; areas in this report are computed on the ellipsoid, not from this picture.</figcaption></figure>",
        img(w, h, &rgba, "Map of relative habitat suitability"),
        match algorithm {
            "rf" => "fraction of the random forest's trees voting presence",
            "ensemble" => "mean percentile rank of MaxEnt and the random forest",
            _ => "cloglog transform",
        },
        retained.len(),
        s.west,
        s.east(),
        s.south(),
        s.north
    );
    let (w, h, rgba) = render(m.stack, m.uncertainty, |v, _| {
        let c = colormap::greys(v / 0.25);
        Some([c[0], c[1], c[2], 255])
    });
    let unc = img(
        w,
        h,
        &rgba,
        "Map of uncertainty across cross-validation models",
    );
    let (w, h, rgba) = render(m.stack, m.mess, |v, i| {
        if v < 0.0 {
            Some([214, 96, 77, 255])
        } else {
            let c = colormap::viridis(m.suitability[i]);
            Some([c[0] / 3 + 150, c[1] / 3 + 150, c[2] / 3 + 150, 255])
        }
    });
    let ext = img(w, h, &rgba, "Map of extrapolation (MESS below zero)");
    let _ = write!(
        out,
        "<div class=\"pair\"><figure>{unc}<figcaption><b>Uncertainty</b>: standard deviation of suitability across the cross-validation models (white 0 → black ≥ 0.25).</figcaption></figure><figure>{ext}<figcaption><b>Extrapolation</b>: red where at least one predictor is outside the training range (MESS &lt; 0); predictions there are least reliable.</figcaption></figure></div>"
    );
    if let (Some(di), Some(t)) = (m.aoa, aoa_threshold) {
        let (w, h, rgba) = render(m.stack, di, |v, i| {
            if f64::from(v) > t {
                Some([120, 60, 160, 255])
            } else {
                let c = colormap::viridis(m.suitability[i]);
                Some([c[0] / 3 + 150, c[1] / 3 + 150, c[2] / 3 + 150, 255])
            }
        });
        let _ = write!(
            out,
            "<figure>{}<figcaption><b>Area of applicability</b> (Meyer &amp; Pebesma 2021): purple where the dissimilarity to the training data exceeds the threshold ({t:.3}), i.e. outside the area where the cross-validation performance applies. Complements the MESS map; it does not change the evidence level.</figcaption></figure>",
            img(w, h, &rgba, "Map of the area of applicability")
        );
    }
    if let Some(e) = m.effort {
        let max = e
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .fold(0.0f32, f32::max);
        let lmax = (1.0 + max).ln().max(f32::EPSILON);
        let (w, h, rgba) = render(m.stack, e, |v, _| {
            if v > 0.0 {
                let c = colormap::effort((1.0 + v).ln() / lmax);
                Some([c[0], c[1], c[2], 255])
            } else {
                Some([235, 235, 235, 255])
            }
        });
        let _ = write!(
            out,
            "<figure>{}<figcaption><b>Sampling effort</b>: GBIF records of the target group per cell (log scale, light = 1 record, dark = {max:.0}); grey: no record. Records, not visits or observers. Used to draw the target-group background.</figcaption></figure>",
            img(w, h, &rgba, "Map of sampling effort of the target group")
        );
    }
    out
}

/// Change-class maps of the projections: faded where fewer than a majority of the GCMs agree,
/// hatched where future conditions are outside the training range under some GCM.
fn future_figures(stack: &EnvStack, maps: &[FutureMap]) -> String {
    let mut out = String::new();
    for f in maps {
        let need = crate::projection::majority(f.gcms) as f32;
        let w0 = stack.spec.width;
        let (w, h, rgba) = render(stack, &f.change, |v, i| {
            let c = crate::projection::class_colour(v)?;
            let faded = f.agreement[i] < need;
            // Hatching in model-grid units, so it stays visible after resampling.
            let (row, col) = (i / w0, i % w0);
            if f.mess[i] < 0.0 && (row + col) % 4 == 0 {
                return Some([60, 60, 60, 255]);
            }
            Some(if faded {
                [c[0] / 2 + 128, c[1] / 2 + 128, c[2] / 2 + 128, 255]
            } else {
                [c[0], c[1], c[2], 255]
            })
        });
        let swatch = |c: [u8; 3]| {
            format!(
                "<span style=\"display:inline-block;width:0.9em;height:0.9em;background:rgb({},{},{});vertical-align:middle\"></span>",
                c[0], c[1], c[2]
            )
        };
        let k = crate::projection::CLASS_COLOURS;
        let _ = write!(
            out,
            "<figure>{}<figcaption><b>Change in suitable conditions, {} {}</b> (mean of {} climate model{}, present P10 threshold): {} suitable now and in the future · {} no longer suitable · {} newly suitable (conditions only; no dispersal modelled) · {} unsuitable in both. Pale: fewer than {} of the {} models agree with the class. Dark diagonal lines: future conditions outside the training range under at least one model (MESS &lt; 0).</figcaption></figure>",
            img(w, h, &rgba, "Map of the change in suitable conditions"),
            esc(&f.period),
            esc(crate::projection::scenario_label(&f.ssp)),
            f.gcms,
            if f.gcms == 1 { "" } else { "s" },
            swatch(k[3]),
            swatch(k[1]),
            swatch(k[2]),
            swatch(k[0]),
            need,
            f.gcms
        );
    }
    out
}

/// `Future climate projections` section of the HTML report.
fn future_section(r: &SpeciesResult, maps: &[FutureMap], stack: Option<&EnvStack>) -> String {
    let mut s = String::from("<h2>Future climate projections</h2>");
    if r.projections.iter().any(|p| p.synthetic) {
        s.push_str("<p class=\"banner\">SYNTHETIC future scenarios (demo): invented warming of the synthetic layers, not CMIP6 data.</p>");
    }
    let _ = write!(s, "<p>{}</p>", esc(&crate::projection::method_text(r)));
    if let Some(p) = r.projections.first() {
        let _ = write!(
            s,
            "<p>Climate predictors projected: {}.{}</p>",
            if p.climate_variables.is_empty() {
                "none (see warnings)".to_string()
            } else {
                esc(&p.climate_variables.join(", "))
            },
            if p.static_variables.is_empty() {
                String::new()
            } else {
                format!(
                    " Kept at present values: {}.",
                    esc(&p.static_variables.join(", "))
                )
            }
        );
    }
    if let Some(st) = stack {
        s.push_str(&future_figures(st, maps));
    }
    s.push_str("<p>Areas in km² (share of the present suitable area); “all GCMs”: part of the class on which every climate model agrees; range: lowest–highest over the individual models.</p><table><thead><tr><th>Period</th><th>Scenario</th><th class=\"n\">GCMs</th><th class=\"n\">present suitable</th><th class=\"n\">future suitable (range)</th><th class=\"n\">suitable in both (all GCMs)</th><th class=\"n\">newly suitable (all GCMs)</th><th class=\"n\">no longer suitable (all GCMs)</th><th class=\"n\">extrapolated</th></tr></thead><tbody>");
    let pct = |x: f64, of: f64| {
        if of > 0.0 {
            format!("{:.0} %", 100.0 * x / of)
        } else {
            "–".into()
        }
    };
    for p in &r.projections {
        let (lo, hi) = p
            .per_gcm
            .iter()
            .map(|g| g.future_suitable_km2)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| {
                (a.min(x), b.max(x))
            });
        let _ = write!(
            s,
            "<tr><td>{}</td><td>{}</td><td class=\"n\">{}</td><td class=\"n\">{:.0}</td><td class=\"n\">{:.0} ({lo:.0}–{hi:.0})</td><td class=\"n\">{:.0} ({}; {:.0})</td><td class=\"n\">{:.0} ({}; {:.0})</td><td class=\"n\">{:.0} ({}; {:.0})</td><td class=\"n\">{:.1} %</td></tr>",
            esc(&p.period),
            esc(crate::projection::scenario_label(&p.ssp)),
            p.gcms.len(),
            p.present_suitable_km2,
            p.future_suitable_km2,
            p.stable_km2,
            pct(p.stable_km2, p.present_suitable_km2),
            p.stable_all_gcms_km2,
            p.gained_km2,
            pct(p.gained_km2, p.present_suitable_km2),
            p.gained_all_gcms_km2,
            p.lost_km2,
            pct(p.lost_km2, p.present_suitable_km2),
            p.lost_all_gcms_km2,
            p.novel_fraction * 100.0
        );
    }
    s.push_str("</tbody></table><p>Per climate model:</p><table><thead><tr><th>Period</th><th>Scenario</th><th>GCM</th><th class=\"n\">future suitable km²</th><th class=\"n\">newly suitable km²</th><th class=\"n\">no longer suitable km²</th><th class=\"n\">extrapolated</th></tr></thead><tbody>");
    for p in &r.projections {
        for g in &p.per_gcm {
            let _ = write!(
                s,
                "<tr><td>{}</td><td>{}</td><td>{}</td><td class=\"n\">{:.0}</td><td class=\"n\">{:.0}</td><td class=\"n\">{:.0}</td><td class=\"n\">{:.1} %</td></tr>",
                esc(&p.period),
                esc(&p.ssp),
                esc(&g.gcm),
                g.future_suitable_km2,
                g.gained_km2,
                g.lost_km2,
                g.novel_fraction * 100.0
            );
        }
    }
    s.push_str("</tbody></table><p>Limits of the projections:</p><ul>");
    for l in crate::projection::LIMITS {
        let _ = write!(s, "<li>{}</li>", esc(l));
    }
    s.push_str("</ul>");
    s
}

/// Small-multiple SVG of the marginal response curves.
fn curves_svg(curves: &[ResponseCurve]) -> String {
    let (w, h, l, b) = (220.0, 130.0, 28.0, 22.0);
    let mut out = String::from("<div class=\"curves\">");
    for c in curves {
        if c.x.is_empty() {
            continue;
        }
        let (x0, x1) = (c.x[0], c.x[c.x.len() - 1]);
        let span = if x1 > x0 { x1 - x0 } else { 1.0 };
        let px = |x: f64| l + (x - x0) / span * (w - l - 6.0);
        let py = |y: f64| 6.0 + (1.0 - y.clamp(0.0, 1.0)) * (h - b - 6.0);
        let mut svg = format!(
            "<svg viewBox=\"0 0 {w} {h}\" role=\"img\" aria-label=\"Response of relative suitability to {}\">",
            esc(&c.name)
        );
        // Outside the training range: shaded (extrapolated, clamped by the model).
        if !c.categorical {
            for (a, z) in [(x0, c.training_min), (c.training_max, x1)] {
                if z > a {
                    let _ = write!(
                        svg,
                        "<rect x=\"{:.1}\" y=\"6\" width=\"{:.1}\" height=\"{:.1}\" class=\"out\"/>",
                        px(a),
                        px(z) - px(a),
                        h - b - 6.0
                    );
                }
            }
        }
        let _ = write!(
            svg,
            "<line x1=\"{l}\" y1=\"{0:.1}\" x2=\"{1:.1}\" y2=\"{0:.1}\" class=\"axis\"/><line x1=\"{l}\" y1=\"6\" x2=\"{l}\" y2=\"{0:.1}\" class=\"axis\"/>",
            h - b,
            w - 6.0
        );
        let _ = write!(
            svg,
            "<text x=\"{:.1}\" y=\"{:.1}\" class=\"tick\" text-anchor=\"end\">1</text><text x=\"{:.1}\" y=\"{:.1}\" class=\"tick\" text-anchor=\"end\">0</text>",
            l - 3.0,
            py(1.0) + 3.0,
            l - 3.0,
            py(0.0)
        );
        let _ = write!(
            svg,
            "<text x=\"{l}\" y=\"{:.1}\" class=\"tick\">{}</text><text x=\"{:.1}\" y=\"{:.1}\" class=\"tick\" text-anchor=\"end\">{}</text>",
            h - 8.0,
            fmt_num(x0),
            w - 6.0,
            h - 8.0,
            fmt_num(x1)
        );
        if c.categorical {
            let bw = ((w - l - 6.0) / c.x.len() as f64 * 0.6).max(2.0);
            for (x, y) in c.x.iter().zip(&c.y) {
                let _ = write!(
                    svg,
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{bw:.1}\" height=\"{:.1}\" class=\"bar\"/>",
                    px(*x) - bw / 2.0,
                    py(*y),
                    py(0.0) - py(*y)
                );
            }
        } else {
            let pts: Vec<String> =
                c.x.iter()
                    .zip(&c.y)
                    .map(|(x, y)| format!("{:.1},{:.1}", px(*x), py(*y)))
                    .collect();
            let _ = write!(
                svg,
                "<polyline points=\"{}\" class=\"line\"/>",
                pts.join(" ")
            );
        }
        svg.push_str("</svg>");
        let _ = write!(
            out,
            "<figure class=\"curve\">{svg}<figcaption>{}</figcaption></figure>",
            esc(&c.name)
        );
    }
    out.push_str("</div>");
    out
}

pub(crate) fn fmt_num(x: f64) -> String {
    if x.abs() >= 100.0 {
        format!("{x:.0}")
    } else if x.abs() >= 1.0 {
        format!("{x:.1}")
    } else {
        format!("{x:.2}")
    }
}

pub(crate) const STYLE: &str = r#"
:root{--ink:#1d2327;--muted:#5b6670;--rule:#d9dee2;--accent:#2a6f4f;--warn:#9a4b00;--bad:#a1281a}
*{box-sizing:border-box}
body{margin:0;background:#fff;color:var(--ink);font:10.5pt/1.45 "Inter","Segoe UI",system-ui,sans-serif}
main{max-width:180mm;margin:0 auto;padding:16mm 0}
h1{font-size:18pt;margin:0 0 2mm}h2{font-size:12.5pt;margin:8mm 0 2mm;border-bottom:1px solid var(--rule);padding-bottom:1mm}
.meta{color:var(--muted);font-size:9pt}
.banner{border:2px solid var(--bad);color:var(--bad);padding:2mm 3mm;font-weight:600;margin:3mm 0}
.notice{background:#f3f6f4;border-left:3px solid var(--accent);padding:2mm 3mm;margin:3mm 0}
.level{display:inline-block;font-weight:700;padding:.5mm 2.5mm;border-radius:1mm;background:var(--accent);color:#fff}
.level.Exploratory{background:var(--warn)}.level.Insufficient{background:var(--bad)}
table{border-collapse:collapse;width:100%;font-size:9pt;margin:2mm 0}
th,td{border-bottom:1px solid var(--rule);padding:1mm 1.5mm;text-align:left;vertical-align:top}
td.n,th.n{text-align:right;font-variant-numeric:tabular-nums}
tr.sel td{font-weight:700}
.warn li{color:var(--warn)}
figure{margin:3mm 0}figcaption{font-size:8.5pt;color:var(--muted)}
img.map{width:100%;height:auto;border:1px solid var(--rule)}
.pair{display:grid;grid-template-columns:1fr 1fr;gap:4mm}
.legend{white-space:nowrap}.ramp{display:inline-block;width:28mm;height:2.5mm;vertical-align:middle;background:linear-gradient(90deg,#440154,#472d7b,#3b528b,#2c728e,#21918c,#28ae80,#5ec962,#addc30,#fde725)}
.curves{display:grid;grid-template-columns:repeat(3,1fr);gap:2mm 4mm}
.curve svg{width:100%;height:auto}
.axis{stroke:#8a949c;stroke-width:.8}.line{fill:none;stroke:var(--accent);stroke-width:1.6}.bar{fill:var(--accent)}
.out{fill:#eceff1}.tick{font-size:8px;fill:var(--muted)}
.cite li{font-size:9pt}
@page{size:A4;margin:14mm 15mm}
@media print{main{padding:0;max-width:none}h2{break-after:avoid}figure,table,.curve{break-inside:avoid}}
"#;

pub fn report_html(r: &SpeciesResult, spec: &JobSpec, maps: Option<&Maps<'_>>) -> String {
    let mut s = String::new();
    let _ = write!(
        s,
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Habitat suitability — {}</title><style>{STYLE}</style></head><body><main>",
        esc(&r.scientific_name)
    );
    let _ = write!(
        s,
        "<h1>Habitat suitability model — <i>{}</i></h1><p class=\"meta\">Study area: {} · Job: {} · Generated by NicheRS {} on {} · GBIF taxon key {}</p>",
        esc(&r.scientific_name),
        esc(&spec.area.name),
        esc(&spec.name),
        env!("CARGO_PKG_VERSION"),
        jiff::Timestamp::now().strftime("%Y-%m-%d %H:%M UTC"),
        r.species_key
    );
    if spec.demo {
        s.push_str("<p class=\"banner\">SYNTHETIC DEMONSTRATION DATA — not a real species or a real environment. Do not use for any decision.</p>");
    }
    let extra = env::uses_additional_predictors(&spec.layers);
    if extra {
        s.push_str("<p class=\"notice\">The map and figures show <b>relative habitat suitability</b> of environmental conditions (climate and the terrain, land-cover or water predictors listed under Data sources), estimated from presence records and background points. It is not a probability of presence, not the occupied range and not a substitute for field survey. Interpretation is the responsibility of the professional who signs the study.</p>");
    } else {
        s.push_str("<p class=\"notice\">The map and figures show <b>relative habitat suitability</b> of climatic conditions, estimated from presence records and background points. It is not a probability of presence, not the occupied range and not a substitute for field survey. Interpretation is the responsibility of the professional who signs the study.</p>");
    }

    let _ = write!(
        s,
        "<h2>Evidence level</h2><p><span class=\"level {0:?}\">{0:?}</span></p><ul>",
        r.evidence.level
    );
    for reason in &r.evidence.reasons {
        let _ = write!(s, "<li>{}</li>", esc(reason));
    }
    s.push_str("</ul>");
    if !r.warnings.is_empty() {
        s.push_str("<h2>Warnings</h2><ul class=\"warn\">");
        for w in &r.warnings {
            let _ = write!(s, "<li><b>{:?}</b>: {}</li>", w.severity, esc(&w.message));
        }
        s.push_str("</ul>");
    }
    if r.status != SpeciesRunStatus::Succeeded {
        if let Some(e) = &r.error {
            let _ = write!(s, "<p>{}</p>", esc(e));
        }
        s.push_str(
            "<p>No model was fitted for this species, so there are no maps or model results.</p>",
        );
    }

    if r.status == SpeciesRunStatus::Succeeded
        && let Some(m) = maps
    {
        s.push_str("<h2>Maps</h2>");
        s.push_str(&map_figures(
            m,
            r.aoa.as_ref().map(|a| a.threshold),
            r.selected.algorithm(),
        ));
    }

    s.push_str("<h2>Data</h2><table><tbody>");
    let row = |s: &mut String, k: &str, v: String| {
        let _ = write!(s, "<tr><th>{k}</th><td>{v}</td></tr>");
    };
    row(
        &mut s,
        "Study area",
        format!(
            "{} — {:.0} km² of land with predictor data",
            esc(&spec.area.name),
            r.study_area_km2
        ),
    );
    row(&mut s, "Occurrence source", esc(&r.data.source));
    if let Some(q) = &r.data.query_url {
        row(&mut s, "Query", format!("<code>{}</code>", esc(q)));
    }
    row(
        &mut s,
        "Records",
        format!(
            "{} retrieved; {} grid cells retained after cleaning (one presence per cell)",
            r.data.records_fetched, r.data.presences
        ),
    );
    row(
        &mut s,
        "Filters",
        format!(
            "records from {} onwards; coordinate uncertainty ≤ {} m; licences {}",
            spec.filters.year_min,
            spec.filters.max_uncertainty_m,
            esc(&format!("{:?}", spec.filters.licences))
        ),
    );
    if let Some(km) = spec.filters.thin_km {
        row(
            &mut s,
            "Distance thinning",
            format!(
                "presence cells at least {km} km apart (spThin, Aiello-Lammens et al. 2015), seeded by the study seed"
            ),
        );
    }
    if let Some(g) = crate::artifacts::generalised_text(r, spec.filters.max_uncertainty_m) {
        row(&mut s, "Generalised coordinates", esc(&g));
    }
    if !spec.demo {
        row(
            &mut s,
            "Sensitive species",
            esc(crate::artifacts::SINP_NOTE),
        );
    }
    row(
        &mut s,
        "Background",
        if r.background_sampling.is_some() {
            esc(&crate::artifacts::background_text(r))
        } else {
            format!(
                "{} random land cells within the study area",
                r.data.background
            )
        },
    );
    if let Some(b) = &r.sampling_bias {
        row(
            &mut s,
            "Sampling-bias diagnostic",
            format!(
                "{} records of the target group {}{} in {} cells ({:.1} % of the study area). {:.0} % of the presence cells lie in the {:.0} % of the study area with most records ({:.0} % expected if presences ignored observer effort). Descriptive only; no effect on the evidence level.",
                b.records,
                esc(&b.target_group),
                if b.synthetic { " (synthetic)" } else { "" },
                b.cells_with_effort,
                b.share_of_area_with_effort * 100.0,
                b.presences_in_top10pct_effort_cells * 100.0,
                b.expected * 100.0,
                b.expected * 100.0
            ),
        );
    }
    s.push_str("</tbody></table>");
    if !r.data.exclusions.is_empty() {
        s.push_str("<table><thead><tr><th>Excluded records — reason</th><th class=\"n\">count</th></tr></thead><tbody>");
        for e in &r.data.exclusions {
            let _ = write!(
                s,
                "<tr><td>{}</td><td class=\"n\">{}</td></tr>",
                esc(&e.reason),
                e.count
            );
        }
        s.push_str("</tbody></table>");
    }

    s.push_str(&crate::independent::sources_html(r, spec));
    if r.status == SpeciesRunStatus::Succeeded {
        let algorithm = r.selected.algorithm();
        if algorithm == "maxent" {
            s.push_str("<h2>Predictors</h2><p>Percent contribution: share of the training gain attributed to the variable. Permutation importance: drop in training AUC when the variable's values are randomly permuted, normalised to 100 (MaxEnt 3.4.4 definitions). Correlated variables share credit: a low value does not mean a variable is ecologically irrelevant.</p>");
            s.push_str("<table><thead><tr><th>Variable</th><th class=\"n\">Percent contribution</th><th class=\"n\">Permutation importance</th></tr></thead><tbody>");
            for c in &r.contributions {
                let _ = write!(
                    s,
                    "<tr><td>{}</td><td class=\"n\">{:.1}</td><td class=\"n\">{}</td></tr>",
                    esc(&c.variable),
                    c.percent,
                    c.permutation_importance
                        .map_or("–".into(), |p| format!("{p:.1}"))
                );
            }
            s.push_str("</tbody></table>");
        } else {
            let _ = write!(
                s,
                "<h2>Predictors</h2><p>{}{}</p>",
                if algorithm == "ensemble" {
                    "MaxEnt member: percent contribution (share of the training gain) and permutation importance (MaxEnt 3.4.4 definitions). "
                } else {
                    ""
                },
                esc(&crate::artifacts::RF_IMPORTANCE_TEXT.replace('`', ""))
            );
            if algorithm == "ensemble" {
                s.push_str("<table><thead><tr><th>Variable</th><th class=\"n\">MaxEnt percent contribution</th><th class=\"n\">MaxEnt permutation importance</th><th class=\"n\">RF permutation importance</th></tr></thead><tbody>");
                for c in &r.contributions {
                    let _ = write!(
                        s,
                        "<tr><td>{}</td><td class=\"n\">{:.1}</td><td class=\"n\">{}</td><td class=\"n\">{}</td></tr>",
                        esc(&c.variable),
                        c.percent,
                        c.permutation_importance
                            .map_or("–".into(), |p| format!("{p:.1}")),
                        c.rf_importance.map_or("–".into(), |p| format!("{p:.1}"))
                    );
                }
            } else {
                s.push_str("<table><thead><tr><th>Variable</th><th class=\"n\">RF permutation importance</th></tr></thead><tbody>");
                for c in &r.contributions {
                    let _ = write!(
                        s,
                        "<tr><td>{}</td><td class=\"n\">{:.1}</td></tr>",
                        esc(&c.variable),
                        c.percent
                    );
                }
            }
            s.push_str("</tbody></table>");
        }
        for d in &r.variables_dropped {
            let _ = write!(
                s,
                "<p class=\"meta\">Dropped {}: {}</p>",
                esc(&d.variable),
                esc(&d.reason)
            );
        }
        if !r.response_curves.is_empty() {
            s.push_str("<h2>Response curves</h2><p>Marginal response of relative suitability (cloglog) to each variable, all other variables held at their mean over the presence records. Grey bands: values outside the training data, where the model is clamped and the curve is an extrapolation. With correlated variables, marginal curves can hide effects of variables changing together.</p>");
            s.push_str(&curves_svg(&r.response_curves));
        }
        if !r.partial_dependence.is_empty() {
            s.push_str("<h2>Partial dependence (random forest)</h2><p>Mean relative suitability (fraction of trees voting presence) as each variable varies over its training range, averaged over the observed values of the other variables (Friedman 2001). Not a MaxEnt response curve: the other variables are not held at their mean. With correlated variables, partial dependence also averages over unrealistic combinations.</p>");
            s.push_str(&curves_svg(&r.partial_dependence));
        }

        if algorithm == "maxent" {
            let _ = write!(
                s,
                "<h2>Model</h2><p>MaxEnt (NicheRS port of MaxEnt 3.4.4; Phillips et al. 2006, 2017; outputs reproduce <code>maxent.jar</code> on the reference cases), cloglog output. Feature classes <b>{}</b> and regularization multiplier <b>{}</b> were selected among {} candidate settings by lowest mean 10th-percentile omission rate, then highest Boyce index, under {} cross-validation ({} folds).</p>",
                esc(&r.selected.feature_classes),
                r.selected.beta_multiplier,
                r.tuning.len().max(1),
                esc(&r.validation.method.replace('_', " ")),
                r.validation.folds.len()
            );
            s.push_str("<table><thead><tr><th>Features</th><th class=\"n\">β</th><th class=\"n\">OR10</th><th class=\"n\">Boyce</th><th class=\"n\">AUC</th><th class=\"n\">non-zero features</th></tr></thead><tbody>");
            for t in &r.tuning {
                let _ = write!(
                    s,
                    "<tr{}><td>{}</td><td class=\"n\">{}</td><td class=\"n\">{:.3}</td><td class=\"n\">{}</td><td class=\"n\">{:.3}</td><td class=\"n\">{}</td></tr>",
                    if t.selected { " class=\"sel\"" } else { "" },
                    esc(&t.feature_classes),
                    t.beta_multiplier,
                    t.or10_mean,
                    t.cbi_mean.map_or("–".into(), |c| format!("{c:.3}")),
                    t.auc_mean,
                    t.n_nonzero
                );
            }
            s.push_str("</tbody></table>");
        } else {
            let _ = write!(
                s,
                "<h2>Model</h2><p>{}</p>",
                esc(&crate::artifacts::model_text(r))
            );
            s.push_str("<table><thead><tr><th>Algorithm</th><th>Setting</th><th class=\"n\">OR10</th><th class=\"n\">Boyce</th><th class=\"n\">AUC</th></tr></thead><tbody>");
            for t in &r.tuning {
                let (name, setting) = match t.mtry {
                    Some(m) => ("Random forest".to_string(), format!("mtry = {m}")),
                    None => (
                        "MaxEnt".to_string(),
                        format!("{} β = {}", t.feature_classes, t.beta_multiplier),
                    ),
                };
                let _ = write!(
                    s,
                    "<tr{}><td>{}</td><td>{}</td><td class=\"n\">{:.3}</td><td class=\"n\">{}</td><td class=\"n\">{:.3}</td></tr>",
                    if t.selected { " class=\"sel\"" } else { "" },
                    name,
                    esc(&setting),
                    t.or10_mean,
                    t.cbi_mean.map_or("–".into(), |c| format!("{c:.3}")),
                    t.auc_mean
                );
            }
            s.push_str("</tbody></table>");
        }
        if let Some(b) = &r.validation.blocks {
            let _ = write!(
                s,
                "<h2>Spatial block design</h2><p>Square spatial blocks of <b>{:.0} km</b> ({}), assigned to folds so that presence counts are balanced (Valavi et al. 2019, blockCV; Roberts et al. 2017). {} folds requested, {} obtained.</p>",
                b.block_km,
                if b.chosen_by == "user" {
                    "size given by the user"
                } else {
                    "sized by the spatial autocorrelation of the predictors"
                },
                b.k_requested,
                b.k_achieved
            );
            if let Some(a) = &b.autocorrelation {
                let _ = write!(
                    s,
                    "<p>Variogram ranges of the predictors on {} background points (exponential, spherical and Gaussian models, weighted least squares; lag cutoff {:.0} km). Median {}; block size clamped to [{:.0}, {:.0}] km.</p><table><thead><tr><th>Variable</th><th class=\"n\">range (km)</th><th>model</th></tr></thead><tbody>",
                    a.n_points,
                    a.cutoff_km,
                    a.median_km
                        .map_or("not estimable".into(), |m| format!("{m:.0} km")),
                    a.min_km,
                    a.max_km
                );
                for v in &a.variables {
                    let _ = write!(
                        s,
                        "<tr><td>{}</td><td class=\"n\">{}</td><td>{}</td></tr>",
                        esc(&v.variable),
                        v.range_km.map_or("–".into(), |x| format!(
                            "{}{x:.0}",
                            if v.at_upper_bound { "≥ " } else { "" }
                        )),
                        esc(v.model.as_deref().unwrap_or("–"))
                    );
                }
                s.push_str("</tbody></table>");
            }
        }
        s.push_str("<h2>Validation on held-out spatial blocks</h2><table><thead><tr><th>Fold</th><th class=\"n\">test presences</th><th class=\"n\">Boyce</th><th class=\"n\">OR10</th><th class=\"n\">ORmtp</th><th class=\"n\">AUC (presence–background)</th></tr></thead><tbody>");
        for f in &r.validation.folds {
            let _ = write!(
                s,
                "<tr><td>{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td class=\"n\">{:.2}</td><td class=\"n\">{:.2}</td><td class=\"n\">{:.2}</td></tr>",
                f.fold,
                f.n_test,
                f.cbi.map_or("–".into(), |c| format!("{c:.2}")),
                f.or10,
                f.ormtp,
                f.auc
            );
        }
        let _ = write!(
            s,
            "<tr class=\"sel\"><td>Mean</td><td></td><td class=\"n\">{}</td><td class=\"n\">{:.2}</td><td class=\"n\">{:.2}</td><td class=\"n\">{:.2}</td></tr></tbody></table><p class=\"meta\">Presence–background AUC cannot reach 1 and depends on the extent of the study area; the Boyce index and omission rates carry most of the evidence.</p>",
            r.validation
                .cbi_mean
                .map_or("–".into(), |c| format!("{c:.2}")),
            r.validation.or10_mean,
            r.validation.ormtp_mean,
            r.validation.auc_mean
        );
        if !r.validation.members.is_empty() {
            s.push_str("<p>These are the metrics of the ensemble itself (fold by fold, the ensemble of the two members' fold models). Members on the same folds:</p><table><thead><tr><th>Member</th><th class=\"n\">mean Boyce</th><th class=\"n\">mean OR10</th><th class=\"n\">mean AUC</th></tr></thead><tbody>");
            for m in &r.validation.members {
                let _ = write!(
                    s,
                    "<tr><td>{}</td><td class=\"n\">{}</td><td class=\"n\">{:.2}</td><td class=\"n\">{:.2}</td></tr>",
                    esc(crate::artifacts::member_label(&m.algorithm)),
                    m.cbi_mean.map_or("–".into(), |c| format!("{c:.2}")),
                    m.or10_mean,
                    m.auc_mean
                );
            }
            s.push_str("</tbody></table>");
        }

        if let Some(nm) = &r.validation.null_model {
            let _ = write!(
                s,
                "<h2>Null models</h2><p>The selected setting was refitted on {} sets of {} random locations ({} sampling over cells with predictor data), each partitioned and evaluated like the real records (Raes &amp; ter Steege 2007; Bohl et al. 2019; ENMeval 2.0, Kass et al. 2021); {} of {} null sets were fitted. <i>p</i> is the empirical one-sided p-value (1 + nulls at least as good) / (n + 1). A small <i>p</i> means the model is better than random-location models; it does not show that the model is ecologically right.</p><table><thead><tr><th>Metric</th><th class=\"n\">observed</th><th class=\"n\">null mean ± SD</th><th class=\"n\">z</th><th class=\"n\">p</th></tr></thead><tbody>",
                nm.requested,
                nm.presences,
                esc(crate::artifacts::null_sampling_label(&nm.sampling)),
                nm.completed,
                nm.requested
            );
            for (label, st) in [
                ("Boyce index (higher is better)", &nm.cbi),
                ("OR10 (lower is better)", &nm.or10),
            ] {
                let f = |x: Option<f64>| x.map_or("–".into(), |v| format!("{v:.3}"));
                let _ = write!(
                    s,
                    "<tr><td>{label}</td><td class=\"n\">{}</td><td class=\"n\">{} ± {}</td><td class=\"n\">{}</td><td class=\"n\">{:.3}</td></tr>",
                    f(st.observed),
                    f(st.mean),
                    f(st.sd),
                    f(st.z),
                    st.p_value
                );
            }
            s.push_str("</tbody></table>");
        }
        s.push_str(&crate::independent::validation_html(r));
        s.push_str("<h2>Area of suitable conditions</h2><p>Sums of exact WGS84 ellipsoidal cell areas above each threshold. They describe the extent of suitable <i>conditions</i>, not the occupied range; the spread across thresholds is part of the result.</p><table><thead><tr><th>Threshold rule</th><th class=\"n\">threshold</th><th class=\"n\">km²</th><th class=\"n\">% of study area</th></tr></thead><tbody>");
        for t in &r.thresholds {
            let _ = write!(
                s,
                "<tr><td>{}</td><td class=\"n\">{:.3}</td><td class=\"n\">{:.0}</td><td class=\"n\">{:.1}</td></tr>",
                esc(&t.label),
                t.threshold,
                t.area_km2,
                t.fraction_of_study_area * 100.0
            );
        }
        s.push_str("</tbody></table>");
        let _ = write!(
            s,
            "<h2>Uncertainty and extrapolation</h2><ul><li>Mean standard deviation of suitability across fold models: {:.3}.</li><li>Share of the study area with MESS &lt; 0 (outside training conditions): {:.1} %.</li></ul>",
            r.uncertainty.mean_fold_sd,
            r.extrapolation.novel_fraction * 100.0
        );
        if let Some(a) = &r.aoa {
            let _ = write!(
                s,
                "<p>Area of applicability (Meyer &amp; Pebesma 2021): <b>{:.1} %</b> of the study area is outside it (dissimilarity index &gt; {:.3}, the upper whisker of the cross-validated training DI; {} training points; predictors weighted by permutation importance). It complements MESS and does not change the evidence level (ADR-0014).</p>",
                a.fraction_outside * 100.0,
                a.threshold,
                a.n_train
            );
        }
    }

    if !r.projections.is_empty() {
        s.push_str(&future_section(
            r,
            maps.map_or(&[][..], |m| m.future),
            maps.map(|m| m.stack),
        ));
    }

    let bias = if r
        .background_sampling
        .as_ref()
        .is_some_and(|b| b.method == "target_group")
    {
        format!("<li>{}</li>", esc(&crate::artifacts::bias_limit_text(r)))
    } else {
        "<li>Opportunistic occurrence data carry sampling bias (observer effort, accessibility); no bias correction was applied.</li>".to_string()
    };
    // The random forest's output semantics (ADR-0017), before the bias sentence.
    let bias = match crate::artifacts::rf_limit_text(r) {
        Some(t) => format!("<li>{}</li>{bias}", esc(t)),
        None => bias,
    };
    if extra {
        s.push_str("<h2>Data sources</h2><table><thead><tr><th>Source</th><th>Layers</th><th>Licence and attribution</th></tr></thead><tbody>");
        for c in env::sources_used(&spec.layers) {
            let _ = write!(
                s,
                "<tr><td>{} ({})</td><td>{}</td><td>{} — <code>{}</code>{}</td></tr>",
                esc(&c.source),
                esc(&c.vintage),
                esc(&c.layers.join(", ")),
                esc(&c.licence),
                esc(&c.licence_url),
                c.attribution
                    .as_ref()
                    .map_or(String::new(), |a| format!("<br>{}", esc(a)))
            );
        }
        let _ = write!(
            s,
            "<tr><td>{}</td><td>land mask</td><td>public domain</td></tr></tbody></table><p>{}</p><ul>",
            esc(niche_data::landmask::SOURCE),
            esc(&env::predictor_methods(spec.resolution_factor))
        );
        for c in env::sources_used(&spec.layers) {
            for m in &c.methods {
                let _ = write!(s, "<li>{}</li>", esc(m));
            }
        }
        s.push_str("</ul>");
        let _ = write!(
            s,
            "<h2>Interpretation limits</h2><ul><li>Values are relative habitat suitability. Presence–background data cannot estimate prevalence, so they are not probabilities of presence.</li>{bias}<li>Predictors are those listed under Data sources: soils, fine habitat structure and dispersal limits are not represented.</li>"
        );
        for l in env::predictor_limits(&spec.layers) {
            let _ = write!(s, "<li>{}</li>", esc(l));
        }
        s.push_str("<li>This report does not replace a field inventory, a proof of presence or absence, or an impact assessment.</li></ul>");
    } else {
        let _ = write!(
            s,
            "<h2>Interpretation limits</h2><ul><li>Values are relative habitat suitability. Presence–background data cannot estimate prevalence, so they are not probabilities of presence.</li>{bias}<li>Predictors are climate (CHELSA v2.1, 1981–2010) only: land cover, soils, habitat structure and dispersal limits are not represented.</li><li>This report does not replace a field inventory, a proof of presence or absence, or an impact assessment.</li></ul>"
        );
    }
    let _ = write!(
        s,
        "<h2>Reproducibility and citations</h2><p>Every input, version, seed and setting of this run is recorded in the job's <code>manifest.json</code> (with file checksums); {}.</p><ul class=\"cite\">",
        match r.selected.algorithm() {
            "rf" => "the final model is <code>model.rf.json</code>",
            "ensemble" =>
                "the final members are <code>model.lambdas</code> (MaxEnt) and <code>model.rf.json</code> (random forest)",
            _ => "the final model is <code>model.lambdas</code>",
        }
    );
    let _ = write!(
        s,
        "<li>Occurrences: {}</li>",
        if spec.demo {
            "synthetic demonstration data".to_string()
        } else {
            esc(&r.data.source)
        }
    );
    let mut credits: Vec<&str> = spec
        .area
        .places
        .iter()
        .map(|p| p.attribution.as_str())
        .collect();
    credits.sort_unstable();
    credits.dedup();
    for c in credits {
        let _ = write!(s, "<li>Study area boundaries: {}</li>", esc(c));
    }
    s.push_str("<li>Karger, D.N. et al. (2017) Climatologies at high resolution for the earth's land surface areas. <i>Scientific Data</i> 4, 170122. CHELSA v2.1.</li>");
    if r.selected.algorithm() != "rf" {
        s.push_str("<li>Phillips, S.J., Anderson, R.P. &amp; Schapire, R.E. (2006) Maximum entropy modeling of species geographic distributions. <i>Ecological Modelling</i> 190, 231–259.</li><li>Phillips, S.J. et al. (2017) Opening the black box: an open-source release of Maxent. <i>Ecography</i> 40, 887–893.</li>");
    }
    s.push_str("<li>Hirzel, A.H. et al. (2006) Evaluating the ability of habitat suitability models to predict species presences. <i>Ecological Modelling</i> 199, 142–152.</li>");
    for c in env::sources_used(&spec.layers) {
        if c.source != niche_data::chelsa::SOURCE {
            let _ = write!(s, "<li>{}</li>", esc(&c.citation));
            if let Some(a) = &c.attribution {
                let _ = write!(s, "<li>{}</li>", esc(a));
            }
        }
    }
    if r.validation.blocks.is_some() {
        s.push_str("<li>Roberts, D.R. et al. (2017) Cross-validation strategies for data with temporal, spatial, hierarchical, or phylogenetic structure. <i>Ecography</i> 40, 913–929.</li><li>Valavi, R. et al. (2019) blockCV: an R package for generating spatially or environmentally separated folds for k-fold cross-validation of species distribution models. <i>Methods in Ecology and Evolution</i> 10, 225–232.</li>");
    }
    if r.validation.null_model.is_some() {
        s.push_str("<li>Raes, N. &amp; ter Steege, H. (2007) A null-model for significance testing of presence-only species distribution models. <i>Ecography</i> 30, 727–736.</li><li>Bohl, C.L., Kass, J.M. &amp; Anderson, R.P. (2019) A new null model approach to quantify performance and significance for ecological niche models of species distributions. <i>Journal of Biogeography</i> 46, 1101–1111.</li><li>Kass, J.M. et al. (2021) ENMeval 2.0: redesigned for customizable and reproducible modeling of species' niches and distributions. <i>Methods in Ecology and Evolution</i> 12, 1602–1608.</li>");
    }
    if r.aoa.is_some() {
        s.push_str("<li>Meyer, H. &amp; Pebesma, E. (2021) Predicting into unknown space? Estimating the area of applicability of spatial prediction models. <i>Methods in Ecology and Evolution</i> 12, 1620–1633.</li>");
    }
    for x in crate::artifacts::references(r, spec)
        .into_iter()
        .chain(crate::artifacts::model_references(r))
    {
        let _ = write!(s, "<li>{}</li>", esc(x));
    }
    if !r.projections.is_empty() {
        for x in crate::projection::references(r.projections.iter().any(|p| p.synthetic)) {
            let _ = write!(s, "<li>{}</li>", esc(&x));
        }
    }
    s.push_str("</ul>");
    s.push_str("</main></body></html>");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        for (i, o) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(i.as_bytes()), o);
        }
    }

    #[test]
    fn escapes_markup() {
        assert_eq!(
            esc("<a href=\"x\">&</a>"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;"
        );
    }
}
