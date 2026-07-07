use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use rodio::source::SeekError;
use rodio::Source;

pub const EQ_BAND_COUNT: usize = 8;
pub const EQ_BAND_FREQUENCIES: [f32; EQ_BAND_COUNT] =
    [63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];

const EQ_Q: f32 = 1.414;

pub struct EqGains {
    pub gains_db: [f32; EQ_BAND_COUNT],
    pub enabled: bool,
    pub version: u64,
}

impl Default for EqGains {
    fn default() -> Self {
        Self {
            gains_db: [0.0; EQ_BAND_COUNT],
            enabled: true,
            version: 0,
        }
    }
}

/// Biquad filter coefficients (normalized: a0 factored out).
#[derive(Clone, Copy)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Default for BiquadCoeffs {
    /// Identity (pass-through) filter.
    fn default() -> Self {
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 }
    }
}

/// Direct Form I biquad filter state.
#[derive(Clone, Copy, Default)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadState {
    fn process(&mut self, x0: f32, c: &BiquadCoeffs) -> f32 {
        let y0 = c.b0 * x0 + c.b1 * self.x1 + c.b2 * self.x2
            - c.a1 * self.y1
            - c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x0;
        self.y2 = self.y1;
        self.y1 = y0;
        y0
    }
}

/// Peaking EQ biquad coefficients from the Audio EQ Cookbook (R. Bristow-Johnson).
fn peaking_coefficients(frequency: f32, sample_rate: f32, gain_db: f32, q: f32) -> BiquadCoeffs {
    let amplitude = 10f32.powf(gain_db / 40.0);
    let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate;
    let cos_omega = omega.cos();
    let alpha = omega.sin() / (2.0 * q);

    let a0 = 1.0 + alpha / amplitude;
    BiquadCoeffs {
        b0: (1.0 + alpha * amplitude) / a0,
        b1: (-2.0 * cos_omega) / a0,
        b2: (1.0 - alpha * amplitude) / a0,
        a1: (-2.0 * cos_omega) / a0,
        a2: (1.0 - alpha / amplitude) / a0,
    }
}

fn compute_coefficients(eq: &EqGains, sample_rate: u32) -> [BiquadCoeffs; EQ_BAND_COUNT] {
    let mut coefficients = [BiquadCoeffs::default(); EQ_BAND_COUNT];
    for (band, (&frequency, &gain_db)) in EQ_BAND_FREQUENCIES
        .iter()
        .zip(eq.gains_db.iter())
        .enumerate()
    {
        let effective_gain = if eq.enabled { gain_db } else { 0.0 };
        coefficients[band] =
            peaking_coefficients(frequency, sample_rate as f32, effective_gain, EQ_Q);
    }
    coefficients
}

/// Wraps any `Source<Item = i16>` and applies an 8-band peaking EQ.
///
/// Coefficients are recomputed on the next sample whenever the shared `EqGains`
/// version changes, making EQ adjustments take effect immediately with no
/// thread synchronisation overhead beyond an atomic pointer load.
pub struct EqualizerSource<S: Source<Item = i16>> {
    inner: S,
    eq: Arc<ArcSwap<EqGains>>,
    cached_version: u64,
    /// `states[channel][band]`
    states: Vec<[BiquadState; EQ_BAND_COUNT]>,
    coefficients: [BiquadCoeffs; EQ_BAND_COUNT],
    current_channel: usize,
}

impl<S: Source<Item = i16>> EqualizerSource<S> {
    pub fn new(inner: S, eq: Arc<ArcSwap<EqGains>>) -> Self {
        let sample_rate = inner.sample_rate();
        let channels = inner.channels() as usize;
        let eq_state = eq.load();
        let coefficients = compute_coefficients(&eq_state, sample_rate);
        let cached_version = eq_state.version;
        drop(eq_state);

        let states = vec![[BiquadState::default(); EQ_BAND_COUNT]; channels.max(1)];

        Self {
            inner,
            eq,
            cached_version,
            states,
            coefficients,
            current_channel: 0,
        }
    }
}

impl<S: Source<Item = i16>> Iterator for EqualizerSource<S> {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        let eq = self.eq.load();
        if eq.version != self.cached_version {
            self.coefficients = compute_coefficients(&eq, self.inner.sample_rate());
            self.cached_version = eq.version;
        }
        drop(eq);

        let sample = self.inner.next()?;
        let channel = self.current_channel;
        self.current_channel = (channel + 1) % self.states.len();

        let mut signal = sample as f32 / 32768.0;
        for band in 0..EQ_BAND_COUNT {
            signal = self.states[channel][band].process(signal, &self.coefficients[band]);
        }
        Some((signal.clamp(-1.0, 1.0) * 32767.0) as i16)
    }
}

