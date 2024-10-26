use std::collections::HashMap;

use crate::{Command, GCodeLine, GCodeModel, G1};
use winnow::{
    combinator::{rest, separated_pair},
    error::InputError,
    token::{one_of, take, take_until, take_while},
    PResult, Parser,
};

// strips a comment from a line, returning a tuple of two strings separated by a ';'
fn parse_comments(mut input: &str) -> PResult<(&str, &str)> {
    if !input.contains(';') {
        return Ok((input, ""));
    }
    let (start, _separator) = (take_until(0.., ';'), take(1_usize)).parse_next(&mut input)?;
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

fn parse_word(mut input: &str) -> PResult<(&str, &str, &str)> {
    Ok((
        take(1_usize),
        take_while(0.., |c: char| c.is_numeric()),
        rest,
    )
        .parse_next(&mut input)?)
}

#[test]
fn test_parse_word() {
    let tests = [
        ("G1", ("G", "1", "")),
        ("M1234", ("M", "1234", "")),
        ("G28W", ("G", "28", "W")),
        (
            "G1 X1.0 Y2.0 Z3.0 E4.0 F5.0",
            ("G", "1", " X1.0 Y2.0 Z3.0 E4.0 F5.0"),
        ),
    ];
    for (input, expected) in tests.iter() {
        assert_eq!(parse_word(*input).unwrap(), *expected);
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

#[test]
fn g1_parameter_parse_test() {
    let mut tests = [
        (
            "X1.0Y2.0Z3.0E4.0F5.0",
            G1 {
                x: Some(String::from("1.0")),
                y: Some(String::from("2.0")),
                z: Some(String::from("3.0")),
                e: Some(String::from("4.0")),
                f: Some(String::from("5.0")),
                comments: None,
            },
        ),
        (
            "X1.0Y2.0Z3.0E4.0",
            G1 {
                x: Some(String::from("1.0")),
                y: Some(String::from("2.0")),
                z: Some(String::from("3.0")),
                e: Some(String::from("4.0")),
                f: None,
                comments: None,
            },
        ),
        (
            "X1.0Y2.0Z3.0",
            G1 {
                x: Some(String::from("1.0")),
                y: Some(String::from("2.0")),
                z: Some(String::from("3.0")),
                e: None,
                f: None,
                comments: None,
            },
        ),
        (
            "X1.0Y2.0",
            G1 {
                x: Some(String::from("1.0")),
                y: Some(String::from("2.0")),
                z: None,
                e: None,
                f: None,
                comments: None,
            },
        ),
        (
            "X1.0",
            G1 {
                x: Some(String::from("1.0")),
                y: None,
                z: None,
                e: None,
                f: None,
                comments: None,
            },
        ),
        (
            "Y-2.0",
            G1 {
                x: None,
                y: Some(String::from("-2.0")),
                z: None,
                e: None,
                f: None,
                comments: None,
            },
        ),
        (
            "Z0.000000001",
            G1 {
                x: None,
                y: None,
                z: Some(String::from("0.000000001")),
                e: None,
                f: None,
                comments: None,
            },
        )];
    for (input, expected) in tests.iter_mut() {
        let mut input = input;
        let result = g1_parameter_parse(&mut input).unwrap();
        assert_eq!(result, *expected);
    }
}

fn outer_parser(input: String) -> PResult<GCodeModel> {
    let mut gcode = GCodeModel::default();
    let lines = input.lines();
    // split a file into lines
    for (i, line) in lines.enumerate() {

        // store a copy of the original line
        let string_copy = String::from(line);

        // parse comments
        let (line, comments) = parse_comments(line)?;

        // clear whitespace
        let line = line.split_whitespace().collect::<String>();
        let line = line.as_str();

        // split off first word from command
        let (command, num,  mut rest) = parse_word(line)?;

        // process rest of command based on first word
        let command = match (command, num) {
            ("G", "1") => {
                let g1 = g1_parameter_parse(&mut rest)?;
                Command::G1(g1)
            }
            ("G", "28") => crate::Command::G28,
            ("G", "90") => {
                gcode.rel_xyz = false;
                Command::G90
            }
            ("G", "91") => {
                gcode.rel_xyz = true;
                Command::G91
            }
            ("M", "82") => {
                gcode.rel_e = false;
                Command::M82
            }
            ("M", "83") => {
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

#[test]
fn outer_parser_test() {
    let input = "G1 X1.0 Y2.0 Z3.0 E4.0 F5.0; hello world\nG28; hello world\nG90; hello world\nG91; hello world\nM82".to_string();
    let result = outer_parser(input).unwrap();
    let expected = GCodeModel {
        id_counter: crate::Counter::new(),
        rel_xyz: false,
        rel_e: false,
        lines: vec![
            GCodeLine {
                id: crate::Id(0),
                line_number: 0,
                command: Command::G1(G1 {
                    x: Some(String::from("1.0")),
                    y: Some(String::from("2.0")),
                    z: Some(String::from("3.0")),
                    e: Some(String::from("4.0")),
                    f: Some(String::from("5.0")),
                    comments: Some(String::from("")),
                }),
                comments: String::from(" hello world"),
            },
            GCodeLine {
                id: crate::Id(1),
                line_number: 1,
                command: Command::G28,
                comments: String::from(" hello world"),
            },
            GCodeLine {
                id: crate::Id(2),
                line_number: 2,
                command: Command::G90,
                comments: String::from(" hello world"),
            },
            GCodeLine {
                id: crate::Id(3),
                line_number: 3,
                command: Command::G91,
                comments: String::from(" hello world"),
            },
            GCodeLine {
                id: crate::Id(4),
                line_number: 4,
                command: Command::M82,
                comments: String::from(""),
            },
        ],
        vertices: HashMap::new(),
    };
}