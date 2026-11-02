//! CART classification tree (Breiman et al. 1984) grown the way `randomForest` grows the trees
//! of a forest (Breiman 2001; Liaw & Wiener 2002, R package `randomForest` 4.7, `classTree.c`).
//!
//! * **Impurity**: Gini. The split maximises `(L₁² + L₀²)/n_L + (R₁² + R₀²)/n_R`, which is
//!   equivalent to minimising the size-weighted Gini impurity of the two children (this is the
//!   criterion `randomForest` evaluates). A split is kept only if it strictly improves on the
//!   parent's `(n₁² + n₀²)/n` (relative tolerance 1e-12, so floating-point noise cannot create
//!   a split).
//! * **Candidate variables**: `mtry` variables drawn without replacement at every node. If none
//!   of them admits an improving split, the node becomes a leaf: like `randomForest`, the search
//!   does not continue with further variables (scikit-learn does; we follow the reference used
//!   by Valavi et al. 2021).
//! * **Thresholds**: the node's values are sorted and every midpoint between consecutive
//!   *distinct* values is a candidate; rows with `x <= threshold` go left. Ties between equally
//!   good splits keep the first one found (variable draw order, then increasing threshold), so
//!   the tree is a deterministic function of the sample and the RNG stream.
//! * **Stopping**: a node is a leaf when it is pure, when it holds at most `min_node_size`
//!   samples (`randomForest`'s `nodesize` semantics; 1 = grow fully), when `max_depth` is
//!   reached, or when no improving split exists (e.g. all candidate variables constant).
//! * **Categorical variables** are treated as ordered numeric codes. This is a simplification
//!   (`randomForest` searches subsets of categories for factors); it can only make some splits
//!   unavailable, never wrong, and fully grown trees can still isolate any single code with two
//!   splits.

use niche_eval::rng::Rng;
use serde::{Deserialize, Serialize};

/// Sentinel stored in [`Tree`]'s variable array for a leaf.
pub(crate) const LEAF: u32 = u32::MAX;

/// A fitted classification tree as flat arrays (one entry per node, root at index 0).
///
/// * `var[n]`: split variable of node `n`, or `u32::MAX` for a leaf;
/// * `value[n]`: the threshold of a split node (`x <= value` goes left), or for a leaf the
///   fraction of presence samples (bootstrap duplicates counted) that reached it;
/// * `left[n]`: index of the left child of a split node; the right child is `left[n] + 1`.
///   Children always have larger indices than their parent (checked on deserialisation), so
///   traversal terminates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tree {
    pub(crate) var: Vec<u32>,
    pub(crate) value: Vec<f64>,
    pub(crate) left: Vec<u32>,
}

impl Tree {
    pub fn n_nodes(&self) -> usize {
        self.var.len()
    }

    pub fn n_leaves(&self) -> usize {
        self.var.iter().filter(|&&v| v == LEAF).count()
    }

    /// Presence fraction of the leaf that `row` falls in (`NaN` if `row` is too short).
    pub fn leaf_value(&self, row: &[f64]) -> f64 {
        let mut n = 0usize;
        loop {
            let (Some(&v), Some(&value), Some(&left)) =
                (self.var.get(n), self.value.get(n), self.left.get(n))
            else {
                return f64::NAN;
            };
            if v == LEAF {
                return value;
            }
            let Some(&x) = row.get(v as usize) else {
                return f64::NAN;
            };
            n = if x <= value {
                left as usize
            } else {
                left as usize + 1
            };
        }
    }

    /// The tree's class vote for `row`: 1 (presence) if the leaf's presence fraction is above
    /// ½, 0 below ½, and ½ on an exact tie. `randomForest` breaks such ties at random; splitting
    /// the vote is its deterministic expectation. Ties only occur in leaves holding identical
    /// predictor values with both labels, or when growth is limited.
    pub fn vote(&self, row: &[f64]) -> f64 {
        let f = self.leaf_value(row);
        if f > 0.5 {
            1.0
        } else if f < 0.5 {
            0.0
        } else if f == 0.5 {
            0.5
        } else {
            f64::NAN
        }
    }

    /// Structural check for trees that come from outside (deserialisation).
    pub(crate) fn check(&self, n_vars: usize) -> Result<(), String> {
        let n = self.var.len();
        if n == 0 || self.value.len() != n || self.left.len() != n {
            return Err("tree arrays are empty or of different lengths".into());
        }
        for i in 0..n {
            if self.var[i] == LEAF {
                if !(0.0..=1.0).contains(&self.value[i]) {
                    return Err(format!("leaf {i} has a value outside [0, 1]"));
                }
            } else {
                let l = self.left[i] as usize;
                if self.var[i] as usize >= n_vars {
                    return Err(format!("node {i} splits on an unknown variable"));
                }
                if l <= i || l + 1 >= n {
                    return Err(format!("node {i} has invalid children"));
                }
                if !self.value[i].is_finite() {
                    return Err(format!("node {i} has a non-finite threshold"));
                }
            }
        }
        Ok(())
    }
}

/// Growth parameters of one tree.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GrowParams {
    pub mtry: usize,
    pub min_node_size: usize,
    pub max_depth: Option<usize>,
}

