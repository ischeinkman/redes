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
    fn from(inner : String) -> Self {
        OutputLabel(inner)   
    }
}

impl AsRef<str> for OutputLabel {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum LangItem {
    Loop {
        expr: Vec<LangItem>,
        repititions: Option<NonZeroU16>,
    },
    NotePress(PressLine),
    Wait(WaitTime),
    Asm(AsmCommand),
}

#[allow(unused)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PressModifier {
    Velocity(PressVelocity),
    Channel(MidiChannel),
    Duration(WaitTime),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PressLine {
    presses: Vec<ChordPress>,
    modifiers: Vec<PressModifier>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ChordPress {
    root: NoteClass,
    octave: Octave,
    kind: ChordKind,
    modifiers: Vec<PressModifier>,
}

#[allow(unused)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ChordKind {
    Raw,
    Fifth,
    Major,
    Minor,
    Major7,
    Minor7,
}

#[allow(unused)]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SongAttribute {
    Signature {
        beats_per_measure: NonZeroU16,
        kind_per_beat: NonZeroU16,
    },
    DefaultDuration(WaitTime),
    Bpm(NonZeroU16),
    TicksPerBeat(NonZeroU16),
    DefaultChannel(MidiChannel),
    DefaultPressVelocity(PressVelocity),
}
