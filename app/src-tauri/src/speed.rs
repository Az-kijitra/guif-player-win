//! Playback speed without a change of pitch (WSOLA, from the wsola crate).
//! At normal speed the samples pass through untouched.

use std::num::{NonZeroU16, NonZeroU32};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rodio::Source;
use wsola::TimeStretch;

/// 0.8 to 2.0 in steps of 0.2, as in the Android version.
pub const PLAYBACK_SPEEDS: [f32; 7] = [0.8, 1.0, 1.2, 1.4, 1.6, 1.8, 2.0];

/// The choice nearest to `speed`: a saved value from another version still makes sense.
pub fn normalize_speed(speed: f32) -> f32 {
    if !speed.is_finite() {
        return 1.0;
    }
    PLAYBACK_SPEEDS
        .iter()
        .copied()
        .min_by(|a, b| (a - speed).abs().total_cmp(&(b - speed).abs()))
        .unwrap_or(1.0)
}

/// The speed shared by the player and the track playing, so a change applies at once.
#[derive(Clone)]
pub struct SharedSpeed(Arc<AtomicU32>);

impl SharedSpeed {
    pub fn new(speed: f32) -> Self {
        Self(Arc::new(AtomicU32::new(speed.to_bits())))
    }

    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }

    pub fn set(&self, speed: f32) {
        self.0.store(speed.to_bits(), Ordering::Relaxed);
    }
}

/// Frames handed to the stretcher at a time
const CHUNK_FRAMES: usize = 2048;

pub struct SpeedSource<S: Source<Item = f32>> {
    inner: S,
    speed: SharedSpeed,
    // None until the speed first leaves 1.0; from then on this track keeps going through it
    stretch: Option<TimeStretch>,
    out: Vec<f32>,
    out_pos: usize,
    input_done: bool,
    // Samples passed through untouched, to switch over only at the start of a frame
    passed: usize,
}

impl<S: Source<Item = f32>> SpeedSource<S> {
    pub fn new(inner: S, speed: SharedSpeed) -> Self {
        Self {
            inner,
            speed,
            stretch: None,
            out: Vec::new(),
            out_pos: 0,
            input_done: false,
            passed: 0,
        }
    }

    fn new_stretch(&self) -> Option<TimeStretch> {
        TimeStretch::new(
            NonZeroU32::new(self.inner.sample_rate())?,
            NonZeroU16::new(self.inner.channels())?,
        )
        .ok()
    }

    /// Fills `out` from the stretcher. False at the end of the track.
    fn refill(&mut self) -> bool {
        let speed = self.speed.get();
        let channels = self.inner.channels().max(1) as usize;
        let Some(stretch) = self.stretch.as_mut() else {
            return false;
        };
        stretch.set_tempo(speed);
        self.out.resize(stretch.min_output_buff_size() * 4, 0.0);
        self.out_pos = 0;
        loop {
            match stretch.pull(&mut self.out) {
                Ok(written) => {
                    self.out.truncate(written);
                    return written > 0;
                }
                Err(wsola::Error::NotEnoughInput) => {
                    if self.input_done {
                        self.out.clear();
                        return false;
                    }
                    let chunk: Vec<f32> =
                        self.inner.by_ref().take(CHUNK_FRAMES * channels).collect();
                    let whole = chunk.len() - chunk.len() % channels;
                    stretch.push(&chunk[..whole]);
                    if chunk.len() < CHUNK_FRAMES * channels {
                        self.input_done = true;
                        stretch.set_draining();
                    }
                }
                Err(_) => {
                    self.out.clear();
                    return false;
                }
            }
        }
    }
}

impl<S: Source<Item = f32>> Iterator for SpeedSource<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        loop {
            if let Some(sample) = self.out.get(self.out_pos) {
                self.out_pos += 1;
                return Some(*sample);
            }
            if self.stretch.is_none() {
                let at_frame_start = self
                    .passed
                    .is_multiple_of(self.inner.channels().max(1) as usize);
                if self.speed.get() == 1.0 || !at_frame_start {
                    self.passed += 1;
                    return self.inner.next();
                }
                self.stretch = self.new_stretch();
                if self.stretch.is_none() {
                    return self.inner.next();
                }
            }
            if !self.refill() {
                return None;
            }
        }
    }
}

