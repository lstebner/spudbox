use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use rodio::source::SeekError;
use rodio::Source;
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

pub const FFT_SIZE: usize = 2048;
pub const NUM_BANDS: usize = 64;

// dB floor for normalization: bands below this are treated as silent (mapped to 0).
const DB_FLOOR: f32 = -80.0;

// Exponential smoothing: bands rise quickly on transients and fall slowly.
const ATTACK_ALPHA: f32 = 0.8;
const DECAY_ALPHA: f32 = 0.15;

/// Precomputes which FFT output bins contribute to each log-spaced frequency band.
/// Returns `NUM_BANDS` entries of `(start_bin, end_bin)` (inclusive).
fn compute_band_bins(sample_rate: u32) -> Vec<(usize, usize)> {
    let min_freq = 20.0f32;
    let max_freq = (sample_rate as f32 / 2.0).min(20_000.0);
    let log_min = min_freq.log2();
    let log_max = max_freq.log2();
    (0..NUM_BANDS)
        .map(|i| {
            let low =
                2f32.powf(log_min + i as f32 * (log_max - log_min) / NUM_BANDS as f32);
            let high = 2f32
                .powf(log_min + (i + 1) as f32 * (log_max - log_min) / NUM_BANDS as f32);
            let start = ((low * FFT_SIZE as f32 / sample_rate as f32).floor() as usize).max(1);
            let end = ((high * FFT_SIZE as f32 / sample_rate as f32).ceil() as usize)
                .min(FFT_SIZE / 2)
                .max(start);
            (start, end)
        })
        .collect()
}

fn hann_window() -> Vec<f32> {
    (0..FFT_SIZE)
        .map(|i| {
            0.5 * (1.0
                - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos())
        })
        .collect()
}

/// Wraps any `Source<Item = i16>` and continuously computes two things:
///
/// - **RMS amplitude**: exponential moving average written to `shared_rms`.
/// - **Frequency spectrum**: 64-band FFT run on every `FFT_SIZE`-sample mono
///   frame, with log-spaced bands and temporal smoothing, written to `shared_fft`.
///
/// Both shared values are updated inside `Iterator::next` so they reflect the
/// audio the device is actually playing, not a lookahead.
pub struct AnalysisSource<S: Source<Item = i16>> {
    inner: S,
    // RMS
    ema_squared: f32,
    alpha_rms: f32,
    shared_rms: Arc<AtomicU32>,
    // FFT — mono mix-down buffer
    channel_accumulator: f32,
    channel_counter: u16,
    channels: u16,
    mono_buffer: Vec<f32>,
    // FFT plan and working buffers (reused each frame)
    fft: Arc<dyn Fft<f32>>,
    fft_buffer: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    window: Vec<f32>,
    band_bins: Vec<(usize, usize)>,
    smoothed_bands: Vec<f32>,
    shared_fft: Arc<ArcSwap<Vec<f32>>>,
}

impl<S: Source<Item = i16>> AnalysisSource<S> {
    pub fn new(
        inner: S,
        shared_rms: Arc<AtomicU32>,
        shared_fft: Arc<ArcSwap<Vec<f32>>>,
    ) -> Self {
        let sample_rate = inner.sample_rate();
        let channels = inner.channels().max(1);

        let alpha_rms = 1.0 / (sample_rate as f32 * 0.1);

        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let scratch_len = fft.get_inplace_scratch_len();

        Self {
            inner,
            ema_squared: 0.0,
            alpha_rms,
            shared_rms,
            channel_accumulator: 0.0,
            channel_counter: 0,
            channels,
            mono_buffer: Vec::with_capacity(FFT_SIZE),
            fft,
            fft_buffer: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            scratch: vec![Complex::new(0.0, 0.0); scratch_len],
            window: hann_window(),
            band_bins: compute_band_bins(sample_rate),
            smoothed_bands: vec![0.0; NUM_BANDS],
            shared_fft,
        }
    }

    fn run_fft(&mut self) {
        // Apply Hann window and fill the FFT input buffer.
        for (i, &sample) in self.mono_buffer.iter().enumerate() {
            self.fft_buffer[i] = Complex::new(sample * self.window[i], 0.0);
        }
        self.fft.process_with_scratch(&mut self.fft_buffer, &mut self.scratch);

        // Compute magnitude for each log-spaced band.
        let scale = 2.0 / FFT_SIZE as f32;
        let mut new_bands = vec![0.0f32; NUM_BANDS];
        for (band, &(start, end)) in self.band_bins.iter().enumerate() {
            let mean_mag = self.fft_buffer[start..=end]
                .iter()
                .map(|c| c.norm() * scale)
                .sum::<f32>()
                / (end - start + 1) as f32;

            // Convert to dB and normalize to [0, 1].
            let db = 20.0 * mean_mag.max(1e-10).log10();
            new_bands[band] = ((db - DB_FLOOR) / DB_FLOOR.abs()).clamp(0.0, 1.0);
        }

        // Temporal smoothing: fast attack, slow decay.
        for i in 0..NUM_BANDS {
            let alpha = if new_bands[i] > self.smoothed_bands[i] {
                ATTACK_ALPHA
            } else {
                DECAY_ALPHA
            };
            self.smoothed_bands[i] =
                self.smoothed_bands[i] * (1.0 - alpha) + new_bands[i] * alpha;
        }

        self.shared_fft.store(Arc::new(self.smoothed_bands.clone()));
        self.mono_buffer.clear();
    }

