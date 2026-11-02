//! Coordinate reference systems used in France and Spain, without PROJ (ADR-0003).
//!
//! Forward (geographic → projected) and inverse transforms between WGS84 geographic coordinates
//! (EPSG:4326, longitude/latitude in degrees) and:
//!
//! | CRS | EPSG | Method (IOGP Guidance Note 7-2, 2019 revision) | Ellipsoid |
//! |---|---|---|---|
//! | RGF93 v1 / Lambert-93 | 2154 | Lambert Conic Conformal (2SP), §3.1.1.1 | GRS80 |
//! | ETRS89-extended / LAEA Europe | 3035 | Lambert Azimuthal Equal Area, §3.1.4.2 (ellipsoidal) | GRS80 |
//! | WGS 84 / Pseudo-Mercator | 3857 | Popular Visualisation Pseudo Mercator, §1.3.3.2 | sphere R = a(WGS84) |
//! | WGS 84 / UTM zone 28N–33N (any zone 1–60 N/S) | 32601–32660, 32701–32760 | Transverse Mercator (JHS formulas, Krüger n-series to n⁴), §3.1.3.1 | WGS84 |
//! | ETRS89 / UTM zone 28N–38N | 25828–25838 | as above | GRS80 → treated as WGS84 UTM, see below |
//!
//! **Datums.** RGF93 and ETRS89 are realisations of ETRS89 that coincided with WGS84 (ITRF89) at
//! epoch 1989.0 and have drifted apart by the Eurasian plate motion (~2.5 cm/yr), i.e. < 1 m
//! today. NicheRS treats RGF93 ≈ ETRS89 ≈ WGS84 (no datum shift), exactly like PROJ's null
//! "ballpark" transformation EPSG:4326 → EPSG:2154; the sub-metre difference is irrelevant at
//! 30″ (~1 km) cells. Lambert-93 and LAEA use their defining GRS80 ellipsoid; ETRS89 UTM codes
//! (258xx) map to [`Crs::Utm`], computed on WGS84: the GRS80/WGS84 flattening difference changes
//! UTM coordinates by about 0.1 mm (checked against PROJ 9.8.1: EPSG:25830 vs EPSG:32630 at
//! 3.7° W 40.42° N differ by 0.1 mm in northing).
//!
//! Out-of-domain inputs (e.g. the antipode in LAEA, latitudes beyond ±90°) yield NaN instead of an
//! error: callers transforming many points can filter with `is_finite()`.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

/// Reference ellipsoid (semi-major axis `a`, flattening `f`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ellipsoid {
    pub a: f64,
    pub f: f64,
}

impl Ellipsoid {
    /// WGS 84 (EPSG:7030).
    pub const WGS84: Ellipsoid = Ellipsoid {
        a: 6_378_137.0,
        f: 1.0 / 298.257_223_563,
    };
    /// GRS 1980 (EPSG:7019).
    pub const GRS80: Ellipsoid = Ellipsoid {
        a: 6_378_137.0,
        f: 1.0 / 298.257_222_101,
    };

    pub fn e2(&self) -> f64 {
        self.f * (2.0 - self.f)
    }

    pub fn e(&self) -> f64 {
        self.e2().sqrt()
    }

    /// Radius of curvature in the prime vertical, ν(φ) (metres).
    pub fn prime_vertical_radius(&self, lat_rad: f64) -> f64 {
        let s = lat_rad.sin();
        self.a / (1.0 - self.e2() * s * s).sqrt()
    }

    /// Radius of curvature in the meridian, ρ(φ) (metres).
    pub fn meridian_radius(&self, lat_rad: f64) -> f64 {
        let s = lat_rad.sin();
        let e2 = self.e2();
        self.a * (1.0 - e2) / (1.0 - e2 * s * s).powf(1.5)
    }
}

/// Metres per degree of longitude and of latitude at `lat_deg` on WGS84 (local metric scale of a
/// geographic grid: dx = ν cosφ · Δλ, dy = ρ · Δφ).
pub fn metres_per_degree(lat_deg: f64) -> (f64, f64) {
    let p = lat_deg.to_radians();
    let e = Ellipsoid::WGS84;
    let k = std::f64::consts::PI / 180.0;
    (
        e.prime_vertical_radius(p) * p.cos() * k,
        e.meridian_radius(p) * k,
    )
}

