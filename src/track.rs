use crate::midi::MidiMessage;
use std::num::NonZeroU16;
use std::time::Duration;

pub trait EventTrack {
    fn get(&self, instruction_idx: usize) -> Option<TrackEvent>;
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

pub struct TrackCursor<TrackData: EventTrack> {
    instruction_pointer: usize,
    cur_bpm: BpmInfo,
    cur_time: Duration,
    jump_counts: Vec<(usize, u16)>,
    data: TrackData,
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum StepOutput {
    End,
    Continue,
    Message { time: Duration, msg: MidiMessage },
}

#[derive(Debug)]
enum StepError {
    #[allow(dead_code)]
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
            jump_counts: data.finite_jumps(),
            data,
        }
    }

    pub fn events_in_range<'a>(
        &'a mut self,
        start: Duration,
        end: Duration,
    ) -> impl Iterator<Item = (Duration, MidiMessage)> + 'a {
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
                StepOutput::Message { time, msg } => {
                    return Some((time, msg));
                }
                StepOutput::Continue => {}
            }
        })
    }

    fn step(&mut self) -> Result<StepOutput, StepError> {
        let next_evt = self
            .data
            .get(self.instruction_pointer)
            .ok_or(StepError::BadInstrPointer(self.instruction_pointer))?;
        match next_evt {
            TrackEvent::End => Ok(StepOutput::End),
            TrackEvent::SetBpm(new_info) => {
                self.cur_bpm = new_info;
                self.instruction_pointer += 1;
                Ok(StepOutput::Continue)
            }
            TrackEvent::SendMessage(msg) => {
                self.instruction_pointer += 1;
                Ok(StepOutput::Message {
                    time: self.cur_time,
                    msg,
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
                //TODO: Verify
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TrackEvent {
    SendMessage(MidiMessage),
    Wait(WaitTime),
    SetBpm(BpmInfo),
    Jump {
        target: usize,
        count: Option<NonZeroU16>,
    },
    #[allow(dead_code)]
    End,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum WaitTime {
    #[allow(dead_code)]
    Time(Duration),
    BeatTicks(NonZeroU16),
}

const fn clamped_to_nonzerou16(raw: u128) -> NonZeroU16 {
    if raw > u16::max_value() as u128 {
        // SAFETY: Guranteed safe b/c `u16::max_value()` is
        // a guranteed non-zero constant.
        unsafe { NonZeroU16::new_unchecked(u16::max_value()) }
    } else if raw == 0 {
        // SAFETY: Guranteed safe b/c `1` is
        // a guranteed non-zero constant.
        unsafe { NonZeroU16::new_unchecked(1) }
    } else {
        // SAFETY: Guranteed safe b/c the zero case
        // and overflow cases were previously handled.
        unsafe { NonZeroU16::new_unchecked(raw as u16) }
    }
}

impl WaitTime {
    #[allow(dead_code)]
    pub const fn as_ticks(&self, bpm_info: BpmInfo) -> NonZeroU16 {
        match *self {
            WaitTime::BeatTicks(ticks) => ticks,
            WaitTime::Time(dur) => {
                let nanos_per_tick = bpm_info.nanos_per_tick() as u128;
                let self_nanos = dur.as_nanos();
                let ticks = self_nanos / nanos_per_tick;
                clamped_to_nonzerou16(ticks)
            }
        }
    }
    pub const fn as_duration(&self, bpm_info: BpmInfo) -> Duration {
        match *self {
            WaitTime::Time(dur) => dur,
            WaitTime::BeatTicks(ticks) => {
                Duration::from_nanos(bpm_info.nanos_per_tick() * (ticks.get() as u64))
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct BpmInfo {
    pub beats_per_minute: NonZeroU16,
    pub ticks_per_beat: NonZeroU16,
}

const NANOS_PER_MICRO: u64 = 1000;
const NANOS_PER_MILLI: u64 = 1000 * NANOS_PER_MICRO;
const NANOS_PER_SECOND: u64 = 1000 * NANOS_PER_MILLI;
const NANOS_PER_MINUTE: u64 = 60 * NANOS_PER_SECOND;

impl BpmInfo {
    const fn nanos_per_beat(&self) -> u64 {
        NANOS_PER_MINUTE / (self.beats_per_minute.get() as u64)
    }
    const fn nanos_per_tick(&self) -> u64 {
        self.nanos_per_beat() / (self.ticks_per_beat.get() as u64)
    }

    #[allow(dead_code)]
    pub const fn beat_duration(&self) -> Duration {
        Duration::from_nanos(self.nanos_per_beat())
    }

    #[allow(dead_code)]
    pub const fn tick_duration(&self) -> Duration {
        Duration::from_nanos(self.nanos_per_tick())
    }
}

impl Default for BpmInfo {
    fn default() -> Self {
        BpmInfo {
            beats_per_minute: NonZeroU16::new(120).unwrap(),
            ticks_per_beat: NonZeroU16::new(32).unwrap(),
        }
    }
}
