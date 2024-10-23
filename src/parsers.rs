use winnow::{
    ascii::{multispace0, till_line_ending}, combinator::{preceded, rest, separated_pair, todo}, error::InputError, stream::Range, token::{literal, one_of, take, take_till, take_until, take_while}, PResult, Parser
};
use std::collections::HashMap;
use crate::{GCodeLine, GCodeModel, G1, Command};


pub fn parse_file(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let out = String::from_utf8(std::fs::read(path)?)?
        .lines()
        .filter_map(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect();
    Ok(out)
}

fn outer_parser(input: &str) -> PResult<GCodeModel> {
    let mut gcode = GCodeModel::default();
    let input = winnow::Located::new(input);
    // split a file into lines and remove all whitespace
    while let Ok ((line, span)) = parse_line_with_span(input) {
        let (line, comments) = parse_comments(line)?;
        let (command, mut rest) = parse_word(line)?;
        match command {
            "G1" => {
                let mut input = rest;
                let g1 = g1_parameter_parse(&mut input)?;
                gcode.lines.push(GCodeLine::Processed(span, gcode.id_counter.get(), Command::G1(g1)));
            }
            "G28" => todo!(),
            "G90" => {gcode.rel_xyz = false; gcode.lines.push(GCodeLine::Processed( span, gcode.id_counter.get(), crate::Command::G90))},
            "G91" => gcode.rel_xyz = true,
            "M82" => gcode.rel_e = false,
            "M83" => gcode.rel_e = true,
            _ => {
                let original_input = String::from(&input[span.clone()]);
                gcode.lines.push(GCodeLine::Unprocessed(span, gcode.id_counter.get(), original_input));

            }
        }
    }
    Ok(gcode)
}

fn parse_line_with_span(mut input: winnow::Located<&str>) -> PResult<(&str, std::ops::Range<usize>)> {
    let mut parser = till_line_ending.with_span();
    let (line, span) = parser.parse_next(&mut input)?;
    Ok((line, span))
}

// strips a comment from a line, returning a tuple of two strings separated by a ';'
fn parse_comments(mut input: &str) -> PResult<(&str, &str)> {
    let start = take_until(0.., ';').parse_next(&mut input)?;
    let separator = take(1_usize).parse_next(&mut input)?;
    assert_eq!(separator, ";");
    Ok((start, input))
}

fn parse_word(mut input: &str) -> PResult<(&str, &str)> {
    let first_char = take_till(0..1, |c: char| c.is_numeric()).parse_next(&mut input)?;
    let rest = take_while(0.., |c: char| c.is_numeric()).parse_next(&mut input)?;
    Ok((first_char, rest))
}

// Helper function to check if a character is part of a number
fn is_number_char(c: char) -> bool {
    c.is_numeric() || c == '.' || c == '-' || c == '+'
}

#[test]
fn number_chars() {
    let tests = ["1.0000231", "-1.02030", "1.2+-0.0001", "-0.0000011"];
    for test in tests {
        for c in test.chars() {
            if !is_number_char(c) {
                panic!("invalid charachter found: {}", c);
            }
        }
    }
}

fn clear_whitespace<'a>(input: &mut &'a str) -> PResult<String> {
    // String to accumulate the output
    let mut out = String::new();

    // Loop until all input is processed
    loop {
        // Consume whitespace and discard it
        loop {
            if multispace0::<&str, winnow::error::ErrorKind>(input).is_err() {
                break;
            }
        }
        // Capture a single non-whitespace character and append it to the output string
        if let Ok(c) = take::<usize, &str, winnow::error::ErrorKind>(1_usize).parse_next(input) {
            out.push_str(c);
        }
    }

    Ok(out)
}
#[test]
fn whitespace_test() {
    let mut test = "       g  a  SS   a S d   d ";
    let res = clear_whitespace(&mut test).unwrap();
    assert_eq!(res.as_str(), "gaSSaSdd");
}

fn g1_comment_parse<'a>(input: &mut &'a str) -> PResult<&'a str> {
    // return any characters following ';',
    // should only be applied as final parse option
    preceded(';', rest).parse_next(input)
}
// FIXME: TEST THIS
fn g1_parameter_parse<'a>(input: &mut &'a str) -> PResult<G1> {
    let mut out = G1::default();
    while let Ok((c, val)) = separated_pair(
        one_of::<_, _, InputError<_>>(['X', 'Y', 'Z', 'E', 'F']),
        winnow::combinator::empty,
        take_while(1.., |c| is_number_char(c)).parse_to(),
    )
    .parse_next(input)
    {
        match c {
            'X' => out.x = Some(val),
            'Y' => out.y = Some(val),
            'Z' => out.z = Some(val),
            'E' => out.e = Some(val),
            'F' => out.f = Some(val),
            _ => {}
        }
    }
    Ok(out)
}