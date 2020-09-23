mod cursor;
pub use cursor::*;

mod multicursor;
pub use multicursor::*;

mod instructions;
pub use instructions::{BpmInfo, TrackEvent, WaitTime, OutputPort};

/// A MIDI event track that represents a constant, static performance that takes no input
/// data, represented as a fixed list of instructions.
pub trait EventTrack {
    /// Gets the instruction at the given position in the instruction list.
    fn get(&self, instruction_idx: usize) -> Option<TrackEvent>;

    /// Gets the number of instructions in this track. 
    fn len(&self) -> usize {
        (0..usize::max_value())
            .map(|idx| self.get(idx))
            .take_while(|opt| opt.is_some())
            .count()
    }

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

    /// Collects all "ports" that this track sends MIDI messages to. 
    /// These ports are separate from Midi "channels", and instead 
    /// map to framework-level objects, EG Jack's MIDI Output ports.
    fn output_ports(&self) -> Vec<OutputPort> {
       let mut all_ports : Vec<_> = (0..usize::max_value())
            .map(|idx| (idx, self.get(idx)))
            .take_while(|(_, res)| res.is_some())
            .filter_map(|(_, evt)| {
                match evt {
                    Some(TrackEvent::SendMessage{port, ..}) => Some(port), 
                    _ => None
                }
            })
            .collect();
        all_ports.sort();
        all_ports.dedup();
        all_ports
    }
}

impl<T: AsRef<[TrackEvent]>> EventTrack for T {
    fn get(&self, instruction_idx: usize) -> Option<TrackEvent> {
        self.as_ref().get(instruction_idx).copied()
    }
    fn len(&self) -> usize {
        self.as_ref().len()
    }
}
