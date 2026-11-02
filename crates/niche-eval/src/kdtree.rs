//! Exact Euclidean nearest-neighbour search with a k-d tree (Friedman, Bentley & Finkel 1977,
//! ACM TOMS 3:209–226).
//!
//! Built once, queried many times (e.g. millions of raster cells against ~10⁴ training rows in the
//! area-of-applicability computation). Splits on the dimension of largest spread at the median;
//! leaves hold up to 16 points stored contiguously. Queries descend to the nearer child first and
//! visit the farther one only when the splitting plane is closer than the best distance so far,
//! so results are exact. Build O(n log n); query ≈ O(log n) in low intrinsic dimension, degrading
//! towards O(n) as intrinsic dimension grows (never worse than brute force by more than a constant).
//! The tree is immutable after construction, hence `Sync` and safe to query from many threads.

const LEAF_SIZE: usize = 16;

#[derive(Debug, Clone)]
enum Node {
    Leaf {
        start: usize,
        end: usize,
    },
    Split {
        dim: usize,
        value: f64,
        left: usize,
        right: usize,
    },
}

/// Static k-d tree over points of equal dimension.
#[derive(Debug, Clone)]
pub struct KdTree {
    dims: usize,
    /// Points in tree order, row-major (`ids.len() × dims`).
    data: Vec<f64>,
    /// Original index of each stored point.
    ids: Vec<usize>,
    nodes: Vec<Node>,
}

impl KdTree {
    /// Builds a tree over `rows` (all of the same length, finite values). Returns `None` when
    /// `rows` is empty, has zero-length rows or rows of different lengths. Non-finite values are
    /// not rejected but make results for those points meaningless; filter them beforehand.
    pub fn new(rows: &[Vec<f64>]) -> Option<Self> {
        let dims = rows.first()?.len();
        if dims == 0 || rows.iter().any(|r| r.len() != dims) {
            return None;
        }
        let flat: Vec<f64> = rows.iter().flatten().copied().collect();
        let mut idx: Vec<usize> = (0..rows.len()).collect();
        let mut nodes = Vec::new();
        build(&flat, dims, &mut idx, 0, &mut nodes);
        let mut data = Vec::with_capacity(flat.len());
        for &i in &idx {
            data.extend_from_slice(&flat[i * dims..(i + 1) * dims]);
        }
        Some(Self {
            dims,
            data,
            ids: idx,
            nodes,
        })
    }