/// A supported coordinate reference system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Crs {
    /// EPSG:4326, longitude/latitude in degrees (x = longitude).
    Wgs84,
    /// EPSG:2154, RGF93 v1 / Lambert-93 (metres).
    Lambert93,
    /// EPSG:3035, ETRS89-extended / LAEA Europe (metres; x = easting).
    Laea3035,
    /// EPSG:3857, WGS 84 / Pseudo-Mercator (metres).
    WebMercator,
    /// WGS 84 UTM (EPSG:326zz north, 327zz south); ETRS89 UTM 258zz maps here too.
    Utm { zone: u8, north: bool },
}

impl Crs {
    /// The CRS for an EPSG code, if supported.
    pub fn from_epsg(code: u32) -> Option<Crs> {
        match code {
            4326 => Some(Crs::Wgs84),
            2154 => Some(Crs::Lambert93),
            3035 => Some(Crs::Laea3035),
            3857 => Some(Crs::WebMercator),
            32601..=32660 => Some(Crs::Utm {
                zone: (code - 32600) as u8,
                north: true,
            }),
            32701..=32760 => Some(Crs::Utm {
                zone: (code - 32700) as u8,
                north: false,
            }),
            25828..=25838 => Some(Crs::Utm {
                zone: (code - 25800) as u8,
                north: true,
            }),
            _ => None,
        }
    }

    /// Canonical EPSG code (WGS 84 UTM codes for [`Crs::Utm`]). `None` for an invalid UTM zone.
    pub fn epsg(&self) -> Option<u32> {
        match *self {
            Crs::Wgs84 => Some(4326),
            Crs::Lambert93 => Some(2154),
            Crs::Laea3035 => Some(3035),
            Crs::WebMercator => Some(3857),
            Crs::Utm { zone, north } if (1..=60).contains(&zone) => {
                Some(if north { 32600 } else { 32700 } + u32::from(zone))
            }
            Crs::Utm { .. } => None,
        }
    }

    /// Projected (or geographic) coordinates → WGS84 (longitude, latitude) in degrees.
    pub fn to_wgs84(&self, x: f64, y: f64) -> (f64, f64) {
        match *self {
            Crs::Wgs84 => (x, y),
            Crs::Lambert93 => LAMBERT93.inverse(x, y),
            Crs::Laea3035 => LAEA3035.inverse(x, y),
            Crs::WebMercator => web_mercator_inverse(x, y),
            Crs::Utm { zone, north } => match utm(zone, north) {
                Some(tm) => tm.inverse(x, y),
                None => (f64::NAN, f64::NAN),
            },
        }
    }

    /// WGS84 (longitude, latitude) in degrees → coordinates in this CRS.
    pub fn from_wgs84(&self, lng: f64, lat: f64) -> (f64, f64) {
        if !(-90.0..=90.0).contains(&lat) || !lng.is_finite() {
            return (f64::NAN, f64::NAN);
        }
        match *self {
            Crs::Wgs84 => (lng, lat),
            Crs::Lambert93 => LAMBERT93.forward(lng, lat),
            Crs::Laea3035 => LAEA3035.forward(lng, lat),
            Crs::WebMercator => web_mercator_forward(lng, lat),
            Crs::Utm { zone, north } => match utm(zone, north) {
                Some(tm) => tm.forward(lng, lat),
                None => (f64::NAN, f64::NAN),
            },
        }
    }
}

/// Normalise a longitude difference to (−π, π].
fn wrap_pi(mut d: f64) -> f64 {
    let two_pi = 2.0 * std::f64::consts::PI;
    while d > std::f64::consts::PI {
        d -= two_pi;
    }
    while d <= -std::f64::consts::PI {
        d += two_pi;
    }
    d
}

// ---------------------------------------------------------------------------------------------
// Lambert Conic Conformal (2SP) — GN 7-2 §3.1.1.1.

