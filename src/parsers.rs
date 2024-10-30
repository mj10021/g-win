use crate::{Command, GCodeLine, GCodeModel, G1};
use winnow::{
    ascii::multispace1,
    combinator::{eof, rest, separated_pair},
    error::{ErrMode, InputError},
    token::{one_of, take, take_till, take_while},
    PResult, Parser,
};

fn parse_line<'a>(input: &mut &'a str) -> PResult<&'a str> {
    // this must always consume at least one character
    let line = take_till(0.., |c| c == '\n' || c == '\r').parse_next(input)?;
    let _: PResult<&str> = multispace1.parse_next(input);
    Ok(line)
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

fn parse_lines<'a>(input: &mut &'a str) -> PResult<Vec<&'a str>> {
    let mut out = Vec::new();
    loop {
        if eof::<&str, ErrMode<InputError<&str>>>
            .parse_next(input)
            .is_ok()
        {
            break;
        }
        out.push(parse_line.parse_next(input)?);
    }
    Ok(out)
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

fn parse_word<'a>(input: &mut &'a str) -> PResult<(&'a str, &'a str, &'a str)> {
    (
        take(1_usize),
        take_while(0.., |c: char| c.is_numeric()),
        rest,
    )
        .parse_next(input)
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
            },
        ),
    ];
    for (mut input, expected) in tests.iter_mut() {
        let result = g1_parameter_parse(&mut input).unwrap();
        assert_eq!(result, *expected);
    }
}

#[derive(Debug, PartialEq)]
pub struct GCodeParseError {
    message: String,
    // Byte spans are tracked, rather than line and column.
    // This makes it easier to operate on programmatically
    // and doesn't limit us to one definition for column count
    // which can depend on the output medium and application.
    span: std::ops::Range<usize>,
    input: String,
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

#[test]
fn gcode_parse_error_test() {
    let test = "0";
    let error = multispace1.parse(test).unwrap_err();
    let error = GCodeParseError::from_parse(error, test);
    assert_eq!(GCodeParseError { message: "".to_string(), span: 0..1, input: "0".to_string() }, error);
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

pub fn gcode_parser(input: &mut &str) -> Result<GCodeModel, GCodeParseError> {
    let mut gcode = GCodeModel::default();
    let lines = parse_lines
        .parse(input)
        .map_err(|e| GCodeParseError::from_parse(e, input))?;
    // split a file into lines
    for (i, line) in lines.into_iter().enumerate() {
        // split off comments before parsing
        let (line, comments) = {
            if line.starts_with(";") {
                ("", line.split_at(1).1)
            } else if let Some((line, comments)) = line.split_once(';') {
                (line, comments)
            } else {
                (line, "")
            }
        };

        // store a copy of the original line for unsupported commands
        let string_copy = String::from(line);

        // clear whitespace
        let line = line.split_whitespace().collect::<String>();
        let mut line = line.as_str();

        // split off first word from command
        let parsed_word = parse_word.parse_next(&mut line);

        if parsed_word.is_err() {
            let id = gcode.id_counter.get();
            gcode.lines.push(GCodeLine {
                id,
                line_number: i,
                command: Command::Raw(string_copy.clone()),
                comments: String::from(comments),
            });
            continue;
        }
        let (command, num, rest) = parsed_word.unwrap();
        // process rest of command based on first word
        let command = match (command, num) {
            ("G", "1") => {
                let g1 = g1_parameter_parse
                    .parse(rest)
                    .map_err(|e| GCodeParseError::from_parse(e, input))?;
                Command::G1(g1)
            }
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
            _ => Command::Raw(string_copy),
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
                line_number: 0,
                command: Command::G1(G1 {
                    x: Some(String::from("1.0")),
                    y: Some(String::from("2.0")),
                    z: Some(String::from("3.0")),
                    e: Some(String::from("4.0")),
                    f: Some(String::from("5.0")),
                }),
                comments: String::from("hello world"),
            },
            GCodeLine {
                id: crate::Id(1),
                line_number: 1,
                command: Command::Raw(String::from("G28 W ")),
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
            GCodeLine {
                id: crate::Id(5),
                line_number: 5,
                command: Command::Raw(String::from("")),
                comments: String::from(" asdf"),
            },
        ],
    };
    for (a, b) in result.lines.iter().zip(expected.lines.iter()) {
        assert_eq!(a, b);
    }
}
