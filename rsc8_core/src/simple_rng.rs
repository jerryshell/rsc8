const DEFAULT_SEED: u16 = 996;
const LCG_A: u16 = 75;
const LCG_C: u16 = 74;

pub struct SimpleRng {
    pub seed: u16,
}

impl Default for SimpleRng {
    fn default() -> Self {
        Self { seed: DEFAULT_SEED }
    }
}

impl Iterator for SimpleRng {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        self.seed = LCG_A.wrapping_mul(self.seed).wrapping_add(LCG_C);
        Some(self.seed)
    }
}
