use std::time::Instant;
use spore_core::spectrum::BANDS;

const ATTACK_SECS: f32 = 0.03;
const RELEASE_SECS: f32 = 0.25;

/// Frame-rate independent attack/release smoothing for spectrum bars (0.0..=1.0).
pub struct SpectrumSmoother {
    bars: [f32; BANDS],
    last: Instant,
}

impl SpectrumSmoother {
    pub fn new() -> Self {
        Self { bars: [0.0; BANDS], last: Instant::now() }
    }

    pub fn bars(&self) -> [f32; BANDS] {
        self.bars
    }

    pub fn update(&mut self, target: [f32; BANDS]) -> [f32; BANDS] {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(0.5);
        self.last = now;

        for i in 0..BANDS {
            let t = target[i].clamp(0.0, 1.0);
            let k = if t > self.bars[i] { 1.0 - (-dt / ATTACK_SECS).exp() } else { 1.0 - (-dt / RELEASE_SECS).exp() };
            self.bars[i] += (t - self.bars[i]) * k;
        }

        self.bars
    }
}

impl Default for SpectrumSmoother {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rises_toward_target() {
        let mut s = SpectrumSmoother::new();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let b = s.update([1.0; BANDS]);
        assert!(b[0] > 0.5 && b[0] <= 1.0);
    }

    #[test]
    fn releases_slower_than_attack() {
        let mut s = SpectrumSmoother::new();
        s.bars = [1.0; BANDS];
        s.last = Instant::now() - std::time::Duration::from_millis(30);
        let b = s.update([0.0; BANDS]);
        assert!(b[0] > 0.8);
    }
}