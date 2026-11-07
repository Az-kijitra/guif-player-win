use crate::models::{AudioTrack, TagUpdateFailure, TagUpdateRequest, TagUpdateResult};
use crate::player::SymphoniaSource;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::{AudioFile, TaggedFile};
use lofty::id3::v2::{Frame, Id3v2Tag, Id3v2Version};
use lofty::mp4::{Ilst, Mp4File};
use lofty::mpeg::MpegFile;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::Tag;
use rodio::Source;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

// Tag writing works on "<name>.guif-player-tmp.<ext>" next to the original
const TAG_TEMP_MARKER: &str = ".guif-player-tmp";
// Writing tags must not change the length; some formats estimate it, so allow for that
const MAX_DURATION_DRIFT: Duration = Duration::from_secs(1);
const VERIFY_SECONDS: usize = 5;
// The frames lofty discards when it writes ID3v2.3
const ID3V24_ONLY_FRAMES: [&str; 14] = [
    "ASPI", "EQU2", "RVA2", "SEEK", "SIGN", "TDEN", "TDRL", "TDTG", "TMOO", "TPRO", "TSOA", "TSOP",
    "TSOT", "TSST",
];

pub fn is_supported_audio(path: &Path) -> bool {
    let is_temp_copy = path
        .file_stem()
        .is_some_and(|stem| stem.to_string_lossy().ends_with(TAG_TEMP_MARKER));
    // macOS leaves "._name" files holding file attributes, not audio, on shared drives
    let is_apple_double = path
        .file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with("._"));
    if is_temp_copy || is_apple_double {
        return false;
    }

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            "mp3" | "flac" | "m4a" | "aac" | "ogg" | "wav" | "aif" | "aiff"
        )
    } else {
        false
    }
}

pub fn parse_track(path: &Path) -> AudioTrack {
    let file_path = path.to_string_lossy().to_string();
    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let mut title = file_name.clone();
    let mut artist = "Unknown Artist".to_string();
    let mut album = "Unknown Album".to_string();
    let mut duration_ms = 0u64;
    let mut track_number = None;
    let mut has_artwork = false;

    if let Ok(tagged_file) = Probe::open(path).and_then(|p| p.read()) {
        let props = tagged_file.properties();
        duration_ms = props.duration().as_millis() as u64;

        if let Some(tag) = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())
        {
            if let Some(t) = tag.title() {
                if !t.trim().is_empty() {
                    title = t.trim().to_string();
                }
            }
            if let Some(a) = tag.artist() {
                if !a.trim().is_empty() {
                    artist = a.trim().to_string();
                }
            }
            if let Some(al) = tag.album() {
                if !al.trim().is_empty() {
                    album = al.trim().to_string();
                }
            }
            track_number = tag.track();
            has_artwork = !tag.pictures().is_empty();
        }
    }

    if !has_artwork {
        // Check if directory has cover image
        if let Some(parent) = path.parent() {
            has_artwork = find_folder_artwork(parent).is_some();
        }
    }

    AudioTrack {
        file_path,
        file_name,
        title,
        artist,
        album,
        duration_ms,
        track_number,
        has_artwork,
    }
}

pub fn extract_artwork(path: &Path) -> Option<String> {
    let (data, mime) = artwork_bytes(path)?;
    Some(format!("data:{};base64,{}", mime, BASE64.encode(data)))
}

/// The artwork of a track and its MIME type: the embedded picture, or else the folder's cover.
pub fn artwork_bytes(path: &Path) -> Option<(Vec<u8>, String)> {
    // 1. Try embedded tag
    if let Ok(tagged_file) = Probe::open(path).and_then(|p| p.read()) {
        if let Some(tag) = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())
        {
            if let Some(pic) = tag.pictures().first() {
                let mime = pic.mime_type().map(|m| m.as_str()).unwrap_or("image/jpeg");
                return Some((pic.data().to_vec(), mime.to_string()));
            }
        }
    }

    // 2. Try parent folder cover
    if let Some(parent) = path.parent() {
        if let Some(cover_path) = find_folder_artwork(parent) {
            if let Ok(data) = std::fs::read(&cover_path) {
                let ext = cover_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("jpeg")
                    .to_lowercase();
                let mime = match ext.as_str() {
                    "png" => "image/png",
                    "webp" => "image/webp",
                    _ => "image/jpeg",
                };
                return Some((data, mime.to_string()));
            }
        }
    }

    None
}