/// Lambert Conic Conformal with two standard parallels.
#[derive(Debug, Clone, Copy)]
pub struct LambertConic2Sp {
    pub ellipsoid: Ellipsoid,
    /// Latitude of false origin φF (degrees).
    pub lat_origin: f64,
    /// Longitude of false origin λF (degrees).
    pub lon_origin: f64,
    /// Standard parallels φ1, φ2 (degrees).
    pub lat_1: f64,
    pub lat_2: f64,
    /// Easting and northing at the false origin (EF, NF), in the ellipsoid's length unit.
    pub false_easting: f64,
    pub false_northing: f64,
}

const LAMBERT93: LambertConic2Sp = LambertConic2Sp {
    ellipsoid: Ellipsoid::GRS80,
    lat_origin: 46.5,
    lon_origin: 3.0,
    lat_1: 49.0,
    lat_2: 44.0,
    false_easting: 700_000.0,
    false_northing: 6_600_000.0,
};

impl LambertConic2Sp {
    fn m(&self, p: f64) -> f64 {
        let s = p.sin();
        p.cos() / (1.0 - self.ellipsoid.e2() * s * s).sqrt()
    }

    fn t(&self, p: f64) -> f64 {
        let e = self.ellipsoid.e();
        let s = p.sin();
        (FRAC_PI_4 - p / 2.0).tan() / ((1.0 - e * s) / (1.0 + e * s)).powf(e / 2.0)
    }

    /// (n, F, rF).
    fn constants(&self) -> (f64, f64, f64) {
        let (p1, p2) = (self.lat_1.to_radians(), self.lat_2.to_radians());
        let (m1, m2) = (self.m(p1), self.m(p2));
        let (t1, t2) = (self.t(p1), self.t(p2));
        let n = if (p1 - p2).abs() < 1e-12 {
            p1.sin()
        } else {
            (m1.ln() - m2.ln()) / (t1.ln() - t2.ln())
        };
        let f = m1 / (n * t1.powf(n));
        let rf = self.ellipsoid.a * f * self.t(self.lat_origin.to_radians()).powf(n);
        (n, f, rf)
    }

    pub fn forward(&self, lng: f64, lat: f64) -> (f64, f64) {
        let (n, f, rf) = self.constants();
        let p = lat.to_radians();
        let r = if (lat.abs() - 90.0).abs() < 1e-12 && lat * n > 0.0 {
            0.0
        } else {
            self.ellipsoid.a * f * self.t(p).powf(n)
        };
        let theta = n * wrap_pi((lng - self.lon_origin).to_radians());
        (
            self.false_easting + r * theta.sin(),
            self.false_northing + rf - r * theta.cos(),
        )
    }

    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let (n, f, rf) = self.constants();
        let de = x - self.false_easting;
        let dn = rf - (y - self.false_northing);
        let r = n.signum() * (de * de + dn * dn).sqrt();
        let t = (r / (self.ellipsoid.a * f)).powf(1.0 / n);
        let theta = if n > 0.0 {
            de.atan2(dn)
        } else {
            (-de).atan2(-dn)
        };
        let e = self.ellipsoid.e();
        let mut p = FRAC_PI_2 - 2.0 * t.atan();
        for _ in 0..15 {
            let s = p.sin();
            let next = FRAC_PI_2 - 2.0 * (t * ((1.0 - e * s) / (1.0 + e * s)).powf(e / 2.0)).atan();
            let done = (next - p).abs() < 1e-14;
            p = next;
            if done {
                break;
            }
        }
        let lng = (theta / n).to_degrees() + self.lon_origin;
        (lng, p.to_degrees())
    }
}

// ---------------------------------------------------------------------------------------------
// Lambert Azimuthal Equal Area (ellipsoidal, oblique) — GN 7-2 §3.1.4.2.

#[derive(Debug, Clone, Copy)]
pub struct LambertAzimuthalEqualArea {
    pub ellipsoid: Ellipsoid,
    pub lat_origin: f64,
    pub lon_origin: f64,
    pub false_easting: f64,
    pub false_northing: f64,
}

