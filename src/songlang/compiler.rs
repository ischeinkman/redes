use super::ast::{AsmCommand, ChordKind, LangItem, OutputLabel, PressLine, SongAttribute};
use crate::midi::{MidiChannel, MidiMessage, MidiNote, NoteOn, PressVelocity};
use crate::model::NoteKey;
use crate::track::{BpmInfo, OutputPort, TrackEvent, WaitTime};
use crate::utils::ONE_NZU16;
use std::collections::HashMap;
use std::num::NonZeroU16;
use thiserror::*;

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("Could not find jump target label {0:?}.")]
    LabelNotFound(String),

    #[error("Duplicate label in track: label {label:?} was declared to target both event {first_location} and event {second_location}.")]
    DuplicateLabel {
        label: String,
        first_location: usize,
        second_location: usize,
    },

    #[error("Attribute {0:?} was encountered outside of the song header.")]
    AttributeOutsideHeader(SongAttribute),

    #[error("Attribute set multiple times: encountered {0:?} and {1:?}")]
    DuplicateAttributes(SongAttribute, SongAttribute),
}

#[derive(Debug, Eq, PartialEq, Clone, Default)]
struct SongAttributes {
    press_dur: Option<WaitTime>,
    press_vel: Option<PressVelocity>,
    bpm: Option<BpmInfo>,
    channel: Option<MidiChannel>,
    outport: Option<OutputLabel>,
}

impl SongAttributes {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default_duration(&self) -> WaitTime {
        self.press_dur.unwrap_or(WaitTime::Ticks(ONE_NZU16))
    }
    pub fn default_velocity(&self) -> PressVelocity {
        self.press_vel
            .unwrap_or_else(|| PressVelocity::from_raw(90).unwrap())
    }
    pub fn default_channel(&self) -> MidiChannel {
        self.channel.unwrap_or_default()
    }
    pub fn default_port(&self) -> Option<OutputLabel> {
        self.outport.clone()
    }

    #[allow(dead_code)]
    pub fn default_bpm(&self) -> BpmInfo {
        self.bpm.unwrap_or_default()
    }

    pub fn push_attribute(&mut self, attr: SongAttribute) -> Result<(), CompilerError> {
        match attr {
            SongAttribute::DefaultDuration(dur) => {
                if let Some(prev) = self.press_dur {
                    return Err(CompilerError::DuplicateAttributes(
                        SongAttribute::DefaultDuration(prev),
                        SongAttribute::DefaultDuration(dur),
                    ));
                }
                self.press_dur = Some(dur);
                Ok(())
            }
            SongAttribute::DefaultPressVelocity(vel) => {
                if let Some(prev) = self.press_vel {
                    return Err(CompilerError::DuplicateAttributes(
                        SongAttribute::DefaultPressVelocity(prev),
                        SongAttribute::DefaultPressVelocity(vel),
                    ));
                }
                self.press_vel = Some(vel);
                Ok(())
            }
            SongAttribute::Signature(bpm) => {
                if let Some(prev) = self.bpm {
                    return Err(CompilerError::DuplicateAttributes(
                        SongAttribute::Signature(prev),
                        SongAttribute::Signature(bpm),
                    ));
                }
                self.bpm = Some(bpm);
                Ok(())
            }
            SongAttribute::DefaultPort(outport) => {
                if let Some(prev) = self.outport.as_ref() {
                    return Err(CompilerError::DuplicateAttributes(
                        SongAttribute::DefaultPort(prev.clone()),
                        SongAttribute::DefaultPort(outport),
                    ));
                }
                self.outport = Some(outport);
                Ok(())
            }
            SongAttribute::DefaultChannel(chan) => {
                if let Some(prev) = self.channel {
                    return Err(CompilerError::DuplicateAttributes(
                        SongAttribute::DefaultChannel(prev),
                        SongAttribute::DefaultChannel(chan),
                    ));
                }
                self.channel = Some(chan);
                Ok(())
            }
        }
    }
}
#[derive(Debug, Eq, PartialEq, Clone, Default)]
struct Compiler {
    attributes: SongAttributes,

    tick_spans: Vec<(usize, WaitTime)>,
    jump_fix_backlog: HashMap<usize, String>,

