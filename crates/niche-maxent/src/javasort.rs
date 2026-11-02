//! Port of `density/DoubleIndexSort.java` (MaxEnt 3.4.4).
//!
//! The Java implementation is an unstable Bentley–McIlroy quicksort over indices. MaxEnt uses
//! it to rank features by loss bound, and ties (frequent: many bounds are exactly 0) are broken
//! by this algorithm's partitioning order. Reproducing feature selection therefore requires the
//! same algorithm, not just any correct sort.

/// Indices of `vals` sorted by increasing value, with MaxEnt's tie-breaking.
///
/// # Panics
/// Panics on NaN or negative infinity, as the Java original throws.
pub fn sort_indices(vals: &[f64]) -> Vec<usize> {
    for &v in vals {
        assert!(!v.is_nan(), "NaN encountered in sort_indices");
        assert!(v != f64::NEG_INFINITY, "-inf encountered in sort_indices");
    }
    let mut x: Vec<usize> = (0..vals.len()).collect();
    sort1(&mut x, 0, vals.len(), vals);
    x
}

fn sort1(x: &mut [usize], off: usize, len: usize, vals: &[f64]) {
    if len < 7 {
        for i in off..len + off {
            let mut j = i;
            while j > off && vals[x[j - 1]] > vals[x[j]] {
                x.swap(j, j - 1);
                j -= 1;
            }
        }
        return;
    }
    let mut m = off + len / 2;
    if len > 7 {
        let mut l = off;
        let mut n = off + len - 1;
        if len > 40 {
            let s = len / 8;
            l = med3(x, l, l + s, l + 2 * s, vals);
            m = med3(x, m - s, m, m + s, vals);
            n = med3(x, n - 2 * s, n - s, n, vals);
        }
        m = med3(x, l, m, n, vals);
    }
    let v = vals[x[m]];
    // Signed indices: `c` and `d` can step below `off` in the Java code.
    let (mut a, mut b, mut c, mut d) = (
        off as isize,
        off as isize,
        (off + len) as isize - 1,
        (off + len) as isize - 1,
    );
    loop {
        while b <= c && vals[x[b as usize]] <= v {
            if vals[x[b as usize]] == v {
                x.swap(a as usize, b as usize);
                a += 1;
            }
            b += 1;
        }
        while c >= b && vals[x[c as usize]] >= v {
            if vals[x[c as usize]] == v {
                x.swap(c as usize, d as usize);
                d -= 1;
            }
            c -= 1;
        }
        if b > c {
            break;
        }
        x.swap(b as usize, c as usize);
        b += 1;
        c -= 1;
    }
    let n = (off + len) as isize;
    let off = off as isize;
    let s = (a - off).min(b - a);
    vecswap(x, off, b - s, s);
    let s = (d - c).min(n - d - 1);
    vecswap(x, b, n - s, s);
    let s = b - a;
    if s > 1 {
        sort1(x, off as usize, s as usize, vals);
    }
    let s = d - c;
    if s > 1 {
        sort1(x, (n - s) as usize, s as usize, vals);
    }
}

fn vecswap(x: &mut [usize], mut a: isize, mut b: isize, n: isize) {
    for _ in 0..n {
        x.swap(a as usize, b as usize);
        a += 1;
        b += 1;
    }
}

fn med3(x: &[usize], a: usize, b: usize, c: usize, vals: &[f64]) -> usize {
    let (va, vb, vc) = (vals[x[a]], vals[x[b]], vals[x[c]]);
    if va < vb {
        if vb < vc {
            b
        } else if va < vc {
            c
        } else {
            a
        }
    } else if vb > vc {
        b
    } else if va > vc {
        c
    } else {
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_values() {
        let vals: Vec<f64> = (0..200).map(|i| ((i * 7919) % 101) as f64 - 50.0).collect();
        let idx = sort_indices(&vals);
        assert_eq!(idx.len(), vals.len());
        for w in idx.windows(2) {
            assert!(vals[w[0]] <= vals[w[1]]);
        }
        let mut seen = idx.clone();
        seen.sort_unstable();
        assert_eq!(seen, (0..200).collect::<Vec<_>>());
    }

    #[test]
    fn small_arrays_use_stable_insertion() {
        let vals = [0.0, 0.0, -1.0, 0.0];
        assert_eq!(sort_indices(&vals), vec![2, 0, 1, 3]);
    }
}