impl<S: Source<Item = i16>> Source for EqualizerSource<S> {
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
            for channel_states in &mut self.states {
                for state in channel_states {
                    *state = BiquadState::default();
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_response_at_zero_db_gives_identity_coefficients() {
        let coefficients = peaking_coefficients(1000.0, 44100.0, 0.0, EQ_Q);
        assert!((coefficients.b0 - 1.0).abs() < 1e-6);
        assert!((coefficients.b1 - coefficients.a1).abs() < 1e-6);
        assert!((coefficients.b2 - coefficients.a2).abs() < 1e-6);
    }

    #[test]
    fn default_biquad_state_and_identity_coefficients_produce_no_change() {
        let coefficients = BiquadCoeffs::default();
        let mut state = BiquadState::default();
        assert_eq!(state.process(0.5, &coefficients), 0.5);
        assert_eq!(state.process(-0.25, &coefficients), -0.25);
        assert_eq!(state.process(0.0, &coefficients), 0.0);
    }

    #[test]
    fn positive_gain_amplifies_at_center_frequency() {
        let gain_db = 6.0_f32;
        let frequency = 1000.0_f32;
        let sample_rate = 44100.0_f32;
        let coefficients = peaking_coefficients(frequency, sample_rate, gain_db, EQ_Q);
        let mut state = BiquadState::default();
        let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate;

        let skip = 200;
        let measure = 4000;
        let mut sum_sq_input = 0.0f32;
        let mut sum_sq_output = 0.0f32;
        for index in 0..(skip + measure) {
            let input = (omega * index as f32).sin();
            let output = state.process(input, &coefficients);
            if index >= skip {
                sum_sq_input += input * input;
                sum_sq_output += output * output;
            }
        }

        let ratio_db = 10.0 * (sum_sq_output / sum_sq_input).log10();
        assert!(
            (ratio_db - gain_db).abs() < 0.5,
            "expected ~{gain_db} dB gain, measured {ratio_db:.2} dB"
        );
    }

    #[test]
    fn negative_gain_attenuates_at_center_frequency() {
        let gain_db = -6.0_f32;
        let frequency = 1000.0_f32;
        let sample_rate = 44100.0_f32;
        let coefficients = peaking_coefficients(frequency, sample_rate, gain_db, EQ_Q);
        let mut state = BiquadState::default();
        let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate;

        let skip = 200;
        let measure = 4000;
        let mut sum_sq_input = 0.0f32;
        let mut sum_sq_output = 0.0f32;
        for index in 0..(skip + measure) {
            let input = (omega * index as f32).sin();
            let output = state.process(input, &coefficients);
            if index >= skip {
                sum_sq_input += input * input;
                sum_sq_output += output * output;
            }
        }

        let ratio_db = 10.0 * (sum_sq_output / sum_sq_input).log10();
        assert!(
            (ratio_db - gain_db).abs() < 0.5,
            "expected ~{gain_db} dB gain, measured {ratio_db:.2} dB"
        );
    }

    struct TestSource {
        samples: std::vec::IntoIter<i16>,
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
            1
        }
        fn sample_rate(&self) -> u32 {
            44100
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    #[test]
    fn equalizer_source_passes_through_with_all_zero_gains() {
        let samples: Vec<i16> = vec![0, 100, -200, 1000, -32000, 32000, 0, 500];
        let source = TestSource { samples: samples.clone().into_iter() };
        let eq_state = Arc::new(ArcSwap::from_pointee(EqGains::default()));
        let mut eq_source = EqualizerSource::new(source, eq_state);

        let output: Vec<i16> = eq_source.by_ref().take(samples.len()).collect();
        for (input, output) in samples.iter().zip(output.iter()) {
            assert!(
                (input - output).abs() <= 1,
                "input {input} → output {output} should differ by at most 1 LSB with flat EQ"
            );
        }
    }

    #[test]
    fn equalizer_source_recomputes_on_version_bump() {
        // Use a 1 kHz sine wave — that's the center frequency of band index 4.
        // Amplitude is small enough (3000) that a 12 dB boost won't clip.
        let sample_rate = 44100u32;
        let frequency = 1000.0_f32;
        let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate as f32;
        let samples: Vec<i16> = (0..600)
            .map(|i| ((omega * i as f32).sin() * 3000.0) as i16)
            .collect();

        let source = TestSource { samples: samples.into_iter() };
        let eq_state = Arc::new(ArcSwap::from_pointee(EqGains::default()));
        let mut eq_source = EqualizerSource::new(source, eq_state.clone());

        // Discard 200 samples to let the filter state settle with flat EQ.
        let _settling: Vec<i16> = eq_source.by_ref().take(200).collect();
        let flat: Vec<i16> = eq_source.by_ref().take(100).collect();

        // Boost 1 kHz (band 4) by 12 dB and trigger a version bump.
        let mut boosted_gains = EqGains::default();
        boosted_gains.gains_db[4] = 12.0;
        boosted_gains.version = 1;
        eq_state.store(Arc::new(boosted_gains));

        let boosted: Vec<i16> = eq_source.by_ref().take(100).collect();

        let flat_rms: f64 =
            (flat.iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / flat.len() as f64).sqrt();
        let boosted_rms: f64 =
            (boosted.iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / boosted.len() as f64)
                .sqrt();

        // 12 dB boost ≈ 4× power = 2× RMS; require at least 1.5× to give some margin.
        assert!(
            boosted_rms > flat_rms * 1.5,
            "boosted RMS {boosted_rms:.0} should significantly exceed flat RMS {flat_rms:.0}"
        );
    }
}
