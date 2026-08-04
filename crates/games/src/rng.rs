use rand_core::{RngCore, SeedableRng, impls};
use serde::{Deserialize, Serialize};

/// A seeded RNG using the PCG32 algorithm (Permuted Congruential Generator).
/// It is deterministic, high-quality, and passes the Dieharder suite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SeededRng {
    state: u64,
    inc: u64,
}

impl SeededRng {
    /// Create a new RNG with a given seed.
    /// The stream ID is fixed for simplicity, but could be exposed if needed.
    pub fn new(seed: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: 1, // Default increment
        };
        rng.seed(seed, 1);
        rng
    }

    /// Initialize the RNG with a seed and a stream ID.
    pub fn seed(&mut self, seed: u64, stream: u64) {
        self.inc = (stream << 1) | 1;
        self.state = 0;
        self.next_u32();
        self.state = self.state.wrapping_add(seed);
        self.next_u32();
    }
}

impl RngCore for SeededRng {
    fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        // Linear Congruential Generator (LCG) step
        self.state = old_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.inc);

        // Output permutation (XSH-RR)
        let xorshifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rot = (old_state >> 59) as u32;
        (xorshifted >> rot) | (xorshifted << ((rot.wrapping_neg()) & 31))
    }

    fn next_u64(&mut self) -> u64 {
        let low = self.next_u32() as u64;
        let high = self.next_u32() as u64;
        (high << 32) | low
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        impls::fill_bytes_via_next(self, dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl SeedableRng for SeededRng {
    type Seed = [u8; 8];

    fn from_seed(seed: Self::Seed) -> Self {
        let s = u64::from_le_bytes(seed);
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determinism() {
        let mut rng1 = SeededRng::new(42);
        let mut rng2 = SeededRng::new(42);

        for _ in 0..100 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
        }
    }

    #[test]
    fn test_different_seeds() {
        let mut rng1 = SeededRng::new(42);
        let mut rng2 = SeededRng::new(43);

        // It's statistically possible but extremely unlikely they match for many iterations
        let mut matches = 0;
        for _ in 0..100 {
            if rng1.next_u32() == rng2.next_u32() {
                matches += 1;
            }
        }
        assert!(matches < 5);
    }

    #[test]
    fn test_cloning() {
        let mut rng1 = SeededRng::new(12345);
        for _ in 0..10 {
            rng1.next_u32();
        }

        let mut rng2 = rng1.clone();
        for _ in 0..100 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
        }
    }
}
