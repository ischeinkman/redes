use crate::midi::MidiMessage;
use std::num::NonZeroU16;
use std::time::Duration;


/// All instructions the MIDI event track VM can run.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TrackEvent {
    /// Outputs a `MidiMessage` along the output port.
    SendMessage{
        message : MidiMessage,
        port : OutputPort,
    },
    /// Moves the internal clock forward by a constant
    /// time / number of beat ticks.
    Wait(WaitTime),
    /// Sets the current song timing information.
    SetBpm(BpmInfo),

    /// Jumps to another event in the track list.
    /// If `count` is `Some(n)`, then the jump acts as a `NOP`
    /// every `n` times this particular instruction is reached.
    /// This is useful for constructs such as fixed length loops. 
    Jump {
        target: usize,
        count: Option<NonZeroU16>,
    },

    /// Represents the end of the playback track. 
    /// If the VM reachs this instruction, it will not continue 
    /// past it at all. 
    End,
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


/// Represents a time that the VM will wait without performing an action.
/// 
/// Song playback oftentimes deals with two parallel ways of measuring time:
/// raw clock time and in beats. Sometimes both methods of keeping time need to be
/// combined in a single track, for example when dealing with looping raw audio 
/// files while manipulating BPM etc. This enum allows for both methods to be 
/// unified into a single API.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum WaitTime {

    /// A wait period measured in clock time.
    Clock(Duration),
    
    /// A wait period measured in beats.
    Beats(NonZeroU16), 

    /// A wait period measured in beat "ticks".
    Ticks(NonZeroU16),

}

impl WaitTime {

    /// Converts this waiting period to beat "ticks", as defined by the provided `bpm_info`. 
    #[allow(dead_code)]
    pub const fn as_ticks(&self, bpm_info: BpmInfo) -> NonZeroU16 {
        match *self {
            WaitTime::Ticks(ticks) => ticks,
            WaitTime::Clock(dur) => {
                let nanos_per_tick = bpm_info.tick_duration().as_nanos();
                let self_nanos = dur.as_nanos();
                let ticks = self_nanos / nanos_per_tick;
                clamped_to_nonzerou16(ticks)
            }
            WaitTime::Beats(b) => {
                let raw = b.get() * bpm_info.ticks_per_beat.get();
                clamped_to_nonzerou16(raw as u128)
            }
        }
    }

    /// Converts this waiting period to raw clock time using the provided BPM information.
    pub const fn as_duration(&self, bpm_info: BpmInfo) -> Duration {
        match *self {
            WaitTime::Beats(b) => {
                let ticks = (bpm_info.ticks_per_beat.get() as u64) * (b.get() as u64);
                let nanos = (bpm_info.tick_duration().as_nanos() as u64) * ticks;
                Duration::from_nanos(nanos)
            }
            WaitTime::Clock(dur) => dur,
            WaitTime::Ticks(ticks) => Duration::from_nanos(
                (bpm_info.tick_duration().as_nanos() as u64) * (ticks.get() as u64),
            ),
        }
    }
}

/// Song timing information.
///
/// This struct contains everything needed to convert a time in beats
/// to a time in real clock seconds, as well as extra information on 
/// divisibility.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct BpmInfo {
    /// The number of full beats in a 60-second period.
    ///
    /// Defaults to 120.
    pub beats_per_minute: NonZeroU16,
    /// The number of "ticks" a single beat will be divided into.
    ///
    /// The smallest addressable beat unit is a single tick; for example, 
    /// if a track has a `ticks_per_beat` value of `4`, then each note press
    /// cannot be shorter than a quarter of a beat.
    /// Defaults to 32.
    pub ticks_per_beat: NonZeroU16,
}

const NANOS_PER_MINUTE: u64 = 60 * 1000 * 1000 * 1000;

impl BpmInfo {
    const fn nanos_per_beat(&self) -> u64 {
        NANOS_PER_MINUTE / (self.beats_per_minute.get() as u64)
    }
    const fn nanos_per_tick(&self) -> u64 {
        self.nanos_per_beat() / (self.ticks_per_beat.get() as u64)
    }

    /// The clock duration between the start of a 
    /// beat and the start of the next.
    #[allow(dead_code)]
    pub const fn beat_duration(&self) -> Duration {
        Duration::from_nanos(self.nanos_per_beat())
    }

    /// The clock duration between the start of a 
    /// beat tick and the start of the next.
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



#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash, Ord, PartialOrd)]
pub struct OutputPort {
    idx : u128, 
}

impl From<usize> for OutputPort {
    fn from(inner : usize) -> Self {
        OutputPort {
            idx : inner as u128
        }
    }
}