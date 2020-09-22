use crate::const_min;
use crate::model::{NoteClass, Octave};
use thiserror::*;

mod notes;
pub use notes::*;

#[derive(Debug, Error)]
pub enum MessageParseError {
    #[error("Wrong midi tag: expected {expected:b}, but found {actual:b}.")]
    WrongTag { expected: u8, actual: u8 },
    #[error("Value out of range: expected number in the range [{min}..={max}], found {found}.")]
    OutOfRange { found: u8, min: u8, max: u8 },
}

pub const fn parse_channel(raw: u8) -> Result<MidiChannel, MessageParseError> {
    let offset = raw & 0xF;
    Ok(MidiChannel { raw: offset })
}

pub const fn parse_note(raw: u8) -> Result<MidiNote, MessageParseError> {
    match MidiNote::from_raw(raw) {
        Some(n) => Ok(n),
        None => Err(MessageParseError::OutOfRange {
            min: 0,
            max: 127,
            found: raw,
        }),
    }
}

pub const fn parse_vel(raw: u8) -> Result<PressVelocity, MessageParseError> {
    match PressVelocity::from_raw(raw) {
        Some(n) => Ok(n),
        None => Err(MessageParseError::OutOfRange {
            min: 0,
            max: 127,
            found: raw,
        }),
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Default, Debug, Hash)]
pub struct PressVelocity {
    value: u8,
}

