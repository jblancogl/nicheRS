//! Port of `java.util.Random` (48-bit linear congruential generator, JDK ≥ 1.2 algorithm).
//!
//! MaxEnt 3.4.4 seeds its own generators (e.g. `new Random(11111)` in
//! `PermutationImportance`), so reproducing its outputs requires the identical stream.

const MULTIPLIER: u64 = 0x5_DEEC_E66D;
const ADDEND: u64 = 0xB;
const MASK: u64 = (1 << 48) - 1;

#[derive(Debug, Clone)]
pub struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    /// `new Random(seed)`.
    pub fn new(seed: i64) -> Self {
        Self {
            seed: (seed as u64 ^ MULTIPLIER) & MASK,
        }
    }

    /// `Random.next(bits)` for `bits` ≤ 32, as the non-negative value Java's int would hold
    /// when `bits` < 32.
    fn next(&mut self, bits: u32) -> u64 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        self.seed >> (48 - bits)
    }

    /// `Random.nextDouble()`: 53 random bits scaled to [0, 1).
    pub fn next_double(&mut self) -> f64 {
        let hi = self.next(26);
        let lo = self.next(27);
        ((hi << 27) + lo) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_java_stream() {
        // Printed by OpenJDK 25 (`new java.util.Random(seed).nextDouble()`); the algorithm is
        // fixed by the Java specification, so these hold for every JDK.
        let mut r = JavaRandom::new(42);
        assert_eq!(r.next_double(), 0.7275636800328681);
        assert_eq!(r.next_double(), 0.6832234717598454);
        assert_eq!(r.next_double(), 0.30871945533265976);
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_double(), 0.730967787376657);
        // The seed of MaxEnt's PermutationImportance.
        assert_eq!(JavaRandom::new(11111).next_double(), 0.14877869593833193);
    }
}