const LAEA3035: LambertAzimuthalEqualArea = LambertAzimuthalEqualArea {
    ellipsoid: Ellipsoid::GRS80,
    lat_origin: 52.0,
    lon_origin: 10.0,
    false_easting: 4_321_000.0,
    false_northing: 3_210_000.0,
};

impl LambertAzimuthalEqualArea {
    fn q(&self, p: f64) -> f64 {
        let e2 = self.ellipsoid.e2();
        let e = e2.sqrt();
        let s = p.sin();
        (1.0 - e2)
            * (s / (1.0 - e2 * s * s) - 1.0 / (2.0 * e) * ((1.0 - e * s) / (1.0 + e * s)).ln())
    }

    /// (qP, βO, Rq, D).
    fn constants(&self) -> (f64, f64, f64, f64) {
        let a = self.ellipsoid.a;
        let e2 = self.ellipsoid.e2();
        let p0 = self.lat_origin.to_radians();
        let qp = self.q(FRAC_PI_2);
        let q0 = self.q(p0);
        let b0 = (q0 / qp).clamp(-1.0, 1.0).asin();
        let rq = a * (qp / 2.0).sqrt();
        let s0 = p0.sin();
        let d = a * (p0.cos() / (1.0 - e2 * s0 * s0).sqrt()) / (rq * b0.cos());
        (qp, b0, rq, d)
    }

    pub fn forward(&self, lng: f64, lat: f64) -> (f64, f64) {
        let (qp, b0, rq, d) = self.constants();
        let p = lat.to_radians();
        let beta = (self.q(p) / qp).clamp(-1.0, 1.0).asin();
        let dl = wrap_pi((lng - self.lon_origin).to_radians());
        let denom = 1.0 + b0.sin() * beta.sin() + b0.cos() * beta.cos() * dl.cos();
        if denom <= 1e-15 {
            return (f64::NAN, f64::NAN);
        }
        let b = rq * (2.0 / denom).sqrt();
        (
            self.false_easting + b * d * beta.cos() * dl.sin(),
            self.false_northing
                + (b / d) * (b0.cos() * beta.sin() - b0.sin() * beta.cos() * dl.cos()),
        )
    }

    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let (qp, b0, rq, d) = self.constants();
        let (de, dn) = (x - self.false_easting, y - self.false_northing);
        let rho = ((de / d).powi(2) + (d * dn).powi(2)).sqrt();
        if rho < 1e-12 {
            return (self.lon_origin, self.lat_origin);
        }
        let arg = rho / (2.0 * rq);
        if arg > 1.0 {
            return (f64::NAN, f64::NAN);
        }
        let c = 2.0 * arg.asin();
        let beta = (c.cos() * b0.sin() + d * dn * c.sin() * b0.cos() / rho)
            .clamp(-1.0, 1.0)
            .asin();
        let lng = self.lon_origin.to_radians()
            + (de * c.sin()).atan2(d * rho * b0.cos() * c.cos() - d * d * dn * b0.sin() * c.sin());
        // Latitude from the authalic latitude: GN 7-2 series as a first guess, then Newton on
        // q(φ) = qP sin β (Snyder 1987, eq. 3-16) so that the round trip is exact to < 1 mm.
        let e2 = self.ellipsoid.e2();
        let (e4, e6) = (e2 * e2, e2 * e2 * e2);
        let mut p = beta
            + (e2 / 3.0 + 31.0 * e4 / 180.0 + 517.0 * e6 / 5040.0) * (2.0 * beta).sin()
            + (23.0 * e4 / 360.0 + 251.0 * e6 / 3780.0) * (4.0 * beta).sin()
            + (761.0 * e6 / 45360.0) * (6.0 * beta).sin();
        let target = qp * beta.sin();
        let e = e2.sqrt();
        for _ in 0..10 {
            let s = p.sin();
            let cp = p.cos();
            if cp.abs() < 1e-12 {
                break;
            }
            let one = 1.0 - e2 * s * s;
            let step = one * one / (2.0 * cp)
                * (target / (1.0 - e2) - s / one
                    + 1.0 / (2.0 * e) * ((1.0 - e * s) / (1.0 + e * s)).ln());
            p += step;
            if step.abs() < 1e-15 {
                break;
            }
        }
        (wrap_pi(lng).to_degrees(), p.to_degrees())
    }
}

