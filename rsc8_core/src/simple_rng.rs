pub struct SimpleRng {
    pub seed: u16,
}

impl SimpleRng {
    pub fn new(seed: u16) -> Self {
        SimpleRng { seed }
    }

    pub fn gen(&mut self) -> u16 {
        const A: u16 = 75;
        const C: u16 = 74;

        self.seed = A.wrapping_mul(self.seed).wrapping_add(C);
        self.seed
    }
}
