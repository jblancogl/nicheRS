//! Regular longitude/latitude grids (EPSG:4326), north-up, row-major from the north-west corner.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridSpec {
    /// Longitude of the western edge of column 0.
    pub west: f64,
    /// Latitude of the northern edge of row 0.
    pub north: f64,
    /// Cell size in degrees (square cells).
    pub cell: f64,
    pub width: usize,
    pub height: usize,
}

impl GridSpec {
    pub fn east(&self) -> f64 {
        self.west + self.cell * self.width as f64
    }

    pub fn south(&self) -> f64 {
        self.north - self.cell * self.height as f64
    }

    pub fn len(&self) -> usize {
        self.width * self.height
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Centre of cell (row, col).
    pub fn center(&self, row: usize, col: usize) -> (f64, f64) {
        (
            self.west + (col as f64 + 0.5) * self.cell,
            self.north - (row as f64 + 0.5) * self.cell,
        )
    }

    /// Cell containing (lng, lat), if inside the grid.
    pub fn cell_of(&self, lng: f64, lat: f64) -> Option<(usize, usize)> {
        let c = ((lng - self.west) / self.cell).floor();
        let r = ((self.north - lat) / self.cell).floor();
        if c < 0.0 || r < 0.0 || c >= self.width as f64 || r >= self.height as f64 {
            return None;
        }
        Some((r as usize, c as usize))
    }

    pub fn index(&self, row: usize, col: usize) -> usize {
        row * self.width + col
    }

    /// Aggregated grid: `factor`×`factor` source cells per output cell (partial edge cells dropped).
    pub fn aggregated(&self, factor: usize) -> GridSpec {
        GridSpec {
            west: self.west,
            north: self.north,
            cell: self.cell * factor as f64,
            width: self.width / factor,
            height: self.height / factor,
        }
    }
}

/// A single-band f32 raster; NaN is NoData.
#[derive(Debug, Clone, PartialEq)]
pub struct Raster {
    pub spec: GridSpec,
    pub data: Vec<f32>,
}

impl Raster {
    pub fn new(spec: GridSpec, fill: f32) -> Self {
        Self {
            spec,
            data: vec![fill; spec.len()],
        }
    }

    pub fn get(&self, row: usize, col: usize) -> f32 {
        self.data[self.spec.index(row, col)]
    }

    pub fn sample(&self, lng: f64, lat: f64) -> Option<f32> {
        let (r, c) = self.spec.cell_of(lng, lat)?;
        let v = self.get(r, c);
        if v.is_nan() { None } else { Some(v) }
    }

    /// Mean aggregation by `factor` (continuous variables). An output cell is NoData when more
    /// than half of its source cells are NoData, to avoid coastal cells represented by a sliver.
    pub fn aggregate_mean(&self, factor: usize) -> Raster {
        if factor <= 1 {
            return self.clone();
        }
        let spec = self.spec.aggregated(factor);
        let mut out = Raster::new(spec, f32::NAN);
        let need = (factor * factor) / 2 + 1;
        for r in 0..spec.height {
            for c in 0..spec.width {
                let (mut s, mut n) = (0.0f64, 0usize);
                for rr in r * factor..(r + 1) * factor {
                    for cc in c * factor..(c + 1) * factor {
                        let v = self.get(rr, cc);
                        if !v.is_nan() {
                            s += f64::from(v);
                            n += 1;
                        }
                    }
                }
                if n >= need {
                    out.data[spec.index(r, c)] = (s / n as f64) as f32;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_lookup_roundtrip() {
        let g = GridSpec {
            west: -5.0,
            north: 51.0,
            cell: 0.5,
            width: 20,
            height: 20,
        };
        let (lng, lat) = g.center(3, 7);
        assert_eq!(g.cell_of(lng, lat), Some((3, 7)));
        assert_eq!(g.cell_of(-6.0, 50.0), None);
    }

    #[test]
    fn aggregate_mean_majority_rule() {
        let g = GridSpec {
            west: 0.0,
            north: 2.0,
            cell: 1.0,
            width: 2,
            height: 2,
        };
        let r = Raster {
            spec: g,
            data: vec![1.0, 3.0, f32::NAN, 5.0],
        };
        let a = r.aggregate_mean(2);
        assert_eq!(a.data, vec![3.0]);
        let r2 = Raster {
            spec: g,
            data: vec![1.0, f32::NAN, f32::NAN, 5.0],
        };
        assert!(r2.aggregate_mean(2).data[0].is_nan());
    }
}
