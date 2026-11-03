use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{Decoder as SymphDecoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::{Duration as SymphDuration, Time};
use crate::models::{AudioTrack, PlaybackStateDto};
use crate::queue::{PlayQueue, REPEAT_ONE};

const MAX_DECODE_RETRIES: usize = 64;

pub struct SymphoniaSource {
    decoder: Box<dyn SymphDecoder>,
    current_frame_offset: usize,
    format: Box<dyn FormatReader>,
    total_duration: Option<Duration>,
    buffer: SampleBuffer<f32>,
    spec: SignalSpec,
    track_id: u32,
}

impl SymphoniaSource {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());

        // Try with file extension hint first, then fallback to no hint
        if let Some(ref ext_str) = ext {
            if let Ok(src) = Self::try_open(path, Some(ext_str)) {
                return Ok(src);
            }
        }
        Self::try_open(path, None)
    }

    fn try_open(path: &Path, ext_hint: Option<&str>) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = ext_hint {
            hint.with_extension(ext);
        }

        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let metadata_opts = MetadataOptions::default();

        let mut format: Box<dyn FormatReader> = match symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
        {
            Ok(probed) => probed.format,
            Err(probe_err) => {
                // Fallback for RIFF-wrapped MP3 files (RMP3 / WAVE_FORMAT_MPEGLAYER3 0x0055)
                let file2 = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
                let mss2 = MediaSourceStream::new(Box::new(file2), Default::default());
                match symphonia::default::formats::MpaReader::try_new(mss2, &format_opts) {
                    Ok(mpa) => Box::new(mpa),
                    Err(_) => return Err(format!("Failed to probe format: {}", probe_err)),
                }
            }
        };

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| "No supported audio track found".to_string())?;

        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        let total_duration = codec_params
            .time_base
            .zip(codec_params.n_frames)
            .map(|(base, frames)| {
                let Time { seconds, frac } = base.calc_time(frames);
                let nanos = (frac.clamp(0.0, 0.999_999_999) * 1_000_000_000.0) as u32;
                Duration::new(seconds, nanos)
            });

        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Failed to create decoder: {}", e))?;

        let mut decode_errors = 0;
        let decoded = loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(e) => return Err(format!("Failed to read initial audio packet: {}", e)),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => break decoded,
                Err(SymphError::DecodeError(_)) | Err(SymphError::ResetRequired) => {
                    decode_errors += 1;
                    if decode_errors > MAX_DECODE_RETRIES {
                        return Err("Too many initial decode errors".to_string());
                    }
                    continue;
                }
                Err(e) => return Err(format!("Decode error: {}", e)),
            }
        };

        let spec = *decoded.spec();
        let buffer = Self::make_buffer(decoded, &spec);

        Ok(Self {
            decoder,
            current_frame_offset: 0,
            format,
            total_duration,
            buffer,
            spec,
            track_id,
        })
    }

    #[inline]
    fn make_buffer(
        decoded: symphonia::core::audio::AudioBufferRef,
        spec: &SignalSpec,
    ) -> SampleBuffer<f32> {
        let duration = SymphDuration::from(decoded.capacity() as u64);
        let mut buffer = SampleBuffer::<f32>::new(duration, *spec);
        buffer.copy_interleaved_ref(decoded);
        buffer
    }
}

impl Iterator for SymphoniaSource {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        while self.current_frame_offset >= self.buffer.len() {
            let mut decode_errors = 0;
            let decoded = loop {
                let packet = self.format.next_packet().ok()?;
                if packet.track_id() != self.track_id {
                    continue;
                }
                match self.decoder.decode(&packet) {
                    Ok(d) => break d,
                    Err(SymphError::DecodeError(_)) | Err(SymphError::ResetRequired) => {
                        decode_errors += 1;
                        if decode_errors > MAX_DECODE_RETRIES {
                            return None;
                        }
                        continue;
                    }
                    Err(_) => return None,
                }
            };
            decoded.spec().clone_into(&mut self.spec);
            self.buffer = Self::make_buffer(decoded, &self.spec);
            self.current_frame_offset = 0;
        }

        let sample = *self.buffer.samples().get(self.current_frame_offset)?;
        self.current_frame_offset += 1;
        Some(sample)
    }
}