impl<S: Source<Item = f32>> Source for SpeedSource<S> {
    fn current_frame_len(&self) -> Option<usize> {
        None
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

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.inner.try_seek(pos)?;
        // What was stretched before the seek must not be heard after it
        self.out.clear();
        self.out_pos = 0;
        self.input_done = false;
        self.passed = 0;
        if self.stretch.is_some() {
            self.stretch = self.new_stretch();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::buffer::SamplesBuffer;

    const RATE: u32 = 44_100;

    fn tone(seconds: f32, channels: u16) -> Vec<f32> {
        let frames = (RATE as f32 * seconds) as usize;
        (0..frames)
            .flat_map(|i| {
                let v = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / RATE as f32).sin() * 0.5;
                std::iter::repeat_n(v, channels as usize)
            })
            .collect()
    }

    fn played(samples: Vec<f32>, channels: u16, speed: f32) -> Vec<f32> {
        let source = SamplesBuffer::new(channels, RATE, samples);
        SpeedSource::new(source, SharedSpeed::new(speed)).collect()
    }

    fn pitch_hz(samples: &[f32], channels: usize) -> f32 {
        let mono: Vec<f32> = samples.iter().step_by(channels).copied().collect();
        let body = &mono[mono.len() / 10..mono.len() * 9 / 10];
        let crossings = body
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        crossings as f32 / (body.len() as f32 / RATE as f32)
    }

    #[test]
    fn speeds_go_from_point_eight_to_two_in_steps_of_point_two() {
        assert_eq!(PLAYBACK_SPEEDS, [0.8, 1.0, 1.2, 1.4, 1.6, 1.8, 2.0]);
    }

    #[test]
    fn saved_speed_outside_the_choices_becomes_the_nearest_choice() {
        assert_eq!(normalize_speed(1.45), 1.4);
        assert_eq!(normalize_speed(1.5), 1.4);
        assert_eq!(normalize_speed(3.0), 2.0);
        assert_eq!(normalize_speed(0.1), 0.8);
        assert_eq!(normalize_speed(f32::NAN), 1.0);
    }

    #[test]
    fn normal_speed_passes_the_samples_through_untouched() {
        let input = tone(0.5, 2);
        assert_eq!(played(input.clone(), 2, 1.0), input);
    }

    #[test]
    fn a_track_played_at_double_speed_finishes_in_about_half_the_time() {
        for (speed, channels) in [(2.0, 2), (1.4, 1), (0.8, 2)] {
            let input = tone(3.0, channels);
            let output = played(input.clone(), channels, speed);
            let ratio = output.len() as f32 / input.len() as f32;
            assert!((ratio - 1.0 / speed).abs() < 0.02, "{speed}x: {ratio}");
            assert_eq!(output.len() % channels as usize, 0, "whole frames");
        }
    }

    #[test]
    fn the_pitch_stays_the_same_at_double_speed() {
        let output = played(tone(3.0, 2), 2, 2.0);
        let hz = pitch_hz(&output, 2);
        assert!((hz - 440.0).abs() < 3.0, "{hz}");
    }

    #[test]
    fn changing_the_speed_mid_track_takes_effect() {
        let speed = SharedSpeed::new(1.0);
        let source = SamplesBuffer::new(2, RATE, tone(4.0, 2));
        let mut playing = SpeedSource::new(source, speed.clone());
        let first: Vec<f32> = playing.by_ref().take(RATE as usize * 2).collect();
        assert_eq!(first.len(), RATE as usize * 2, "one second at normal speed");

        speed.set(2.0);
        let rest = playing.count();
        // three seconds of input left, played twice as fast
        let seconds = rest as f32 / 2.0 / RATE as f32;
        assert!((seconds - 1.5).abs() < 0.1, "{seconds}");
    }

    #[test]
    fn seeking_drops_what_was_stretched_before() {
        let speed = SharedSpeed::new(2.0);
        let source = SamplesBuffer::new(1, RATE, tone(4.0, 1));
        let mut playing = SpeedSource::new(source, speed);
        let _: Vec<f32> = playing.by_ref().take(RATE as usize / 2).collect();

        playing.try_seek(Duration::from_secs(3)).unwrap();

        let rest = playing.count();
        let seconds = rest as f32 / RATE as f32;
        assert!(
            (seconds - 0.5).abs() < 0.1,
            "the last second, at double speed: {seconds}"
        );
    }
}