// ---------------------------------------------------------------------------------------------
// Transverse Mercator, JHS formulas (Krüger n-series to n⁴) — GN 7-2 §3.1.3.1.

#[derive(Debug, Clone, Copy)]
pub struct TransverseMercator {
    pub ellipsoid: Ellipsoid,
    pub lat_origin: f64,
    pub lon_origin: f64,
    pub scale: f64,
    pub false_easting: f64,
    pub false_northing: f64,
}

/// UTM zone `zone` (1–60) on WGS84.
pub fn utm(zone: u8, north: bool) -> Option<TransverseMercator> {
    if !(1..=60).contains(&zone) {
        return None;
    }
    Some(TransverseMercator {
        ellipsoid: Ellipsoid::WGS84,
        lat_origin: 0.0,
        lon_origin: f64::from(zone) * 6.0 - 183.0,
        scale: 0.9996,
        false_easting: 500_000.0,
        false_northing: if north { 0.0 } else { 10_000_000.0 },
    })
}

impl TransverseMercator {
    fn n(&self) -> f64 {
        let f = self.ellipsoid.f;
        f / (2.0 - f)
    }

    fn big_b(&self) -> f64 {
        let n = self.n();
        self.ellipsoid.a / (1.0 + n) * (1.0 + n * n / 4.0 + n.powi(4) / 64.0)
    }

    fn h_forward(&self) -> [f64; 4] {
        let n = self.n();
        let (n2, n3, n4) = (n * n, n.powi(3), n.powi(4));
        [
            n / 2.0 - 2.0 / 3.0 * n2 + 5.0 / 16.0 * n3 + 41.0 / 180.0 * n4,
            13.0 / 48.0 * n2 - 3.0 / 5.0 * n3 + 557.0 / 1440.0 * n4,
            61.0 / 240.0 * n3 - 103.0 / 140.0 * n4,
            49561.0 / 161280.0 * n4,
        ]
    }

    fn h_inverse(&self) -> [f64; 4] {
        let n = self.n();
        let (n2, n3, n4) = (n * n, n.powi(3), n.powi(4));
        [
            n / 2.0 - 2.0 / 3.0 * n2 + 37.0 / 96.0 * n3 - 1.0 / 360.0 * n4,
            1.0 / 48.0 * n2 + 1.0 / 15.0 * n3 - 437.0 / 1440.0 * n4,
            17.0 / 480.0 * n3 - 37.0 / 840.0 * n4,
            4397.0 / 161280.0 * n4,
        ]
    }

    /// Conformal latitude β(φ) via isometric latitude Q.
    fn beta(&self, p: f64) -> f64 {
        let e = self.ellipsoid.e();
        let q = p.tan().asinh() - e * (e * p.sin()).atanh();
        q.sinh().atan()
    }

    /// Meridional arc mO at the latitude of origin.
    fn m_origin(&self) -> f64 {
        let p0 = self.lat_origin.to_radians();
        if p0 == 0.0 {
            return 0.0;
        }
        let b = self.big_b();
        let h = self.h_forward();
        if (p0.abs() - FRAC_PI_2).abs() < 1e-15 {
            return b * p0;
        }
        let xi0 = self.beta(p0).sin().asin();
        let xi = xi0
            + (1..=4)
                .map(|i| h[i - 1] * (2.0 * i as f64 * xi0).sin())
                .sum::<f64>();
        b * xi
    }

