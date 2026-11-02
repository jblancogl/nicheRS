//! Colour ramps for rendering, chosen by data semantics (docs/design/design-tokens.md):
//! suitability → viridis (perceptually uniform, colour-vision-deficiency safe; Crameri et al.
//! 2020); uncertainty → single-hue lightness ramp; extrapolation → hatching.

/// Viridis anchor stops (matplotlib), linearly interpolated in sRGB.
const VIRIDIS: [(f32, [u8; 3]); 9] = [
    (0.0, [0x44, 0x01, 0x54]),
    (0.125, [0x48, 0x28, 0x78]),
    (0.25, [0x3e, 0x49, 0x89]),
    (0.375, [0x31, 0x68, 0x8e]),
    (0.5, [0x26, 0x82, 0x8e]),
    (0.625, [0x1f, 0x9e, 0x89]),
    (0.75, [0x35, 0xb7, 0x79]),
    (0.875, [0x6e, 0xce, 0x58]),
    (1.0, [0xfd, 0xe7, 0x25]),
];

pub fn viridis(t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    for w in VIRIDIS.windows(2) {
        let ((t0, c0), (t1, c1)) = (w[0], w[1]);
        if t <= t1 {
            let f = (t - t0) / (t1 - t0);
            let mix =
                |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * f).round() as u8;
            return [mix(c0[0], c1[0]), mix(c0[1], c1[1]), mix(c0[2], c1[2])];
        }
    }
    VIRIDIS[8].1
}

/// Uncertainty: light grey (low) → near-black (high); `t` already normalized to 0..1.
pub fn greys(t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let v = (235.0 - t * 200.0).round() as u8;
    [v, v, v]
}

/// Binary suitable class colour (matches viridis high end for continuity).
pub const SUITABLE: [u8; 3] = [0x21, 0x91, 0x8c];
/// Hatch colour for extrapolation overlays.
pub const HATCH: [u8; 3] = [0xc2, 0x41, 0x0c];

/// Sampling effort (records per cell): light orange (few) → dark brown (many), ColorBrewer
/// "Oranges" anchors, distinct from the suitability ramp; `t` already normalized to 0..1 (the
/// caller applies a log scale).
pub fn effort(t: f32) -> [u8; 3] {
    const STOPS: [(f32, [u8; 3]); 3] = [
        (0.0, [0xfe, 0xe6, 0xce]),
        (0.5, [0xf1, 0x69, 0x13]),
        (1.0, [0x7f, 0x27, 0x04]),
    ];
    let t = t.clamp(0.0, 1.0);
    for w in STOPS.windows(2) {
        let ((t0, c0), (t1, c1)) = (w[0], w[1]);
        if t <= t1 {
            let f = (t - t0) / (t1 - t0);
            let mix =
                |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * f).round() as u8;
            return [mix(c0[0], c1[0]), mix(c0[1], c1[1]), mix(c0[2], c1[2])];
        }
    }
    STOPS[2].1
}

fn ramp(stops: &[(f32, [u8; 3])], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    for w in stops.windows(2) {
        let ((t0, c0), (t1, c1)) = (w[0], w[1]);
        if t <= t1 {
            let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            let mix =
                |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * f).round() as u8;
            return [mix(c0[0], c1[0]), mix(c0[1], c1[1]), mix(c0[2], c1[2])];
        }
    }
    stops.last().map_or([0, 0, 0], |s| s.1)
}

/// Survey priority score (ADR-0018): pale yellow (low) → red → deep purple (high), so it is not
/// read as the suitability ramp.
pub fn priority(t: f32) -> [u8; 3] {
    ramp(
        &[
            (0.0, [0xff, 0xff, 0xcc]),
            (0.35, [0xfd, 0x8d, 0x3c]),
            (0.7, [0xe3, 0x1a, 0x1c]),
            (1.0, [0x54, 0x27, 0x88]),
        ],
        t,
    )
}

/// Survey-priority classes (ADR-0018), codes 1–4: prospect first, already well sampled, lower
/// suitability and little sampled, lower suitability and well sampled.
pub const PRIORITY_CLASSES: [[u8; 3]; 4] = [
    [0xd9, 0x5f, 0x02],
    [0x1b, 0x9e, 0x77],
    [0xfd, 0xd4, 0x9e],
    [0xc7, 0xe9, 0xc0],
];

/// Diverging ramp for differences (−1 … +1): blue (negative) → white → red (positive),
/// ColorBrewer "RdBu" anchors reversed.
pub fn diverging(t: f32) -> [u8; 3] {
    ramp(
        &[
            (0.0, [0x21, 0x66, 0xac]),
            (0.25, [0x67, 0xa9, 0xcf]),
            (0.5, [0xf7, 0xf7, 0xf7]),
            (0.75, [0xef, 0x8a, 0x62]),
            (1.0, [0xb2, 0x18, 0x2b]),
        ],
        (t + 1.0) / 2.0,
    )
}

/// Hotspot colour (multi-species aggregation).
pub const HOTSPOT: [u8; 3] = [0xe7, 0x29, 0x8a];