impl Source for SymphoniaSource {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.spec.channels.count().max(1) as u16
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.spec.rate.max(1)
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        let target_time = Time::from(pos.as_secs_f64());
        let to_skip = if self.channels() > 0 {
            self.current_frame_offset % self.channels() as usize
        } else {
            0
        };

        let _ = self.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: target_time,
                track_id: Some(self.track_id),
            },
        );
        self.decoder.reset();
        self.current_frame_offset = self.buffer.len();

        // Advance to next valid decoded frame and align channel phase
        for _ in 0..to_skip {
            let _ = self.next();
        }
        Ok(())
    }
}

pub struct AudioPlayer {
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    current_track: Option<AudioTrack>,
    queue: PlayQueue,
    is_playing: bool,
    is_paused: bool,
    start_time: Option<Instant>,
    elapsed_before_pause: Duration,
    volume: f32,
}

pub type SharedPlayer = Arc<Mutex<AudioPlayer>>;

impl AudioPlayer {
    pub fn new() -> SharedPlayer {
        let stream_handle = match OutputStream::try_default() {
            Ok((stream, handle)) => {
                // Keep audio output stream alive for application lifetime
                std::mem::forget(stream);
                Some(handle)
            }
            Err(e) => {
                eprintln!("Failed to initialize audio output stream: {}", e);
                None
            }
        };

        Arc::new(Mutex::new(Self::with_output(stream_handle)))
    }

    fn with_output(stream_handle: Option<OutputStreamHandle>) -> Self {
        Self {
            stream_handle,
            sink: None,
            current_track: None,
            queue: PlayQueue::default(),
            is_playing: false,
            is_paused: false,
            start_time: None,
            elapsed_before_pause: Duration::ZERO,
            volume: 1.0,
        }
    }

    pub fn play_track(&mut self, mut track: AudioTrack, initial_position_ms: u64) -> Result<(), String> {
        let handle = self.stream_handle.as_ref().ok_or("No audio output device available")?;

        // Open and decode the new track BEFORE stopping the current sink
        let mut source = SymphoniaSource::from_path(Path::new(&track.file_path))?;
        if track.duration_ms == 0 {
            if let Some(dur) = source.total_duration() {
                track.duration_ms = dur.as_millis() as u64;
            }
        }
        if initial_position_ms > 0 {
            let _ = source.try_seek(Duration::from_millis(initial_position_ms));
        }

        let sink = Sink::try_new(handle).map_err(|e| format!("Failed to create sink: {}", e))?;
        sink.set_volume(self.volume);
        sink.append(source);

        // Now safely stop the old sink
        if let Some(s) = self.sink.take() {
            s.stop();
        }

        if initial_position_ms > 0 {
            self.elapsed_before_pause = Duration::from_millis(initial_position_ms);
        } else {
            self.elapsed_before_pause = Duration::ZERO;
        }

        self.sink = Some(sink);
        self.current_track = Some(track);
        self.is_playing = true;
        self.is_paused = false;
        self.start_time = Some(Instant::now());

        Ok(())
    }

    pub fn set_queue(
        &mut self,
        queue: Vec<AudioTrack>,
        start_index: usize,
        initial_pos_ms: u64,
    ) -> Result<(), String> {
        if let Some(track) = queue.get(start_index).cloned() {
            self.play_track(track, initial_pos_ms)?;
            self.queue.set_tracks(queue, start_index);
        }
        Ok(())
    }

    pub fn toggle_play_pause(&mut self) -> Result<(), String> {
        if self.sink.is_none() && !self.is_paused {
            // Playback has finished: start the last track again
            if let Some(track) = self.current_track.clone() {
                self.play_track(track, 0)?;
            }
            return Ok(());
        }

        if let Some(sink) = &self.sink {
            if self.is_paused {
                sink.play();
                self.is_paused = false;
                self.is_playing = true;
                self.start_time = Some(Instant::now());
            } else if self.is_playing {
                sink.pause();
                if let Some(st) = self.start_time {
                    self.elapsed_before_pause += st.elapsed();
                }
                self.start_time = None;
                self.is_paused = true;
                self.is_playing = false;
            }
        }
        Ok(())
    }

    pub fn pause(&mut self) {
        if let Some(sink) = &self.sink {
            if self.is_playing && !self.is_paused {
                sink.pause();
                if let Some(st) = self.start_time {
                    self.elapsed_before_pause += st.elapsed();
                }
                self.start_time = None;
                self.is_paused = true;
                self.is_playing = false;
            }
        }
    }

