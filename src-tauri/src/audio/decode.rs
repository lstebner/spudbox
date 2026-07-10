use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use rodio::source::SeekError;
use rodio::Source;
use symphonia::core::audio::{AudioBufferRef, SampleBuffer, SignalSpec};
use symphonia::core::codecs::{CodecRegistry, Decoder as SymphoniaCodec, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, SeekedTo};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::{self, Time};
use symphonia::default::{get_codecs, get_probe};

const MAX_DECODE_RETRIES: usize = 3;

// Seeking exactly at (or past) a track's total duration leaves symphonia's
// format reader with no packet to land on, so an end-of-track seek target
// is nudged just inside the track by this many seconds instead.
const END_OF_TRACK_SEEK_EPSILON: f64 = 0.0001;

/// Wraps a file with a correctly-reported byte length, unlike rodio 0.20's
/// internal `ReadSeekSource`, which hardcodes `byte_len()` to `None`. Without
/// a known byte length, symphonia's FLAC/format readers can't compute a seek
/// target and `FormatReader::seek` fails with `Unseekable`.
struct FileMediaSource {
    inner: BufReader<File>,
    len: u64,
}

impl Read for FileMediaSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for FileMediaSource {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl MediaSource for FileMediaSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("symphonia error: {0}")]
    Symphonia(#[from] SymphoniaError),
    #[error("no supported audio track found")]
    NoSupportedTrack,
    #[error("too many consecutive decode errors")]
    TooManyDecodeErrors,
}

#[derive(Debug, thiserror::Error)]
enum TrackSeekError {
    #[error("could not get next packet while refining seek position: {0}")]
    Refining(SymphoniaError),
    #[error("format reader failed to seek: {0}")]
    BaseSeek(SymphoniaError),
    #[error("decoding failed while retrying after seek: {0}")]
    Retrying(SymphoniaError),
    #[error("decoding failed on multiple consecutive packets after seek: {0}")]
    Decoding(SymphoniaError),
}

/// A `rodio::Source` backed directly by symphonia, bypassing rodio's
/// `Decoder` convenience wrapper to fix the seek bug above. This is the
/// "drive symphonia directly" fallback the architecture plan anticipated
/// for gapless control in a later phase, pulled forward because basic
/// seeking needs it now.
pub struct FileSource {
    decoder: Box<dyn SymphoniaCodec>,
    current_frame_offset: usize,
    format: Box<dyn FormatReader>,
    track_id: u32,
    total_duration: Option<Time>,
    buffer: SampleBuffer<i16>,
    spec: SignalSpec,
}

impl FileSource {
    pub fn open(path: &Path) -> Result<Self, DecodeError> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let media_source = FileMediaSource {
            inner: BufReader::new(file),
            len,
        };
        let mss = MediaSourceStream::new(Box::new(media_source), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let metadata_opts = MetadataOptions::default();
        let mut probed = get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;

        let default_track = probed
            .format
            .default_track()
            .ok_or(DecodeError::NoSupportedTrack)?;
        let total_duration = default_track
            .codec_params
            .time_base
            .zip(default_track.codec_params.n_frames)
            .map(|(base, frames)| base.calc_time(frames));

        let track_id = probed
            .format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(DecodeError::NoSupportedTrack)?
            .id;
        let track = probed
            .format
            .tracks()
            .iter()
            .find(|t| t.id == track_id)
            .unwrap();

        let registry: &CodecRegistry = get_codecs();
        let mut decoder = registry.make(&track.codec_params, &DecoderOptions::default())?;

        let mut decode_errors = 0usize;
        let decoded = loop {
            let packet = match probed.format.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(_)) => break decoder.last_decoded(),
                Err(e) => return Err(e.into()),
            };
            if packet.track_id() != track_id {
                continue;
            }
            match decoder.decode(&packet) {
                Ok(decoded) => break decoded,
                Err(SymphoniaError::DecodeError(_)) => {
                    decode_errors += 1;
                    if decode_errors > MAX_DECODE_RETRIES {
                        return Err(DecodeError::TooManyDecodeErrors);
                    }
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        };

        let spec = *decoded.spec();
        let buffer = Self::sample_buffer(decoded, &spec);

        Ok(Self {
            decoder,
            current_frame_offset: 0,
            format: probed.format,
            track_id,
            total_duration,
            buffer,
            spec,
        })
    }

    fn sample_buffer(decoded: AudioBufferRef, spec: &SignalSpec) -> SampleBuffer<i16> {
        let duration = units::Duration::from(decoded.capacity() as u64);
        let mut buffer = SampleBuffer::<i16>::new(duration, *spec);
        buffer.copy_interleaved_ref(decoded);
        buffer
    }

