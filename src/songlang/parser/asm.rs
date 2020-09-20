use crate::midi::{MidiChannel, MidiMessage, MidiNote, NoteOff, NoteOn, PressVelocity, RawMessage};
use crate::track::BpmInfo;

use nom::{
    branch::alt,
    bytes::complete::tag,
    bytes::complete::tag_no_case,
    character::complete::alpha1,
    character::complete::{space0, space1},
    combinator::map,
    error::context,
};

use super::{
    nonzerou16, parse_channel, parse_notepitch, parse_rawduration, parse_velocity, ParseResult,
};
use crate::songlang::ast::{AsmCommand, OutputLabel};

mod utils {
    use super::*;
    pub fn consume_commalist_seperator(input: &str) -> ParseResult<()> {
        let (input, _) = space0(input)?;
        let (input, _) = tag(",")(input)?;
        let (input, _) = space0(input)?;
        Ok((input, ()))
    }
    pub fn parse_rawlabel(input: &str) -> ParseResult<&str> {
        alpha1(input)
    }
}
use utils::*;

mod midimessages {
    use super::*;

    fn parse_notemsg_args(input: &str) -> ParseResult<(MidiChannel, MidiNote, PressVelocity)> {
        let (input, channel) = parse_channel(input)?;

        let (input, _) = consume_commalist_seperator(input)?;
        let (input, (noteclass, octave)) = parse_notepitch(input)?;
        let note = MidiNote::from_note_octave(noteclass, octave);

        let (input, _) = consume_commalist_seperator(input)?;
        let (input, vel) = parse_velocity(input)?;

        Ok((input, (channel, note, vel)))
    }

    pub fn parse_noteon(input: &str) -> ParseResult<NoteOn> {
        let (input, _) = tag_no_case("NOTEON")(input)?;
        let (input, _) = space1(input)?;

        let (input, (channel, note, vel)) = parse_notemsg_args(input)?;
        let res = NoteOn::new(channel, note, vel);
        Ok((input, res))
    }

    pub fn parse_noteoff(input: &str) -> ParseResult<NoteOff> {
        let (input, _) = tag_no_case("NOTEOFF")(input)?;
        let (input, _) = space1(input)?;

        let (input, (channel, note, vel)) = parse_notemsg_args(input)?;
        let res = NoteOff::new(channel, note, vel);
        Ok((input, res))
    }

    pub fn parse_rawmsg(input: &str) -> ParseResult<RawMessage> {
        let (input, _) = tag_no_case("RAW")(input)?;
        let (_, _) = space1(input)?;
        todo!()
    }

    pub fn parse_midimsg(input: &str) -> ParseResult<MidiMessage> {
        alt((
            map(parse_noteoff, MidiMessage::NoteOff),
            map(parse_rawmsg, MidiMessage::Other),
            map(parse_noteon, MidiMessage::NoteOn),
        ))(input)
    }

    pub fn parse_outputlabel(input: &str) -> ParseResult<Option<OutputLabel>> {
        let success_parser = |input| {
            let (input, _) = consume_commalist_seperator(input)?;
            let (input, _) = tag_no_case("output")(input)?;
            let (input, _) = space0(input)?;
            let (input, _) = tag_no_case("=")(input)?;
            let (input, _) = space0(input)?;
            let (input, name) = alpha1(input)?;
            let res = Some(OutputLabel::from(name.to_owned()));
            Ok((input, res))
        };
        let fail_parser = |input| {
            let (input, _) = space0(input)?;
            Ok((input, None))
        };
        alt((success_parser, fail_parser))(input)
    }
}
use midimessages::*;

fn parse_setbpm(input: &str) -> ParseResult<AsmCommand> {
    let (input, _) = tag_no_case("SETBPM")(input)?;
    let (input, _) = space1(input)?;
    let (input, bpm) = nonzerou16(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag(",")(input)?;
    let (input, _) = space0(input)?;
    let (input, ticks) = nonzerou16(input)?;
    let res = BpmInfo {
        ticks_per_beat: ticks,
        beats_per_minute: bpm,
    };
    let evt = AsmCommand::SetBpm(res);
    Ok((input, evt))
}

fn parse_jump(input: &str) -> ParseResult<AsmCommand> {
    let (input, _) = tag_no_case("JUMP")(input)?;
    let (input, _) = space1(input)?;
    let (input, label) = parse_rawlabel(input)?;
    let count_parser = alt((
        map(nom::sequence::preceded(space1, nonzerou16), Some),
        map(space0, |_| None),
    ));
    let (input, count) = count_parser(input)?;
    let res = AsmCommand::Jump {
        label: label.to_owned(),
        count,
    };
    Ok((input, res))
}

fn parse_label(input: &str) -> ParseResult<AsmCommand> {
    let (input, _) = tag_no_case("LABEL")(input)?;
    let (input, _) = space1(input)?;
    let (input, label) = parse_rawlabel(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag(":")(input)?;
    Ok((input, AsmCommand::Label(label.to_owned())))
}

fn parse_sendmessage(input: &str) -> ParseResult<AsmCommand> {
    let (input, _) = tag_no_case("SEND")(input)?;
    let (input, _) = space1(input)?;
    let (input, message) = parse_midimsg(input)?;
    let (input, _) = space0(input)?;
    let (input, port) = parse_outputlabel(input)?;
    Ok((input, AsmCommand::Send { message, port }))
}

fn parse_wait(input: &str) -> ParseResult<AsmCommand> {
    let (input, _) = tag_no_case("WAIT")(input)?;
    let (input, _) = space1(input)?;
    let (input, time) = parse_rawduration(input)?;
    let res = AsmCommand::Wait(time);
    Ok((input, res))
}

pub fn parse_asm_command(input: &str) -> ParseResult<AsmCommand> {
    alt((
        context("ASM SEND", parse_sendmessage),
        context("ASM SETBPM", parse_setbpm),
        context("ASM WAIT", parse_wait),
        context("ASM LABEL", parse_label),
        context("ASM JUMP", parse_jump),
    ))(input)
}