pub fn find_folder_artwork(dir: &Path) -> Option<std::path::PathBuf> {
    let candidate_names = [
        "cover.jpg",
        "cover.jpeg",
        "cover.png",
        "folder.jpg",
        "folder.jpeg",
        "folder.png",
        "front.jpg",
        "front.jpeg",
        "front.png",
        "artwork.jpg",
        "artwork.png",
    ];

    for name in candidate_names {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn parse_data_url(data_url: &str) -> Result<(MimeType, Vec<u8>), String> {
    let (header, b64_part) = data_url.split_once(',').ok_or("画像を読み取れません")?;

    let mime_type = match header {
        "data:image/jpeg;base64" => MimeType::Jpeg,
        "data:image/png;base64" => MimeType::Png,
        "data:image/gif;base64" => MimeType::Gif,
        _ => return Err("対応していない画像形式です。JPEG、PNG、GIF を選んでください".to_string()),
    };

    let bytes = BASE64
        .decode(b64_part.trim())
        .map_err(|e| format!("画像を読み取れません: {}", e))?;

    Ok((mime_type, bytes))
}

/// Writes tags file by file. A file that fails is reported and left exactly as it was,
/// and the rest are still written. `Err` means nothing was attempted.
pub fn update_tags(req: &TagUpdateRequest) -> Result<TagUpdateResult, String> {
    let parsed_artwork = if let Some(ref data_url) = req.artwork_data_url {
        Some(parse_data_url(data_url)?)
    } else {
        None
    };

    let mut result = TagUpdateResult {
        updated: Vec::new(),
        failed: Vec::new(),
    };

    for (index, file_path) in req.file_paths.iter().enumerate() {
        let path = Path::new(file_path);
        match update_file_tags(path, req, index, parsed_artwork.as_ref()) {
            Ok(()) => result.updated.push(parse_track(path)),
            Err(message) => result.failed.push(TagUpdateFailure {
                file_path: file_path.clone(),
                message,
            }),
        }
    }

    Ok(result)
}

fn read_tagged_file(path: &Path) -> Result<TaggedFile, String> {
    Probe::open(path)
        .and_then(|p| p.read())
        .map_err(|e| format!("音声ファイルとして読み取れません: {}", e))
}

fn update_file_tags(
    path: &Path,
    req: &TagUpdateRequest,
    index: usize,
    artwork: Option<&(MimeType, Vec<u8>)>,
) -> Result<(), String> {
    let file_info = fs::metadata(path).map_err(|e| format!("ファイルを開けません: {}", e))?;
    if file_info.permissions().readonly() {
        return Err("読み取り専用のファイルです".to_string());
    }

    // All writing happens on a copy, so a failure or a crash cannot damage the original
    let temp = TempCopy(temp_copy_path(path));
    fs::copy(path, &temp.0).map_err(|e| format!("一時コピーを作成できません: {}", e))?;

    let duration = read_tagged_file(&temp.0)?.properties().duration();
    let audio_before = decoded_start(&temp.0);

    let changes = TagChanges {
        path,
        req,
        index,
        artwork,
    };
    let extension = path.extension().map(|e| e.to_string_lossy().to_lowercase());
    match extension.as_deref() {
        Some("mp3") => write_mp3_tags(&temp.0, &changes),
        Some("m4a") => write_mp4_tags(&temp.0, &changes),
        _ => write_generic_tags(&temp.0, &changes),
    }
    .map_err(|e| format!("タグを書き込めません: {}", e))?;

    check_written_file(&temp.0, req, duration, audio_before.as_deref())?;

    replace_file(path, &temp.0).map_err(|e| format!("ファイルを置き換えられません: {}", e))
}

struct TagChanges<'a> {
    path: &'a Path,
    req: &'a TagUpdateRequest,
    index: usize,
    artwork: Option<&'a (MimeType, Vec<u8>)>,
}

impl TagChanges<'_> {
    fn apply_text<T: Accessor>(&self, tag: &mut T) {
        let req = self.req;

        // 1. Title
        if req.filename_to_title {
            if let Some(stem) = self.path.file_stem().and_then(|s| s.to_str()) {
                tag.set_title(stem.to_string());
            }
        } else if let Some(ref title) = req.title {
            tag.set_title(title.clone());
        }

        // 2. Artist
        if let Some(ref artist) = req.artist {
            tag.set_artist(artist.clone());
        }

        // 3. Album
        if let Some(ref album) = req.album {
            tag.set_album(album.clone());
        }

        // 4. Track Number
        if req.auto_number_tracks {
            tag.set_track((self.index + 1) as u32);
        } else if req.clear_track_number {
            tag.remove_track();
        } else if let Some(num) = req.track_number {
            tag.set_track(num);
        }
    }

    fn removes_artwork(&self) -> bool {
        self.req.clear_artwork || self.artwork.is_some()
    }

    fn new_artwork(&self) -> Option<Picture> {
        if self.req.clear_artwork {
            return None;
        }
        self.artwork.map(|(mime, bytes)| {
            Picture::unchecked(bytes.clone())
                .pic_type(PictureType::CoverFront)
                .mime_type(mime.clone())
                .build()
        })
    }
}

