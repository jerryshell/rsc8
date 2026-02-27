const DEFAULT_SEED: u16 = 996;
// Full-period LCG constants for modulus 2^16.
// a % 4 == 1 and c is odd.
const LCG_A: u16 = 25_173;
const LCG_C: u16 = 13_849;

pub struct LinearCongruentialGenerator {
    pub seed: u16,
}

impl Default for LinearCongruentialGenerator {
    fn default() -> Self {
        Self { seed: DEFAULT_SEED }
    }
}

impl Iterator for LinearCongruentialGenerator {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        self.seed = LCG_A.wrapping_mul(self.seed).wrapping_add(LCG_C);
        Some(self.seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_always_returns_some() {
        let mut rng = LinearCongruentialGenerator::default();
        assert!(rng.next().is_some());
    }

    #[test]
    fn sequence_has_full_period_for_default_seed() {
        let mut rng = LinearCongruentialGenerator::default();
        let start = rng.seed;

        let mut period: u32 = 0;
        loop {
            period += 1;
            let value = rng.next().unwrap();
            if value == start {
                break;
            }
            assert!(period <= u16::MAX as u32 + 1);
        }

        assert_eq!(period, u16::MAX as u32 + 1);
    }

    #[test]
    fn low_bit_is_not_constant() {
        let mut rng = LinearCongruentialGenerator::default();
        let mut has_even = false;
        let mut has_odd = false;

        for _ in 0..512 {
            let value = rng.next().unwrap();
            has_even |= value & 1 == 0;
            has_odd |= value & 1 == 1;
            if has_even && has_odd {
                break;
            }
        }

        assert!(has_even && has_odd);
    }

    #[test]
    fn sample_seeds_do_not_stick_at_fixed_points() {
        for seed in [0, 1, 2, 3, 32_767, 65_535] {
            let mut rng = LinearCongruentialGenerator { seed };
            assert_ne!(rng.next().unwrap(), seed);
        }
    }
}
