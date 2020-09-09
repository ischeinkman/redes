use jack::{Client, ClientOptions, MidiIn, MidiOut, ProcessScope, RawMidi};
use std::fmt::{self, Formatter, LowerHex, UpperHex};
use std::sync::mpsc;
use std::time::Duration;
mod midi;
mod model;
use model::NoteKey;
mod utils;
use midi::{parse_midimessage, MidiChannel, MidiMessage, MidiNote, NoteOff, NoteOn, PressVelocity};
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

const SECONDS_PER_MIN: u64 = 60;
const MILLIS_PER_SECOND: u64 = 1000;
const MICROS_PER_MILLI: u64 = 1000;
const NANOS_PER_MICRO: u64 = 1000;

const MILLIS_PER_MIN: u64 = MILLIS_PER_SECOND * SECONDS_PER_MIN;
const MICROS_PER_MIN: u64 = MICROS_PER_MILLI * MILLIS_PER_MIN;
const NANOS_PER_MIN: u64 = NANOS_PER_MICRO * MICROS_PER_MIN;

const fn bpm_to_beat_time(bpm: u64) -> Duration {
    let nanos = NANOS_PER_MIN / bpm;
    Duration::from_nanos(nanos)
}

fn main() {
    let beat_time = bpm_to_beat_time(120);

    #[cfg(feature = "rt-alloc-panic")]
    eprintln!("RT-ALLOC-PANIC was enabled: will panic if the realtime thread allocates.");

    let (client, _status) = Client::new("Midi Test 1", ClientOptions::NO_START_SERVER).unwrap();
    let mut out = client
        .register_port("Midi Output 1", MidiOut::default())
        .unwrap();

    let inp = client
        .register_port("Midi Input 1", MidiIn::default())
        .unwrap();
    let mut state = NoteState::default();

    let (send, recv) = mpsc::sync_channel(1024);
    let cb = move |client: &Client, ps: &ProcessScope| {
        #[cfg(feature = "rt-alloc-panic")]
        malloc::MYALLOC.set_rt();
        let mut outcon = out.writer(ps);
        let mut new_state = state;
        for rawdata in inp.iter(ps) {
            let buff = rawdata.bytes;
            let mut owned_buff = [0; 3];
            let bufflen = buff.len();
            let cplen = bufflen.min(owned_buff.len());
            (&mut owned_buff).copy_from_slice(&buff[..cplen]);
            let parsed = parse_midimessage(owned_buff);
            match parsed {
                Ok(midi::MidiMessage::NoteOn(data)) if data.channel() == MidiChannel::all()[0] => {
                    new_state = new_state.with_press(data.note());
                }
                Ok(midi::MidiMessage::NoteOff(data)) if data.channel() == MidiChannel::all()[0] => {
                    new_state = new_state.with_release(data.note());
                }
                Ok(_) => {
                    outcon.write(&rawdata).unwrap();
                }
                _ => {}
            }
        }
        let (cur_usecs, nxt_usecs) = ps
            .cycle_times()
            .map(|data| (data.current_usecs, data.next_usecs))
            .unwrap_or_else(|_| {
                let cur_frames = ps.last_frame_time();
                let nxt_frames = cur_frames + ps.n_frames();
                let cur_usecs = client.frames_to_time(cur_frames);
                let nxt_usecs = client.frames_to_time(nxt_frames);
                (cur_usecs, nxt_usecs)
            });
        let cur_beat = (cur_usecs as u128) / beat_time.as_micros();
        let nxt_beat = (nxt_usecs as u128) / beat_time.as_micros();

        let released = state.notes().filter(|n| !new_state.is_pressed(*n));
        for released_note in released {
            let beat_offset = cur_beat % 3;
            let key = NoteKey::major(released_note.note());
            let arped_class = key.nth((2 * beat_offset) as isize);
            let mut arped = MidiNote::from_note_octave(arped_class, released_note.octave());
            if arped < released_note {
                arped = arped.wrapping_add(12);
            }
            let noteoff = NoteOff::new(
                MidiChannel::all()[0],
                arped,
                PressVelocity::from_raw(0).unwrap(),
            );
            let msg = RawMidi {
                time: 0,
                bytes: &noteoff.as_bytes(),
            };
            outcon.write(&msg).unwrap();
            send.send(MidiMessage::from(noteoff)).unwrap();
        }

        if cur_beat >= nxt_beat {
            #[cfg(feature = "rt-alloc-panic")]
            malloc::MYALLOC.unset_rt();
            state = new_state;
            return jack::Control::Continue;
        }

        let mut touchmask = 0u128;
        for base in new_state.notes() {
            let key = NoteKey::major(base.note());

            let prev_offset = cur_beat % 3;
            let prev_class = key.nth((2 * prev_offset) as isize);
            let mut prev_note = MidiNote::from_note_octave(prev_class, base.octave());
            if prev_note < base {
                prev_note = prev_note.wrapping_add(12);
            }
            let prev_mask = 1u128 << prev_note.as_u8();
            if prev_mask & touchmask == 0 {
                let prev_noteoff = NoteOff::new(
                    MidiChannel::all()[0],
                    prev_note,
                    PressVelocity::from_raw(90).unwrap(),
                );
                let prev_time = 0; //TODO
                let prev_msg = RawMidi {
                    time: prev_time,
                    bytes: &prev_noteoff.as_bytes(),
                };
                outcon.write(&prev_msg).unwrap();
                send.send(prev_noteoff.into()).unwrap();
            }

            let nxt_offset = nxt_beat % 3;
            let nxt_class = key.nth((2 * nxt_offset) as isize);
            let mut nxt_note = MidiNote::from_note_octave(nxt_class, base.octave());
            if nxt_note < base {
                nxt_note = nxt_note.wrapping_add(12);
            }
            let nxt_mask = 1u128 << nxt_note.as_u8();
            touchmask |= nxt_mask;
            let nxt_noteon = NoteOn::new(
                MidiChannel::all()[0],
                nxt_note,
                PressVelocity::from_raw(90).unwrap(),
            );
            let nxt_time = 0; //TODO
            let nxt_msg = RawMidi {
                time: nxt_time,
                bytes: &nxt_noteon.as_bytes(),
            };
            outcon.write(&nxt_msg).unwrap();
            send.send(nxt_noteon.into()).unwrap();
        }

        #[cfg(feature = "rt-alloc-panic")]
        malloc::MYALLOC.unset_rt();
        state = new_state;
        jack::Control::Continue
    };
    let active_client = client
        .activate_async((), jack::ClosureProcessHandler::new(cb))
        .unwrap();
    loop {
        let msg = recv.recv().unwrap();
        println!("{} => {:?}", active_client.as_client().frame_time(), msg);
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
