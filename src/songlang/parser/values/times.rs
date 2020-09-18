
use super::{nonzerou16, nonzerou64};
use crate::songlang::ParseResult;
use crate::track::WaitTime;
use nom::{
    branch::alt,
    bytes::complete::tag_no_case,
    character::complete::{space0, space1},
    sequence::delimited,
};
use std::time::Duration;

fn parse_ticks(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou16(input)?;
    let (input, _) = alt((
        delimited(space1, tag_no_case("ticks"), space0),
        delimited(space1, tag_no_case("tick"), space0),
        delimited(space0, tag_no_case("t"), space0),
    ))(input)?;
    Ok((input, WaitTime::Ticks(n)))
}

fn parse_beats(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou16(input)?;
    let (input, _) = alt((
        delimited(space1, tag_no_case("beats"), space0),
        delimited(space1, tag_no_case("beat"), space0),
        delimited(space0, tag_no_case("b"), space0),
    ))(input)?;
    Ok((input, WaitTime::Beats(n)))
}

fn parse_minutes(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou64(input)?;
    let (input, _) = alt((
        delimited(space1, tag_no_case("minutes"), space0),
        delimited(space1, tag_no_case("mins"), space0),
        delimited(space0, tag_no_case("m"), space0),
    ))(input)?;
    let res = WaitTime::Clock(Duration::from_secs(n.get() * 60));
    Ok((input, res))
}

fn parse_seconds(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou64(input)?;
    let (input, _) = alt((
        delimited(space1, tag_no_case("seconds"), space0),
        delimited(space1, tag_no_case("secs"), space0),
        delimited(space0, tag_no_case("s"), space0),
    ))(input)?;
    let res = WaitTime::Clock(Duration::from_secs(n.get()));
    Ok((input, res))
}

fn parse_millis(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou64(input)?;
    let (input, _) = alt((
        delimited(space1, tag_no_case("milliseconds"), space0),
        delimited(space1, tag_no_case("millis"), space0),
        delimited(space0, tag_no_case("ms"), space0),
    ))(input)?;
    let res = WaitTime::Clock(Duration::from_millis(n.get()));
    Ok((input, res))
}

fn parse_micros(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou64(input)?;
    let (input, _) = alt((
        delimited(space1, tag_no_case("microseconds"), space0),
        delimited(space1, tag_no_case("micros"), space0),
        delimited(space0, tag_no_case("us"), space0),
    ))(input)?;
    let res = WaitTime::Clock(Duration::from_micros(n.get()));
    Ok((input, res))
}

fn parse_nanos(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou64(input)?;
    let (input, _) = alt((
        delimited(space1, tag_no_case("nanoseconds"), space0),
        delimited(space1, tag_no_case("nanos"), space0),
        delimited(space0, tag_no_case("ns"), space0),
    ))(input)?;
    let res = WaitTime::Clock(Duration::from_nanos(n.get()));
    Ok((input, res))
}

pub fn parse_rawduration(input: &str) -> ParseResult<WaitTime> {
    alt((
        // parse_notediv,
        parse_beats,
        parse_ticks,
        parse_minutes,
        parse_seconds,
        parse_millis,
        parse_micros,
        parse_nanos,
    ))(input)
}
