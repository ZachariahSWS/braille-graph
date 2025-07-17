//! Tiny, fast LCG + Box-Muller.
//! Avoids rand dependency

#[derive(Clone)]
pub struct Lcg(u64);

impl Lcg {
    #[must_use]
    pub fn seed(seed: u64) -> Self {
        Self(seed)
    }
    pub fn seed_from_time() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        Self(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        )
    }

    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.0 >> 32) as u32
    }
    #[inline]
    fn next_f64(&mut self) -> f64 {
        f64::from(self.next_u32()) / (f64::from(u32::MAX))
    }

    /// Standard normal 𝒩(0, 1) sample.
    #[inline]
    pub fn randn(&mut self) -> f64 {
        let u1 = self.next_f64().max(f64::MIN_POSITIVE);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}
