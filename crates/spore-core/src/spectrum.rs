use rustfft::{num_complex::Complex, FftPlanner};

pub const BANDS: usize = 12;
pub const FFT_SIZE: usize = 1024;

#[derive(Debug, Clone, Default)]
pub struct SpectrumTimeline {
    hop_ms: f64,
    frames: Vec<[f32; BANDS]>,
}

impl SpectrumTimeline {
    pub fn frame_at(&self, ms: u64) -> [f32; BANDS] {
        if self.frames.is_empty() || self.hop_ms <= 0.0 {
            return [0.0; BANDS];
        }
        let idx = ((ms as f64 / self.hop_ms) as usize).min(self.frames.len() - 1);
        self.frames[idx]
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

pub fn analyze(samples: &[i16], channels: u16, sample_rate: u32) -> SpectrumTimeline {
    let ch = channels.max(1) as usize;
    let mono: Vec<f32> = samples.chunks(ch)
        .map(|c| c.iter().map(|&s| s as f32).sum::<f32>() / (c.len() as f32 * 32768.0))
        .collect();

    if sample_rate == 0 || mono.len() < FFT_SIZE {
        return SpectrumTimeline::default();
    }

    let hop = (sample_rate as usize / 43).max(1);
    let hop_ms = (hop as f64 * 1000.0) / sample_rate as f64;

    let window: Vec<f32> = (0..FFT_SIZE)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos())
        .collect();

    let half = FFT_SIZE / 2;
    let bin = |f: f32| ((f * FFT_SIZE as f32 / sample_rate as f32) as usize).clamp(1, half);
    let edge = |i: usize| {
        40.0f32 * (16000.0f32 / 40.0).powf(i as f32 / BANDS as f32)
    };

    let ranges: Vec<(usize, usize)> = (0..BANDS)
        .map(|b| {
            let lo = bin(edge(b));
            let hi = bin(edge(b + 1)).max(lo + 1).min(half + 1);
            (lo, hi)
        })
        .collect();

    let fft = FftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
    let mut buf = vec![Complex::new(0.0f32, 0.0); FFT_SIZE];

    let mut frames = Vec::with_capacity((mono.len() - FFT_SIZE + hop) / hop);

    for start in (0..=mono.len() - FFT_SIZE).step_by(hop) {
        buf.iter_mut().enumerate().for_each(|(i, c)| {
            *c = Complex::new(mono[start + i] * window[i], 0.0);
        });

        fft.process(&mut buf);

        let mut frame = [0.0; BANDS];
        for b in 0..BANDS {
            let lo = ranges[b].0;
            let hi = ranges[b].1;
            let mag = buf[lo..hi]
                .iter()
                .map(|c| c.norm())
                .fold(0.0, f32::max)
                * 2.0
                / FFT_SIZE as f32;
            let db = 20.0 * (mag.max(1e-9)).log10();
            frame[b] = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
        }
        frames.push(frame);
    }

    SpectrumTimeline { hop_ms, frames }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_zero() {
        let t = analyze(&vec![0i16; 44100 * 2], 2, 44100);
        assert!(!t.is_empty());
        for i in 0..t.len() {
            for v in t.frame_at((i as f64 * 23.2) as u64) {
                assert!(v < 0.01);
            }
        }
    }

    #[test]
    fn sine_1khz_peaks_in_right_band() {
        let sr = 44100u32;
        let samples: Vec<i16> = (0..sr).map(|n| (16000.0 * (2.0 * std::f32::consts::PI * 1000.0 * n as f32 / sr as f32).sin()) as i16).collect();
        let t = analyze(&samples, 1, sr);
        let f = t.frame_at(500);
        let argmax = (0..BANDS).max_by(|&a, &b| f[a].partial_cmp(&f[b]).unwrap()).unwrap();
        let expected = (0..BANDS).find(|&b| {
            let lo = 40.0f32 * 400f32.powf(b as f32 / BANDS as f32);
            let hi = 40.0f32 * 400f32.powf((b + 1) as f32 / BANDS as f32);
            lo <= 1000.0 && 1000.0 < hi
        }).unwrap();
        assert_eq!(argmax, expected);
    }

    #[test]
    fn frame_at_out_of_range_no_panic() {
        let t = analyze(&vec![0i16; 44100], 1, 44100);
        let last = t.frame_at(u64::MAX / 2);
        assert_eq!(t.frame_at(10_000_000), last);
        assert_eq!(SpectrumTimeline::default().frame_at(5), [0.0; BANDS]);
    }
}