    fn reset(&mut self) {
        self.ema_squared = 0.0;
        self.channel_accumulator = 0.0;
        self.channel_counter = 0;
        self.mono_buffer.clear();
        self.smoothed_bands.fill(0.0);
        self.shared_rms.store(0.0f32.to_bits(), Ordering::Relaxed);
        self.shared_fft.store(Arc::new(vec![0.0; NUM_BANDS]));
    }
}

impl<S: Source<Item = i16>> Iterator for AnalysisSource<S> {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        let sample = self.inner.next()?;

        // Update RMS.
        let normalized = sample as f32 / 32768.0;
        self.ema_squared =
            self.ema_squared * (1.0 - self.alpha_rms) + normalized * normalized * self.alpha_rms;
        self.shared_rms.store(self.ema_squared.sqrt().to_bits(), Ordering::Relaxed);

        // Mix down to mono for FFT.
        self.channel_accumulator += normalized;
        self.channel_counter += 1;
        if self.channel_counter == self.channels {
            let mono = self.channel_accumulator / self.channels as f32;
            self.mono_buffer.push(mono);
            self.channel_counter = 0;
            self.channel_accumulator = 0.0;

            if self.mono_buffer.len() == FFT_SIZE {
                self.run_fft();
            }
        }

        Some(sample)
    }
}

impl<S: Source<Item = i16>> Source for AnalysisSource<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        let result = self.inner.try_seek(pos);
        if result.is_ok() {
            self.reset();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSource {
        samples: std::vec::IntoIter<i16>,
        sample_rate: u32,
        channels: u16,
    }

    impl Iterator for TestSource {
        type Item = i16;
        fn next(&mut self) -> Option<i16> {
            self.samples.next()
        }
    }

    impl Source for TestSource {
        fn current_frame_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> u16 {
            self.channels
        }
        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    fn make_source(samples: Vec<i16>) -> TestSource {
        TestSource { samples: samples.into_iter(), sample_rate: 44100, channels: 1 }
    }

    #[test]
    fn silence_gives_zero_rms() {
        let shared_rms = Arc::new(AtomicU32::new(0));
        let shared_fft = Arc::new(ArcSwap::from_pointee(vec![0.0f32; NUM_BANDS]));
        let source = make_source(vec![0i16; FFT_SIZE * 2]);
        let mut analysis = AnalysisSource::new(source, shared_rms.clone(), shared_fft);
        for _ in 0..FFT_SIZE * 2 {
            analysis.next();
        }
        let rms = f32::from_bits(shared_rms.load(Ordering::Relaxed));
        assert!(rms < 1e-6, "expected near-zero RMS for silence, got {rms}");
    }

    #[test]
    fn loud_signal_gives_nonzero_rms() {
        let shared_rms = Arc::new(AtomicU32::new(0));
        let shared_fft = Arc::new(ArcSwap::from_pointee(vec![0.0f32; NUM_BANDS]));
        let samples = vec![16000i16; FFT_SIZE * 4];
        let source = make_source(samples);
        let mut analysis = AnalysisSource::new(source, shared_rms.clone(), shared_fft);
        for _ in 0..FFT_SIZE * 4 {
            analysis.next();
        }
        let rms = f32::from_bits(shared_rms.load(Ordering::Relaxed));
        assert!(rms > 0.1, "expected nonzero RMS for loud signal, got {rms}");
    }

    #[test]
    fn fft_runs_after_enough_samples_and_produces_nonzero_bands() {
        let shared_rms = Arc::new(AtomicU32::new(0));
        let shared_fft = Arc::new(ArcSwap::from_pointee(vec![0.0f32; NUM_BANDS]));

        // Generate a 1 kHz sine wave.
        let sample_rate = 44100u32;
        let frequency = 1000.0f32;
        let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate as f32;
        let samples: Vec<i16> =
            (0..FFT_SIZE * 2).map(|i| ((omega * i as f32).sin() * 16000.0) as i16).collect();

        let source =
            TestSource { samples: samples.into_iter(), sample_rate, channels: 1 };
        let mut analysis =
            AnalysisSource::new(source, shared_rms, shared_fft.clone());
        for _ in 0..FFT_SIZE * 2 {
            analysis.next();
        }

        let bands = shared_fft.load();
        let max_band = bands.iter().cloned().fold(0.0f32, f32::max);
        assert!(max_band > 0.1, "expected nonzero FFT output for 1 kHz tone, max band was {max_band}");
    }

    #[test]
    fn band_bins_cover_audible_range() {
        let bins = compute_band_bins(44100);
        assert_eq!(bins.len(), NUM_BANDS);
        // First band must start at a bin > 0 (DC component excluded).
        assert!(bins[0].0 >= 1);
        // Last band must not exceed Nyquist.
        assert!(bins[NUM_BANDS - 1].1 <= FFT_SIZE / 2);
        // Bins should be monotonically non-decreasing.
        for i in 1..NUM_BANDS {
            assert!(bins[i].0 >= bins[i - 1].0);
        }
    }
}
