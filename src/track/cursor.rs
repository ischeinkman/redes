use super::{BpmInfo, EventTrack, OutputPort, TrackEvent};
use crate::midi::MidiMessage;
use std::collections::HashMap;
use std::num::NonZeroU16;
use std::time::Duration;

/// A cursor along an `EventTrack`.
/// Allows for stepping through the track and acts as a sort of
/// "register list" for an event track "VM".
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TrackCursor<TrackData: EventTrack> {
    instruction_pointer: usize,
    cur_bpm: BpmInfo,
    cur_time: Duration,
    cur_ticks : u16,  
    jump_counts: JumpCounts,
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
    BadJumpTarget { target: usize },
    JumpIdxNotFound { target: usize },
    BadInstrPointer(usize),
}

impl<T: EventTrack> TrackCursor<T> {
    pub fn new(data: T) -> Self {
        TrackCursor {
            instruction_pointer: 0,
            cur_bpm: BpmInfo::default(),
            cur_time: Duration::from_nanos(0),
            cur_ticks : 0,  
            jump_counts: JumpCounts::from_iter(data.len(), data.finite_jumps()),
            data,
        }
    }

    /// Gets the current instruction pointer.
    #[allow(dead_code)]
    pub fn pc(&self) -> usize {
        self.instruction_pointer
    }

    /// Gets the current BPM value.
    #[allow(dead_code)]
    pub fn bpm(&self) -> BpmInfo {
        self.cur_bpm
    }

    /// Gets the current clock time in the track.
    #[allow(dead_code)]
    pub fn cur_clock(&self) -> Duration {
        self.cur_time
    }

    /// Gets the number of beat "ticks" that have occured in the track.
    /// Note that this is NOT a true measure of time, since the length 
    /// of a single tick can change between SET BPM commands. 
    #[allow(dead_code)]
    pub fn cur_ticks(&self) -> u16 {
        self.cur_ticks
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
                self.cur_ticks += time.as_ticks(self.cur_bpm).get();
                Ok(StepOutput::Continue)
            }
            TrackEvent::Jump { target, count } => {
                let new_pc = self
                    .jump_counts
                    .do_jump(self.instruction_pointer, target, count)?;
                self.instruction_pointer = new_pc;
                Ok(StepOutput::Continue)
            }
        }
    }
}

/// State information about jumps with finite counters.
#[derive(Debug, Clone, Eq, PartialEq)]
struct JumpCounts {
    /// The maximum valid target a jump can point to
    max_target: usize,
    /// The map of jump instruction addresses to the number of times
    /// those jumps have been processes.
    /// Note that the current implementation does NOT contain values
    /// for jumps without counters.
    data: HashMap<usize, u16>,
}

impl JumpCounts {
    /// Processes a single JUMP instruction.
    ///
    /// Verifies that the target is in bounds and
    /// that the JUMP index cache was initialized correctly,
    /// processes this jump's current counter (if it has one),
    /// and returns the value of the instruction pointer after this
    /// jump (either `target` or `idx + 1`, depending on counter states).
    pub fn do_jump(
        &mut self,
        idx: usize,
        target: usize,
        count: Option<NonZeroU16>,
    ) -> Result<usize, StepError> {
        if target > self.max_target {
            return Err(StepError::BadJumpTarget { target });
        }
        let count = match count {
            Some(n) => n.get(),
            None => {
                return Ok(target);
            }
        };
        let cur_count = self
            .data
            .get_mut(&idx)
            .ok_or_else(|| StepError::JumpIdxNotFound { target: idx })?;
        if *cur_count == 0 {
            *cur_count = count;
            Ok(idx + 1)
        } else {
            *cur_count -= 1;
            Ok(target)
        }
    }
    pub fn from_iter(track_len: usize, itr: impl IntoIterator<Item = (usize, u16)>) -> Self {
        Self {
            max_target: track_len - 1,
            data: itr.into_iter().collect(),
        }
    }
}
