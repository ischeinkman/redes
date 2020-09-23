use jack::{Client, ClientOptions, MidiOut, ProcessScope};
use nom::error::convert_error as convert_nom_error;
use nom::Err as NomErr;
use std::collections::HashMap;
use std::env::args;
use std::fs::OpenOptions;
use std::io::BufRead;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::*;

use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;

mod midi;
use midi::{MidiChannel, MidiMessage, MidiNote, NoteOn, PressVelocity};
mod model;
mod songlang;
use songlang::{compile_song, parse_file, LangItem, PortList};
mod track;
mod utils;
use track::*;
pub use utils::*;

#[cfg(feature = "rt-alloc-panic")]
mod malloc;

pub type PortIdent = (usize, OutputPort);

#[derive(Debug, Error)]
pub enum MyError {
    #[error(transparent)]
    Jack(#[from] jack::Error),
    #[error("Could not send message to Port ID {0:?}: Not Found.")]
    InvalidPortId(PortIdent),
    #[error(transparent)]
    Compiler(#[from] crate::songlang::CompilerError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parser(String),
}

impl From<String> for MyError {
    fn from(parse_err: String) -> Self {
        MyError::Parser(parse_err)
    }
}

fn get_tracks() -> impl Iterator<Item = (String, Result<Vec<LangItem>, MyError>)> {
    TuplerIter::new(args().skip(1), |raw_path| {
        let trimmed_path = raw_path.trim();
        let mut fh = OpenOptions::new().read(true).open(trimmed_path)?;
        let mut buff = String::new();
        fh.read_to_string(&mut buff)?;
        let (out, res) = parse_file(&buff).map_err(|e| match e {
            NomErr::Error(e) | NomErr::Failure(e) => format!(
                "Parse error: {}\n\nRaw:\n{:?}",
                convert_nom_error(&buff, e.clone()),
                e
            ),
            NomErr::Incomplete(ic) => format!("Incomplete: {:?}", ic),
        })?;

        if !out.trim().is_empty() {
            return Err(MyError::Parser(format!(
                "Could not parse full file. Data: {:?}, Rest: {:?}",
                &res, &out
            )));
        }

        Ok(res)
    })
}

fn scope_range(
    client: &Client,
    ps: &ProcessScope,
) -> ((jack::Frames, jack::Time), (jack::Frames, jack::Time)) {
    let ps_frame_len = ps.n_frames();
    let cycle_res = ps.cycle_times().map(|data| {
        let cur = (data.current_frames, data.current_usecs);
        let nxt = (data.current_frames + ps_frame_len, data.next_usecs);
        (cur, nxt)
    });
    cycle_res.unwrap_or_else(|_| {
        let cur_frames = ps.last_frame_time();
        let nxt_frames = cur_frames + ps.n_frames();
        let cur_usecs = client.frames_to_time(cur_frames);
        let nxt_usecs = client.frames_to_time(nxt_frames);

        ((cur_frames, cur_usecs), (nxt_frames, nxt_usecs))
    })
}

fn make_writer_allocator(num_writers: usize) -> Bump {
    const BYTES_PADDING: usize = 0;
    const ELM_PADDING: usize = 2;
    let elm_size = std::mem::size_of::<(OutputPort, jack::MidiWriter<'static>)>();
    let allocation_size = (num_writers + ELM_PADDING) * elm_size + BYTES_PADDING;
    Bump::with_capacity(allocation_size)
}

fn initialize_client<I: IntoIterator<Item = PortList>>(
    all_ports: I,
) -> Result<(jack::Client, HashMap<PortIdent, jack::Port<MidiOut>>), MyError> {
    let (client, _status) = Client::new("Midi Test 1", ClientOptions::NO_START_SERVER)?;

    // Necessary to avoid dynamic symbol resolution in the hotloop.
    // Safe-ish since while we technically do not actually verify safety variants/invariants
    // aside from the fact that the client pointer is alive (IE we verify nothing about the
    // frame time, client state aside from liveness, etc), we are really only resolving a dynamic
    // symbol and discarding the result.
    unsafe {
        let ps = ProcessScope::from_raw(client.frame_time(), client.raw());
        let _ = ps.cycle_times();
    }

    let mut port_iter = all_ports
        .into_iter()
        .enumerate()
        .flat_map(|(idx, iter)| iter.into_iter().map(move |(label, id)| (idx, label, id)))
        .peekable();

    let mut jack_resolver: HashMap<PortIdent, jack::Port<MidiOut>> = HashMap::new();
    let has_multiple = port_iter.peek().is_some();
    for (track, label, id) in port_iter {
        let mapped_label = match (label, has_multiple) {
            (None, false) => ":1".to_owned(),
            (None, true) => format!("track_{}:1", track),
            (Some(lbl), false) => lbl.as_ref().to_owned(),
            (Some(lbl), true) => format!("track_{}:{}", track, lbl.as_ref()),
        };
        let port = client.register_port(&mapped_label, MidiOut::default())?;
        jack_resolver.insert((track, id), port);
    }
    Ok((client, jack_resolver))
}

fn send_alloff(writers: &mut BumpVec<(PortIdent, jack::MidiWriter)>) -> Result<(), MyError> {
    let mut cur_res = Ok(());
    for writer in writers.iter_mut().map(|(_, v)| v) {
        for note in MidiNote::all() {
            let msg = MidiMessage::NoteOn(NoteOn::new(
                MidiChannel::default(),
                *note,
                PressVelocity::from_raw(0).unwrap(),
            ))
            .as_raw();
            let wrapped_msg = jack::RawMidi {
                time: 0,
                bytes: &msg.bytes(),
            };
            if let Err(e) = writer.write(&wrapped_msg) {
                cur_res = Err(e.into());
            }
        }
    }
    cur_res
}

fn main() {
    let (tracks, ports) = get_tracks()
        .map(|(file, res)| {
            (
                file,
                res.and_then(|r| compile_song(r).map_err(|e| e.into())),
            )
        })
        .fold(
            (Vec::new(), Vec::new()),
            |(mut tracks, mut ports), (cur_file, res)| {
                let (cur_track, cur_ports) = match res {
                    Ok(data) => data,
                    Err(e) => {
                        panic!("Error in file {:?} : {}", cur_file, e);
                    }
                };
                tracks.push(TrackCursor::new(cur_track));
                ports.push(cur_ports);
                (tracks, ports)
            },
        );
    let mut cursor = VecMultiCursor::new(tracks);
    let (client, mut outs) = initialize_client(ports).unwrap();

    #[cfg(feature = "rt-alloc-panic")]
    eprintln!("RT-ALLOC-PANIC was enabled: will panic if the realtime thread allocates.");

    let mut start_usecs = None;

    let mut writer_allocator = make_writer_allocator(outs.len());

    let flags = Arc::new((AtomicBool::new(false), AtomicBool::new(false)));
    let flagref = Arc::clone(&flags);
    let cb = move |client: &Client, ps: &ProcessScope| {
        #[cfg(feature = "rt-alloc-panic")]
        malloc::MYALLOC.set_rt();

        let ((cur_frames, cur_usecs), (_nxt_frames, nxt_usecs)) = scope_range(client, ps);

        let writer_iter = outs.iter_mut().map(|(id, port)| (*id, port.writer(ps)));
        let mut writers = BumpVec::from_iter_in(writer_iter, &writer_allocator);

        let should_restart = flagref.1.compare_and_swap(true, false, Ordering::AcqRel);
        if should_restart {
            send_alloff(&mut writers).unwrap();
            cursor.reset();
            start_usecs = None;
        }

        let is_paused = flagref.0.load(Ordering::Acquire);
        if is_paused {
            if let Some(start_usecs) = start_usecs.as_mut() {
                *start_usecs += nxt_usecs - cur_usecs;
            }

            #[cfg(feature = "rt-alloc-panic")]
            malloc::MYALLOC.unset_rt();
            return jack::Control::Continue;
        }

        let start_time = Duration::from_micros(*start_usecs.get_or_insert(cur_usecs));
        let nxt_time = Duration::from_micros(nxt_usecs)
            .checked_sub(start_time)
            .unwrap_or_default();

        for evt in cursor.step_until(nxt_time) {
            let (time, port, msg) = evt;

            let sys_time = (time.as_micros() + start_time.as_micros()) as u64;
            let sys_frames = client.time_to_frames(sys_time);
            let frame_offset = sys_frames.saturating_sub(cur_frames);

            let rawmsg = msg.as_raw();
            let outdata = jack::RawMidi {
                time: frame_offset,
                bytes: rawmsg.bytes(),
            };

            let outcon = writers
                .iter_mut()
                .find(|(id, _)| id == &port)
                .map(|(_, writer)| writer)
                .ok_or_else(|| MyError::InvalidPortId(port))
                .unwrap();
            let write_res = outcon.write(&outdata).map_err(MyError::Jack);
            match write_res {
                Ok(_) => {}
                Err(MyError::Jack(jack::Error::NotEnoughSpace)) => {
                    #[cfg(feature = "rt-alloc-panic")]
                    malloc::MYALLOC.unset_rt();
                    todo!("Handle a backlog.");
                }
                Err(_) => {
                    #[cfg(feature = "rt-alloc-panic")]
                    malloc::MYALLOC.unset_rt();
                    write_res.unwrap();
                }
            }
        }
        drop(writers);
        writer_allocator.reset();
        #[cfg(feature = "rt-alloc-panic")]
        malloc::MYALLOC.unset_rt();

        jack::Control::Continue
    };
    let _active_client = client
        .activate_async((), jack::ClosureProcessHandler::new(cb))
        .unwrap();
    let inp = std::io::stdin();
    let mut inplock = inp.lock();
    loop {
        eprintln!("Hit top of loop.");
        let mut line = String::new();
        inplock.read_line(&mut line).unwrap();
        if line
            .trim()
            .starts_with(|c: char| c.eq_ignore_ascii_case(&'p'))
        {
            eprintln!("Hit pause.");
            let try1_res = flags.0.compare_and_swap(false, true, Ordering::AcqRel);
            if try1_res {
                eprintln!("Pause was true: setting to false.");
                flags.0.compare_and_swap(true, false, Ordering::AcqRel);
            } else {
                eprintln!("Pause was false: setting to true.");
            }
        } else if line
            .trim()
            .starts_with(|c: char| c.eq_ignore_ascii_case(&'r'))
        {
            eprintln!("Hit restart.");
            flags.1.store(true, Ordering::Release);
        } else if !line.trim().is_empty() {
            eprintln!("Bad cmd: {:?}", line);
        } else {
            eprintln!("Empty line.");
        }
    }
}
