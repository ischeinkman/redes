use super::ast::*;
use nom::{
    branch::alt, bytes::complete::tag, character::complete::multispace0,
    character::complete::multispace1, character::complete::space0, character::complete::space1,
    combinator::map, error::context, multi::many0, sequence::delimited,
};

use std::num::NonZeroU16;

mod asm;
pub use asm::*;

mod utils;
pub use utils::*;

mod values;
pub use values::*;

pub type ParseError<'a> = nom::error::VerboseError<&'a str>;

pub type ParseResult<'a, T> = nom::IResult<&'a str, T, ParseError<'a>>;

pub fn parse_file(input: &str) -> ParseResult<Vec<LangItem>> {
    let lineparser = context(
        "Single Expr Parser",
        delimited(multispace0, parse_expr, alt((multispace1, eof))),
    );
    let (input, res) = many0(lineparser)(input)?;
    Ok((input, res))
}

pub fn parse_expr(input: &str) -> ParseResult<LangItem> {
    let rawres = nom::error::context(
        "Songlang Expression",
        alt((parse_loop, map(parse_asm_command, LangItem::Asm))),
    )(input);
    let (input, res) = rawres?;
    Ok((input, res))
}

pub fn parse_block(input: &str) -> ParseResult<Vec<LangItem>> {
    delimited(tag("{"), many0(parse_expr), tag("}"))(input)
}

pub fn parse_loop(input: &str) -> ParseResult<LangItem> {
    let (input, _) = tag("loop")(input)?;
    let loopcount_parser = |input| {
        let (input, _) = space1(input)?;
        let (input, res) = nonzerou16(input)?;
        let (input, _) = space0(input)?;
        Ok((input, Some(res)))
    };
    let nocount_parser = |input| {
        let (input, _) = space0(input)?;
        Ok((input, None))
    };

    let (input, loopcount): (_, Option<NonZeroU16>) =
        alt((loopcount_parser, nocount_parser))(input)?;

    let (input, body) = parse_block(input)?;
    let res = LangItem::Loop {
        expr: body,
        repititions: loopcount,
    };
    Ok((input, res))
}
