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
    queue: Vec<AudioTrack>,
    queue_index: usize,
    is_playing: bool,
    is_paused: bool,
    start_time: Option<Instant>,
    elapsed_before_pause: Duration,
    volume: f32,
    shuffle_mode: bool,
    repeat_mode: u8, // 0 = off, 1 = one, 2 = all
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

        Arc::new(Mutex::new(Self {
            stream_handle,
            sink: None,
            current_track: None,
            queue: Vec::new(),
            queue_index: 0,
            is_playing: false,
            is_paused: false,
            start_time: None,
            elapsed_before_pause: Duration::ZERO,
            volume: 1.0,
            shuffle_mode: false,
            repeat_mode: 0,
        }))
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
            self.queue = queue;
            self.queue_index = start_index;
        }
        Ok(())
    }

    pub fn toggle_play_pause(&mut self) {
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
        if self.queue.is_empty() {
            return Ok(());
        }

        if self.shuffle_mode {
            let rng_val = Instant::now().elapsed().subsec_nanos() as usize;
            self.queue_index = rng_val % self.queue.len();
        } else if self.queue_index + 1 < self.queue.len() {
            self.queue_index += 1;
        } else if self.repeat_mode == 2 {
            self.queue_index = 0;
        } else {
            return Ok(());
        }

        if let Some(track) = self.queue.get(self.queue_index).cloned() {
            self.play_track(track, 0)?;
        }
        Ok(())
    }

    pub fn play_previous(&mut self) -> Result<(), String> {
        if self.queue.is_empty() {
            return Ok(());
        }

        let current_pos = self.get_position_ms();
        if current_pos > 3000 {
            self.seek(0);
            return Ok(());
        }

        if self.queue_index > 0 {
            self.queue_index -= 1;
        } else if self.repeat_mode == 2 {
            self.queue_index = self.queue.len().saturating_sub(1);
        }

        if let Some(track) = self.queue.get(self.queue_index).cloned() {
            self.play_track(track, 0)?;
        }
        Ok(())
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle_mode = !self.shuffle_mode;
    }

    pub fn toggle_repeat(&mut self) {
        self.repeat_mode = (self.repeat_mode + 1) % 3;
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
            shuffle_mode: self.shuffle_mode,
            repeat_mode: self.repeat_mode,
            queue: self.queue.clone(),
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
        for q_item in &mut self.queue {
            if let Some(updated) = updated_tracks.iter().find(|u| u.file_path == q_item.file_path) {
                *q_item = updated.clone();
            }
        }

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
