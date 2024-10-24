use crate::{Command, GCodeLine, GCodeModel, G1};
use winnow::{
    ascii::till_line_ending,
    combinator::separated_pair,
    error::InputError,
    token::{one_of, take, take_till, take_until, take_while},
    PResult, Parser,
};

fn outer_parser(input: &str) -> PResult<GCodeModel> {
    let mut gcode = GCodeModel::default();
    let input = winnow::Located::new(input);
    // split a file into lines and remove all whitespace
    while let Ok((line, span)) = parse_line_with_span(input) {
        let (line, comments) = parse_comments(line)?;
        let (command, rest) = parse_word(line)?;
        let command = match command {
            "G1" => {
                let mut input = rest;
                let g1 = g1_parameter_parse(&mut input)?;
                Command::G1(g1)
            }
            "G28" => crate::Command::G28,
            "G90" => {
                gcode.rel_xyz = false;
                Command::G90
            }
            "G91" => {
                gcode.rel_xyz = true;
                Command::G91
            }
            "M82" => {
                gcode.rel_e = false;
                Command::M82
            }
            "M83" => {
                gcode.rel_e = true;
                Command::M83
            }
            _ => {
                Command::Unsupported(String::from(&input[span.clone()]))
            }
        };
        let id = gcode.id_counter.get();
        gcode.lines.push(GCodeLine {
            id,
            span,
            command,
            comments: String::from(comments)
        });
    }
    Ok(gcode)
}

fn parse_line_with_span(
    mut input: winnow::Located<&str>,
) -> PResult<(&str, std::ops::Range<usize>)> {
    let mut parser = till_line_ending.with_span();
    let (line, span) = parser.parse_next(&mut input)?;
    Ok((line, span))
}

// strips a comment from a line, returning a tuple of two strings separated by a ';'
fn parse_comments(mut input: &str) -> PResult<(&str, &str)> {
    if !input.contains(';') {
        return Ok((input, ""));
    }
    let start = take_until(0.., ';').parse_next(&mut input)?;
    let _separator = take(1_usize).parse_next(&mut input)?;
    return Ok((start, input));
}

#[test]
fn test_parse_comments() {
    let tests = [
        ("hello;world", ("hello", "world")),
        ("hello;world;more", ("hello", "world;more")),
        ("hello", ("hello", "")),
    ];
    for (input, expected) in tests.iter() {
        let (start, rest) = parse_comments(input).unwrap();
        assert_eq!((start, rest), *expected);
    }
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

// FIXME: TEST THIS
fn g1_parameter_parse(input: &mut &str) -> PResult<G1> {
    let mut out = G1::default();
    while let Ok((c, val)) = separated_pair(
        one_of::<_, _, InputError<_>>(['X', 'Y', 'Z', 'E', 'F']),
        winnow::combinator::empty,
        take_while(1.., is_number_char).parse_to(),
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

#[cfg(test)]
const TEST_FILE: &str = "tests/test.gcode";