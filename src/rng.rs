//! Exact Darwin/macOS `libSystem.B.dylib` Park-Miller linear congruential RNG.

/// Park-Miller LCG matching macOS `srand` / `rand` (`RAND_MAX = 2_147_483_647`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacOsRng {
    state: u32,
}

impl Default for MacOsRng {
    fn default() -> Self {
        Self::new(1)
    }
}

impl MacOsRng {
    pub fn new(seed: u32) -> Self {
        let mut rng = Self { state: 1 };
        rng.srand(seed);
        rng
    }

    pub fn srand(&mut self, seed: u32) {
        let mut s = seed % 2_147_483_647;
        if s == 0 {
            s = 123_459_876;
        }
        self.state = s;
    }

    pub fn state(&self) -> u32 {
        self.state
    }

    pub fn from_raw(state: u32) -> Self {
        let mut rng = Self { state: 1 };
        rng.srand(state);
        rng
    }

    pub fn rand_int(&mut self) -> i32 {
        self.state = (((self.state as u64) * 16_807) % 2_147_483_647) as u32;
        self.state as i32
    }

    /// Equivalent to C `Rand()` = `rand() / (1.0 + (double)RAND_MAX)`.
    pub fn rand_f64(&mut self) -> f64 {
        (self.rand_int() as f64) / 2_147_483_648.0
    }

    /// Exponential random variable: `-avrage * ln(1e-7 + Rand())`.
    pub fn expran(&mut self, avrage: f64) -> f64 {
        -avrage * (1e-7 + self.rand_f64()).ln()
    }

    /// Random quadrant coordinates in `1..=8`.
    pub fn iran8(&mut self) -> (i32, i32) {
        let x = (self.rand_f64() * 8.0 + 1.0) as i32;
        let y = (self.rand_f64() * 8.0 + 1.0) as i32;
        (x, y)
    }

    /// Random sector coordinates in `1..=10`.
    pub fn iran10(&mut self) -> (i32, i32) {
        let x = (self.rand_f64() * 10.0 + 1.0) as i32;
        let y = (self.rand_f64() * 10.0 + 1.0) as i32;
        (x, y)
    }
}
