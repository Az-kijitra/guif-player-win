use crate::models::AudioTrack;

pub const REPEAT_OFF: u8 = 0;
pub const REPEAT_ONE: u8 = 1;
pub const REPEAT_ALL: u8 = 2;

/// Track order, shuffle and repeat rules. Knows nothing about audio output.
pub struct PlayQueue {
    tracks: Vec<AudioTrack>,
    version: u64,
    index: usize,
    shuffle: bool,
    repeat_mode: u8,
    // Shuffle round: tracks not played yet (the next one is at the end)
    remaining: Vec<usize>,
    // Shuffle: tracks already played, for going back
    history: Vec<usize>,
    rng: fastrand::Rng,
}

impl Default for PlayQueue {
    fn default() -> Self {
        Self::with_rng(fastrand::Rng::new())
    }
}

impl PlayQueue {
    fn with_rng(rng: fastrand::Rng) -> Self {
        Self {
            tracks: Vec::new(),
            version: 0,
            index: 0,
            shuffle: false,
            repeat_mode: REPEAT_OFF,
            remaining: Vec::new(),
            history: Vec::new(),
            rng,
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<AudioTrack>, start_index: usize) {
        self.tracks = tracks;
        self.version += 1;
        self.index = start_index;
        self.start_shuffle_round();
    }

    /// Changes whenever `tracks` does.
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn tracks(&self) -> &[AudioTrack] {
        &self.tracks
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    pub fn repeat_mode(&self) -> u8 {
        self.repeat_mode
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.start_shuffle_round();
    }

    pub fn toggle_repeat(&mut self) {
        self.repeat_mode = (self.repeat_mode + 1) % 3;
    }

    /// Moves to the next track. `None` means the queue has nothing more to play.
    pub fn advance(&mut self) -> Option<&AudioTrack> {
        if self.tracks.is_empty() {
            return None;
        }

        if self.shuffle {
            if self.remaining.is_empty() {
                if self.repeat_mode != REPEAT_ALL {
                    return None;
                }
                self.fill_next_round();
            }
            let next = self.remaining.pop()?;
            self.history.push(self.index);
            self.index = next;
        } else if self.index + 1 < self.tracks.len() {
            self.index += 1;
        } else if self.repeat_mode == REPEAT_ALL {
            self.index = 0;
        } else {
            return None;
        }

        self.tracks.get(self.index)
    }

    /// Moves to the previous track, or stays on the current one when there is none before it.
    pub fn previous(&mut self) -> Option<&AudioTrack> {
        if self.shuffle {
            if let Some(previous) = self.history.pop() {
                self.remaining.push(self.index);
                self.index = previous;
            }
        } else if self.index > 0 {
            self.index -= 1;
        } else if self.repeat_mode == REPEAT_ALL {
            self.index = self.tracks.len().saturating_sub(1);
        }

        self.tracks.get(self.index)
    }

    pub fn update_tracks(&mut self, updated_tracks: &[AudioTrack]) {
        self.version += 1;
        for track in &mut self.tracks {
            if let Some(updated) = updated_tracks
                .iter()
                .find(|u| u.file_path == track.file_path)
            {
                *track = updated.clone();
            }
        }
    }

    fn start_shuffle_round(&mut self) {
        self.history.clear();
        self.remaining.clear();
        if self.shuffle {
            let current = self.index;
            self.remaining = (0..self.tracks.len()).filter(|i| *i != current).collect();
            self.rng.shuffle(&mut self.remaining);
        }
    }

    fn fill_next_round(&mut self) {
        self.remaining = (0..self.tracks.len()).collect();
        self.rng.shuffle(&mut self.remaining);
        // Do not play the same track twice in a row across rounds
        let last = self.remaining.len() - 1;
        if last > 0 && self.remaining[last] == self.index {
            self.remaining.swap(0, last);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(n: usize) -> AudioTrack {
        AudioTrack {
            file_path: format!("track{n}.mp3"),
            file_name: format!("track{n}.mp3"),
            title: format!("Track {n}"),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_ms: 1000,
            track_number: None,
            has_artwork: false,
        }
    }

    fn queue_of(count: usize, seed: u64) -> PlayQueue {
        let mut queue = PlayQueue::with_rng(fastrand::Rng::with_seed(seed));
        queue.set_tracks((0..count).map(track).collect(), 0);
        queue
    }

    fn next_path(queue: &mut PlayQueue) -> Option<String> {
        queue.advance().map(|t| t.file_path.clone())
    }

    #[test]
    fn next_stops_at_the_end_when_repeat_is_off() {
        let mut queue = queue_of(2, 1);
        assert_eq!(next_path(&mut queue).as_deref(), Some("track1.mp3"));
        assert_eq!(next_path(&mut queue), None);
    }

    #[test]
    fn next_wraps_around_when_repeat_all_is_on() {
        let mut queue = queue_of(2, 1);
        queue.toggle_repeat();
        queue.toggle_repeat();
        assert_eq!(queue.repeat_mode(), REPEAT_ALL);
        assert_eq!(next_path(&mut queue).as_deref(), Some("track1.mp3"));
        assert_eq!(next_path(&mut queue).as_deref(), Some("track0.mp3"));
    }

    #[test]
    fn next_on_an_empty_queue_is_none() {
        let mut queue = queue_of(0, 1);
        assert_eq!(next_path(&mut queue), None);
        assert!(queue.previous().is_none());
    }

    #[test]
    fn previous_stays_on_the_first_track_when_repeat_is_off() {
        let mut queue = queue_of(3, 1);
        assert_eq!(queue.previous().unwrap().file_path, "track0.mp3");
    }

    #[test]
    fn previous_wraps_to_the_last_track_when_repeat_all_is_on() {
        let mut queue = queue_of(3, 1);
        queue.toggle_repeat();
        queue.toggle_repeat();
        assert_eq!(queue.previous().unwrap().file_path, "track2.mp3");
    }

    #[test]
    fn shuffle_plays_every_track_once_and_then_stops() {
        let mut queue = queue_of(12, 7);
        queue.toggle_shuffle();

        let mut played = vec!["track0.mp3".to_string()];
        while let Some(path) = next_path(&mut queue) {
            played.push(path);
        }

        let mut sorted = played.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 12, "played: {played:?}");
        assert_eq!(played.len(), 12, "played: {played:?}");
    }

    #[test]
    fn shuffle_order_depends_on_the_random_source() {
        let order = |seed| {
            let mut queue = queue_of(12, seed);
            queue.toggle_shuffle();
            std::iter::from_fn(|| next_path(&mut queue)).collect::<Vec<_>>()
        };
        assert_ne!(order(1), order(2));
    }

    #[test]
    fn shuffle_with_repeat_all_keeps_going_without_an_immediate_repeat() {
        for seed in 0..50 {
            let mut queue = queue_of(3, seed);
            queue.toggle_shuffle();
            queue.toggle_repeat();
            queue.toggle_repeat();

            let mut previous = "track0.mp3".to_string();
            for _ in 0..30 {
                let current = next_path(&mut queue).expect("repeat all never ends");
                assert_ne!(current, previous, "seed {seed}");
                previous = current;
            }
        }
    }

    #[test]
    fn shuffle_with_repeat_all_replays_a_single_track() {
        let mut queue = queue_of(1, 1);
        queue.toggle_shuffle();
        queue.toggle_repeat();
        queue.toggle_repeat();
        assert_eq!(next_path(&mut queue).as_deref(), Some("track0.mp3"));
    }

    #[test]
    fn shuffle_previous_walks_back_and_next_returns_to_the_same_track() {
        let mut queue = queue_of(12, 3);
        queue.toggle_shuffle();

        let first = next_path(&mut queue).unwrap();
        let second = next_path(&mut queue).unwrap();
        assert_eq!(queue.previous().unwrap().file_path, first);
        assert_eq!(next_path(&mut queue).as_deref(), Some(second.as_str()));
    }

    #[test]
    fn setting_tracks_starts_a_new_shuffle_round() {
        let mut queue = queue_of(5, 3);
        queue.toggle_shuffle();
        queue.advance();

        queue.set_tracks((10..14).map(track).collect(), 1);
        assert_eq!(queue.previous().unwrap().file_path, "track11.mp3");

        let played: Vec<_> = std::iter::from_fn(|| next_path(&mut queue)).collect();
        assert_eq!(played.len(), 3);
        assert!(!played.contains(&"track11.mp3".to_string()));
    }

    #[test]
    fn the_version_follows_the_tracks_and_nothing_else() {
        let mut queue = queue_of(3, 1);
        let start = queue.version();

        queue.advance();
        queue.previous();
        queue.toggle_shuffle();
        queue.toggle_repeat();
        assert_eq!(queue.version(), start);

        queue.update_tracks(&[track(1)]);
        assert_eq!(queue.version(), start + 1);
        queue.set_tracks(vec![track(5)], 0);
        assert_eq!(queue.version(), start + 2);
    }

    #[test]
    fn update_tracks_replaces_matching_tracks_only() {
        let mut queue = queue_of(2, 1);
        let mut renamed = track(1);
        renamed.title = "Renamed".to_string();
        queue.update_tracks(&[renamed]);
        assert_eq!(queue.tracks()[0].title, "Track 0");
        assert_eq!(queue.tracks()[1].title, "Renamed");
    }
}
