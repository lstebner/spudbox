use super::TrackInfo;

pub struct Queue {
    tracks: Vec<TrackInfo>,
    index: usize,
}

impl Queue {
    pub fn new(tracks: Vec<TrackInfo>, start_index: usize) -> Self {
        let index = start_index.min(tracks.len().saturating_sub(1));
        Self { tracks, index }
    }

    pub fn current(&self) -> Option<&TrackInfo> {
        self.tracks.get(self.index)
    }

    pub fn peek_next(&self) -> Option<&TrackInfo> {
        self.tracks.get(self.index + 1)
    }

    /// Moves to the next track, returning the new current track if any.
    pub fn advance(&mut self) -> Option<&TrackInfo> {
        self.index += 1;
        self.current()
    }

    /// Moves back to the first track, e.g. so playback can loop back to the
    /// start (paused) once the queue naturally runs out at the end.
    pub fn reset_to_start(&mut self) {
        self.index = 0;
    }

    pub fn move_to_previous(&mut self) -> Option<&TrackInfo> {
        if self.index == 0 {
            return None;
        }
        self.index -= 1;
        self.current()
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn track_ids(&self) -> Vec<i64> {
        self.tracks.iter().map(|t| t.track_id).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn track(id: i64) -> TrackInfo {
        TrackInfo {
            track_id: id,
            path: PathBuf::from(format!("/music/track-{id}.flac")),
            duration_ms: 1000,
            title: format!("Track {id}"),
            artist: "Some Artist".to_string(),
            album: "Some Album".to_string(),
            album_id: Some(1),
            art_path: None,
        }
    }

    fn queue_of(ids: &[i64], start_index: usize) -> Queue {
        Queue::new(ids.iter().map(|&id| track(id)).collect(), start_index)
    }

    #[test]
    fn starts_at_requested_index() {
        let q = queue_of(&[1, 2, 3], 1);
        assert_eq!(q.current().unwrap().track_id, 2);
        assert_eq!(q.index(), 1);
    }

    #[test]
    fn clamps_out_of_range_start_index_to_last_track() {
        let q = queue_of(&[1, 2, 3], 99);
        assert_eq!(q.index(), 2);
        assert_eq!(q.current().unwrap().track_id, 3);
    }

    #[test]
    fn empty_queue_has_no_current_track() {
        let q = Queue::new(Vec::new(), 0);
        assert!(q.current().is_none());
        assert_eq!(q.index(), 0);
    }

    #[test]
    fn peek_next_is_none_at_end_of_queue() {
        let q = queue_of(&[1, 2], 1);
        assert!(q.peek_next().is_none());
        let q = queue_of(&[1, 2], 0);
        assert_eq!(q.peek_next().unwrap().track_id, 2);
    }

    #[test]
    fn advance_walks_forward_then_runs_out() {
        let mut q = queue_of(&[1, 2, 3], 0);
        assert_eq!(q.advance().unwrap().track_id, 2);
        assert_eq!(q.advance().unwrap().track_id, 3);
        assert!(q.advance().is_none(), "advancing past the last track should yield no current track");
    }

    #[test]
    fn move_to_previous_walks_backward_then_stops_at_start() {
        let mut q = queue_of(&[1, 2, 3], 2);
        assert_eq!(q.move_to_previous().unwrap().track_id, 2);
        assert_eq!(q.move_to_previous().unwrap().track_id, 1);
        assert!(q.move_to_previous().is_none(), "moving before the first track should return None, not wrap/panic");
        assert_eq!(q.index(), 0, "index should stay at 0, not underflow");
    }

    #[test]
    fn reset_to_start_returns_to_the_first_track() {
        let mut q = queue_of(&[1, 2, 3], 0);
        q.advance();
        q.advance();
        assert!(q.advance().is_none(), "should be past the last track");
        q.reset_to_start();
        assert_eq!(q.index(), 0);
        assert_eq!(q.current().unwrap().track_id, 1);
    }

    #[test]
    fn track_ids_preserves_order() {
        let q = queue_of(&[5, 3, 9], 0);
        assert_eq!(q.track_ids(), vec![5, 3, 9]);
    }
}
