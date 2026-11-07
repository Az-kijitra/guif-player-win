//! The audio output, following the Windows default output device (headphones in or out,
//! a change in the sound settings): playback moves to the new device and goes on.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{OutputStream, OutputStreamHandle};

use crate::player::SharedPlayer;

/// How often the default device is looked at
const POLL_INTERVAL: Duration = Duration::from_secs(1);

fn default_device_name() -> Option<String> {
    rodio::cpal::default_host()
        .default_output_device()?
        .name()
        .ok()
}

fn open_default() -> Option<(OutputStream, OutputStreamHandle)> {
    match OutputStream::try_default() {
        Ok(output) => Some(output),
        Err(e) => {
            eprintln!("Failed to open the audio output: {}", e);
            None
        }
    }
}

/// Which default device playback is on, to tell when it changes.
#[derive(Default)]
pub struct DeviceWatch {
    current: Option<String>,
}

impl DeviceWatch {
    /// True when `now` is a device other than the one in use. No device at all is not a change:
    /// playback stays where it is until a device appears.
    pub fn changed(&mut self, now: Option<String>) -> bool {
        match now {
            Some(name) if self.current.as_deref() != Some(name.as_str()) => {
                self.current = Some(name);
                true
            }
            _ => false,
        }
    }

    /// Opening the device failed: look again next time.
    pub fn forget(&mut self) {
        self.current = None;
    }
}

/// Opens the default output for `player` and keeps it on the default device from then on.
/// The output stream lives on its own thread, because it cannot move between threads.
pub fn follow_default_output(player: SharedPlayer) {
    let (opened, first) = mpsc::sync_channel(1);
    let thread_player = player.clone();
    thread::spawn(move || {
        let mut watch = DeviceWatch::default();
        watch.changed(default_device_name());
        // Held, not read: the device plays only while its stream is open
        let mut _stream = open_default();
        if _stream.is_none() {
            watch.forget();
        }
        let _ = opened.send(_stream.as_ref().map(|(_, handle)| handle.clone()));

        loop {
            thread::sleep(POLL_INTERVAL);
            if !watch.changed(default_device_name()) {
                continue;
            }
            let Some(new_stream) = open_default() else {
                watch.forget();
                continue;
            };
            thread_player
                .lock()
                .unwrap()
                .switch_output(new_stream.1.clone());
            // The old stream is closed only after playback has moved
            _stream = Some(new_stream);
        }
    });

    let handle = first.recv().ok().flatten();
    if let Some(handle) = handle {
        player.lock().unwrap().switch_output(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_device_is_followed_only_when_it_changes() {
        let mut watch = DeviceWatch::default();
        assert!(watch.changed(Some("Speakers".into())), "the first device");
        assert!(!watch.changed(Some("Speakers".into())), "the same device");
        assert!(watch.changed(Some("Headphones".into())), "headphones in");
        assert!(!watch.changed(None), "no device: stay where we are");
        assert!(watch.changed(Some("Speakers".into())), "headphones out");
    }

    #[test]
    fn a_device_that_failed_to_open_is_tried_again() {
        let mut watch = DeviceWatch::default();
        assert!(watch.changed(Some("Speakers".into())));
        watch.forget();
        assert!(watch.changed(Some("Speakers".into())));
    }
}
