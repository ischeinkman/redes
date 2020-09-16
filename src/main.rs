use jack::{Client, ClientOptions, MidiOut, ProcessScope};
use std::fmt::{self, Formatter, LowerHex, UpperHex};
use std::time::Duration;

mod midi;
mod model;
mod track;
mod utils;
use midi::MidiNote;
use track::*;
pub use utils::*;

#[cfg(feature = "rt-alloc-panic")]
mod malloc;

#[inline(always)]
const fn mask(note: MidiNote) -> u128 {
    let bt = note.as_u8();
    1 << bt
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Default)]
pub struct NoteState {
    data: u128,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DifferenceSide {
    Left,
    Right,
}

impl NoteState {
    pub const fn new() -> Self {
        Self { data: 0 }
    }
    pub const fn is_pressed(&self, note: MidiNote) -> bool {
        self.data & mask(note) != 0
    }
    pub const fn with_press(mut self, note: MidiNote) -> NoteState {
        self.data |= mask(note);
        self
    }
    pub const fn with_release(mut self, note: MidiNote) -> NoteState {
        let press_mask = mask(note);
        let release_mask = !press_mask;
        self.data &= release_mask;
        self
    }
    pub const fn with_toggled(mut self, note: MidiNote) -> NoteState {
        let mask = mask(note);
        self.data ^= mask;
        self
    }
    pub fn notes<'a>(&'a self) -> impl Iterator<Item = MidiNote> + 'a {
        (0..128)
            .filter_map(MidiNote::from_raw)
            .filter(move |&note| self.is_pressed(note))
    }

    pub fn difference<'a>(
        &'a self,
        other: &'a NoteState,
    ) -> impl Iterator<Item = (MidiNote, DifferenceSide)> + 'a {
        (0..128)
            .filter_map(MidiNote::from_raw)
            .filter(move |&note| self.is_pressed(note) != other.is_pressed(note))
            .map(move |note| {
                if self.is_pressed(note) {
                    (note, DifferenceSide::Left)
                } else {
                    (note, DifferenceSide::Right)
                }
            })
    }
}

fn main() {
    let track = Vec::new();

    let mut cursor = TrackCursor::new(track);

    #[cfg(feature = "rt-alloc-panic")]
    eprintln!("RT-ALLOC-PANIC was enabled: will panic if the realtime thread allocates.");

    let (client, _status) = Client::new("Midi Test 1", ClientOptions::NO_START_SERVER).unwrap();
    let mut out = client
        .register_port("Midi Output 1", MidiOut::default())
        .unwrap();

    let mut start_usecs = None;
    let cb = move |client: &Client, ps: &ProcessScope| {
        #[cfg(feature = "rt-alloc-panic")]
        malloc::MYALLOC.set_rt();
        let mut outcon = out.writer(ps);

        let (cur_frames, cur_usecs, nxt_usecs) = ps
            .cycle_times()
            .map(|data| (data.current_frames, data.current_usecs, data.next_usecs))
            .unwrap_or_else(|_| {
                let cur_frames = ps.last_frame_time();
                let nxt_frames = cur_frames + ps.n_frames();
                let cur_usecs = client.frames_to_time(cur_frames);
                let nxt_usecs = client.frames_to_time(nxt_frames);
                (cur_frames, cur_usecs, nxt_usecs)
            });

        let start_time = Duration::from_micros(*start_usecs.get_or_insert(cur_usecs));
        let cur_time = Duration::from_micros(cur_usecs)
            .checked_sub(start_time)
            .unwrap_or_default();
        let nxt_time = Duration::from_micros(nxt_usecs)
            .checked_sub(start_time)
            .unwrap_or_default();
        for evt in cursor.events_in_range(cur_time, nxt_time) {
            let (time, msg) = evt;
            let sys_time = (time.as_micros() + start_time.as_micros()) as u64;
            let sys_frames = client.time_to_frames(sys_time);
            let frame_offset = sys_frames.saturating_sub(cur_frames);
            let rawmsg = msg.as_raw();
            let outdata = jack::RawMidi {
                time: frame_offset,
                bytes: rawmsg.bytes(),
            };
            outcon.write(&outdata).unwrap();
        }

        #[cfg(feature = "rt-alloc-panic")]
        malloc::MYALLOC.unset_rt();
        jack::Control::Continue
    };
    let _active_client = client
        .activate_async((), jack::ClosureProcessHandler::new(cb))
        .unwrap();
    loop {
        std::thread::sleep(Duration::from_millis(1000));
    }
}

pub struct ByteWrapper<S: AsRef<[u8]> + ?Sized> {
    inner: S,
}
impl<S: AsRef<[u8]>> ByteWrapper<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S: AsRef<[u8]> + ?Sized> UpperHex for ByteWrapper<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let sls = self.inner.as_ref();
        let (head, tail) = match sls.split_first() {
            Some(inner) => inner,
            None => {
                return f.write_str("[]");
            }
        };
        f.write_fmt(format_args!("[ 0x{:X}", head))?;
        for bt in tail {
            f.write_fmt(format_args!(", 0x{:X}", bt))?;
        }
        f.write_str(" ]")?;
        Ok(())
    }
}

impl<S: AsRef<[u8]> + ?Sized> LowerHex for ByteWrapper<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let sls = self.inner.as_ref();
        let (head, tail) = match sls.split_first() {
            Some(inner) => inner,
            None => {
                return f.write_str("[]");
            }
        };
        f.write_fmt(format_args!("[ 0x{:x}", head))?;
        for bt in tail {
            f.write_fmt(format_args!(", 0x{:x}", bt))?;
        }
        f.write_str(" ]")?;
        Ok(())
    }
}
