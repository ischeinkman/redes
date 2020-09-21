use super::{
    parse_channel, parse_fullchord, parse_outputlabel, parse_rawduration, parse_velocity,
    ChordPress, ParseResult, PressLine, PressModifier,
};

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{space0, space1},
    combinator::{map, opt},
    multi::{separated_list, separated_nonempty_list},
    sequence::{delimited, preceded},
};

pub fn parse_pressline(input: &str) -> ParseResult<PressLine> {
    let (input, _) = tag_no_case("play")(input)?;
    let (input, _) = space1(input)?;
    let (input, modifiers) = parse_press_modifiers(input)?;
    let (input, _) = if modifiers.is_empty() {
        space0(input)?
    } else {
        space1(input)?
    };
    let press_sep = delimited(space0, tag(","), space0);
    let (input, presses) = separated_nonempty_list(press_sep, parse_chordpress)(input)?;
    let res = PressLine { presses, modifiers };
    Ok((input, res))
}

fn parse_chordpress(input: &str) -> ParseResult<ChordPress> {
    let (input, (root, octave, kind)) = parse_fullchord(input)?;
    let (input, _) = space1(input)?;
    let (input, modifiers) = parse_press_modifiers(input)?;
    let res = ChordPress {
        root,
        octave,
        kind,
        modifiers,
    };
    Ok((input, res))
}

fn parse_press_modifiers(input: &str) -> ParseResult<Vec<PressModifier>> {
    let mod_parser = |input| {
        alt((
            map(parse_duration_mod, |res| (Some(res), None)),
            map(parse_velocity_mod, |res| (Some(res), None)),
            parse_outputline_mod,
        ))(input)
    };
    let (input, pairs) = separated_list(space1, mod_parser)(input)?;
    let modifiers = pairs
        .into_iter()
        .flat_map(|(first, scnd)| first.into_iter().chain(scnd.into_iter()))
        .collect();
    Ok((input, modifiers))
}

fn parse_velocity_mod(input: &str) -> ParseResult<PressModifier> {
    let (input, _) = alt((tag_no_case("vel"), tag_no_case("velocity")))(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = space0(input)?;
    let (input, vel) = parse_velocity(input)?;
    let res = PressModifier::Velocity(vel);
    Ok((input, res))
}

fn parse_duration_mod(input: &str) -> ParseResult<PressModifier> {
    let (input, _) = tag_no_case("for")(input)?;
    let (input, _) = space1(input)?;
    let (input, dur) = parse_rawduration(input)?;
    let res = PressModifier::Duration(dur);
    Ok((input, res))
}

fn parse_outputline_mod(
    input: &str,
) -> ParseResult<(Option<PressModifier>, Option<PressModifier>)> {
    let (input, _) = tag_no_case("on")(input)?;
    let (input, port) = opt(preceded(space1, parse_outputline_port))(input)?;
    let (input, channel) = opt(preceded(space1, parse_outputline_channel))(input)?;
    Ok((input, (port, channel)))
}

fn parse_outputline_port(input: &str) -> ParseResult<PressModifier> {
    let (input, _) = tag_no_case("output")(input)?;
    let (input, _) = space1(input)?;
    let (input, port) = delimited(tag("\""), parse_outputlabel, tag("\""))(input)?;
    let res = PressModifier::Port(port);
    Ok((input, res))
}

fn parse_outputline_channel(input: &str) -> ParseResult<PressModifier> {
    let (input, _) = tag_no_case("channel")(input)?;
    let (input, _) = space1(input)?;
    let (input, channel) = parse_channel(input)?;
    let res = PressModifier::Channel(channel);
    Ok((input, res))
}
