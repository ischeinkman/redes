use super::{BpmInfo, EventTrack, OutputPort, TrackEvent};
use crate::midi::MidiMessage;
use std::time::Duration;

/// A cursor along an `EventTrack`.
/// Allows for stepping through the track and acts as a sort of
/// "register list" for an event track "VM".
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TrackCursor<TrackData: EventTrack> {
    instruction_pointer: usize,
    cur_bpm: BpmInfo,
    cur_time: Duration,
    jump_counts: Vec<(usize, u16)>,
    data: TrackData,
}

/// Possible signal values that can be returned from `step()`.
#[derive(Debug, Eq, PartialEq, Hash)]
enum StepOutput {
    /// Playback has ended; all subsequent calls to `step()`
    /// will always return `Ok(StepOutput::end)`.
    End,
    /// No event has been emitted during this call to `step()`,
    /// but more may be still emitted in subsequent calls.
    Continue,
    /// The current `step()` call has emittd a MIDI event;
    /// implies that playback will continue.
    Message {
        time: Duration,
        port: OutputPort,
        msg: MidiMessage,
    },
}

/// Errors that may occur when calling `step()`.
#[derive(Debug)]
enum StepError {
    #[allow(dead_code)]
    BadJumpTarget {
        target: usize,
    },
    JumpIdxNotFound {
        target: usize,
    },
    BadInstrPointer(usize),
}

impl<T: EventTrack> TrackCursor<T> {
    pub fn new(data: T) -> Self {
        TrackCursor {
            instruction_pointer: 0,
            cur_bpm: BpmInfo::default(),
            cur_time: Duration::from_nanos(0),
            jump_counts: data.finite_jumps(),
            data,
        }
    }

    /// Gets all MIDI events lying within a time period.
    ///
    /// Both `start` and `end` are measured since the start of
    /// track playback. The time period includes `start`, but does
    /// not include `end`. Events will be returned with the timestamp
    /// of the event, again measured since the start of track playback.
    pub fn events_in_range<'a>(
        &'a mut self,
        start: Duration,
        end: Duration,
    ) -> impl Iterator<Item = (Duration, OutputPort, MidiMessage)> + 'a {
        while self.cur_time < start {
            if self.step().unwrap() == StepOutput::End {
                break;
            }
        }
        std::iter::from_fn(move || loop {
            if self.cur_time > end {
                return None;
            }
            match self.step().unwrap() {
                StepOutput::End => {
                    return None;
                }
                StepOutput::Message { time, port, msg } => {
                    return Some((time, port, msg));
                }
                StepOutput::Continue => {}
            }
        })
    }

    /// Runs the instruction at the current instruction pointer
    /// and progresses the cursor state forward.
    fn step(&mut self) -> Result<StepOutput, StepError> {
        let next_evt = self
            .data
            .get(self.instruction_pointer)
            .ok_or(StepError::BadInstrPointer(self.instruction_pointer))?;
        match next_evt {
            // Note: **NOT** stepping the instruction pointer
            // to gurantee that all following calls to `step()` also produce
            // `StepOutput::End`.
            TrackEvent::End => Ok(StepOutput::End),
            TrackEvent::SetBpm(new_info) => {
                self.cur_bpm = new_info;
                self.instruction_pointer += 1;
                Ok(StepOutput::Continue)
            }
            TrackEvent::SendMessage { message, port } => {
                self.instruction_pointer += 1;
                Ok(StepOutput::Message {
                    time: self.cur_time,
                    port,
                    msg: message,
                })
            }
            TrackEvent::Wait(time) => {
                self.instruction_pointer += 1;
                self.cur_time += time.as_duration(self.cur_bpm);
                Ok(StepOutput::Continue)
            }
            TrackEvent::Jump {
                target,
                count: None,
            } => {
                //TODO: Verify target validity
                self.instruction_pointer = target;
                Ok(StepOutput::Continue)
            }
            TrackEvent::Jump {
                target,
                count: Some(max),
            } => {
                let cur_idx = self
                    .jump_counts
                    .binary_search_by_key(&self.instruction_pointer, |(idx, _)| *idx)
                    .map_err(|_| StepError::JumpIdxNotFound {
                        target: self.instruction_pointer,
                    })?;
                if self.jump_counts[cur_idx].1 == 0 {
                    self.jump_counts[cur_idx].1 = max.get();
                    self.instruction_pointer += 1;
                    Ok(StepOutput::Continue)
                } else {
                    self.jump_counts[cur_idx].1 -= 1;
                    self.instruction_pointer = target;
                    Ok(StepOutput::Continue)
                }
            }
        }
    }
}
