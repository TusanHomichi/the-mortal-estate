#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicRng {
    state: u32,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed as u32 }
    }

    pub(crate) const fn checkpoint_state(&self) -> u32 {
        self.state
    }

    pub(crate) const fn from_checkpoint_state(state: u32) -> Self {
        Self { state }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(17).wrapping_add(11);
        self.state
    }

    pub fn roll_d20(&mut self) -> u32 {
        (self.next_u32() % 20) + 1
    }

    pub fn roll_percent(&mut self) -> u32 {
        (self.next_u32() % 100) + 1
    }

    pub fn roll_bounded(&mut self, denominator: u32) -> Result<u32, &'static str> {
        if denominator == 0 {
            return Err("roll denominator must be positive");
        }
        Ok((self.next_u32() % denominator) + 1)
    }

    pub fn weighted_index(&mut self, weights: &[u32]) -> Result<usize, &'static str> {
        if weights.is_empty() {
            return Err("weighted outcomes must be non-empty");
        }
        let total_weight = weights.iter().try_fold(0_u32, |total, weight| {
            if *weight == 0 {
                None
            } else {
                total.checked_add(*weight)
            }
        });
        let Some(total_weight) = total_weight else {
            return Err("weighted outcomes must have positive checked total weight");
        };
        if weights.len() == 1 {
            return Ok(0);
        }
        let bucket = self.next_u32() % total_weight;
        let mut cumulative = 0_u32;
        for (index, weight) in weights.iter().enumerate() {
            cumulative = cumulative
                .checked_add(*weight)
                .ok_or("weighted outcome total overflow")?;
            if bucket < cumulative {
                return Ok(index);
            }
        }
        Err("weighted outcome selection failed")
    }
}

#[cfg(test)]
mod tests {
    use super::DeterministicRng;

    #[test]
    fn low_32_seed_mapping_and_replay_are_stable() {
        let mut low = DeterministicRng::new(7);
        let mut high = DeterministicRng::new((1_u64 << 40) | 7);
        assert_eq!(low.roll_d20(), high.roll_d20());
        assert_eq!(low.weighted_index(&[1, 1]), high.weighted_index(&[1, 1]));
    }

    #[test]
    fn singleton_weight_does_not_advance_state() {
        let mut selected = DeterministicRng::new(11);
        let untouched = selected.clone();
        assert_eq!(selected.weighted_index(&[7]), Ok(0));
        assert_eq!(selected, untouched);
    }

    #[test]
    fn weighted_selection_uses_one_shared_transition() {
        let mut selected = DeterministicRng::new(7);
        let mut rolled = DeterministicRng::new(7);
        let index = selected.weighted_index(&[1, 1]).unwrap();
        let roll = rolled.roll_d20();
        assert_eq!(index, usize::try_from((roll - 1) % 2).unwrap());
        assert_eq!(selected, rolled);
    }

    #[test]
    fn percent_roll_uses_one_shared_transition_and_stays_in_range() {
        for seed in 0..200 {
            let mut percent = DeterministicRng::new(seed);
            let mut weighted = DeterministicRng::new(seed);
            let roll = percent.roll_percent();
            assert!((1..=100).contains(&roll));
            assert_eq!(
                weighted.weighted_index(&[1; 100]).unwrap() + 1,
                roll as usize
            );
            assert_eq!(percent, weighted);
        }
    }

    #[test]
    fn bounded_roll_rejects_zero_without_advancing_state() {
        let mut rng = DeterministicRng::new(7);
        let before = rng.clone();
        assert_eq!(
            rng.roll_bounded(0),
            Err("roll denominator must be positive")
        );
        assert_eq!(rng, before);
    }

    #[test]
    fn bounded_roll_uses_one_shared_transition_and_stays_in_range() {
        for denominator in [1, 2, 7, 20, 101] {
            for seed in 0..40 {
                let mut bounded = DeterministicRng::new(seed);
                let mut expected = DeterministicRng::new(seed);
                let roll = bounded.roll_bounded(denominator).unwrap();
                let transition = expected.next_u32();
                assert_eq!(roll, transition % denominator + 1);
                assert!((1..=denominator).contains(&roll));
                assert_eq!(bounded, expected);
            }
        }
    }
}
