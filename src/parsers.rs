use crate::{Command, GCodeLine, GCodeModel, G1};
use microns::Microns;
use winnow::{
    ascii::multispace1,
    combinator::{rest, separated_pair},
    error::InputError,
    token::{one_of, take, take_till, take_while},
    PResult, Parser,
};

/// parse a line until '\n' or '\r' and then clear all following whitespace
fn parse_line<'a>(input: &mut &'a str) -> PResult<&'a str> {
    // this must always consume at least one character
    let line = take_till(0.., |c| c == '\n' || c == '\r').parse_next(input)?;
    let _: PResult<&str> = multispace1.parse_next(input);
    Ok(line)
}

/// repeat the parse_line fn until the input is empty and collect to Vec
fn parse_lines<'a>(input: &mut &'a str) -> PResult<Vec<&'a str>> {
    let mut out = Vec::new();
    loop {
        if input.is_empty() {
            break;
        }
        out.push(parse_line.parse_next(input)?);
    }
    Ok(out)
}

/// parse the first word of a line by taking the first char
/// and then parsing all following numeric characters, not accepting
/// floats, negative numbers, or scientific notation
fn parse_word<'a>(input: &mut &'a str) -> PResult<(&'a str, &'a str, &'a str)> {
    (
        take(1_usize),
        take_while(0.., |c: char| c.is_numeric()),
        rest,
    )
        .parse_next(input)
}

/// Helper function to check if a character is part of a number
fn is_number_char(c: char) -> bool {
    c.is_numeric() || c == '.' || c == '-' || c == '+'
}

/// parses g1 params once the first word ("G1") has been parsed
fn g1_parameter_parse(input: &mut &str) -> PResult<G1> {
    let mut out = G1::default();
    while let Ok((c, val)) = separated_pair(
        one_of::<_, _, InputError<_>>(['X', 'Y', 'Z', 'E', 'F']),
        winnow::combinator::empty,
        take_while(1.., is_number_char).parse_to::<String>(),
    )
    .parse_next(input)
    {
        if let Ok(val) = val.parse::<f32>() {
            let val = Microns::from(val);
            match c {
                'X' => out.x = Some(val),
                'Y' => out.y = Some(val),
                'Z' => out.z = Some(val),
                'E' => out.e = Some(val),
                'F' => out.f = Some(val),
                _ => {}
            }
        }
    }
    Ok(out)
}

/// Custom error type for integrating winnow errors
/// with the main application
#[derive(Debug, PartialEq)]
pub struct GCodeParseError {
    pub message: String,
    // Byte spans are tracked, rather than line and column.
    // This makes it easier to operate on programmatically
    // and doesn't limit us to one definition for column count
    // which can depend on the output medium and application.
    pub span: std::ops::Range<usize>,
    pub input: String,
}

impl GCodeParseError {
    pub fn from_parse(
        error: winnow::error::ParseError<&str, winnow::error::ContextError>,
        input: &str,
    ) -> Self {
        // The default renderer for `ContextError` is still used but that can be
        // customized as well to better fit your needs.
        let message = error.inner().to_string();
        let input = input.to_owned();
        let start = error.offset();
        // Assume the error span is only for the first `char`.
        // Semantic errors are free to choose the entire span returned by `Parser::with_span`.
        let end = (start + 1..)
            .find(|e| input.is_char_boundary(*e))
            .unwrap_or(start);
        Self {
            message,
            span: start..end,
            input,
        }
    }
}

impl std::fmt::Display for GCodeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = annotate_snippets::Level::Error
            .title(&self.message)
            .snippet(
                annotate_snippets::Snippet::source(&self.input)
                    .fold(true)
                    .annotation(annotate_snippets::Level::Error.span(self.span.clone())),
            );
        let renderer = annotate_snippets::Renderer::plain();
        let rendered = renderer.render(message);
        rendered.fmt(f)
    }
}

impl std::error::Error for GCodeParseError {}

/// Outermost parser for gcode files
pub fn gcode_parser(input: &mut &str) -> Result<GCodeModel, GCodeParseError> {
    let mut gcode = GCodeModel::default();
    let lines = parse_lines
        .parse(input)
        .map_err(|e| GCodeParseError::from_parse(e, input))?;
    // split a file into lines
    for line in lines {
        // split off comments before parsing
        let (line, comments) = line.split_once(';').unwrap_or((line, ""));

        // store a copy of the original line for unsupported commands
        let string_copy = String::from(line);

        // clear whitespace
        let line = line.split_whitespace().collect::<String>();
        let mut line = line.as_str();

        // generate id and check first word of command
        let id = gcode.id_counter.get();
        let command = match parse_word.parse_next(&mut line) {
            // process rest of command based on first word
            Ok(("G", "1", rest)) => {
                let g1 = g1_parameter_parse
                    .parse(rest)
                    .map_err(|e| GCodeParseError::from_parse(e, input))?;
                Command::G1(g1)
            }
            Ok(("G", "90", _)) => {
                gcode.rel_xyz = false;
                Command::G90
            }
            Ok(("G", "91", _)) => {
                gcode.rel_xyz = true;
                Command::G91
            }
            Ok(("M", "82", _)) => {
                gcode.rel_e = false;
                Command::M82
            }
            Ok(("M", "83", _)) => {
                gcode.rel_e = true;
                Command::M83
            }
            _ => Command::Raw(string_copy),
        };
        gcode.lines.push(GCodeLine {
            id,
            command,
            comments: String::from(comments),
        });
    }
    Ok(gcode)
}