/// Grow a tree on a sample given column-major (`cols[var][k]`) with labels `y[k]`
/// (`true` = presence). Variables and thresholds are drawn from `rng`.
pub(crate) fn grow(cols: &[Vec<f64>], y: &[bool], params: GrowParams, rng: &mut Rng) -> Tree {
    let n = y.len();
    let mut idx: Vec<u32> = (0..n as u32).collect();
    let mut tree = Tree {
        var: vec![LEAF],
        value: vec![0.0],
        left: vec![0],
    };
    let mut pairs: Vec<(f64, bool)> = Vec::with_capacity(n);
    // (node, start, end, depth) over `idx`; LIFO with the left child pushed last so it is
    // processed first (depth-first, left to right).
    let mut stack = vec![(0usize, 0usize, n, 0usize)];
    while let Some((node, start, end, depth)) = stack.pop() {
        let size = end - start;
        let n1 = idx[start..end].iter().filter(|&&k| y[k as usize]).count();
        let pure = n1 == 0 || n1 == size;
        let can_split =
            !pure && size > params.min_node_size && params.max_depth.is_none_or(|d| depth < d);
        let split = if can_split {
            best_split(cols, y, &idx[start..end], n1, params.mtry, rng, &mut pairs)
        } else {
            None
        };
        match split {
            Some((var, thr)) => {
                let col = &cols[var];
                let mid = partition(&mut idx[start..end], |k| col[k as usize] <= thr) + start;
                let l = tree.var.len();
                tree.var.extend([LEAF, LEAF]);
                tree.value.extend([0.0, 0.0]);
                tree.left.extend([0, 0]);
                tree.var[node] = var as u32;
                tree.value[node] = thr;
                tree.left[node] = l as u32;
                stack.push((l + 1, mid, end, depth + 1));
                stack.push((l, start, mid, depth + 1));
            }
            None => {
                tree.value[node] = if size == 0 {
                    0.0
                } else {
                    n1 as f64 / size as f64
                };
            }
        }
    }
    tree
}

/// In-place partition; returns the number of elements satisfying `pred` (moved to the front).
fn partition(v: &mut [u32], pred: impl Fn(u32) -> bool) -> usize {
    let mut i = 0;
    for j in 0..v.len() {
        if pred(v[j]) {
            v.swap(i, j);
            i += 1;
        }
    }
    i
}

/// Best Gini split of the node holding sample indices `idx` (`n1` of them presences) among
/// `mtry` randomly drawn variables, or `None` if no split improves on the parent.
fn best_split(
    cols: &[Vec<f64>],
    y: &[bool],
    idx: &[u32],
    n1: usize,
    mtry: usize,
    rng: &mut Rng,
    pairs: &mut Vec<(f64, bool)>,
) -> Option<(usize, f64)> {
    let n = idx.len();
    let n0 = n - n1;
    let (n1f, n0f) = (n1 as f64, n0 as f64);
    let parent = (n1f * n1f + n0f * n0f) / n as f64;
    let mut best_crit = parent;
    let mut best = None;
    for var in rng.sample_indices(cols.len(), mtry) {
        let col = &cols[var];
        pairs.clear();
        pairs.extend(idx.iter().map(|&k| (col[k as usize], y[k as usize])));
        pairs.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
        if pairs[0].0 == pairs[n - 1].0 {
            continue;
        }
        let (mut l1, mut l0) = (0usize, 0usize);
        for i in 0..n - 1 {
            if pairs[i].1 {
                l1 += 1;
            } else {
                l0 += 1;
            }
            let (a, b) = (pairs[i].0, pairs[i + 1].0);
            if a < b {
                let nl = (i + 1) as f64;
                let nr = (n - i - 1) as f64;
                let (l1f, l0f) = (l1 as f64, l0 as f64);
                let (r1f, r0f) = ((n1 - l1) as f64, (n0 - l0) as f64);
                let crit = (l1f * l1f + l0f * l0f) / nl + (r1f * r1f + r0f * r0f) / nr;
                if crit > best_crit {
                    best_crit = crit;
                    best = Some((var, midpoint(a, b)));
                }
            }
        }
    }
    if best_crit > parent * (1.0 + 1e-12) {
        best
    } else {
        None
    }
}

/// Midpoint of `a < b` that still separates them (`a <= m < b`) despite rounding.
fn midpoint(a: f64, b: f64) -> f64 {
    let m = a * 0.5 + b * 0.5;
    if m >= a && m < b { m } else { a }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> GrowParams {
        GrowParams {
            mtry: 1,
            min_node_size: 1,
            max_depth: None,
        }
    }

    #[test]
    fn separable_one_split() {
        let cols = vec![vec![1.0, 2.0, 3.0, 10.0, 11.0, 12.0]];
        let y = [false, false, false, true, true, true];
        let t = grow(&cols, &y, params(), &mut Rng::new(1));
        assert_eq!(t.n_nodes(), 3);
        assert_eq!(t.value[0], 6.5);
        assert_eq!(t.vote(&[0.0]), 0.0);
        assert_eq!(t.vote(&[7.0]), 1.0);
        assert!(t.check(1).is_ok());
    }

    #[test]
    fn identical_values_give_mixed_leaf() {
        let cols = vec![vec![1.0, 1.0, 1.0, 1.0]];
        let y = [false, true, false, true];
        let t = grow(&cols, &y, params(), &mut Rng::new(1));
        assert_eq!(t.n_nodes(), 1);
        assert_eq!(t.leaf_value(&[1.0]), 0.5);
        assert_eq!(t.vote(&[1.0]), 0.5);
    }

    #[test]
    fn midpoint_separates_adjacent_floats() {
        let a = 1.0f64;
        let b = f64::from_bits(a.to_bits() + 1);
        let m = midpoint(a, b);
        assert!(a <= m && m < b);
    }

    #[test]
    fn check_rejects_cycles() {
        let t = Tree {
            var: vec![0, LEAF, LEAF],
            value: vec![0.0, 0.0, 1.0],
            left: vec![0, 0, 0],
        };
        assert!(t.check(1).is_err());
    }
}
