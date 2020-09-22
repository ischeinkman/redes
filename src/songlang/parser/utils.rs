use nom::{
    bytes::complete::{tag, take_while1},
    combinator::map_res,
    combinator::{opt, recognize},
    eof, named,
    sequence::preceded,
};

use super::{ParseResult, ParseError};
use std::num::{NonZeroU128, NonZeroU16, NonZeroU64};
use std::str::FromStr;

pub fn rawuint(input: &str) -> ParseResult<&str> {
    take_while1(|c: char| c.is_digit(10))(input)
}

pub fn rawint(input: &str) -> ParseResult<&str> {
    recognize(preceded(opt(tag("-")), rawuint))(input)
}

pub fn nonzerou16(input: &str) -> ParseResult<NonZeroU16> {
    map_res(rawuint, NonZeroU16::from_str)(input)
}

#[allow(dead_code)]
pub fn nonzerou128(input: &str) -> ParseResult<NonZeroU128> {
    map_res(rawuint, NonZeroU128::from_str)(input)
}
pub fn nonzerou64(input: &str) -> ParseResult<NonZeroU64> {
    map_res(rawuint, NonZeroU64::from_str)(input)
}

named!(
    pub eof<&str, &str, ParseError>,
    eof!()
);
