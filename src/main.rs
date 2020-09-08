use jack::{AsyncClient, Client, ClientOptions, MidiIn, MidiOut, ProcessScope, RawMidi};
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display, Formatter, LowerHex, UpperHex};
use std::sync::mpsc;
use std::sync::mpsc::{RecvError, TryRecvError, TrySendError};
use std::time::Duration;
mod midi;
mod model;
use model::{NoteKey};
mod utils;
use midi::{
    parse_midimessage, parse_noteoff, parse_noteon, MessageParseError, MidiChannel, MidiNote,
    NoteOn, RawMessage, 
};
pub use utils::*;

#[inline(always)]
const fn mask(note: u8) -> u128 {
    let bt = note as u8;
    1 << bt
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Default)]
pub struct NoteState {
    data: u128,
}

impl NoteState {
    pub const fn new() -> Self {
        Self { data: 0 }
    }
    pub const fn is_pressed(&self, note: u8) -> bool {
        self.data & mask(note) != 0
    }
    pub const fn with_press(mut self, note: u8) -> NoteState {
        self.data |= mask(note);
        self
    }
    pub const fn with_release(mut self, note: u8) -> NoteState {
        let press_mask = mask(note);
        let release_mask = !press_mask;
        self.data &= release_mask;
        self
    }
    pub const fn with_toggled(mut self, note: u8) -> NoteState {
        let mask = mask(note);
        self.data ^= mask;
        self
    }
    pub fn notes<'a>(&'a self) -> impl Iterator<Item = MidiNote> + 'a {
        (0..128).filter_map(move |raw| {
            if self.is_pressed(raw) {
                MidiNote::from_raw(raw)
            } else {
                None
            }
        })
    }
}

fn main() -> ! {
    let (client, _status) = Client::new("Midi Test 1", ClientOptions::NO_START_SERVER).unwrap();
    let mut out = client
        .register_port("Midi Output 1", MidiOut::default())
        .unwrap();

    let inp = client
        .register_port("Midi Input 1", MidiIn::default())
        .unwrap();
    let (msg_send, msg_recv) = mpsc::sync_channel(8);
    let mut fail_count = 0;
    let mut state = NoteState::default();
    let cb = move |_client: &Client, ps: &ProcessScope| {
        let mut new_state = state;
        let mut outcon = out.writer(ps);
        for rawdata in inp.iter(ps) {
            let buff = rawdata.bytes;
            let mut owned_buff: [u8; 3] = [0; 3];
            let bufflen = buff.len();
            let cplen = bufflen.min(owned_buff.len());
            (&mut owned_buff).copy_from_slice(&buff[..cplen]);

            let ts = ps.last_frame_time() + ps.n_frames();
            let parsed = parse_midimessage(owned_buff);
            match parsed {
                Ok(midi::MidiMessage::NoteOn(data)) => {
                    new_state = new_state.with_press(data.note().as_u8());
                    let major = NoteKey::major(data.note().note());
                    let mut major_3 = MidiNote::from_note_octave(major.nth(2), data.note().octave());
                    if major_3 < data.note() {
                        major_3 = major_3.wrapping_add(12);
                    }
                    let mut major_5 = MidiNote::from_note_octave(major.nth(4), data.note().octave());
                    if major_5 < major_3 {
                        major_5 = major_5.wrapping_add(12);
                    }
                    let major_3 = RawMidi {
                        time : rawdata.time, 
                        bytes : &data.with_note(major_3).as_bytes()
                    };
                    let major_5 = RawMidi {
                        time : rawdata.time, 
                        bytes : &data.with_note(major_5).as_bytes()
                    };
                    outcon.write(&rawdata).unwrap();
                    outcon.write(&major_3).unwrap();
                    outcon.write(&major_5).unwrap();

                }
                Ok(midi::MidiMessage::NoteOff(data)) => {
                    new_state = new_state.with_release(data.note().as_u8());
                    new_state = new_state.with_press(data.note().as_u8());
                    let major = NoteKey::major(data.note().note());
                    let mut major_3 = MidiNote::from_note_octave(major.nth(2), data.note().octave());
                    if major_3 < data.note() {
                        major_3 = major_3.wrapping_add(12);
                    }
                    let mut major_5 = MidiNote::from_note_octave(major.nth(4), data.note().octave());
                    if major_5 < major_3 {
                        major_5 = major_5.wrapping_add(12);
                    }
                    let major_3 = RawMidi {
                        time : rawdata.time, 
                        bytes : &data.with_note(major_3).as_bytes()
                    };
                    let major_5 = RawMidi {
                        time : rawdata.time, 
                        bytes : &data.with_note(major_5).as_bytes()
                    };
                    outcon.write(&rawdata).unwrap();
                    outcon.write(&major_3).unwrap();
                    outcon.write(&major_5).unwrap();
                }
                _ => {}
            }
            match msg_send.try_send((ts, owned_buff, bufflen, fail_count)) {
                Ok(()) => {
                    fail_count = 0;
                }
                Err(TrySendError::Full(_)) => {
                    fail_count += 1;
                }
                Err(TrySendError::Disconnected(_)) => {
                    panic!("Disconnected!");
                }
            }
            outcon.write(&rawdata).unwrap();
        }
        jack::Control::Continue
    };
    let _active_client = client
        .activate_async((), jack::ClosureProcessHandler::new(cb))
        .unwrap();
    let mut cur_worst_dt = 0;
    loop {
        match msg_recv.recv() {
            Ok((ts, buff, bufflen, fails)) => {
                let mf = _active_client.as_client().frame_time();
                let dt = i64::from(ts) - i64::from(mf);
                if dt > cur_worst_dt {
                    cur_worst_dt = dt;
                }
                println!();
                let prsed = parse_midimessage(buff);
                println!(
                    "{} ({}), {:?}, {:x}, {}, {:?}, {}, {}",
                    ts,
                    (i64::from(ts) - i64::from(mf)),
                    buff,
                    ByteWrapper::new(buff),
                    bufflen,
                    prsed,
                    fails,
                    if fails != 0 { "FAIL" } else { "" }
                );
            }
            Err(RecvError) => {
                panic!("Channel disconnected.");
            }
        }
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
