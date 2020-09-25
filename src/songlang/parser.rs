use super::ast::*;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{line_ending, not_line_ending},
    combinator::{complete, cut, map},
    error::context,
    multi::separated_list,
    sequence::delimited,
    sequence::{preceded, terminated},
};

use std::num::NonZeroU16;

mod asm;
pub use asm::*;

mod utils;
pub use utils::*;

mod values;
pub use values::*;

mod playcmd;
pub use playcmd::*;

pub type ParseError<'a> = nom::error::VerboseError<&'a str>;

pub type ParseResult<'a, T> = nom::IResult<&'a str, T, ParseError<'a>>;

pub fn parse_file(input: &str) -> ParseResult<Vec<LangItem>> {
    let (input, _) = multispace0(input)?;
    let (input, res) = context(
        "File root command parser",
        separated_list(multispace1, parse_expr),
    )(input)?;
    let (input, _) = complete(multispace0)(input)?;
    Ok((input, res))
}

pub fn parse_expr(input: &str) -> ParseResult<LangItem> {
    context(
        "Songlang Expression",
        alt((
            parse_loop,
            map(parse_pressline, LangItem::NotePress),
            map(parse_asm_command, LangItem::Asm),
        )),
    )(input)
}

pub fn parse_block(input: &str) -> ParseResult<Vec<LangItem>> {
    let lines_parser = context(
        "Sub-block lines parser",
        separated_list(multispace1, parse_expr),
    );
    delimited(
        terminated(tag("{"), multispace0),
        lines_parser,
        preceded(multispace0, tag("}")),
    )(input)
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

pub fn parse_comment_inline(input: &str) -> ParseResult<()> {
    let body_parser = |inp: &str| {
        let endparser = alt((eof, tag("*/"), line_ending));
        let mut idx = 0;
        loop {
            let (head, tail) = input.split_at(idx);
            if endparser(tail).is_ok() {
                return Ok((tail, head));
            }
            idx += 1;
            while !inp.is_char_boundary(idx) {
                idx += 1;
            }
        }
    };
    let parser = delimited(tag("/*"), body_parser, cut(tag("*/")));
    context("CommentInline", map(parser, |_| ()))(input)
}

pub fn parse_comment_fullline(input: &str) -> ParseResult<()> {
    let line_beginning = alt((tag("#"), tag("//")));
    let parser = delimited(line_beginning, not_line_ending, alt((line_ending, eof)));
    let (input, _) = context("CommentFullline", parser)(input)?;
    Ok((input, ()))
}

pub fn parse_comment(input: &str) -> ParseResult<()> {
    context(
        "CommentAny",
        alt((parse_comment_inline, parse_comment_fullline)),
    )(input)
}
