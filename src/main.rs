use jack::{Client, ClientOptions, MidiOut, ProcessScope};
use std::collections::HashMap;
use std::env::args;
use std::fmt::{self, Formatter, LowerHex, UpperHex};
use std::fs::OpenOptions;
use std::io::Read;
use std::time::Duration;

mod midi;
mod model;
mod songlang;
use songlang::{parse_file, AsmCommand, LangItem};
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
    let parsed = args().nth(1).map(|path| {
        let mut file = OpenOptions::new().read(true).open(path).unwrap();
        let mut input = String::new();
        file.read_to_string(&mut input).unwrap();

        let (_, res) = parse_file(&input)
            .map_err(|e| match e {
                nom::Err::Error(e) | nom::Err::Failure(e) => format!(
                    "Parse error: {}\n\nRaw:\n{:?}",
                    nom::error::convert_error(&input, e.clone()),
                    e
                ),
                nom::Err::Incomplete(ic) => format!("Incomplete: {:?}", ic),
            })
            .map_err(|e| {
                eprintln!("ERROR: {}\n\n\n===========", e);
                e
            })
            .unwrap();
        res
    });
    let parsed = parsed.unwrap_or_default();
    for (idx, instr) in parsed.iter().enumerate() {
        println!("{:02} : {:?}", idx, instr);
    }
    let mut track = Vec::new();
    let mut labels = HashMap::new();
    let mut jumps_to_labels = HashMap::new();
    for instr in parsed.iter() {
        match instr {
            LangItem::Asm(AsmCommand::Wait(dur)) => {
                let evt = TrackEvent::Wait(*dur);
                track.push(evt);
            }
            LangItem::Asm(AsmCommand::Label(name)) => {
                assert!(labels.insert(name, track.len()).is_none());
            }
            LangItem::Asm(AsmCommand::Jump { label, count }) => {
                let cmd = TrackEvent::Jump {
                    target: 0,
                    count: *count,
                };
                jumps_to_labels.insert(track.len(), label);
                track.push(cmd);
            }
            LangItem::Asm(AsmCommand::SetBpm(bpm)) => {
                let cmd = TrackEvent::SetBpm(*bpm);
                track.push(cmd);
            }
            LangItem::Asm(AsmCommand::Send(msg)) => {
                let cmd = TrackEvent::SendMessage(*msg);
                track.push(cmd);
            }
            other => {
                todo!("Instr {:?} not yet implemented.", other);
            }
        }
    }
    track.push(TrackEvent::End);

    for (idx, label) in jumps_to_labels {
        let actual_target = labels.get(label).cloned().unwrap();
        if let Some(TrackEvent::Jump { ref mut target, .. }) = track.get_mut(idx) {
            *target = actual_target;
        } else {
            panic!("Error in label assignments; had mapping of {} => {}, but the event at IDX {} is {:?}.", idx, label, 
            idx, track.get(idx));
        }
    }

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