    pub fn forward(&self, lng: f64, lat: f64) -> (f64, f64) {
        let p = lat.to_radians();
        let dl = wrap_pi((lng - self.lon_origin).to_radians());
        let beta = if (p.abs() - FRAC_PI_2).abs() < 1e-15 {
            p
        } else {
            self.beta(p)
        };
        let eta0 = (beta.cos() * dl.sin()).atanh();
        let xi0 = (beta.sin() * eta0.cosh()).clamp(-1.0, 1.0).asin();
        let h = self.h_forward();
        let (mut xi, mut eta) = (xi0, eta0);
        for i in 1..=4 {
            let k = 2.0 * i as f64;
            xi += h[i - 1] * (k * xi0).sin() * (k * eta0).cosh();
            eta += h[i - 1] * (k * xi0).cos() * (k * eta0).sinh();
        }
        let b = self.big_b();
        (
            self.false_easting + self.scale * b * eta,
            self.false_northing + self.scale * (b * xi - self.m_origin()),
        )
    }

    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let b = self.big_b();
        let eta1 = (x - self.false_easting) / (b * self.scale);
        let xi1 = ((y - self.false_northing) + self.scale * self.m_origin()) / (b * self.scale);
        let h = self.h_inverse();
        let (mut xi0, mut eta0) = (xi1, eta1);
        for i in 1..=4 {
            let k = 2.0 * i as f64;
            xi0 -= h[i - 1] * (k * xi1).sin() * (k * eta1).cosh();
            eta0 -= h[i - 1] * (k * xi1).cos() * (k * eta1).sinh();
        }
        let beta = (xi0.sin() / eta0.cosh()).clamp(-1.0, 1.0).asin();
        let e = self.ellipsoid.e();
        let q1 = beta.tan().asinh();
        let mut q = q1;
        for _ in 0..20 {
            let next = q1 + e * (e * q.tanh()).atanh();
            let done = (next - q).abs() < 1e-15;
            q = next;
            if done {
                break;
            }
        }
        let lat = q.sinh().atan();
        let lng = self.lon_origin.to_radians() + (eta0.tanh() / beta.cos()).clamp(-1.0, 1.0).asin();
        (wrap_pi(lng).to_degrees(), lat.to_degrees())
    }
}

// ---------------------------------------------------------------------------------------------
// Popular Visualisation Pseudo-Mercator — GN 7-2 §1.3.3.2 (spherical formulas, R = a).

const WEB_MERCATOR_R: f64 = 6_378_137.0;

fn web_mercator_forward(lng: f64, lat: f64) -> (f64, f64) {
    let p = lat.to_radians();
    (
        WEB_MERCATOR_R * wrap_pi(lng.to_radians()),
        WEB_MERCATOR_R * (FRAC_PI_4 + p / 2.0).tan().ln(),
    )
}

