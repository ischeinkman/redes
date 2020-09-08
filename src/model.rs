#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum NoteClass {
    C,
    Cs,
    D,
    Ds,
    E,
    F,
    Fs,
    G,
    Gs,
    A,
    As,
    B,
}

impl NoteClass {
    pub const fn all() -> &'static [NoteClass] {
        &[
            NoteClass::C,
            NoteClass::Cs,
            NoteClass::D,
            NoteClass::Ds,
            NoteClass::E,
            NoteClass::F,
            NoteClass::Fs,
            NoteClass::G,
            NoteClass::Gs,
            NoteClass::A,
            NoteClass::As,
            NoteClass::B,
        ]
    }
    pub const fn shift(&self, offset: i8) -> Self {
        let raw = (self.as_u8() as i16) + (offset as i16);
        let clamped = if raw < 0 { 12 + (raw % 12) } else { raw % 12 };
        NoteClass::from_u8(clamped as u8)
    }
    pub const fn from_u8(raw: u8) -> Self {
        match raw % 12 {
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
    pub const fn as_u8(&self) -> u8 {
        match self {
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
        }
    }
}

impl NoteClass {}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Octave(i8);

impl Octave {
    pub const fn as_raw(&self) -> i8 {
        self.0
    }
    pub const fn clamp(raw: i8) -> Self {
        let clamped = if raw < -1 {
            -1
        } else if raw > 9 {
            9
        } else {
            raw
        };
        Octave(clamped)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct NoteKey {
    // Bit structure:
    // `0rrr_nnnn_nnnn_nnnn`, where `rrr` is the root note number and `nnnn_nnnn_nnnn`
    // is a mask where the "k"th bit being set says that the note with notenumber "k" is in the key.
    // For example, if the least significant bit is 1, then the C key is pressed.
    notes_with_root: u16,
}

const NOTES_MASK: u16 = 0x0FFF;
const ROOT_MASK: u16 = 0xF000;

impl NoteKey {
    const fn with_note(mut self, note: NoteClass) -> Self {
        let mask = 1 << (note.as_u8());
        self.notes_with_root |= mask;
        self
    }
    const fn without_note(mut self, note: NoteClass) -> Self {
        let mask = !(1 << (note.as_u8()));
        self.notes_with_root &= mask;
        self
    }
    const fn empty() -> Self {
        NoteKey { notes_with_root: 0 }
    }

    pub const fn major(root: NoteClass) -> Self {
        let mut retvl = Self::empty();
        let root_mask = (root.as_u8() as u16) << 12;
        retvl.notes_with_root |= root_mask;
        retvl = retvl.with_note(root);
        retvl = retvl.with_note(root.shift(2));
        retvl = retvl.with_note(root.shift(4));
        retvl = retvl.with_note(root.shift(5));
        retvl = retvl.with_note(root.shift(7));
        retvl = retvl.with_note(root.shift(9));
        retvl = retvl.with_note(root.shift(11));
        retvl
    }

    pub const fn minor(root: NoteClass) -> Self {
        let mut retvl = Self::major(root);
        retvl = retvl.without_note(root.shift(4)).with_note(root.shift(3));
        retvl = retvl.without_note(root.shift(9)).with_note(root.shift(8));
        retvl = retvl.without_note(root.shift(11)).with_note(root.shift(10));
        retvl
    }

    pub const fn equivalent(&self, other: &NoteKey) -> bool {
        let self_notes = self.notes_with_root & NOTES_MASK;
        let other_notes = other.notes_with_root & NOTES_MASK;
        self_notes == other_notes
    }

    pub const fn root(&self) -> NoteClass {
        let raw_root = (self.notes_with_root & ROOT_MASK) >> 12;
        NoteClass::from_u8(raw_root as u8)
    }
    pub const fn contains(&self, note: NoteClass) -> bool {
        let raw = note.as_u8();
        let mask = 1 << raw;
        self.notes_with_root & mask != 0
    }
    pub const fn len(&self) -> usize {
        let notes = self.notes_with_root & NOTES_MASK;
        notes.count_ones() as usize
    }
    pub const fn nth(&self, keystep: isize) -> NoteClass {
        let mapped_step = if keystep < 0 {
            self.len() as isize + (keystep % self.len() as isize)
        } else {
            keystep % self.len() as isize
        };
        let mut step = 0;
        let mut note = self.root();
        loop {
            if step >= mapped_step {
                return note;
            }
            while !self.contains(note.shift(1)) {
                note = note.shift(1);
            }
            note = note.shift(1);
            step += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notes() {
        let c = NoteClass::C;
        assert_eq!(NoteClass::D, c.shift(2));
        assert_eq!(NoteClass::As, c.shift(-2));
    }

    #[test]
    fn test_key() {
        let c_major = NoteKey::major(NoteClass::C);
        let a_minor = NoteKey::minor(NoteClass::A);

        assert!(
            a_minor.equivalent(&c_major) && c_major.equivalent(&a_minor),
            "Difference : {:?}",
            NoteClass::all()
                .iter()
                .copied()
                .filter(|note| a_minor.contains(*note) != c_major.contains(*note))
                .collect::<Vec<_>>()
        );
        assert_eq!(NoteClass::C, c_major.root());
        assert_eq!(NoteClass::A, a_minor.root());
        assert_eq!(NoteClass::C, c_major.nth(0));
        assert_eq!(NoteClass::E, c_major.nth(2));
        assert_eq!(NoteClass::G, c_major.nth(4));
        assert_eq!(NoteClass::B, c_major.nth(6));
        assert_eq!(NoteClass::C, c_major.nth(7));
        for idx in -(c_major.len() as isize) * 2..(c_major.len() as isize) * 2 {
            let c_note = c_major.nth(idx);
            let a_note = a_minor.nth(idx + 2);
            assert_eq!(c_note, a_note, "IDX: {}", idx);
        }
    }
}