    fn refine_position(&mut self, seek_res: SeekedTo) -> Result<(), TrackSeekError> {
        let mut samples_to_pass = seek_res.required_ts.saturating_sub(seek_res.actual_ts);
        let packet = loop {
            let candidate = self.format.next_packet().map_err(TrackSeekError::Refining)?;
            if candidate.dur() > samples_to_pass {
                break candidate;
            }
            samples_to_pass -= candidate.dur();
        };

        let mut decoded = self.decoder.decode(&packet);
        for _ in 0..MAX_DECODE_RETRIES {
            if decoded.is_err() {
                let packet = self.format.next_packet().map_err(TrackSeekError::Retrying)?;
                decoded = self.decoder.decode(&packet);
            } else {
                break;
            }
        }
        let decoded = decoded.map_err(TrackSeekError::Decoding)?;
        self.spec = *decoded.spec();
        self.buffer = Self::sample_buffer(decoded, &self.spec);
        self.current_frame_offset = samples_to_pass as usize * self.channels() as usize;
        Ok(())
    }
}

/// Nudges a `Time` back by `END_OF_TRACK_SEEK_EPSILON`, borrowing a whole
/// second (via `1.0 + frac`, not `1.0 - frac`) when the subtraction pushes
/// the fractional part negative — `Time::frac` must stay within `[0.0,
/// 1.0)` or symphonia's `TimeBase::calc_timestamp` panics.
fn time_just_before(duration: Time) -> Time {
    let mut t = duration;
    t.frac -= END_OF_TRACK_SEEK_EPSILON;
    if t.frac < 0.0 {
        t.seconds = t.seconds.saturating_sub(1);
        t.frac += 1.0;
    }
    t
}

impl Iterator for FileSource {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        if self.current_frame_offset >= self.buffer.len() {
            let packet = self.format.next_packet().ok()?;
            let mut decoded = self.decoder.decode(&packet);
            for _ in 0..MAX_DECODE_RETRIES {
                if decoded.is_err() {
                    let packet = self.format.next_packet().ok()?;
                    decoded = self.decoder.decode(&packet);
                } else {
                    break;
                }
            }
            let decoded = decoded.ok()?;
            self.spec = *decoded.spec();
            self.buffer = Self::sample_buffer(decoded, &self.spec);
            self.current_frame_offset = 0;
        }

        let sample = *self.buffer.samples().get(self.current_frame_offset)?;
        self.current_frame_offset += 1;
        Some(sample)
    }
}

impl Source for FileSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.buffer.samples().len())
    }

    fn channels(&self) -> u16 {
        self.spec.channels.count() as u16
    }

    fn sample_rate(&self) -> u32 {
        self.spec.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.total_duration.map(|Time { seconds, frac }| {
            Duration::new(seconds, (frac * 1_000_000_000.0).round() as u32)
        })
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        let seek_beyond_end = self
            .total_duration()
            .is_some_and(|dur| dur.saturating_sub(pos).as_millis() < 1);

        let time: Time = if seek_beyond_end {
            time_just_before(self.total_duration.expect("checked by seek_beyond_end above"))
        } else {
            pos.as_secs_f64().into()
        };

        let to_skip = self.current_frame_offset % self.channels() as usize;

        let seek_res = self
            .format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|e| SeekError::Other(Box::new(TrackSeekError::BaseSeek(e))))?;

        self.refine_position(seek_res)
            .map_err(|e| SeekError::Other(Box::new(e)))?;
        self.current_frame_offset += to_skip;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_valid_time(time: Time) {
        assert!(
            time.frac >= 0.0 && time.frac < 1.0,
            "Time::frac must stay in [0.0, 1.0), got {}",
            time.frac
        );
    }

    #[test]
    fn time_just_before_subtracts_the_epsilon_without_borrowing() {
        let result = time_just_before(Time::new(10, 0.5));
        assert_valid_time(result);
        assert_eq!(result.seconds, 10);
        assert!((result.frac - (0.5 - END_OF_TRACK_SEEK_EPSILON)).abs() < f64::EPSILON);
    }

    #[test]
    fn time_just_before_borrows_a_second_when_frac_underflows() {
        // A previous version computed `1.0 - t.frac` here on an already-negative
        // `t.frac`, producing e.g. 1.00005 and panicking deep in symphonia's
        // `TimeBase::calc_timestamp` ("Invalid range for Time fractional part").
        let result = time_just_before(Time::new(5, 0.00005));
        assert_valid_time(result);
        assert_eq!(result.seconds, 4);
        assert!((result.frac - 0.99995).abs() < 1e-9);
    }

    #[test]
    fn time_just_before_saturates_seconds_at_zero_for_sub_second_tracks() {
        let result = time_just_before(Time::new(0, 0.0));
        assert_valid_time(result);
        assert_eq!(result.seconds, 0);
    }
}
