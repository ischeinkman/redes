use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{
        multispace0 as nom_multispace0, multispace1 as nom_multispace1, space0 as nom_space0,
        space1 as nom_space1,
    },
    combinator::{map, map_res},
    combinator::{opt, recognize},
    eof,
    error::context,
    multi::{many0, many1},
    named,
    sequence::{delimited, preceded},
};

use super::{parse_comment, parse_comment_fullline, parse_comment_inline, ParseError, ParseResult};
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

pub fn space0(input: &str) -> ParseResult<()> {
    let nom_space0_wrapped = map(nom_space0, |_| ());
    let parser = preceded(
        nom_space0,
        opt(many0(alt((nom_space0_wrapped, parse_comment_inline)))),
    );
    context("utils::Space0", map(parser, |_| ()))(input)
}

pub fn space1(input: &str) -> ParseResult<()> {
    let parser = delimited(many0(parse_comment_inline), nom_space1, space0);
    context("utils::Space1", map(parser, |_| ()))(input)
}

pub fn multispace0(input: &str) -> ParseResult<()> {
    let nom_multispace1_wrapped = map(nom_multispace1, |_| ());
    let comment_wrapped = map(many1(parse_comment), |_| ());
    let single_parser = alt((nom_multispace1_wrapped, comment_wrapped));
    let multiparser = map(opt(many0(single_parser)), |_| ());
    context("utils::Multispace0", multiparser)(input)
}

pub fn multispace1(input: &str) -> ParseResult<()> {
    let comment_parser = preceded(parse_comment_fullline, nom_multispace0);
    let line_parser = alt((comment_parser, nom_multispace1));
    let raw_parser = delimited(space0, line_parser, multispace0);
    let parser = map(raw_parser, |_| ());
    context("utils::Multispace1", parser)(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::branch::alt;
    use nom::character::complete::alphanumeric1;
    use nom::combinator::map;
    use nom::multi::{fold_many1, separated_list};
    use nom::sequence::terminated;
    use nom::Err as NomErr;

    #[test]
    fn test_comments() {
        let raw = r"ELM1 ELM2   ELM3/* inline comment 1*/ELM3b /*Inline comment 2*/ELM4 // Line comment
        // Line comment 2
        # line comment 3
        L2ELM1 /* inline comment 3*/L2ELM2 // Line comment 4
        L3ELM3 L3ELM4 L3ELM5 #line comment 5
        /* inline comment 3 *//*inline comment 4*/ 
        # line comment 5 /* inline comment 6*/";

        let list_elm_parser = fold_many1(
            alt((alphanumeric1, map(parse_comment_inline, |_| ""))),
            String::new(),
            |mut acc, cur| {
                acc.push_str(cur);
                acc
            },
        );
        let line_parser = terminated(separated_list(space1, list_elm_parser), multispace1);
        let res1 = line_parser(raw).map_err(|e| match e {
            NomErr::Error(e) => format!("Error: {}", nom::error::convert_error(raw, e)),
            NomErr::Failure(e) => format!("Failure: {}", nom::error::convert_error(raw, e)),
            NomErr::Incomplete(e) => format!("Incomplete {:?}, {:?}", e, raw),
        });
        let (rest, first_line_res) = match res1 {
            Ok(r) => r,
            Err(msg) => panic!(msg),
        };
        assert_eq!(vec!["ELM1", "ELM2", "ELM3ELM3b", "ELM4"], first_line_res);

        let (rest, second_line_res) = line_parser(rest).unwrap();
        assert_eq!(
            vec!["L2ELM1", "L2ELM2"],
            second_line_res,
            "Rest = {:?}",
            rest
        );

        let (rest, third_line_res) = line_parser(rest).unwrap();
        assert_eq!(vec!["L3ELM3", "L3ELM4", "L3ELM5"], third_line_res);

        assert_eq!(Ok(""), multispace0(rest).map(|(r, _)| r));
    }
}