fn open_for_reading(path: &Path) -> Result<fs::File, String> {
    fs::File::open(path).map_err(|e| e.to_string())
}

// MP3 and M4A use their own tag types: going through the generic `Tag` loses what it has no
// mapping for (malformed track numbers, the other tags in the file) and raises ID3v2.3 to v2.4.
fn write_mp3_tags(path: &Path, changes: &TagChanges) -> Result<(), String> {
    let mut file = MpegFile::read_from(&mut open_for_reading(path)?, ParseOptions::new())
        .map_err(|e| e.to_string())?;
    if file.id3v2().is_none() {
        file.set_id3v2(Id3v2Tag::new());
    }
    let tag = file.id3v2_mut().ok_or("タグを作成できません")?;

    changes.apply_text(tag);
    if changes.removes_artwork() {
        tag.retain(|frame| !matches!(frame, Frame::Picture(_)));
    }
    if let Some(picture) = changes.new_artwork() {
        tag.insert_picture(picture);
    }

    // Written as is, a language outside ASCII (some encoders leave zeros) makes the whole save fail
    tag.retain_mut(|frame| {
        let language = match frame {
            Frame::Comment(comment) => Some(&mut comment.language),
            Frame::UnsynchronizedText(lyrics) => Some(&mut lyrics.language),
            _ => None,
        };
        if let Some(language) = language {
            if !language.iter().all(u8::is_ascii_alphabetic) {
                *language = *b"XXX";
            }
        }
        true
    });

    // Saving only this tag leaves the bytes of any APE or ID3v1 tag alone. Saving the whole file
    // rewrites them too, and lofty cuts 32 bytes of audio when an APE tag has no header.
    // Older software may not read v2.4, so stay on v2.3 unless that would lose a frame.
    // iTunes puts v2.4 frames in v2.3 tags, and writing v2.3 would discard them.
    let keep_older_version = tag.original_version() != Id3v2Version::V4
        && !tag
            .iter()
            .any(|frame| ID3V24_ONLY_FRAMES.contains(&frame.id_str()));
    tag.save_to_path(path, WriteOptions::new().use_id3v23(keep_older_version))
        .map_err(|e| e.to_string())
}

fn write_mp4_tags(path: &Path, changes: &TagChanges) -> Result<(), String> {
    let mut file = Mp4File::read_from(&mut open_for_reading(path)?, ParseOptions::new())
        .map_err(|e| e.to_string())?;
    if file.ilst().is_none() {
        file.set_ilst(Ilst::new());
    }
    let tag = file.ilst_mut().ok_or("タグを作成できません")?;

    changes.apply_text(tag);
    if changes.removes_artwork() {
        tag.remove_pictures();
    }
    if let Some(picture) = changes.new_artwork() {
        tag.insert_picture(picture);
    }

    file.save_to_path(path, WriteOptions::new())
        .map_err(|e| e.to_string())
}