    ports: HashMap<Option<OutputLabel>, OutputPort>,
    labels: HashMap<String, usize>,

    track: Vec<TrackEvent>,
}

pub type PortList = HashMap<Option<OutputLabel>, OutputPort>;

pub fn compile_song(song: Vec<LangItem>) -> Result<(Vec<TrackEvent>, PortList), CompilerError> {
    let mut compiler = Compiler::new();
    for itm in song {
        compiler.compile_item(itm)?;
    }
    compiler.track.push(TrackEvent::End);
    compiler.resolve_jumps()?;
    compiler.resolve_tickspans()?;
    Ok((compiler.track, compiler.ports))
}

impl Compiler {
    pub fn new() -> Self {
        Self::default()
    }

    fn port_label_to_idx(&mut self, port: Option<OutputLabel>) -> OutputPort {
        let defaultidx = self.ports.len().into();
        let port = *self.ports.entry(port).or_insert(defaultidx);
        port
    }

    #[allow(unused)]
    fn resolve_tickspans(&mut self) -> Result<(), CompilerError> {
        self.tick_spans
            .sort_by_key(|(idx, _)| (*idx as i128).saturating_neg());
        while let Some((next_instr_idx, next_span)) = self.tick_spans.pop() {
            todo!()
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn insert_event(&mut self, idx: usize, event: TrackEvent) {
        self.track.insert(idx, event);
        for instr in self.track.iter_mut() {
            if let TrackEvent::Jump { target, .. } = instr {
                if *target >= idx {
                    *target = target.wrapping_add(1);
                }
            }
        }
        for (curidx, _) in self.tick_spans.iter_mut() {
            if *curidx >= idx {
                *curidx += 1;
            }
        }
    }

    fn resolve_jumps(&mut self) -> Result<(), CompilerError> {
        for (instr_idx, lbl) in self.jump_fix_backlog.drain() {
            let new_target = self
                .labels
                .get(&lbl)
                .copied()
                .ok_or_else(|| CompilerError::LabelNotFound(lbl.clone()))?;
            match self.track.get_mut(instr_idx) {
                Some(TrackEvent::Jump { target, .. }) => {
                    *target = new_target;
                }
                other => {
                    todo!(
                        "Bad jump resolution: {:?} / {:?} => {:?}",
                        instr_idx,
                        lbl,
                        other
                    );
                }
            }
        }
        Ok(())
    }

    fn encounter_setlabel(&mut self, lbl: String) -> Result<(), CompilerError> {
        let prev = self.labels.get(&lbl).copied();
        let target = self.track.len();
        match prev {
            Some(p) => Err(CompilerError::DuplicateLabel {
                label: lbl,
                first_location: p,
                second_location: target,
            }),
            None => {
                self.labels.insert(lbl, target);
                Ok(())
            }
        }
    }

    fn encounter_pressline(&mut self, data: PressLine) -> Result<(), CompilerError> {
        let line_duration = data.duration();
        let line_vel = data.velocity();
        let line_channel = data.channel();
        let line_port = data.port().cloned();
        for press in data.presses {
            let channel = press
                .channel()
                .or(line_channel)
                .unwrap_or_else(|| self.attributes.default_channel());

            let vel = press
                .velocity()
                .or(line_vel)
                .unwrap_or_else(|| self.attributes.default_velocity());

            let duration = press
                .duration()
                .or(line_duration)
                .unwrap_or_else(|| self.attributes.default_duration());

            let port = press
                .port()
                .cloned()
                .or_else(|| line_port.clone())
                .or_else(|| self.attributes.default_port());

            let port = self.port_label_to_idx(port);
            let offsets: &[_] = match press.kind {
                ChordKind::Raw => &[0],
                ChordKind::Fifth => &[0, 4],
                ChordKind::Major | ChordKind::Minor => &[0, 2, 4],
                ChordKind::Major7 | ChordKind::Minor7 => &[0, 2, 4, 7],
            };
            let key = match press.kind {
                ChordKind::Minor | ChordKind::Minor7 => NoteKey::minor(press.root),
                ChordKind::Major | ChordKind::Major7 | ChordKind::Raw | ChordKind::Fifth => {
                    NoteKey::major(press.root)
                }
            };
            let root_pitch = MidiNote::from_note_octave(press.root, press.octave);
            let mut prev_pitch = root_pitch;
            for offset in offsets {
                let mut cur_pitch = MidiNote::from_note_octave(key.nth(*offset), press.octave);
                if cur_pitch < prev_pitch {
                    cur_pitch = cur_pitch.wrapping_add(12);
                }
                let noteon = NoteOn::new(channel, cur_pitch, vel);
                let evt = TrackEvent::SendMessage {
                    message: MidiMessage::from(noteon),
                    port,
                };
                self.tick_spans.push((self.track.len(), duration));
                self.track.push(evt);
                prev_pitch = cur_pitch;
            }
        }
        let line_wait = TrackEvent::Wait(WaitTime::Ticks(ONE_NZU16));
        self.track.push(line_wait);
        Ok(())
    }

    fn encounter_loop(
        &mut self,
        rawcount: Option<NonZeroU16>,
        body: Vec<LangItem>,
    ) -> Result<(), CompilerError> {
        let count = rawcount.map(|n| {
            n.get()
                .checked_sub(1)
                .and_then(NonZeroU16::new)
                .unwrap_or(ONE_NZU16)
        });
        let target = self.track.len();
        for itm in body {
            self.compile_item(itm)?;
        }
        let jmp = TrackEvent::Jump { target, count };
        self.track.push(jmp);
        Ok(())
    }

    fn encounter_jump(
        &mut self,
        count: Option<NonZeroU16>,
        label: String,
    ) -> Result<(), CompilerError> {
        let target_opt = self.labels.get(&label).copied();
        let target = target_opt.unwrap_or_else(|| {
            self.jump_fix_backlog.insert(self.track.len(), label);
            usize::max_value()
        });
        let evt = TrackEvent::Jump { count, target };
        self.track.push(evt);
        Ok(())
    }

    fn encounter_setattr(&mut self, attr: SongAttribute) -> Result<(), CompilerError> {
        let new_bpm = match attr {
            SongAttribute::Signature(bpm) => Some(bpm),
            _ => None,
        };
        match self.track.first() {
            None => {
                self.attributes.push_attribute(attr)?;
                if let Some(bpm) = new_bpm {
                    self.track.push(TrackEvent::SetBpm(bpm));
                }
                Ok(())
            }
            Some(TrackEvent::SetBpm(old_bpm)) if new_bpm == Some(*old_bpm) => Ok(()),
            Some(TrackEvent::SetBpm(_)) if new_bpm.is_none() => {
                self.attributes.push_attribute(attr)?;
                Ok(())
            }
            Some(_) => Err(CompilerError::AttributeOutsideHeader(attr)),
        }
    }

    pub fn compile_item(&mut self, item: LangItem) -> Result<(), CompilerError> {
        match item {
            LangItem::Loop { repititions, expr } => {
                self.encounter_loop(repititions, expr)?;
                Ok(())
            }
            LangItem::NotePress(data) => {
                self.encounter_pressline(data)?;
                Ok(())
            }
            LangItem::Wait(dur) | LangItem::Asm(AsmCommand::Wait(dur)) => {
                let evt = TrackEvent::Wait(dur);
                self.track.push(evt);
                Ok(())
            }
            LangItem::Asm(AsmCommand::SetBpm(bpm)) => {
                let evt = TrackEvent::SetBpm(bpm);
                self.track.push(evt);
                Ok(())
            }
            LangItem::Asm(AsmCommand::Label(lbl)) => {
                self.encounter_setlabel(lbl)?;
                Ok(())
            }
            LangItem::Asm(AsmCommand::Send { message, port }) => {
                let port = self.port_label_to_idx(port);
                let evt = TrackEvent::SendMessage { message, port };
                self.track.push(evt);
                Ok(())
            }
            LangItem::Asm(AsmCommand::Jump { count, label }) => {
                self.encounter_jump(count, label)?;
                Ok(())
            }
            LangItem::SetAttribute(attr) => {
                self.encounter_setattr(attr)?;
                Ok(())
            }
            #[allow(unreachable_patterns)]
            other => todo!("LangItem not implemented: {:?}", other),
        }
    }
}
