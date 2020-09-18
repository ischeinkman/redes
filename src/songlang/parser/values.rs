use nom::{
    alt,
    bytes::complete::tag,
    combinator::{map, map_opt, map_res, opt},
    error::context,
    named,
    sequence::tuple,
    tag, tag_no_case,
};

use super::{nonzerou16, nonzerou64, rawint, rawuint, ParseError, ParseResult};
use crate::midi::{MidiChannel, PressVelocity};
use crate::model::{NoteClass, Octave};
use crate::songlang::ast::ChordKind;
use std::str::FromStr;

mod times;
pub use times::*;

pub fn parse_channel(input: &str) -> ParseResult<MidiChannel> {
    let num_parser = map_res(rawuint, u8::from_str);
    let has_raw_parser = map(opt(tag("r")), |opt| opt.is_some());
    let data_parser = tuple((num_parser, has_raw_parser));
    let channel_num_parser = map(data_parser, |(rawn, is_raw)| {
        rawn.wrapping_sub(1 - u8::from(is_raw))
    });
    let channel_parser = map_opt(channel_num_parser, MidiChannel::from_raw);
    channel_parser(input)
}

pub fn parse_octave(input: &str) -> ParseResult<Octave> {
    let i8_parser = map_res(rawint, i8::from_str);
    map_opt(i8_parser, Octave::from_raw)(input)
}

pub fn parse_notepitch(input: &str) -> ParseResult<(NoteClass, Octave)> {
    let (input, note) = context("Parse Noteclass", parse_noteclass)(input)?;
    let (input, octave) = context("Parse Octave", parse_octave)(input)?;
    Ok((input, (note, octave)))
}

pub fn parse_velocity(input: &str) -> ParseResult<PressVelocity> {
    let rawmapper = map_res(rawuint, u8::from_str);
    let pressmapper = map_opt(rawmapper, PressVelocity::from_raw);
    pressmapper(input)
}

named!(
    pub parse_chordkind<&str, ChordKind, ParseError>,
    alt!(
        tag!("m7") => {|_| ChordKind::Minor7} |
        tag!("M7") => {|_| ChordKind::Major7} |
        tag!("m") => {|_| ChordKind::Minor} |
        tag!("M") => {|_| ChordKind::Major} |
        tag!("5") => {|_| ChordKind::Fifth} |
        tag!("") => {|_| ChordKind::Raw}
    )
);

named!(
    pub parse_noteclass<&str, NoteClass, ParseError>,
    alt!(
        tag_no_case!("Ab") => {|_| NoteClass::Gs} |
        tag_no_case!("A#") => {|_| NoteClass::As} |
        tag_no_case!("As") => {|_| NoteClass::As} |
        tag_no_case!("A") => {|_| NoteClass::A} |

        tag_no_case!("Bb") => {|_| NoteClass::As} |
        tag_no_case!("B") => {|_| NoteClass::B} |

        tag_no_case!("C#") => {|_| NoteClass::Cs} |
        tag_no_case!("Cs") => {|_| NoteClass::Cs} |
        tag_no_case!("C") => {|_| NoteClass::C} |

        tag_no_case!("D#") => {|_| NoteClass::Ds} |
        tag_no_case!("Ds") => {|_| NoteClass::Ds} |
        tag_no_case!("Db") => {|_| NoteClass::Cs} |
        tag_no_case!("D") => {|_| NoteClass::D} |

        tag_no_case!("Eb") => {|_| NoteClass::Ds} |
        tag_no_case!("E") => {|_| NoteClass::E} |

        tag_no_case!("F#") => {|_| NoteClass::Fs} |
        tag_no_case!("Fs") => {|_| NoteClass::Fs} |
        tag_no_case!("F") => {|_| NoteClass::F} |

        tag_no_case!("Gb") => {|_| NoteClass::Fs} |
        tag_no_case!("G#") => {|_| NoteClass::Gs} |
        tag_no_case!("Gs") => {|_| NoteClass::Gs} |
        tag_no_case!("G") => {|_| NoteClass::G}

    )
);