fn write_generic_tags(path: &Path, changes: &TagChanges) -> Result<(), String> {
    let mut tagged_file = read_tagged_file(path)?;
    if tagged_file.first_tag().is_none() {
        tagged_file.insert_tag(Tag::new(tagged_file.primary_tag_type()));
    }
    let tag = if tagged_file.primary_tag().is_some() {
        tagged_file.primary_tag_mut()
    } else {
        tagged_file.first_tag_mut()
    }
    .ok_or("タグを作成できません")?;

    changes.apply_text(tag);
    if changes.removes_artwork() {
        while !tag.pictures().is_empty() {
            tag.remove_picture(0);
        }
    }
    if let Some(picture) = changes.new_artwork() {
        tag.push_picture(picture);
    }

    tagged_file
        .save_to_path(path, WriteOptions::default())
        .map_err(|e| e.to_string())
}

/// The first seconds of the sound, or `None` for a file the decoder cannot read.
fn decoded_start(path: &Path) -> Option<Vec<f32>> {
    let source = SymphoniaSource::from_path(path).ok()?;
    let samples = source.sample_rate() as usize * source.channels() as usize * VERIFY_SECONDS;
    Some(source.take(samples).collect())
}

/// Last check before the original is replaced: the sound is untouched and the tags can be read back.
fn check_written_file(
    path: &Path,
    req: &TagUpdateRequest,
    duration: Duration,
    audio_before: Option<&[f32]>,
) -> Result<(), String> {
    let written = read_tagged_file(path)?;
    if written.properties().duration().abs_diff(duration) > MAX_DURATION_DRIFT {
        return Err("書き込み後の確認に失敗しました: 再生時間が変わりました".to_string());
    }
    if let Some(before) = audio_before {
        // A prefix check: symphonia reads 8 bytes past the sound chunk when an AIFF has a tag after it
        if !decoded_start(path).is_some_and(|after| after.starts_with(before)) {
            return Err("書き込み後の確認に失敗しました: 音声が変わりました".to_string());
        }
    }

    let tag = written.primary_tag().or_else(|| written.first_tag());
    let title = tag.and_then(|t| t.title());
    let artist = tag.and_then(|t| t.artist());
    let album = tag.and_then(|t| t.album());
    let requested_title = if req.filename_to_title {
        &None
    } else {
        &req.title
    };
    for (requested, actual) in [
        (requested_title, title),
        (&req.artist, artist),
        (&req.album, album),
    ] {
        if let Some(value) = requested.as_deref().filter(|v| !v.is_empty()) {
            if actual.as_deref() != Some(value) {
                return Err("書き込み後の確認に失敗しました: タグを読み戻せません".to_string());
            }
        }
    }
    Ok(())
}

/// The working copy keeps the extension, which is how lofty tells the format.
fn temp_copy_path(path: &Path) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let mut name = format!("{}{}", stem, TAG_TEMP_MARKER);
    if let Some(ext) = path.extension() {
        name.push('.');
        name.push_str(&ext.to_string_lossy());
    }
    path.with_file_name(name)
}

struct TempCopy(PathBuf);

