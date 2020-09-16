use super::{
    parse_channel, parse_note, parse_vel, MessageParseError, MidiChannel, MidiNote, PressVelocity,
};

use crate::const_try;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u8)]
enum NoteEventTag {
    Off = 0b1000_0000,
    On = 0b1001_0000,
}

const fn parse_tag_expected(
    byte: u8,
    expected: NoteEventTag,
) -> Result<NoteEventTag, MessageParseError> {
    let head = byte & 0xF0;
    if head != expected as u8 {
        Err(MessageParseError::WrongTag {
            expected: expected as u8,
            actual: head,
        })
    } else {
        Ok(expected)
    }
}

struct NoteEventPayload {
    channel: MidiChannel,
    note: MidiNote,
    velocity: PressVelocity,
}
impl NoteEventPayload {
    pub const fn new(channel: MidiChannel, note: MidiNote, velocity: PressVelocity) -> Self {
        Self {
            channel,
            note,
            velocity,
        }
    }
    pub const fn as_bytes(&self, tag: u8) -> [u8; 3] {
        let b1 = (tag & 0xF0) | self.channel.as_u8();
        let b2 = self.note.as_u8();
        let b3 = self.velocity.as_u8();
        [b1, b2, b3]
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct NoteOn {
    channel: MidiChannel,
    note: MidiNote,
    vel: PressVelocity,
}

impl NoteOn {
    pub const fn as_bytes(&self) -> [u8; 3] {
        NoteEventPayload::new(self.channel(), self.note(), self.vel())
            .as_bytes(NoteEventTag::On as u8)
    }
    pub const fn new(channel: MidiChannel, note: MidiNote, vel: PressVelocity) -> Self {
        Self { channel, note, vel }
    }
    #[allow(dead_code)]
    pub const fn with_channel(self, channel: MidiChannel) -> Self {
        NoteOn { channel, ..self }
    }
    #[allow(dead_code)]
    pub const fn with_note(self, note: MidiNote) -> Self {
        NoteOn { note, ..self }
    }
    #[allow(dead_code)]
    pub const fn with_vel(self, vel: PressVelocity) -> Self {
        NoteOn { vel, ..self }
    }
    pub const fn channel(&self) -> MidiChannel {
        self.channel
    }
    pub const fn note(&self) -> MidiNote {
        self.note
    }
    pub const fn vel(&self) -> PressVelocity {
        self.vel
    }
}

pub const fn parse_noteon(bytes: [u8; 3]) -> Result<NoteOn, MessageParseError> {
    const_try!(parse_tag_expected(bytes[0], NoteEventTag::On));
    let payload = const_try!(parse_noteevent(bytes));
    Ok(NoteOn {
        channel: payload.channel,
        note: payload.note,
        vel: payload.velocity,
    })
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct NoteOff {
    channel: MidiChannel,
    note: MidiNote,
    vel: PressVelocity,
}

impl NoteOff {
    pub const fn as_bytes(&self) -> [u8; 3] {
        NoteEventPayload::new(self.channel(), self.note(), self.vel())
            .as_bytes(NoteEventTag::Off as u8)
    }
    #[allow(dead_code)]
    pub const fn with_channel(self, channel: MidiChannel) -> Self {
        NoteOff { channel, ..self }
    }
    #[allow(dead_code)]
    pub const fn with_note(self, note: MidiNote) -> Self {
        NoteOff { note, ..self }
    }
    #[allow(dead_code)]
    pub const fn with_vel(self, vel: PressVelocity) -> Self {
        NoteOff { vel, ..self }
    }
    pub const fn new(channel: MidiChannel, note: MidiNote, vel: PressVelocity) -> Self {
        Self { channel, note, vel }
    }
    pub const fn channel(&self) -> MidiChannel {
        self.channel
    }
    pub const fn note(&self) -> MidiNote {
        self.note
    }
    pub const fn vel(&self) -> PressVelocity {
        self.vel
    }
}

pub const fn parse_noteoff(bytes: [u8; 3]) -> Result<NoteOff, MessageParseError> {
    const_try!(parse_tag_expected(bytes[0], NoteEventTag::Off));
    let payload = const_try!(parse_noteevent(bytes));
    Ok(NoteOff {
        channel: payload.channel,
        note: payload.note,
        vel: payload.velocity,
    })
}

const fn parse_noteevent(bytes: [u8; 3]) -> Result<NoteEventPayload, MessageParseError> {
    let channel = const_try!(parse_channel(bytes[0]));
    let note = const_try!(parse_note(bytes[1]));
    let velocity = const_try!(parse_vel(bytes[2]));
    Ok(NoteEventPayload {
        channel,
        note,
        velocity,
    })
}
