use super::{EventTrack, TrackCursor};
use crate::midi::MidiMessage;
use crate::PortIdent;
use std::time::Duration;

/// A cursor aggregator that wraps multiple `TrackCursor`s into a single
/// progressable cursor. 
pub struct VecMultiCursor<T: EventTrack> {
    cursors: Vec<TrackCursor<T>>,
}

impl<T: EventTrack> VecMultiCursor<T> {
    pub fn new(cursors: Vec<TrackCursor<T>>) -> Self {
        Self { cursors }
    }

    /// Gets the inner list of `TrackCursor<T>`s that this
    /// struct combines.
    #[allow(dead_code)]
    pub fn cursors(&self) -> &[TrackCursor<T>] {
        &self.cursors
    }

    /// Unwraps the cursors back into a `Vec<TrackCursor<T>>`.
    #[allow(dead_code)]
    pub fn into_inner(self) -> Vec<TrackCursor<T>> {
        self.cursors
    }

    /// Gets the current clock time in the track.
    /// Note that if there are no currently wrapped `TrackCursor`s
    /// in this `VecMultiCursor`, then the clock time is always 0.
    #[allow(dead_code)]
    pub fn cur_clock(&self) -> Duration {
        // Technically they should all be equal; however, even if not, 
        // the `max()` time should still be the actual current play time
        // since we assume that the inner cursors are not lying about their 
        // play time. 
        self.cursors
            .iter()
            .map(|cursor| cursor.cur_clock())
            .max() 
            .unwrap_or_default()
    }

    /// Gets all MIDI events lying within a time period.
    ///
    /// Both `start` and `end` are measured since the start of
    /// track playback. The time period includes `start`, but does
    /// not include `end`. Events will be returned with the timestamp
    /// of the event, again measured since the start of track playback.
    ///
    /// Message output ports are given as `(usize, OutputPort)` instead of the regular `OutputPort`
    /// with the `usize` corresponding to the index in the vector of the track that produced the
    /// message. This allows for more dynamic mapping of track + port label index -> actual output
    /// port structure.
    pub fn events_in_range<'a>(
        &'a mut self,
        start: Duration,
        end: Duration,
    ) -> impl Iterator<Item = (Duration, PortIdent, MidiMessage)> + 'a {
        let cursor_mapper = move |(idx, cursor): (_, &'a mut TrackCursor<_>)| {
            cursor
                .events_in_range(start, end)
                .map(move |(time, port, msg)| (time, (idx, port), msg))
        };
        self.cursors.iter_mut().enumerate().flat_map(cursor_mapper)
    }
}
