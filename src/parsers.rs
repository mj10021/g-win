use crate::{Command, GCodeLine, GCodeModel, G1};
use winnow::{
    ascii::{alphanumeric0, alphanumeric1, crlf, newline},
    combinator::{alt, eof, separated, separated_pair, repeat},
    error::InputError,
    stream::Accumulate,
    token::{none_of, one_of, take, take_till, take_until, take_while},
    Located, PResult, Parser,
};
fn clear_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<String>()
}

fn parse_lines<'a>(mut input: &str) -> PResult<(Vec<&str>)> {
    let mut lines: Vec<&str>  = separated(1.., alphanumeric1, alt(("\n", crlf, eof)))
        .parse_next(&mut input)?;
    let rest = repeat(0.., none_of(['\r', '\n', ' '])).parse_next(&mut input)?;
    lines.push(rest);
    Ok(lines)
}

#[test]
fn test_parse_lines() {
    let tests = [
        (
            "hello\nworld\nasdf\r\nasdf\r\n\r\n n",
            vec!["hello", "world", "asdf", "asdf", " n"],
        ),
        ("hello", vec!["hello"]),
    ];
    for (input, expected) in tests {
        let results = parse_lines(&input).unwrap();
        assert_eq!(results, expected);
    }
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
        ("hello;", ("hello", "")),
        (";", ("", "")),
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

#[test]
fn test_parse_word() {
    let tests = [
        ("G1", ("G", "1")),
        ("M1234", ("M", "1234")),
        ("G28 W", ("G", "28 W")),
        (
            "G1 X1.0 Y2.0 Z3.0 E4.0 F5.0",
            ("G1", "X1.0 Y2.0 Z3.0 E4.0 F5.0"),
        ),
    ];
    for (input, expected) in tests.iter() {
        let (start, rest) = parse_word(input).unwrap();
        assert_eq!((start, rest), *expected);
    }
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

fn outer_parser(input: &str) -> PResult<GCodeModel> {
    let mut gcode = GCodeModel::default();
    let lines = parse_lines(&input)?;
    // split a file into lines
    for (i, line) in lines.iter().enumerate() {
        let string_copy = String::from(*line);
        // clear whitespace
        let line = line.split_whitespace().collect::<String>();
        let line = line.as_str();
        // split off comments
        let (line, comments) = parse_comments(line)?;
        // split off first word from command
        let (command, rest) = parse_word(line)?;
        // process rest of command based on first word
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
            _ => Command::Unsupported(string_copy),
        };
        let id = gcode.id_counter.get();
        gcode.lines.push(GCodeLine {
            id,
            line_number: i,
            command,
            comments: String::from(comments),
        });
    }
    Ok(gcode)
}

#[cfg(test)]
const TEST_FILE: &str = "tests/test.gcode";
