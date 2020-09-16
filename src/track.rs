mod cursor;
pub use cursor::*;

mod instructions;
pub use instructions::{BpmInfo, TrackEvent, WaitTime};

/// A MIDI event track that represents a constant, static performance that takes no input
/// data, represented as a fixed list of instructions.
pub trait EventTrack {
    /// Gets the instruction at the given position in the instruction list.
    fn get(&self, instruction_idx: usize) -> Option<TrackEvent>;

    /// Collects the location and count of all `TrackEvent::Jump` instructions
    /// whose `count` value is not `None`. This is used internally to pre-allocate
    /// loop indices by the `TrackCursor`.
    fn finite_jumps(&self) -> Vec<(usize, u16)> {
        (0..usize::max_value())
            .map(|idx| (idx, self.get(idx)))
            .take_while(|(_, res)| res.is_some())
            .filter_map(|(idx, evt)| {
                if let Some(TrackEvent::Jump {
                    count: Some(cnt), ..
                }) = evt
                {
                    Some((idx, cnt.get()))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl<T: AsRef<[TrackEvent]>> EventTrack for T {
    fn get(&self, instruction_idx: usize) -> Option<TrackEvent> {
        self.as_ref().get(instruction_idx).copied()
    }
}
