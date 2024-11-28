pub struct SimpleRng {
    pub seed: u16,
}

impl SimpleRng {
    pub fn new(seed: u16) -> Self {
        SimpleRng { seed }
    }
}

impl Iterator for SimpleRng {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        const A: u16 = 75;
        const C: u16 = 74;
        self.seed = A.wrapping_mul(self.seed).wrapping_add(C);
        Some(self.seed)
    }
}