    pub fn resume(&mut self) {
        if let Some(sink) = &self.sink {
            if self.is_paused {
                sink.play();
                self.is_paused = false;
                self.is_playing = true;
                self.start_time = Some(Instant::now());
            }
        }
    }

    pub fn seek(&mut self, position_ms: u64) {
        if let Some(sink) = &self.sink {
            let dur = Duration::from_millis(position_ms);
            let _ = sink.try_seek(dur);
            self.elapsed_before_pause = dur;
            if self.is_playing && !self.is_paused {
                self.start_time = Some(Instant::now());
            } else {
                self.start_time = None;
            }
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume);
        }
    }

    pub fn play_next(&mut self) -> Result<(), String> {
        if let Some(track) = self.queue.advance().cloned() {
            self.play_track(track, 0)?;
        }
        Ok(())
    }

    pub fn play_previous(&mut self) -> Result<(), String> {
        if self.queue.tracks().is_empty() {
            return Ok(());
        }

        let current_pos = self.get_position_ms();
        if current_pos > 3000 {
            self.seek(0);
            return Ok(());
        }

        if let Some(track) = self.queue.previous().cloned() {
            self.play_track(track, 0)?;
        }
        Ok(())
    }

    /// Decides what plays after the current track has run out.
    pub fn handle_track_end(&mut self) -> Result<(), String> {
        let next_track = if self.queue.repeat_mode() == REPEAT_ONE {
            self.current_track.clone()
        } else {
            self.queue.advance().cloned()
        };

        let Some(track) = next_track else {
            self.finish_playback();
            return Ok(());
        };

        let result = self.play_track(track, 0);
        if result.is_err() && self.queue.repeat_mode() == REPEAT_ONE {
            // The same file would fail again on every tick
            self.finish_playback();
        }
        result
    }

    fn finish_playback(&mut self) {
        self.sink = None;
        self.is_playing = false;
        self.is_paused = false;
        self.start_time = None;
        self.elapsed_before_pause = Duration::ZERO;
    }

    pub fn toggle_shuffle(&mut self) {
        self.queue.toggle_shuffle();
    }

    pub fn toggle_repeat(&mut self) {
        self.queue.toggle_repeat();
    }

    pub fn get_position_ms(&self) -> u64 {
        if let Some(st) = self.start_time {
            let running = st.elapsed();
            (self.elapsed_before_pause + running).as_millis() as u64
        } else {
            self.elapsed_before_pause.as_millis() as u64
        }
    }

    pub fn check_track_end(&mut self) -> bool {
        if let Some(sink) = &self.sink {
            if sink.empty() && self.is_playing {
                return true;
            }
        }
        false
    }

    pub fn get_state_dto(&self) -> PlaybackStateDto {
        let duration_ms = self.current_track.as_ref().map(|t| t.duration_ms).unwrap_or(0);
        let pos_ms = self.get_position_ms().min(duration_ms);

        PlaybackStateDto {
            current_track: self.current_track.clone(),
            is_playing: self.is_playing,
            position_ms: pos_ms,
            duration_ms,
            volume: self.volume,
            shuffle_mode: self.queue.shuffle(),
            repeat_mode: self.queue.repeat_mode(),
            queue: self.queue.tracks().to_vec(),
        }
    }

    pub fn suspend_if_playing(&mut self, paths: &[String]) -> Option<(AudioTrack, u64, bool)> {
        if let Some(ref cur) = self.current_track {
            if paths.iter().any(|p| p == &cur.file_path) {
                let pos = self.get_position_ms();
                let was_playing = self.is_playing && !self.is_paused;
                let cur_clone = cur.clone();
                if let Some(s) = self.sink.take() {
                    s.stop();
                }
                self.is_playing = false;
                self.is_paused = true;
                self.start_time = None;
                self.elapsed_before_pause = Duration::from_millis(pos);
                return Some((cur_clone, pos, was_playing));
            }
        }
        None
    }

    pub fn restore_after_tag_update(
        &mut self,
        updated_tracks: &[AudioTrack],
        suspended: Option<(AudioTrack, u64, bool)>,
    ) {
        // Update queue metadata
        self.queue.update_tracks(updated_tracks);

        // Update current_track metadata
        if let Some(ref mut cur) = self.current_track {
            if let Some(updated) = updated_tracks.iter().find(|u| u.file_path == cur.file_path) {
                *cur = updated.clone();
            }
        }

        // Restore playback if it was suspended
        if let Some((old_track, pos_ms, was_playing)) = suspended {
            let track_to_play = updated_tracks
                .iter()
                .find(|u| u.file_path == old_track.file_path)
                .cloned()
                .unwrap_or(old_track);

            if self.play_track(track_to_play, pos_ms).is_ok() && !was_playing {
                self.pause();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn pcm_samples() -> Vec<i16> {
        (0..4410).map(|i| ((i % 100) * 100) as i16).collect()
    }

    fn wav_bytes() -> Vec<u8> {
        let data: Vec<u8> = pcm_samples().iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
        bytes.extend_from_slice(&44100u32.to_le_bytes());
        bytes.extend_from_slice(&88200u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&data);
        bytes
    }

    fn aiff_bytes() -> Vec<u8> {
        let samples = pcm_samples();
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_be_bytes()).collect();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"FORM");
        bytes.extend_from_slice(&(4 + 26 + 16 + data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(b"AIFFCOMM");
        bytes.extend_from_slice(&18u32.to_be_bytes());
        bytes.extend_from_slice(&1u16.to_be_bytes()); // mono
        bytes.extend_from_slice(&(samples.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&16u16.to_be_bytes());
        // 44100 as an 80-bit extended float
        bytes.extend_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
        bytes.extend_from_slice(b"SSND");
        bytes.extend_from_slice(&(8 + data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&[0; 8]); // offset and block size
        bytes.extend_from_slice(&data);
        bytes
    }

    fn decoded_sample_count(file_name: &str, bytes: &[u8]) -> usize {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(file_name);
        fs::write(&path, bytes).unwrap();
        SymphoniaSource::from_path(&path).unwrap().count()
    }

    #[test]
    fn decodes_wav() {
        assert_eq!(decoded_sample_count("a.wav", &wav_bytes()), pcm_samples().len());
    }

    #[test]
    fn decodes_aiff() {
        assert_eq!(decoded_sample_count("a.aiff", &aiff_bytes()), pcm_samples().len());
    }

    fn track(file_path: &str) -> AudioTrack {
        AudioTrack {
            file_path: file_path.to_string(),
            file_name: file_path.to_string(),
            title: file_path.to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_ms: 1000,
            track_number: None,
            has_artwork: false,
        }
    }

    /// A player whose track has just run out: no audio device is needed.
    fn player_at_track_end(queue_len: usize) -> AudioPlayer {
        let mut player = AudioPlayer::with_output(None);
        let tracks: Vec<_> = (0..queue_len).map(|n| track(&format!("{n}.mp3"))).collect();
        player.current_track = tracks.first().cloned();
        player.queue.set_tracks(tracks, 0);
        let (sink, _queue) = Sink::new_idle();
        player.sink = Some(sink);
        player.is_playing = true;
        player.start_time = Some(Instant::now());
        player
    }

    #[test]
    fn last_track_ending_stops_playback_once() {
        let mut player = player_at_track_end(1);
        assert!(player.check_track_end());

        player.handle_track_end().unwrap();

        assert!(!player.check_track_end());
        let state = player.get_state_dto();
        assert!(!state.is_playing);
        assert_eq!(state.position_ms, 0);
        assert_eq!(state.current_track.unwrap().file_path, "0.mp3");
    }

    #[test]
    fn repeat_one_stops_when_the_track_cannot_be_played_again() {
        let mut player = player_at_track_end(2);
        player.toggle_repeat();

        assert!(player.handle_track_end().is_err());

        assert!(!player.check_track_end());
        assert!(!player.get_state_dto().is_playing);
    }

    #[test]
    fn a_track_that_cannot_be_played_leaves_the_rest_of_the_queue_to_try() {
        let mut player = player_at_track_end(2);

        assert!(player.handle_track_end().is_err());

        // The next tick moves on and finds the end of the queue
        assert!(player.check_track_end());
        player.handle_track_end().unwrap();
        assert!(!player.check_track_end());
    }

    #[test]
    fn toggling_after_the_end_plays_the_last_track_again() {
        let mut player = player_at_track_end(1);
        player.handle_track_end().unwrap();

        // No output device here, so starting again reports that instead of doing nothing
        assert!(player.toggle_play_pause().is_err());
    }
}
