use super::{nonzerou16, nonzerou64};
use crate::songlang::ParseResult;
use crate::track::WaitTime;
use nom::{
    branch::alt,
    bytes::complete::tag_no_case,
    character::complete::{space0, space1},
    sequence::preceded,
};
use std::time::Duration;

fn parse_ticks(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou16(input)?;
    let (input, _) = alt((
        preceded(space1, tag_no_case("ticks")),
        preceded(space1, tag_no_case("tick")),
        preceded(space0, tag_no_case("t")),
    ))(input)?;
    Ok((input, WaitTime::Ticks(n)))
}

fn parse_beats(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou16(input)?;
    let (input, _) = alt((
        preceded(space1, tag_no_case("beats")),
        preceded(space1, tag_no_case("beat")),
        preceded(space0, tag_no_case("b")),
    ))(input)?;
    Ok((input, WaitTime::Beats(n)))
}

fn parse_minutes(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou64(input)?;
    let (input, _) = alt((
        preceded(space1, tag_no_case("minutes")),
        preceded(space1, tag_no_case("mins")),
        preceded(space0, tag_no_case("m")),
    ))(input)?;
    let res = WaitTime::Clock(Duration::from_secs(n.get() * 60));
    Ok((input, res))
}

fn parse_seconds(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou64(input)?;
    let (input, _) = alt((
        preceded(space1, tag_no_case("seconds")),
        preceded(space1, tag_no_case("secs")),
        preceded(space0, tag_no_case("s")),
    ))(input)?;
    let res = WaitTime::Clock(Duration::from_secs(n.get()));
    Ok((input, res))
}

fn parse_millis(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou64(input)?;
    let (input, _) = alt((
        preceded(space1, tag_no_case("milliseconds")),
        preceded(space1, tag_no_case("millis")),
        preceded(space0, tag_no_case("ms")),
    ))(input)?;
    let res = WaitTime::Clock(Duration::from_millis(n.get()));
    Ok((input, res))
}

fn parse_micros(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou64(input)?;
    let (input, _) = alt((
        preceded(space1, tag_no_case("microseconds")),
        preceded(space1, tag_no_case("micros")),
        preceded(space0, tag_no_case("us")),
    ))(input)?;
    let res = WaitTime::Clock(Duration::from_micros(n.get()));
    Ok((input, res))
}

fn parse_nanos(input: &str) -> ParseResult<WaitTime> {
    let (input, n) = nonzerou64(input)?;
    let (input, _) = alt((
        preceded(space1, tag_no_case("nanoseconds")),
        preceded(space1, tag_no_case("nanos")),
        preceded(space0, tag_no_case("ns")),
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