impl PressVelocity {
    pub const fn as_u8(&self) -> u8 {
        self.value
    }
    pub const fn from_raw(raw: u8) -> Option<PressVelocity> {
        if raw > 127 {
            None
        } else {
            Some(PressVelocity { value: raw })
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Default, Debug, Hash)]
pub struct MidiChannel {
    raw: u8,
}

const fn make_all_channels() -> [MidiChannel; 16] {
    let mut retvl = [MidiChannel { raw: 0 }; 16];
    let mut idx = 0;
    while idx < retvl.len() {
        retvl[idx] = MidiChannel { raw: idx as u8 };
        idx += 1;
    }
    retvl
}

impl MidiChannel {
    pub const fn all() -> &'static [MidiChannel] {
        const ALL: [MidiChannel; 16] = make_all_channels();
        &ALL
    }
    pub const fn as_u8(&self) -> u8 {
        self.raw
    }
    pub const fn from_raw(raw: u8) -> Option<MidiChannel> {
        let raw = raw as usize;
        if raw >= Self::all().len() {
            None
        } else {
            Some(Self::all()[raw])
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub struct RawMessage {
    bytes: [u8; 3],
}

impl RawMessage {
    pub const fn empty() -> Self {
        Self {
            bytes: [0, 0xFF, 0xFF],
        }
    }

    pub const fn from_raw(raw: &[u8]) -> RawMessage {
        let mut retvl = RawMessage::empty();
        let cplen = const_min!(retvl.len(), raw.len());
        let mut idx = 0;
        while idx < cplen {
            retvl.bytes[idx] = raw[idx];
            idx += 1;
        }
        retvl
    }

    #[allow(dead_code)]
    pub const fn tag(&self) -> u8 {
        self.bytes[0] & 0xF0
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.len()]
    }

    pub const fn len(&self) -> usize {
        if self.bytes[0] & 0x80 == 0 {
            0
        } else if self.bytes[1] & 0x80 != 0 {
            1
        } else if self.bytes[2] & 0x80 != 0 {
            2
        } else {
            3
        }
    }
}

impl Default for RawMessage {
    fn default() -> Self {
        RawMessage {
            bytes: [0x0, 0xFF, 0xFF],
        }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct MidiNote {
    raw: u8,
}

const fn all_notes() -> [MidiNote; 128] {
    let mut retvl = [MidiNote { raw: 0 }; 128];
    let mut idx = 0;
    while (idx as usize) < retvl.len() {
        retvl[idx as usize] = MidiNote { raw: idx };
        idx += 1;
    }
    retvl
}

#[allow(dead_code)]
impl MidiNote {
    pub fn all() -> &'static [MidiNote] {
        const RET: [MidiNote; 128] = all_notes();
        &RET
    }
    pub const fn wrapping_add(self, steps: i8) -> Self {
        let inner = self.raw as i16;
        let mut new = inner + (steps as i16);
        if new < 0 {
            new += 128;
        }
        if new >= 128 {
            new -= 128;
        }
        let new = new as u8;
        MidiNote { raw: new }
    }
    pub const fn from_note_octave(note: NoteClass, octave: Octave) -> Self {
        let midi_octave = (octave.as_raw() + 1) as u8;
        let octave_base = midi_octave * 12;
        let note_offset = match note {
            NoteClass::C => 0,
            NoteClass::Cs => 1,
            NoteClass::D => 2,
            NoteClass::Ds => 3,
            NoteClass::E => 4,
            NoteClass::F => 5,
            NoteClass::Fs => 6,
            NoteClass::G => 7,
            NoteClass::Gs => 8,
            NoteClass::A => 9,
            NoteClass::As => 10,
            NoteClass::B => 11,
        };
        let raw = octave_base + note_offset;
        MidiNote { raw }
    }
    pub const fn clamp(raw: u8) -> MidiNote {
        let raw = const_min!(127, raw);
        MidiNote { raw }
    }
    pub const fn mask(raw: u8) -> MidiNote {
        let raw = raw & 0x7F;
        MidiNote { raw }
    }
    pub const fn from_raw(raw: u8) -> Option<MidiNote> {
        if raw >= 128 {
            None
        } else {
            Some(MidiNote { raw })
        }
    }
    pub const fn octave(&self) -> Octave {
        let octave_shift = (self.as_u8() / 12) as i8;
        let octave = octave_shift - 1;
        Octave::clamp(octave)
    }
    pub const fn note(&self) -> NoteClass {
        match self.as_u8() % 12 {
            0 => NoteClass::C,
            1 => NoteClass::Cs,
            2 => NoteClass::D,
            3 => NoteClass::Ds,
            4 => NoteClass::E,
            5 => NoteClass::F,
            6 => NoteClass::Fs,
            7 => NoteClass::G,
            8 => NoteClass::Gs,
            9 => NoteClass::A,
            10 => NoteClass::As,
            // Always 11
            _ => NoteClass::B,
        }
    }
    pub const fn as_u8(self) -> u8 {
        self.raw
    }
    pub const fn checked_add(self, steps: i8) -> Option<Self> {
        let raw_result = (self.raw as i16) + (steps as i16);
        if raw_result < 0 {
            None
        } else {
            MidiNote::from_raw(raw_result as u8)
        }
    }
    pub fn frequency(&self) -> f64 {
        // Note -> freq
        // Given a Midi note number `a`, get the frequency in Hz.
        // The frequency doubles every 12 notes, and A440 = Midi note 69 = 440 Hz,
        // so the basic formula is `F(a) = 440.0 * (2.0).powf((a - 69.0)/12.0)` .
        // However, for the sake of `const`-ness, we want to use integers as much as possible.
        //
        // A note can be described as `a = 12*k + n`, for integer `k` and integer `n` where `0 <= n <= 11`.
        // A440 is then `69 = 12 * (5) + 9`, and the formula becomes
        // `F(k, n) = 440.0 * (2.0).powf((12 * k + n - 69)/12 )
        //          = 440.0 * (2.0).powf( k - 5 + (n - 9)/12 )
        //          = 440.0 * (2.0).powf( k - 5) * (2.0).powf( (n - 9)/12 )
        // Since 440.0 = 55.0 * 2**3:
        //          = 55.0 * (2.0).powf( k - 2) * (2.0).powf( (n - 9)/12 )
        //          = 55.0 * (2.0).powf( k - 2 + n/12 - 0.75)
        //          = 55.0 * (2.0).powf(k - 2.75 + n/12)
        //          = 55.0 * (2.0).powf(-2.75) * (2.0).powf(k) * 2.0.powf(n/12)
        // We can then pre-compute 55.0 * (2.0).powf(-2.75) * 2.0.powf(n/12) for 0 <= n <= 11, set up a `match` statement function on the note classes
        // (let it be `G(n)`), and then do `F(k, n) = G(n) * (1 << k)`.

        // However, since it is really REALLY difficult to accomplish that in a const way atm, let's just use the regular way.
        let a440_steps = (self.raw as i8) - 69;
        let coeff = ((a440_steps as f64) / 12.0).exp2();
        440.0 * coeff
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MidiMessage {
    NoteOn(NoteOn),
    NoteOff(NoteOff),
    Other(RawMessage),
}

impl MidiMessage {
    pub const fn as_raw(self) -> RawMessage {
        match self {
            MidiMessage::Other(k) => k,
            MidiMessage::NoteOff(data) => RawMessage {
                bytes: data.as_bytes(),
            },
            MidiMessage::NoteOn(data) => RawMessage {
                bytes: data.as_bytes(),
            },
        }
    }
}
impl From<RawMessage> for MidiMessage {
    fn from(inner: RawMessage) -> Self {
        MidiMessage::Other(inner)
    }
}
impl From<NoteOff> for MidiMessage {
    fn from(inner: NoteOff) -> Self {
        MidiMessage::NoteOff(inner)
    }
}

impl From<NoteOn> for MidiMessage {
    fn from(inner: NoteOn) -> Self {
        MidiMessage::NoteOn(inner)
    }
}

#[allow(dead_code)]
pub const fn parse_midimessage(bytes: [u8; 3]) -> Result<MidiMessage, MessageParseError> {
    let noteon_res = parse_noteon(bytes);
    match noteon_res {
        Ok(ret) => {
            return Ok(MidiMessage::NoteOn(ret));
        }
        Err(MessageParseError::WrongTag { .. }) => {}
        Err(e) => {
            return Err(e);
        }
    }
    let noteoff_res = parse_noteoff(bytes);
    match noteoff_res {
        Ok(ret) => {
            return Ok(MidiMessage::NoteOff(ret));
        }
        Err(MessageParseError::WrongTag { .. }) => {}
        Err(e) => {
            return Err(e);
        }
    }

    Ok(MidiMessage::Other(RawMessage::from_raw(&bytes)))
}