#[test]
fn gcode_parser_test() {
    let input = "G1 X1.0 Y2.0 Z3.0 E4.0 F5.0;hello world\nG28 W ; hello world\nG90; hello world\nG91; hello world\nM82\n; asdf".to_string();
    let mut input = input.as_str();
    let result = gcode_parser(&mut input).unwrap();
    let expected = GCodeModel {
        id_counter: crate::Counter { count: 5 },
        rel_xyz: true,
        rel_e: false,
        lines: vec![
            GCodeLine {
                id: crate::Id(0),
                command: Command::G1(G1 {
                    x: Some(Microns::from(1.0)),
                    y: Some(Microns::from(2.0)),
                    z: Some(Microns::from(3.0)),
                    e: Some(Microns::from(4.0)),
                    f: Some(Microns::from(5.0)),
                }),
                comments: String::from("hello world"),
            },
            GCodeLine {
                id: crate::Id(1),
                command: Command::Raw(String::from("G28 W ")),
                comments: String::from(" hello world"),
            },
            GCodeLine {
                id: crate::Id(2),
                command: Command::G90,
                comments: String::from(" hello world"),
            },
            GCodeLine {
                id: crate::Id(3),
                command: Command::G91,
                comments: String::from(" hello world"),
            },
            GCodeLine {
                id: crate::Id(4),
                command: Command::M82,
                comments: String::from(""),
            },
            GCodeLine {
                id: crate::Id(5),
                command: Command::Raw(String::from("")),
                comments: String::from(" asdf"),
            },
        ],
    };
    for (a, b) in result.lines.iter().zip(expected.lines.iter()) {
        assert_eq!(a, b);
    }
}

#[test]
fn parse_line_test() {
    let mut tests = [
        ("hello\n", "hello"),
        ("hello", "hello"),
        ("hello\nworld", "hello"),
        ("hello\nworld\n", "hello"),
        ("hello\nworld\nmore", "hello"),
        ("hello\nworld\nmore\n", "hello"),
        ("hello\nworld\nmore\n\n", "hello"),
        ("\n", ""),
        ("\r", ""),
        ("", ""),
    ];
    for (input, expected) in tests.iter_mut() {
        let debug = String::from(*input);
        let result = parse_line(input).expect(format!("failed to parse: {}", debug).as_str());
        assert_eq!(result, *expected);
    }
}

#[test]
fn parse_lines_test() {
    let mut tests = [
        ("hello\nworld\nmore\n", vec!["hello", "world", "more"]),
        ("hello\nworld\nmore", vec!["hello", "world", "more"]),
        ("hello\nworld\nmore\n\n", vec!["hello", "world", "more"]),
        ("hello", vec!["hello"]),
        ("hello\n", vec!["hello"]),
        ("\n", vec![""]),
        ("", vec![]),
    ];
    for (input, expected) in tests.iter_mut() {
        let result = parse_lines(input).unwrap();
        assert_eq!(result, *expected);
    }
}
#[test]
fn parse_word_test() {
    let tests = [
        ("G1", ("G", "1", "")),
        ("M1234", ("M", "1234", "")),
        ("G28W", ("G", "28", "W")),
        (
            "G1 X1.0 Y2.0 Z3.0 E4.0 F5.0",
            ("G", "1", " X1.0 Y2.0 Z3.0 E4.0 F5.0"),
        ),
    ];
    for (mut input, expected) in tests.iter() {
        assert_eq!(parse_word(&mut input).unwrap(), *expected);
    }
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

#[test]
fn g1_parameter_parse_test() {
    let mut tests = [
        (
            "X1.0Y2.0Z3.0E4.0F5.0",
            G1 {
                x: Some(Microns::from(1.0)),
                y: Some(Microns::from(2.0)),
                z: Some(Microns::from(3.0)),
                e: Some(Microns::from(4.0)),
                f: Some(Microns::from(5.0)),
            },
        ),
        (
            "X1.0Y2.0Z3.0E4.0",
            G1 {
                x: Some(Microns::from(1.0)),
                y: Some(Microns::from(2.0)),
                z: Some(Microns::from(3.0)),
                e: Some(Microns::from(4.0)),
                f: None,
            },
        ),
        (
            "X1.0Y2.0Z3.0",
            G1 {
                x: Some(Microns::from(1.0)),
                y: Some(Microns::from(2.0)),
                z: Some(Microns::from(3.0)),
                e: None,
                f: None,
            },
        ),
        (
            "X1.0Y2.0",
            G1 {
                x: Some(Microns::from(1.0)),
                y: Some(Microns::from(2.0)),
                z: None,
                e: None,
                f: None,
            },
        ),
        (
            "X1.0",
            G1 {
                x: Some(Microns::from(1.0)),
                y: None,
                z: None,
                e: None,
                f: None,
            },
        ),
        (
            "Y-2.0",
            G1 {
                x: None,
                y: Some(Microns::from(-2.0)),
                z: None,
                e: None,
                f: None,
            },
        ),
        (
            "Z0.000000001",
            G1 {
                x: None,
                y: None,
                z: Some(Microns::from(0.000000001)),
                e: None,
                f: None,
            },
        ),
    ];
    for (mut input, expected) in tests.iter_mut() {
        let result = g1_parameter_parse(&mut input).unwrap();
        assert_eq!(result, *expected);
    }
}
#[test]
fn gcode_parse_error_test() {
    let test = "0";
    let error = multispace1.parse(test).unwrap_err();
    let error = GCodeParseError::from_parse(error, test);
    assert_eq!(
        GCodeParseError {
            message: "".to_string(),
            span: 0..1,
            input: "0".to_string()
        },
        error
    );
}