fn web_mercator_inverse(x: f64, y: f64) -> (f64, f64) {
    let lat = FRAC_PI_2 - 2.0 * (-y / WEB_MERCATOR_R).exp().atan();
    ((x / WEB_MERCATOR_R).to_degrees(), lat.to_degrees())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dms(d: f64, m: f64, s: f64) -> f64 {
        d.signum() * (d.abs() + m / 60.0 + s / 3600.0)
    }

    /// GN 7-2 §3.1.1.1 example: NAD27 / Texas South Central (EPSG:32040), Clarke 1866 in US
    /// survey feet. φ = 28°30'N, λ = 96°W → E = 2 963 503.91 ftUS, N = 254 759.80 ftUS.
    #[test]
    fn lcc2sp_iogp_example() {
        let ft = 0.304_800_609_601_219_2;
        let (a, b) = (6_378_206.4 / ft, 6_356_583.8 / ft);
        let p = LambertConic2Sp {
            ellipsoid: Ellipsoid { a, f: (a - b) / a },
            lat_origin: dms(27.0, 50.0, 0.0),
            lon_origin: -99.0,
            lat_1: dms(28.0, 23.0, 0.0),
            lat_2: dms(30.0, 17.0, 0.0),
            false_easting: 2_000_000.0,
            false_northing: 0.0,
        };
        let (e, n) = p.forward(-96.0, 28.5);
        assert!((e - 2_963_503.91).abs() < 0.01, "E {e}");
        assert!((n - 254_759.80).abs() < 0.01, "N {n}");
        let (lng, lat) = p.inverse(e, n);
        assert!((lng + 96.0).abs() < 1e-10 && (lat - 28.5).abs() < 1e-10);
    }

    /// GN 7-2 §3.1.3.1 example (JHS formulas): OSGB 1936 / British National Grid (EPSG:27700),
    /// Airy 1830. φ = 50°30'N, λ = 0°30'E → E = 577 274.98 m, N = 69 740.49 m.
    #[test]
    fn tm_iogp_example() {
        let tm = TransverseMercator {
            ellipsoid: Ellipsoid {
                a: 6_377_563.396,
                f: 1.0 / 299.324_964_6,
            },
            lat_origin: 49.0,
            lon_origin: -2.0,
            scale: 0.999_601_271_7,
            false_easting: 400_000.0,
            false_northing: -100_000.0,
        };
        let (e, n) = tm.forward(0.5, 50.5);
        assert!((e - 577_274.98).abs() < 0.01, "E {e}");
        assert!((n - 69_740.49).abs() < 0.01, "N {n}");
        let (lng, lat) = tm.inverse(e, n);
        assert!((lng - 0.5).abs() < 1e-10 && (lat - 50.5).abs() < 1e-10);
    }

    /// GN 7-2 §3.1.4.2 example: ETRS89 / LAEA Europe (EPSG:3035).
    /// φ = 50°N, λ = 5°E → E = 3 962 799.45 m, N = 2 999 718.85 m.
    #[test]
    fn laea_iogp_example() {
        let (e, n) = Crs::Laea3035.from_wgs84(5.0, 50.0);
        assert!((e - 3_962_799.45).abs() < 0.01, "E {e}");
        assert!((n - 2_999_718.85).abs() < 0.01, "N {n}");
    }

    /// GN 7-2 §1.3.3.2 example: WGS 84 / Pseudo-Mercator (EPSG:3857).
    /// φ = 24°22'54.433"N, λ = 100°20'00"W → E = −11 169 055.58 m, N = 2 800 000.00 m.
    #[test]
    fn web_mercator_iogp_example() {
        let lat = dms(24.0, 22.0, 54.433);
        let lng = -dms(100.0, 20.0, 0.0);
        let (e, n) = Crs::WebMercator.from_wgs84(lng, lat);
        assert!((e + 11_169_055.58).abs() < 0.01, "E {e}");
        assert!((n - 2_800_000.00).abs() < 0.01, "N {n}");
    }

    /// Reference values computed with PROJ 9.8.1 (pyproj 3.8.0, `Transformer.from_crs("EPSG:4326",
    /// code, always_xy=True)`, 2026-10-20), which itself reproduces the four GN 7-2 examples above
    /// to 0.01 m. Points: Paris, Brest, Corsica (Bastia region), Nice, Tenerife, Santiago de
    /// Compostela, Madrid, Barcelona, Marseille region, and two central meridians.
    const PROJ_REFS: &[(u32, f64, f64, f64, f64)] = &[
        (2154, 2.3522, 48.8566, 652469.0227, 6862035.2594),
        (2154, -4.4861, 48.3904, 146632.9785, 6836262.3267),
        (2154, 9.45, 42.7, 1228546.0453, 6199420.6440),
        (2154, 7.26, 43.7, 1043310.2914, 6298259.4456),
        (3035, 2.3522, 48.8566, 3760771.8648, 2889484.8019),
        (3035, -16.25, 28.46, 1749072.1069, 1046372.6418),
        (3857, 2.3522, 48.8566, 261845.7062, 6250564.3495),
        (32628, -16.25, 28.46, 377617.3720, 3148796.8855),
        (32629, -8.5, 42.88, 540833.4343, 4747610.3801),
        (32630, -3.7, 40.42, 440615.6618, 4474610.0200),
        (32631, 2.35, 48.86, 452324.3882, 5412096.5025),
        (32632, 9.0, 42.0, 500000.0000, 4649776.2248),
        (32633, 15.0, 45.0, 500000.0000, 4982950.4002),
        (32631, 5.5, 43.5, 702114.2775, 4819377.6408),
        (25830, -3.7, 40.42, 440615.6618, 4474610.0199),
        (25831, 2.17, 41.39, 430608.4966, 4582384.5609),
    ];

    #[test]
    fn matches_proj_reference_points() {
        for &(code, lng, lat, x, y) in PROJ_REFS {
            let crs = Crs::from_epsg(code).expect("supported");
            let (px, py) = crs.from_wgs84(lng, lat);
            // 1 mm: JHS 4th-order series vs PROJ's 6th-order Poder/Engsager within 3° of the CM.
            assert!(
                (px - x).abs() < 1e-3 && (py - y).abs() < 1e-3,
                "EPSG:{code} ({lng},{lat}): got ({px:.4},{py:.4}) want ({x},{y})"
            );
        }
    }

    /// Lambert-93 false origin: (3°E, 46.5°N) ↦ (700 000, 6 600 000) by definition (IGN,
    /// "Projection cartographique conique conforme de Lambert", NT/G 71; EPSG:2154 parameters).
    #[test]
    fn lambert93_origin() {
        let (x, y) = Crs::Lambert93.from_wgs84(3.0, 46.5);
        assert!((x - 700_000.0).abs() < 1e-6 && (y - 6_600_000.0).abs() < 1e-6);
    }

    #[test]
    fn round_trips_below_one_millimetre() {
        let crss = [
            Crs::Lambert93,
            Crs::Laea3035,
            Crs::WebMercator,
            Crs::Utm {
                zone: 28,
                north: true,
            },
            Crs::Utm {
                zone: 30,
                north: true,
            },
            Crs::Utm {
                zone: 31,
                north: true,
            },
            Crs::Utm {
                zone: 33,
                north: true,
            },
            Crs::Utm {
                zone: 31,
                north: false,
            },
        ];
        for crs in crss {
            for i in 0..=20 {
                for j in 0..=20 {
                    let lng = -18.0 + 37.0 * f64::from(i) / 20.0;
                    let lat = 27.0 + 25.0 * f64::from(j) / 20.0;
                    let lat = if matches!(crs, Crs::Utm { north: false, .. }) {
                        -lat
                    } else {
                        lat
                    };
                    if let Crs::Utm { zone, .. } = crs {
                        let cm = f64::from(zone) * 6.0 - 183.0;
                        if (lng - cm).abs() > 6.0 {
                            continue;
                        }
                    }
                    let (x, y) = crs.from_wgs84(lng, lat);
                    let (lng2, lat2) = crs.to_wgs84(x, y);
                    let (x2, y2) = crs.from_wgs84(lng2, lat2);
                    let err = ((x - x2).powi(2) + (y - y2).powi(2)).sqrt();
                    assert!(err < 1e-3, "{crs:?} ({lng},{lat}) round trip {err} m");
                    let (mx, my) = metres_per_degree(lat);
                    let ground = (((lng - lng2) * mx).powi(2) + ((lat - lat2) * my).powi(2)).sqrt();
                    assert!(
                        ground < 1e-3,
                        "{crs:?} ({lng},{lat}) inverse error {ground} m"
                    );
                }
            }
        }
    }

    #[test]
    fn epsg_codes() {
        assert_eq!(Crs::from_epsg(4326), Some(Crs::Wgs84));
        assert_eq!(
            Crs::from_epsg(25830),
            Some(Crs::Utm {
                zone: 30,
                north: true
            })
        );
        assert_eq!(Crs::from_epsg(32628).and_then(|c| c.epsg()), Some(32628));
        assert_eq!(Crs::from_epsg(27700), None);
        assert_eq!(
            Crs::Utm {
                zone: 0,
                north: true
            }
            .epsg(),
            None
        );
        assert!(Crs::Lambert93.from_wgs84(0.0, 95.0).0.is_nan());
    }

    #[test]
    fn metric_scale_at_45n() {
        // WGS84 at 45°: 1° of longitude = 78 846.8 m, 1° of latitude = 111 131.8 m (the values
        // tabulated in geodesy references for the WGS84 ellipsoid, e.g. NGA "Length of a degree").
        let (mx, my) = metres_per_degree(45.0);
        assert!((mx - 78_846.8).abs() < 1.0, "{mx}");
        assert!((my - 111_131.8).abs() < 1.0, "{my}");
    }
}