    pub fn dims(&self) -> usize {
        self.dims
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Nearest stored point to `q`: `(original row index, Euclidean distance)`. `None` if `q` has
    /// the wrong length. On exact distance ties any of the tied points may be returned.
    pub fn nearest(&self, q: &[f64]) -> Option<(usize, f64)> {
        self.nearest_excluding(q, usize::MAX)
    }

    /// As [`KdTree::nearest`] but ignoring the stored point whose original index is `exclude`
    /// (leave-one-out queries of a training point against its own set). `None` if no other point
    /// exists or `q` has the wrong length.
    pub fn nearest_excluding(&self, q: &[f64], exclude: usize) -> Option<(usize, f64)> {
        if q.len() != self.dims {
            return None;
        }
        let mut best = (usize::MAX, f64::INFINITY);
        self.search(0, q, exclude, &mut best);
        (best.0 != usize::MAX).then(|| (self.ids[best.0], best.1.sqrt()))
    }

    fn search(&self, node: usize, q: &[f64], exclude: usize, best: &mut (usize, f64)) {
        match self.nodes[node] {
            Node::Leaf { start, end } => {
                for pos in start..end {
                    if self.ids[pos] == exclude {
                        continue;
                    }
                    let row = &self.data[pos * self.dims..(pos + 1) * self.dims];
                    let mut d = 0.0;
                    for (a, b) in row.iter().zip(q) {
                        let t = a - b;
                        d += t * t;
                        if d >= best.1 {
                            break;
                        }
                    }
                    if d < best.1 {
                        *best = (pos, d);
                    }
                }
            }
            Node::Split {
                dim,
                value,
                left,
                right,
            } => {
                let diff = q[dim] - value;
                let (near, far) = if diff < 0.0 {
                    (left, right)
                } else {
                    (right, left)
                };
                self.search(near, q, exclude, best);
                if diff * diff < best.1 {
                    self.search(far, q, exclude, best);
                }
            }
        }
    }
}

/// Builds the subtree over `idx` (positions `offset..offset + idx.len()` in tree order) and
/// returns its node index. Left child holds coordinates ≤ split value, right child ≥ it.
fn build(
    flat: &[f64],
    dims: usize,
    idx: &mut [usize],
    offset: usize,
    nodes: &mut Vec<Node>,
) -> usize {
    let me = nodes.len();
    let leaf = Node::Leaf {
        start: offset,
        end: offset + idx.len(),
    };
    nodes.push(leaf.clone());
    if idx.len() <= LEAF_SIZE {
        return me;
    }
    let mut dim = 0;
    let mut spread = 0.0;
    for d in 0..dims {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for &i in idx.iter() {
            let v = flat[i * dims + d];
            lo = lo.min(v);
            hi = hi.max(v);
        }
        if hi - lo > spread {
            spread = hi - lo;
            dim = d;
        }
    }
    // All points identical (or non-finite spread): keep as one leaf.
    if spread.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return me;
    }
    let mid = idx.len() / 2;
    idx.select_nth_unstable_by(mid, |&a, &b| {
        flat[a * dims + dim].total_cmp(&flat[b * dims + dim])
    });
    let value = flat[idx[mid] * dims + dim];
    let (l, r) = idx.split_at_mut(mid);
    let left = build(flat, dims, l, offset, nodes);
    let right = build(flat, dims, r, offset + mid, nodes);
    nodes[me] = Node::Split {
        dim,
        value,
        left,
        right,
    };
    me
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    fn brute(rows: &[Vec<f64>], q: &[f64], exclude: usize) -> f64 {
        rows.iter()
            .enumerate()
            .filter(|(i, _)| *i != exclude)
            .map(|(_, r)| r.iter().zip(q).map(|(a, b)| (a - b) * (a - b)).sum::<f64>())
            .fold(f64::INFINITY, f64::min)
            .sqrt()
    }

    fn random_rows(n: usize, d: usize, r: &mut Rng) -> Vec<Vec<f64>> {
        (0..n)
            .map(|_| (0..d).map(|_| r.uniform() * 10.0 - 5.0).collect())
            .collect()
    }

    #[test]
    fn exact_against_brute_force() {
        let mut r = Rng::new(3);
        for &(n, d) in &[(1, 1), (17, 2), (500, 3), (2000, 8), (1500, 20)] {
            let rows = random_rows(n, d, &mut r);
            let tree = KdTree::new(&rows).expect("tree");
            assert_eq!(tree.len(), n);
            for _ in 0..200 {
                let q: Vec<f64> = (0..d).map(|_| r.uniform() * 12.0 - 6.0).collect();
                let (i, dist) = tree.nearest(&q).expect("nn");
                let b = brute(&rows, &q, usize::MAX);
                assert!((dist - b).abs() <= 1e-12 * (1.0 + b), "n={n} d={d}");
                assert!((brute(&rows[i..=i], &q, usize::MAX) - b).abs() <= 1e-12 * (1.0 + b));
            }
        }
    }

    #[test]
    fn excluding_self_matches_leave_one_out() {
        let mut r = Rng::new(4);
        let rows = random_rows(800, 5, &mut r);
        let tree = KdTree::new(&rows).expect("tree");
        for (i, row) in rows.iter().enumerate() {
            let (j, d) = tree.nearest_excluding(row, i).expect("nn");
            assert_ne!(i, j);
            assert!((d - brute(&rows, row, i)).abs() < 1e-12);
        }
    }

    #[test]
    fn duplicates_and_degenerate_inputs() {
        let rows = vec![vec![1.0, 1.0]; 100];
        let tree = KdTree::new(&rows).expect("tree");
        assert_eq!(tree.nearest(&[1.0, 1.0]).expect("nn").1, 0.0);
        assert_eq!(tree.nearest_excluding(&[1.0, 1.0], 5).expect("nn").1, 0.0);
        let one = KdTree::new(&[vec![0.0]]).expect("tree");
        assert!(one.nearest_excluding(&[0.0], 0).is_none());
        assert!(one.nearest(&[0.0, 1.0]).is_none());
        assert!(KdTree::new(&[]).is_none());
        assert!(KdTree::new(&[vec![1.0], vec![1.0, 2.0]]).is_none());
    }
}
