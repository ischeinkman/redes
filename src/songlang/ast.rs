use crate::midi::{MidiChannel, MidiMessage, PressVelocity};
use crate::model::{NoteClass, Octave};
use crate::track::{BpmInfo, WaitTime};

use std::num::NonZeroU16;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AsmCommand {
    Wait(WaitTime),
    Send {
        message: MidiMessage,
        port: Option<OutputLabel>,
    },
    Jump {
        label: String,
        count: Option<NonZeroU16>,
    },
    SetBpm(BpmInfo),
    Label(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct OutputLabel(String);

impl From<String> for OutputLabel {
    fn from(inner: String) -> Self {
        OutputLabel(inner)
    }
}

impl AsRef<str> for OutputLabel {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum LangItem {
    Loop {
        expr: Vec<LangItem>,
        repititions: Option<NonZeroU16>,
    },
    NotePress(PressLine),
    #[allow(dead_code)]
    Wait(WaitTime),
    Asm(AsmCommand),
    #[allow(dead_code)]
    SetAttribute(SongAttribute), 
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum PressModifier {
    Velocity(PressVelocity),
    Channel(MidiChannel),
    Duration(WaitTime),
    Port(OutputLabel),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PressLine {
    pub presses: Vec<ChordPress>,
    pub modifiers: Vec<PressModifier>,
}
impl PressLine {
    pub fn port(&self) -> Option<&OutputLabel> {
        self.modifiers.iter().find_map(|md| match md {
            PressModifier::Port(lbl) => Some(lbl),
            _ => None,
        })
    }
    pub fn channel(&self) -> Option<MidiChannel> {
        self.modifiers.iter().find_map(|md| match md {
            PressModifier::Channel(c) => Some(*c),
            _ => None,
        })
    }
    pub fn velocity(&self) -> Option<PressVelocity> {
        self.modifiers.iter().find_map(|md| match md {
            PressModifier::Velocity(v) => Some(*v),
            _ => None,
        })
    }
    pub fn duration(&self) -> Option<WaitTime> {
        self.modifiers.iter().find_map(|md| match md {
            PressModifier::Duration(d) => Some(*d),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ChordPress {
    pub root: NoteClass,
    pub octave: Octave,
    pub kind: ChordKind,
    pub modifiers: Vec<PressModifier>,
}

impl ChordPress {
    pub fn port(&self) -> Option<&OutputLabel> {
        self.modifiers.iter().find_map(|md| match md {
            PressModifier::Port(lbl) => Some(lbl),
            _ => None,
        })
    }
    pub fn channel(&self) -> Option<MidiChannel> {
        self.modifiers.iter().find_map(|md| match md {
            PressModifier::Channel(c) => Some(*c),
            _ => None,
        })
    }
    pub fn velocity(&self) -> Option<PressVelocity> {
        self.modifiers.iter().find_map(|md| match md {
            PressModifier::Velocity(v) => Some(*v),
            _ => None,
        })
    }
    pub fn duration(&self) -> Option<WaitTime> {
        self.modifiers.iter().find_map(|md| match md {
            PressModifier::Duration(d) => Some(*d),
            _ => None,
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ChordKind {
    Raw,
    Fifth,
    Major,
    Minor,
    Major7,
    Minor7,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SongAttribute {
    Signature(BpmInfo), 
    DefaultDuration(WaitTime),
    DefaultChannel(MidiChannel),
    DefaultPort(OutputLabel),
    DefaultPressVelocity(PressVelocity),
}