impl Drop for TempCopy {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(windows)]
fn replace_file(target: &Path, replacement: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        ReplaceFileW, REPLACEFILE_IGNORE_ACL_ERRORS, REPLACEFILE_IGNORE_MERGE_ERRORS,
    };

    let wide = |p: &Path| -> Vec<u16> {
        p.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let (target_w, replacement_w) = (wide(target), wide(replacement));

    // Unlike a rename, this keeps the original's creation date, attributes and permissions
    let replaced = unsafe {
        ReplaceFileW(
            target_w.as_ptr(),
            replacement_w.as_ptr(),
            std::ptr::null(),
            REPLACEFILE_IGNORE_MERGE_ERRORS | REPLACEFILE_IGNORE_ACL_ERRORS,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if replaced != 0 {
        return Ok(());
    }
    // Not every file system supports it. A rename also leaves the original intact on failure
    fs::rename(replacement, target)
}

#[cfg(not(windows))]
fn replace_file(target: &Path, replacement: &Path) -> std::io::Result<()> {
    fs::rename(replacement, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_audio::{
        aiff_bytes, aiff_pcm, ape_tag, flac_audio, flac_bytes, flac_comment_block,
        flac_padding_block, flac_with_metadata, mp3_bytes, mp3_with_id3, wav_bytes, wav_pcm,
    };

    const PNG_DATA_URL: &str = "data:image/png;base64,iVBORw0KGgo=";

    fn request(paths: &[&Path]) -> TagUpdateRequest {
        TagUpdateRequest {
            file_paths: paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            title: None,
            artist: None,
            album: None,
            track_number: None,
            clear_track_number: false,
            auto_number_tracks: false,
            filename_to_title: false,
            artwork_data_url: None,
            clear_artwork: false,
        }
    }

    fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    fn write_title(file_name: &str, bytes: &[u8], audio: &[u8]) {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), file_name, bytes);
        let mut req = request(&[&path]);
        req.title = Some("新しい題名".to_string());
        req.artist = Some("Artist".to_string());
        req.album = Some("Album".to_string());
        req.track_number = Some(7);

        let result = update_tags(&req).unwrap();

        assert!(result.failed.is_empty(), "{:?}", result.failed);
        let track = parse_track(&path);
        assert_eq!(track.title, "新しい題名");
        assert_eq!(track.artist, "Artist");
        assert_eq!(track.album, "Album");
        assert_eq!(track.track_number, Some(7));
        assert_eq!(result.updated[0].title, "新しい題名");
        assert!(contains(&fs::read(&path).unwrap(), audio), "audio changed");
        assert_eq!(file_names(dir.path()), vec![file_name.to_string()]);
    }

    #[test]
    fn writes_tags_and_keeps_the_audio_in_wav() {
        write_title("a.wav", &wav_bytes(), &wav_pcm());
    }

    #[test]
    fn writes_tags_and_keeps_the_audio_in_aiff() {
        write_title("a.aiff", &aiff_bytes(), &aiff_pcm());
    }

    #[test]
    fn writes_tags_and_keeps_the_audio_in_mp3() {
        write_title("a.mp3", &mp3_bytes(), &mp3_bytes());
    }

    #[test]
    fn writes_tags_and_keeps_the_audio_in_flac() {
        write_title("a.flac", &flac_bytes(), &flac_audio());
    }

    fn saves_title(path: &Path) {
        let mut req = request(&[path]);
        req.title = Some("Title".to_string());
        let result = update_tags(&req).unwrap();
        assert!(result.failed.is_empty(), "{:?}", result.failed);
        assert_eq!(parse_track(path).title, "Title");
    }

    #[test]
    fn mp3_keeps_its_id3_version() {
        for version in [3, 4] {
            let dir = tempfile::tempdir().unwrap();
            let path = write_file(
                dir.path(),
                "a.mp3",
                &mp3_with_id3(version, &[("TIT2", b"\0Old".to_vec())], &[]),
            );

            saves_title(&path);

            assert_eq!(fs::read(&path).unwrap()[3], version);
        }
    }

    #[test]
    fn mp3_moves_to_v24_rather_than_lose_a_frame() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(
            dir.path(),
            "a.mp3",
            &mp3_with_id3(
                3,
                &[
                    ("TIT2", b"\0Old".to_vec()),
                    ("TSOA", b"\0Sort Album".to_vec()),
                    ("TDRL", b"\x002014-02-15".to_vec()),
                ],
                &[],
            ),
        );

        saves_title(&path);

        let written = fs::read(&path).unwrap();
        assert_eq!(written[3], 4);
        assert!(contains(&written, b"Sort Album"));
        assert!(contains(&written, b"TDRL"));
    }

    #[test]
    fn mp3_keeps_a_podcast_feed_url() {
        for version in [3, 4] {
            let dir = tempfile::tempdir().unwrap();
            let path = write_file(
                dir.path(),
                "a.mp3",
                &mp3_with_id3(
                    version,
                    &[
                        ("TIT2", b"\0Old".to_vec()),
                        ("WFED", b"https://example.com/feed".to_vec()),
                    ],
                    &[],
                ),
            );

            saves_title(&path);

            assert!(
                contains(&fs::read(&path).unwrap(), b"example.com/feed"),
                "v2.{version}"
            );
        }
    }

    // All of these came from real libraries
    #[test]
    fn mp3_keeps_what_the_request_did_not_touch() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = mp3_with_id3(
            3,
            &[
                ("TIT2", b"\0Old".to_vec()),
                ("TRCK", b"\x0011/".to_vec()), // a malformed track number
                ("TPE1", b"\0Keep".to_vec()),
                ("PRIV", b"owner\0private data".to_vec()),
            ],
            &ape_tag("MP3GAIN_UNDO", "-002,-002,N"),
        );
        let path = write_file(dir.path(), "a.mp3", &bytes);

        saves_title(&path);

        let written = fs::read(&path).unwrap();
        assert!(contains(&written, b"11/"), "track number");
        assert!(contains(&written, b"private data"), "private frame");
        assert!(contains(&written, b"MP3GAIN_UNDO"), "APE tag");
        let audio = mp3_bytes();
        let start = written
            .windows(4)
            .position(|w| w == [0xFF, 0xFB, 0x90, 0x00])
            .unwrap();
        let same = written[start..]
            .iter()
            .zip(&audio)
            .take_while(|(a, b)| a == b)
            .count();
        assert_eq!(
            same,
            audio.len(),
            "audio differs at byte {same}; file {} bytes, audio starts {start}",
            written.len()
        );
        assert_eq!(parse_track(&path).artist, "Keep");
    }

    #[test]
    fn mp3_with_an_invalid_comment_language_can_be_saved() {
        for language in [[0u8, 0, 0], [0, b'H', 0xF3]] {
            let dir = tempfile::tempdir().unwrap();
            let mut comment = vec![0];
            comment.extend_from_slice(&language);
            comment.extend_from_slice(b"\0note text");
            let path = write_file(
                dir.path(),
                "a.mp3",
                &mp3_with_id3(3, &[("TIT2", b"\0Old".to_vec()), ("COMM", comment)], &[]),
            );

            saves_title(&path);

            assert!(contains(&fs::read(&path).unwrap(), b"note text"));
        }
    }

    #[test]
    fn mp3_without_a_tag_gets_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "a.mp3", &mp3_bytes());

        saves_title(&path);
    }

    #[test]
    fn mp3_artwork_is_replaced_and_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(
            dir.path(),
            "a.mp3",
            &mp3_with_id3(3, &[("TIT2", b"\0Old".to_vec())], &[]),
        );
        for _ in 0..2 {
            let mut req = request(&[&path]);
            req.artwork_data_url = Some(PNG_DATA_URL.to_string());
            assert!(update_tags(&req).unwrap().failed.is_empty());
        }

        let tagged = read_tagged_file(&path).unwrap();
        assert_eq!(tagged.primary_tag().unwrap().pictures().len(), 1);
        assert!(extract_artwork(&path)
            .unwrap()
            .starts_with("data:image/png;base64,"));

        let mut req = request(&[&path]);
        req.clear_artwork = true;
        assert!(update_tags(&req).unwrap().failed.is_empty());
        assert!(!parse_track(&path).has_artwork);
    }

    // lofty 0.21 damaged the audio of FLAC files whose metadata did not end in padding
    #[test]
    fn flac_audio_survives_every_metadata_layout() {
        for (layout, bytes) in [
            ("streaminfo only", flac_with_metadata(&[])),
            (
                "comment last",
                flac_with_metadata(&[flac_comment_block(true)]),
            ),
            (
                "comment, then padding",
                flac_with_metadata(&[flac_comment_block(false), flac_padding_block()]),
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = write_file(dir.path(), "a.flac", &bytes);
            let mut req = request(&[&path]);
            req.title = Some("Title".to_string());

            let result = update_tags(&req).unwrap();

            assert!(result.failed.is_empty(), "{layout}: {:?}", result.failed);
            assert!(
                fs::read(&path).unwrap().ends_with(&flac_audio()),
                "{layout}"
            );
            assert_eq!(parse_track(&path).title, "Title", "{layout}");
        }
    }

    #[test]
    fn existing_flac_comments_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(
            dir.path(),
            "a.flac",
            &flac_with_metadata(&[flac_comment_block(true)]),
        );
        let mut req = request(&[&path]);
        req.title = Some("Title".to_string());

        update_tags(&req).unwrap();

        assert_eq!(parse_track(&path).artist, "Keep");
    }

    #[test]
    fn the_final_check_notices_changed_audio() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "a.wav", &wav_bytes());
        let duration = read_tagged_file(&path).unwrap().properties().duration();
        let before = decoded_start(&path).unwrap();
        let req = request(&[&path]);
        assert!(check_written_file(&path, &req, duration, Some(&before)).is_ok());

        let mut changed = before;
        changed[10] += 0.25;
        let message = check_written_file(&path, &req, duration, Some(&changed)).unwrap_err();
        assert!(message.contains("音声"), "{message}");
    }

    #[test]
    fn the_final_check_notices_a_changed_length() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "a.wav", &wav_bytes());
        let req = request(&[&path]);

        let message = check_written_file(&path, &req, Duration::from_secs(60), None).unwrap_err();
        assert!(message.contains("再生時間"), "{message}");
    }

    #[test]
    fn the_final_check_notices_tags_that_were_not_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "a.wav", &wav_bytes());
        let duration = read_tagged_file(&path).unwrap().properties().duration();
        let mut req = request(&[&path]);
        req.artist = Some("Artist".to_string());

        let message = check_written_file(&path, &req, duration, None).unwrap_err();
        assert!(message.contains("タグ"), "{message}");
    }

    #[test]
    fn a_failed_file_stays_unchanged_and_the_others_are_still_written() {
        let dir = tempfile::tempdir().unwrap();
        let first = write_file(dir.path(), "1.wav", &wav_bytes());
        let broken = write_file(dir.path(), "2.wav", b"not audio at all");
        let last = write_file(dir.path(), "3.wav", &wav_bytes());
        let mut req = request(&[&first, &broken, &last]);
        req.artist = Some("Artist".to_string());

        let result = update_tags(&req).unwrap();

        assert_eq!(result.updated.len(), 2);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].file_path, broken.to_string_lossy());
        assert!(!result.failed[0].message.is_empty());
        assert_eq!(parse_track(&first).artist, "Artist");
        assert_eq!(parse_track(&last).artist, "Artist");
        assert_eq!(fs::read(&broken).unwrap(), b"not audio at all");
        assert_eq!(file_names(dir.path()), vec!["1.wav", "2.wav", "3.wav"]);
    }

    #[test]
    fn a_missing_file_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.wav");
        let mut req = request(&[&missing]);
        req.artist = Some("Artist".to_string());

        let result = update_tags(&req).unwrap();

        assert_eq!(result.failed.len(), 1);
        assert!(file_names(dir.path()).is_empty());
    }

    #[test]
    fn a_read_only_file_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "a.wav", &wav_bytes());
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions.clone()).unwrap();
        let mut req = request(&[&path]);
        req.artist = Some("Artist".to_string());

        let result = update_tags(&req).unwrap();

        assert_eq!(result.failed[0].message, "読み取り専用のファイルです");
        assert_eq!(fs::read(&path).unwrap(), wav_bytes());
        assert_eq!(file_names(dir.path()), vec!["a.wav"]);

        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&path, permissions).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn a_file_held_open_by_another_program_stays_unchanged() {
        use std::os::windows::fs::OpenOptionsExt;

        const FILE_SHARE_READ: u32 = 1;
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "a.wav", &wav_bytes());
        let holder = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&path)
            .unwrap();
        let mut req = request(&[&path]);
        req.artist = Some("Artist".to_string());

        let result = update_tags(&req).unwrap();

        drop(holder);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(fs::read(&path).unwrap(), wav_bytes());
        assert_eq!(file_names(dir.path()), vec!["a.wav"]);
    }

    #[cfg(windows)]
    #[test]
    fn the_creation_date_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "a.wav", &wav_bytes());
        let created = fs::metadata(&path).unwrap().created().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut req = request(&[&path]);
        req.artist = Some("Artist".to_string());

        let result = update_tags(&req).unwrap();

        assert!(result.failed.is_empty());
        assert_eq!(fs::metadata(&path).unwrap().created().unwrap(), created);
    }

    #[test]
    fn a_working_copy_left_by_a_crash_is_replaced_and_not_listed() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "a.wav", &wav_bytes());
        let leftover = temp_copy_path(&path);
        fs::write(&leftover, b"left by a crash").unwrap();
        assert!(!is_supported_audio(&leftover));
        let mut req = request(&[&path]);
        req.artist = Some("Artist".to_string());

        let result = update_tags(&req).unwrap();

        assert!(result.failed.is_empty());
        assert_eq!(file_names(dir.path()), vec!["a.wav"]);
    }

    #[test]
    fn numbers_tracks_in_the_order_given_and_clears_numbers() {
        let dir = tempfile::tempdir().unwrap();
        let second = write_file(dir.path(), "b.wav", &wav_bytes());
        let first = write_file(dir.path(), "a.wav", &wav_bytes());
        let mut req = request(&[&second, &first]);
        req.auto_number_tracks = true;

        update_tags(&req).unwrap();

        assert_eq!(parse_track(&second).track_number, Some(1));
        assert_eq!(parse_track(&first).track_number, Some(2));

        let mut req = request(&[&second]);
        req.clear_track_number = true;
        update_tags(&req).unwrap();
        assert_eq!(parse_track(&second).track_number, None);
    }

    #[test]
    fn uses_the_file_name_as_the_title() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "曲名.wav", &wav_bytes());
        let mut req = request(&[&path]);
        req.filename_to_title = true;

        update_tags(&req).unwrap();

        assert_eq!(parse_track(&path).title, "曲名");
    }

    #[test]
    fn sets_and_clears_artwork() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "a.wav", &wav_bytes());
        let mut req = request(&[&path]);
        req.artwork_data_url = Some(PNG_DATA_URL.to_string());

        update_tags(&req).unwrap();

        assert!(parse_track(&path).has_artwork);
        assert!(extract_artwork(&path)
            .unwrap()
            .starts_with("data:image/png;base64,"));

        let mut req = request(&[&path]);
        req.clear_artwork = true;
        update_tags(&req).unwrap();
        assert!(!parse_track(&path).has_artwork);
    }

    #[test]
    fn an_unsupported_image_stops_before_any_file_is_touched() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "a.wav", &wav_bytes());

        for url in [
            "data:image/webp;base64,AAAA",
            "data:image/svg+xml;base64,AAAA",
            "data:image/png;base64,!!!",
            "no comma",
        ] {
            let mut req = request(&[&path]);
            req.artwork_data_url = Some(url.to_string());
            assert!(update_tags(&req).is_err(), "{url}");
        }
        assert_eq!(fs::read(&path).unwrap(), wav_bytes());
    }

    #[test]
    fn playable_formats_are_listed() {
        for name in [
            "a.mp3", "a.flac", "a.m4a", "a.aac", "a.ogg", "a.wav", "a.aif", "a.aiff", "A.MP3",
        ] {
            assert!(is_supported_audio(Path::new(name)), "{name}");
        }
    }

    #[test]
    fn formats_without_a_decoder_are_not_listed() {
        for name in [
            "a.opus",
            "a.wma",
            "a.alac",
            "a.txt",
            "mp3",
            "._a.mp3",
            "dir/._a.m4a",
        ] {
            assert!(!is_supported_audio(Path::new(name)), "{name}");
        }
    }
}
