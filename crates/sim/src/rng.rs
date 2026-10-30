//! The world's one source of chance: seeded, and saved with the world, so a
//! replay rolls exactly what the original did.

use serde::{Deserialize, Serialize};

/// SplitMix64: small, fast and the same everywhere.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number in 0..n; n must be above 0.
    pub fn below(&mut self, n: u32) -> u32 {
        // Multiply-shift, which is unbiased enough for a game and has no loop.
        (((self.next_u64() >> 32) * u64::from(n)) >> 32) as u32
    }

    /// A number in -n..=n.
    pub fn spread(&mut self, n: i32) -> i32 {
        let width = n.unsigned_abs() * 2 + 1;
        self.below(width) as i32 - n.abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_rolls_the_same() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        let rolls: Vec<u32> = (0..100).map(|_| a.below(6)).collect();
        assert_eq!(rolls, (0..100).map(|_| b.below(6)).collect::<Vec<_>>());
        assert!(rolls.iter().all(|r| *r < 6));
        assert!((0..6).all(|face| rolls.contains(&face)));
    }

    #[test]
    fn spread_stays_in_range() {
        let mut r = Rng::new(1);
        for _ in 0..1000 {
            let s = r.spread(3);
            assert!((-3..=3).contains(&s));
        }
    }
}
