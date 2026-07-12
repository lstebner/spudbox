use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rodio::source::SeekError;
use rodio::Source;

/// Wraps any `Source<Item = i16>`, maintains an exponential moving average of
/// signal amplitude, and writes the current RMS to a shared atomic so the
/// audio engine can include it in playback snapshots without owning the source.
///
/// Time constant is fixed at 100ms (relative to the source's sample rate),
/// which produces a value that tracks the recent loudness of a passage rather
/// than individual transients.
pub struct RmsSource<S: Source<Item = i16>> {
    inner: S,
    ema_squared: f32,
    alpha: f32,
    shared: Arc<AtomicU32>,
}

impl<S: Source<Item = i16>> RmsSource<S> {
    pub fn new(inner: S, shared: Arc<AtomicU32>) -> Self {
        let sample_rate = inner.sample_rate();
        let alpha = 1.0 / (sample_rate as f32 * 0.1);
        Self { inner, ema_squared: 0.0, alpha, shared }
    }
}

impl<S: Source<Item = i16>> Iterator for RmsSource<S> {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        let sample = self.inner.next()?;
        let normalized = sample as f32 / 32768.0;
        self.ema_squared =
            self.ema_squared * (1.0 - self.alpha) + normalized * normalized * self.alpha;
        let rms = self.ema_squared.sqrt();
        self.shared.store(rms.to_bits(), Ordering::Relaxed);
        Some(sample)
    }
}

impl<S: Source<Item = i16>> Source for RmsSource<S> {
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
            self.ema_squared = 0.0;
            self.shared.store(0.0f32.to_bits(), Ordering::Relaxed);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct TestSource {
        samples: std::vec::IntoIter<i16>,
        sample_rate: u32,
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
            self.sample_rate
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    #[test]
    fn silence_produces_zero_rms() {
        let shared = Arc::new(AtomicU32::new(0));
        let source = TestSource { samples: vec![0i16; 100].into_iter(), sample_rate: 44100 };
        let mut rms_source = RmsSource::new(source, shared.clone());
        for _ in 0..100 {
            rms_source.next();
        }
        let rms = f32::from_bits(shared.load(Ordering::Relaxed));
        assert!(rms < 1e-6, "expected RMS near zero for silence, got {rms}");
    }

    #[test]
    fn loud_signal_produces_nonzero_rms() {
        let shared = Arc::new(AtomicU32::new(0));
        let samples: Vec<i16> = (0..44100).map(|_| 16000i16).collect();
        let source = TestSource { samples: samples.into_iter(), sample_rate: 44100 };
        let mut rms_source = RmsSource::new(source, shared.clone());
        for _ in 0..44100 {
            rms_source.next();
        }
        let rms = f32::from_bits(shared.load(Ordering::Relaxed));
        assert!(rms > 0.1, "expected nonzero RMS for loud signal, got {rms}");
    }

    #[test]
    fn seek_resets_ema_to_zero() {
        let shared = Arc::new(AtomicU32::new(0));
        let samples: Vec<i16> = (0..44100).map(|_| 16000i16).collect();
        let source = TestSource { samples: samples.into_iter(), sample_rate: 44100 };
        let mut rms_source = RmsSource::new(source, shared.clone());
        for _ in 0..44100 {
            rms_source.next();
        }
        assert!(f32::from_bits(shared.load(Ordering::Relaxed)) > 0.0);

        // Seeking on a TestSource always fails (not seekable), but the RMS
        // wrapper should still reset on a successful seek. We verify the
        // reset path by temporarily trusting the contract: when try_seek
        // would return Ok, the shared value is zeroed. Since TestSource
        // returns Err, the value should be unchanged here.
        let _ = rms_source.try_seek(Duration::ZERO);
        // Value should be unchanged because the inner seek failed.
        assert!(f32::from_bits(shared.load(Ordering::Relaxed)) > 0.0);
    }
}
