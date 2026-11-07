use crate::models::{AudioTrack, PlaybackStateDto};
use crate::queue::{PlayQueue, REPEAT_ONE};
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{Decoder as SymphDecoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::{Duration as SymphDuration, Time};

const MAX_DECODE_RETRIES: usize = 64;

/// Reads an MP4 file as if its `udta` atom were a `free` one, leaving the file itself alone.
/// Some taggers leave bytes in `udta` that make symphonia give up on the whole file. Playback
/// does not need that atom: the tags are read with lofty.
struct Mp4WithoutUserData {
    file: File,
    position: u64,
    name_position: u64,
    len: u64,
}

impl Mp4WithoutUserData {
    fn open(path: &Path) -> Option<Self> {
        let mut file = File::open(path).ok()?;
        let name_position = find_user_data_atom(&mut file).ok()??;
        let len = file.metadata().ok()?.len();
        file.seek(SeekFrom::Start(0)).ok()?;
        Some(Self {
            file,
            position: 0,
            name_position,
            len,
        })
    }
}

impl Read for Mp4WithoutUserData {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.file.read(buf)?;
        for (offset, byte) in buf[..read].iter_mut().enumerate() {
            let position = self.position + offset as u64;
            if position >= self.name_position && position < self.name_position + 4 {
                *byte = b"free"[(position - self.name_position) as usize];
            }
        }
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for Mp4WithoutUserData {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.position = self.file.seek(pos)?;
        Ok(self.position)
    }
}

impl MediaSource for Mp4WithoutUserData {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

/// Protected MP4 audio is marked by the codec name in `moov/trak/mdia/minf/stbl/stsd`.
fn is_drm_protected(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    sample_entry_name(&mut file)
        .ok()
        .flatten()
        .is_some_and(|name| matches!(&name, b"drms" | b"enca"))
}

fn sample_entry_name<R: Read + Seek>(reader: &mut R) -> std::io::Result<Option<[u8; 4]>> {
    let end = reader.seek(SeekFrom::End(0))?;
    let (mut start, mut stop) = (0, end);
    for name in [b"moov", b"trak", b"mdia", b"minf", b"stbl", b"stsd"] {
        let Some(atom) = find_atom(reader, start, stop, name)? else {
            return Ok(None);
        };
        (start, stop) = (atom.body_start, atom.end);
    }
    // stsd: version and flags, entry count, then the entries, each with a length and a name
    let name_position = start + 8 + 4;
    if name_position + 4 > stop {
        return Ok(None);
    }
    reader.seek(SeekFrom::Start(name_position))?;
    let mut name = [0u8; 4];
    reader.read_exact(&mut name)?;
    Ok(Some(name))
}

/// Where the name of `moov/udta` is, if the file has one.
fn find_user_data_atom<R: Read + Seek>(reader: &mut R) -> std::io::Result<Option<u64>> {
    let end = reader.seek(SeekFrom::End(0))?;
    let Some(moov) = find_atom(reader, 0, end, b"moov")? else {
        return Ok(None);
    };
    let udta = find_atom(reader, moov.body_start, moov.end, b"udta")?;
    Ok(udta.map(|atom| atom.name_position))
}

struct AtomBounds {
    name_position: u64,
    body_start: u64,
    end: u64,
}

/// The first atom called `name` between `start` and `end`.
fn find_atom<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    end: u64,
    name: &[u8; 4],
) -> std::io::Result<Option<AtomBounds>> {
    let mut position = start;
    while position + 8 <= end {
        reader.seek(SeekFrom::Start(position))?;
        let mut header = [0u8; 8];
        reader.read_exact(&mut header)?;

        let mut len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as u64;
        let mut header_len = 8;
        if len == 1 {
            let mut long_len = [0u8; 8];
            reader.read_exact(&mut long_len)?;
            len = u64::from_be_bytes(long_len);
            header_len = 16;
        } else if len == 0 {
            len = end - position;
        }
        if len < header_len || position + len > end {
            return Ok(None);
        }

        if &header[4..8] == name {
            return Ok(Some(AtomBounds {
                name_position: position + 4,
                body_start: position + header_len,
                end: position + len,
            }));
        }
        position += len;
    }
    Ok(None)
}

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
        Self::try_open(path, None).map_err(|e| {
            if is_drm_protected(path) {
                "DRM で保護された曲は再生できません".to_string()
            } else {
                e
            }
        })
    }

    fn try_open(path: &Path, ext_hint: Option<&str>) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("ファイルを開けません: {}", e))?;
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

        let mut format: Box<dyn FormatReader> = match symphonia::default::get_probe().format(
            &hint,
            mss,
            &format_opts,
            &metadata_opts,
        ) {
            Ok(probed) => probed.format,
            Err(_) => {
                let without_user_data = Mp4WithoutUserData::open(path).and_then(|source| {
                    let mss = MediaSourceStream::new(Box::new(source), Default::default());
                    symphonia::default::get_probe()
                        .format(&hint, mss, &format_opts, &metadata_opts)
                        .ok()
                });
                if let Some(probed) = without_user_data {
                    probed.format
                } else {
                    // Fallback for RIFF-wrapped MP3 files (RMP3 / WAVE_FORMAT_MPEGLAYER3 0x0055)
                    let file2 =
                        File::open(path).map_err(|e| format!("ファイルを開けません: {}", e))?;
                    let mss2 = MediaSourceStream::new(Box::new(file2), Default::default());
                    match symphonia::default::formats::MpaReader::try_new(mss2, &format_opts) {
                        Ok(mpa) => Box::new(mpa),
                        Err(_) => {
                            return Err("対応していない形式か、ファイルが壊れています".to_string())
                        }
                    }
                }
            }
        };

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| "再生できる音声がありません".to_string())?;

        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        let total_duration =
            codec_params
                .time_base
                .zip(codec_params.n_frames)
                .map(|(base, frames)| {
                    let Time { seconds, frac } = base.calc_time(frames);
                    let nanos = (frac.clamp(0.0, 0.999_999_999) * 1_000_000_000.0) as u32;
                    Duration::new(seconds, nanos)
                });

        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .map_err(|_| "この音声形式には対応していません".to_string())?;

        let mut decode_errors = 0;
        let decoded = loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(_) => {
                    return Err(
                        "音声を読み取れません。ファイルが壊れている可能性があります".to_string()
                    )
                }
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => break decoded,
                Err(SymphError::DecodeError(_)) | Err(SymphError::ResetRequired) => {
                    decode_errors += 1;
                    if decode_errors > MAX_DECODE_RETRIES {
                        return Err("音声を読み取れません。ファイルが壊れている可能性があります"
                            .to_string());
                    }
                    continue;
                }
                Err(_) => {
                    return Err(
                        "音声を読み取れません。ファイルが壊れている可能性があります".to_string()
                    )
                }
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
    failed_in_a_row: usize,
}

