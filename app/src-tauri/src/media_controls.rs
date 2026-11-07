//! Windows media controls (System Media Transport Controls): the track and its artwork in the
//! volume flyout and on the lock screen, and play, pause, next and previous from there,
//! from the media keys and from headset buttons.

use crate::models::PlaybackStateDto;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaButton {
    Play,
    Pause,
    Next,
    Previous,
}

/// What Windows is told about the playing track.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NowPlaying {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
}

impl NowPlaying {
    /// Whether the title, artwork and so on must be sent again, not only the play state.
    pub fn same_track(&self, other: &NowPlaying) -> bool {
        self.path == other.path
            && self.title == other.title
            && self.artist == other.artist
            && self.album == other.album
    }
}

pub fn now_playing(state: &PlaybackStateDto) -> Option<NowPlaying> {
    let track = state.current_track.as_ref()?;
    Some(NowPlaying {
        path: track.file_path.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        playing: state.is_playing,
    })
}

#[cfg(windows)]
pub use windows_smtc::start;

#[cfg(windows)]
mod windows_smtc {
    use std::path::Path;
    use std::sync::mpsc::{self, Sender};
    use std::sync::Arc;
    use std::thread;

    use windows::core::{factory, Result, HSTRING};
    use windows::Foundation::TypedEventHandler;
    use windows::Media::{
        MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
        SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
    };
    use windows::Storage::Streams::{
        DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference,
    };
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::WinRT::{
        ISystemMediaTransportControlsInterop, RoInitialize, RO_INIT_MULTITHREADED,
    };

    use super::{MediaButton, NowPlaying};
    use crate::metadata::artwork_bytes;

    /// Connects the window to the media controls. Send `Some` for each change of track or play
    /// state, `None` when nothing is loaded. None when Windows would not give the controls.
    pub fn start(
        hwnd: isize,
        on_button: impl Fn(MediaButton) + Send + Sync + 'static,
    ) -> Option<Sender<Option<NowPlaying>>> {
        let smtc = match controls_for(hwnd, Arc::new(on_button)) {
            Ok(smtc) => smtc,
            Err(e) => {
                eprintln!("Media controls are not available: {}", e);
                return None;
            }
        };
        let (tx, rx) = mpsc::channel::<Option<NowPlaying>>();
        thread::spawn(move || {
            let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
            let mut shown: Option<NowPlaying> = None;
            for now in rx {
                if let Err(e) = show(&smtc, now.as_ref(), shown.as_ref()) {
                    eprintln!("Failed to update the media controls: {}", e);
                }
                shown = now;
            }
        });
        Some(tx)
    }

    fn controls_for(
        hwnd: isize,
        on_button: Arc<dyn Fn(MediaButton) + Send + Sync>,
    ) -> Result<SystemMediaTransportControls> {
        let interop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()?;
        let smtc: SystemMediaTransportControls =
            unsafe { interop.GetForWindow(HWND(hwnd as *mut _)) }?;
        smtc.SetIsPlayEnabled(true)?;
        smtc.SetIsPauseEnabled(true)?;
        smtc.SetIsNextEnabled(true)?;
        smtc.SetIsPreviousEnabled(true)?;
        smtc.ButtonPressed(&TypedEventHandler::<
            SystemMediaTransportControls,
            SystemMediaTransportControlsButtonPressedEventArgs,
        >::new(move |_, args| {
            let button = match args.ok()?.Button()? {
                SystemMediaTransportControlsButton::Play => MediaButton::Play,
                SystemMediaTransportControlsButton::Pause => MediaButton::Pause,
                SystemMediaTransportControlsButton::Next => MediaButton::Next,
                SystemMediaTransportControlsButton::Previous => MediaButton::Previous,
                _ => return Ok(()),
            };
            on_button(button);
            Ok(())
        }))?;
        // Turned on when a track is loaded, so Windows shows no empty entry before that
        smtc.SetIsEnabled(false)?;
        Ok(smtc)
    }

    fn show(
        smtc: &SystemMediaTransportControls,
        now: Option<&NowPlaying>,
        shown: Option<&NowPlaying>,
    ) -> Result<()> {
        let Some(now) = now else {
            smtc.SetPlaybackStatus(MediaPlaybackStatus::Closed)?;
            let updater = smtc.DisplayUpdater()?;
            updater.ClearAll()?;
            updater.Update()?;
            return smtc.SetIsEnabled(false);
        };
        smtc.SetIsEnabled(true)?;
        if !shown.is_some_and(|shown| shown.same_track(now)) {
            let updater = smtc.DisplayUpdater()?;
            updater.ClearAll()?;
            updater.SetType(MediaPlaybackType::Music)?;
            let music = updater.MusicProperties()?;
            music.SetTitle(&HSTRING::from(now.title.as_str()))?;
            music.SetArtist(&HSTRING::from(now.artist.as_str()))?;
            music.SetAlbumTitle(&HSTRING::from(now.album.as_str()))?;
            if let Some((bytes, _mime)) = artwork_bytes(Path::new(&now.path)) {
                // A track whose artwork cannot be read still shows its title
                if let Ok(thumbnail) = thumbnail(&bytes) {
                    updater.SetThumbnail(&thumbnail)?;
                }
            }
            updater.Update()?;
        }
        smtc.SetPlaybackStatus(if now.playing {
            MediaPlaybackStatus::Playing
        } else {
            MediaPlaybackStatus::Paused
        })
    }

    fn thumbnail(bytes: &[u8]) -> Result<RandomAccessStreamReference> {
        let stream = InMemoryRandomAccessStream::new()?;
        let writer = DataWriter::CreateDataWriter(&stream)?;
        writer.WriteBytes(bytes)?;
        writer.StoreAsync()?.join()?;
        writer.FlushAsync()?.join()?;
        writer.DetachStream()?;
        stream.Seek(0)?;
        RandomAccessStreamReference::CreateFromStream(&stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AudioTrack;

    fn state(title: &str, playing: bool) -> PlaybackStateDto {
        PlaybackStateDto {
            current_track: Some(AudioTrack {
                file_path: "C:/Music/a.mp3".into(),
                file_name: "a.mp3".into(),
                title: title.into(),
                artist: "Artist".into(),
                album: "Album".into(),
                duration_ms: 1000,
                track_number: None,
                has_artwork: false,
            }),
            is_playing: playing,
            position_ms: 0,
            duration_ms: 1000,
            volume: 1.0,
            shuffle_mode: false,
            repeat_mode: 0,
            queue_version: 0,
            speed: 1.0,
        }
    }

    #[test]
    fn windows_is_told_the_title_artist_album_and_play_state() {
        let now = now_playing(&state("Song", true)).unwrap();
        assert_eq!(
            (now.title.as_str(), now.artist.as_str(), now.album.as_str()),
            ("Song", "Artist", "Album")
        );
        assert!(now.playing);
        let mut stopped = state("Song", false);
        stopped.current_track = None;
        assert_eq!(now_playing(&stopped), None, "nothing loaded");
    }

    #[test]
    fn a_pause_does_not_send_the_track_again_but_a_tag_edit_does() {
        let playing = now_playing(&state("Song", true)).unwrap();
        let paused = now_playing(&state("Song", false)).unwrap();
        let renamed = now_playing(&state("New title", true)).unwrap();
        assert!(playing.same_track(&paused));
        assert!(!playing.same_track(&renamed));
    }
}