pub type SharedPlayer = Arc<Mutex<AudioPlayer>>;

impl AudioPlayer {
    /// A player that has been playing `track` for `elapsed`, with no audio device behind it.
    #[cfg(test)]
    pub(crate) fn playing_for_tests(track: AudioTrack, elapsed: Duration) -> Self {
        let mut player = Self::with_output(None);
        player.current_track = Some(track.clone());
        player.queue.set_tracks(vec![track], 0);
        let (sink, _queue) = Sink::new_idle();
        player.sink = Some(sink);
        player.is_playing = true;
        player.start_time = Instant::now().checked_sub(elapsed);
        player
    }

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
            failed_in_a_row: 0,
        }
    }

    pub fn play_track(
        &mut self,
        mut track: AudioTrack,
        initial_position_ms: u64,
    ) -> Result<(), String> {
        let handle = self
            .stream_handle
            .as_ref()
            .ok_or("音声の出力先が見つかりません")?;

        // Open and decode the new track BEFORE stopping the current sink
        let mut source = SymphoniaSource::from_path(Path::new(&track.file_path))
            .map_err(|e| format!("{}: {}", track.file_name, e))?;
        if track.duration_ms == 0 {
            if let Some(dur) = source.total_duration() {
                track.duration_ms = dur.as_millis() as u64;
            }
        }
        if initial_position_ms > 0 {
            let _ = source.try_seek(Duration::from_millis(initial_position_ms));
        }

        let sink =
            Sink::try_new(handle).map_err(|e| format!("音声の出力を開始できません: {}", e))?;
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
        shuffle: Option<bool>,
    ) -> Result<(), String> {
        if let Some(track) = queue.get(start_index).cloned() {
            self.play_track(track, initial_pos_ms)?;
            if let Some(shuffle) = shuffle {
                self.queue.set_shuffle(shuffle);
            }
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
        if result.is_ok() {
            self.failed_in_a_row = 0;
        } else {
            self.failed_in_a_row += 1;
            // Otherwise repeat would keep failing on every tick, forever
            let every_track_failed = self.failed_in_a_row >= self.queue.tracks().len();
            if self.queue.repeat_mode() == REPEAT_ONE || every_track_failed {
                self.finish_playback();
            }
        }
        result
    }

    fn finish_playback(&mut self) {
        self.failed_in_a_row = 0;
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
        let duration_ms = self
            .current_track
            .as_ref()
            .map(|t| t.duration_ms)
            .unwrap_or(0);
        let pos_ms = self.get_position_ms().min(duration_ms);

        PlaybackStateDto {
            current_track: self.current_track.clone(),
            is_playing: self.is_playing,
            position_ms: pos_ms,
            duration_ms,
            volume: self.volume,
            shuffle_mode: self.queue.shuffle(),
            repeat_mode: self.queue.repeat_mode(),
            queue_version: self.queue.version(),
        }
    }

    pub fn queue_tracks(&self) -> Vec<AudioTrack> {
        self.queue.tracks().to_vec()
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
    use crate::test_audio::{aiff_bytes, pcm_samples, wav_bytes};
    use std::fs;
    use std::io::Cursor;

    fn decoded_sample_count(file_name: &str, bytes: &[u8]) -> usize {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(file_name);
        fs::write(&path, bytes).unwrap();
        SymphoniaSource::from_path(&path).unwrap().count()
    }

    fn atom(name: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut bytes = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(body);
        bytes
    }

    fn mp4_bytes() -> Vec<u8> {
        let moov_body = [atom(b"mvhd", &[1; 20]), atom(b"udta", &[2; 30])].concat();
        [
            atom(b"ftyp", b"M4A "),
            atom(b"moov", &moov_body),
            atom(b"mdat", &[3; 40]),
        ]
        .concat()
    }

    #[test]
    fn finds_the_user_data_atom() {
        let bytes = mp4_bytes();
        let position = find_user_data_atom(&mut Cursor::new(&bytes))
            .unwrap()
            .unwrap() as usize;
        assert_eq!(&bytes[position..position + 4], b"udta");
    }

    #[test]
    fn finds_the_user_data_atom_after_a_64_bit_atom() {
        let mut long = 1u32.to_be_bytes().to_vec();
        long.extend_from_slice(b"mdat");
        long.extend_from_slice(&26u64.to_be_bytes());
        long.extend_from_slice(&[3; 10]);
        let bytes = [long, atom(b"moov", &atom(b"udta", &[2; 4]))].concat();

        let position = find_user_data_atom(&mut Cursor::new(&bytes))
            .unwrap()
            .unwrap() as usize;
        assert_eq!(&bytes[position..position + 4], b"udta");
    }

    #[test]
    fn finds_the_name_of_a_user_data_atom_with_a_64_bit_length() {
        let mut udta = 1u32.to_be_bytes().to_vec();
        udta.extend_from_slice(b"udta");
        udta.extend_from_slice(&20u64.to_be_bytes());
        udta.extend_from_slice(&[2; 4]);
        let bytes = atom(b"moov", &udta);

        let position = find_user_data_atom(&mut Cursor::new(&bytes))
            .unwrap()
            .unwrap() as usize;
        assert_eq!(&bytes[position..position + 4], b"udta");
    }

    #[test]
    fn files_without_the_atom_or_with_broken_lengths_are_left_alone() {
        for bytes in [
            wav_bytes(),
            atom(b"moov", &atom(b"mvhd", &[1; 8])),
            [0, 0, 0, 3, b'm', b'o', b'o', b'v'].to_vec(),
            [0xFF, 0xFF, 0xFF, 0xFF, b'm', b'o', b'o', b'v'].to_vec(),
            Vec::new(),
        ] {
            assert_eq!(find_user_data_atom(&mut Cursor::new(&bytes)).unwrap(), None);
        }
    }

    #[test]
    fn the_atom_reads_as_free_however_the_reads_are_split_and_the_file_is_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.m4a");
        let original = mp4_bytes();
        fs::write(&path, &original).unwrap();
        let name = find_user_data_atom(&mut Cursor::new(&original))
            .unwrap()
            .unwrap() as usize;
        let mut expected = original.clone();
        expected[name..name + 4].copy_from_slice(b"free");

        for chunk in [1, 2, 3, 7, 1000] {
            let mut source = Mp4WithoutUserData::open(&path).unwrap();
            let mut seen = Vec::new();
            let mut buf = vec![0; chunk];
            loop {
                let read = source.read(&mut buf).unwrap();
                if read == 0 {
                    break;
                }
                seen.extend_from_slice(&buf[..read]);
            }
            assert_eq!(seen, expected, "chunk {chunk}");
        }

        let mut source = Mp4WithoutUserData::open(&path).unwrap();
        source.seek(SeekFrom::Start(name as u64 + 2)).unwrap();
        let mut rest = [0; 2];
        source.read_exact(&mut rest).unwrap();
        assert_eq!(&rest, b"ee");
        assert_eq!(fs::read(&path).unwrap(), original);
    }

    #[test]
    fn decodes_wav() {
        assert_eq!(
            decoded_sample_count("a.wav", &wav_bytes()),
            pcm_samples().len()
        );
    }

    #[test]
    fn decodes_aiff() {
        assert_eq!(
            decoded_sample_count("a.aiff", &aiff_bytes()),
            pcm_samples().len()
        );
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
    fn repeat_all_stops_once_every_track_has_failed() {
        let mut player = player_at_track_end(3);
        player.toggle_repeat();

        assert!(player.handle_track_end().is_err());
        assert!(
            player.check_track_end(),
            "keeps trying the rest of the queue"
        );
        assert!(player.handle_track_end().is_err());
        assert!(
            player.check_track_end(),
            "keeps trying the rest of the queue"
        );
        assert!(player.handle_track_end().is_err());

        assert!(!player.check_track_end());
        assert!(!player.get_state_dto().is_playing);
    }

    #[test]
    fn a_broken_file_is_described_and_the_message_names_the_track() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.wav");
        fs::write(&path, b"not audio").unwrap();

        let message = SymphoniaSource::from_path(&path).err().unwrap();
        assert!(message.contains("壊れて"), "{message}");

        let mut player = AudioPlayer::with_output(None);
        // The output is checked first, so this reports the missing device
        let message = player.play_track(track("a.mp3"), 0).unwrap_err();
        assert_eq!(message, "音声の出力先が見つかりません");
    }

    fn mp4_with_codec(codec: &[u8; 4]) -> Vec<u8> {
        let mut stsd_body = vec![0; 4];
        stsd_body.extend_from_slice(&1u32.to_be_bytes());
        stsd_body.extend_from_slice(&atom(codec, &[0; 8]));
        let stsd = atom(b"stsd", &stsd_body);
        let mdia = atom(b"mdia", &atom(b"minf", &atom(b"stbl", &stsd)));
        [
            atom(b"ftyp", b"M4A "),
            atom(b"moov", &atom(b"trak", &mdia)),
            atom(b"mdat", &[3; 40]),
        ]
        .concat()
    }

    #[test]
    fn protected_audio_gets_its_own_message() {
        for (codec, protected) in [(b"drms", true), (b"enca", true), (b"mp4a", false)] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("a.m4a");
            fs::write(&path, mp4_with_codec(codec)).unwrap();

            assert_eq!(is_drm_protected(&path), protected);
            let message = SymphoniaSource::from_path(&path).err().unwrap();
            assert_eq!(message.contains("DRM"), protected, "{message}");
        }
    }

    #[test]
    fn toggling_after_the_end_plays_the_last_track_again() {
        let mut player = player_at_track_end(1);
        player.handle_track_end().unwrap();

        // No output device here, so starting again reports that instead of doing nothing
        assert!(player.toggle_play_pause().is_err());
    }
}
