use crate::{Command, GCodeLine, GCodeModel, Microns, PrintMetadata};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

fn parse_gcode(input: PathBuf) -> Result<GCodeModel, Box<dyn std::error::Error>> {
    let mut out = GCodeModel::default();
    let file = File::open(input)?;
    let mut reader = BufReader::new(file);
    let mut buf = Vec::with_capacity(4096);
    // keep reading until the file and the buffer are empty
    while reader.read(&mut buf)? > 0 || buf.len() > 0 {
        out.lines.push(next_line(buf.as_mut_slice()));
    }
    out.metadata = PrintMetadata::from(&out);
    Ok(out)
}
fn next_line(mut input: &mut [u8]) -> GCodeLine {
    let mut line = String::new();
    while let Some((first, rest)) = input.split_first_mut() {
        if *first == b'\n' {
            break;
        }
        line.push(*first as char);
        input = rest;
    }
    let (line, comments) = line.split_once(';').unwrap_or((&line, ""));
    let (mut line, comments) = (
        line.split_ascii_whitespace().collect::<String>(),
        comments.to_string(),
    );
    let line = unsafe { line.as_bytes_mut() };
    let command = parse_command(line);
    GCodeLine { command, comments }
}
fn clear_whitespace(mut input: &mut [u8]) {
    while input
        .first()
        .map_or(false, |byte| byte.is_ascii_whitespace())
    {
        input = &mut input[1..];
    }
}
// decommented line to command
fn parse_command(input: &mut [u8]) -> Command {
    let (first, rest) = next_word(input).expect("no characters found");
    match (first, rest.as_str()) {
        ('G', "1") => {
            let params = parse_params(input);
            Command::G1 {
                x: params
                    .get(&'X')
                    .and_then(|v| v.parse::<f32>().ok().and_then(|f| Some(Microns::from(f)))),
                y: params
                    .get(&'Y')
                    .and_then(|v| v.parse::<f32>().ok().and_then(|f| Some(Microns::from(f)))),
                z: params
                    .get(&'Z')
                    .and_then(|v| v.parse::<f32>().ok().and_then(|f| Some(Microns::from(f)))),
                e: params
                    .get(&'E')
                    .and_then(|v| v.parse::<f32>().ok().and_then(|f| Some(Microns::from(f)))),
                f: params
                    .get(&'F')
                    .and_then(|v| v.parse::<f32>().ok().and_then(|f| Some(Microns::from(f)))),
            }
        }
        ('G', "90") => Command::G90,
        ('G', "91") => Command::G91,
        ('M', "82") => Command::M82,
        ('M', "83") => Command::M83,
        ('G', "28") => Command::Home((String::from_utf8_lossy(input).to_string())),
        _ => Command::Raw(String::from_utf8_lossy(input).to_string()),
    }
}

// after the first word is parsed, parse the rest of the line
// assuming it is a set of key value pairs, and handling the line differently
// if not
fn parse_params(input: &mut [u8]) -> std::collections::HashMap<char, String> {
    let mut out = std::collections::HashMap::new();
    while let Ok((key, val)) = next_word(input) {
        out.insert(key, val);
    }
    out
}

fn is_number_char(byte: u8) -> bool {
    byte.is_ascii_digit() || byte == b'.' || byte == b'-' || byte == b'+'
}

fn next_word(mut input: &mut [u8]) -> Result<(char, String), &'static str> {
    if input.is_empty() {
        return Err("empty input");
    }
    let first = input[0];
    if !first.is_ascii_alphabetic() {
        return Err("first char is not alphabetic");
    }
    input = &mut input[1..];
    let mut out = String::new();
    while let Some((first, rest)) = input.split_first_mut() {
        if !(*first as char).is_numeric() {
            break;
        }
        out.push(*first as char);
        input = rest;
    }
    Ok((first as char, out))
}

/*





















use core::str;
use std::{
    default,
    io::{BufReader, Read},
};

use crate::*;
use winnow::{
    ascii::multispace1,
    combinator::{alt, repeat, rest, separated_pair},
    error::InputError,
    stream::AsBStr,
    token::{one_of, take, take_till, take_while},
    Bytes, PResult, Parser,
};

/// parse a line until '\n' or '\r' and then clear all following whitespace
fn parse_line_text<'a>(input: &mut &'a str) -> PResult<&'a str> {
    // this must always consume at least one character
    take_till(0.., |c| c == '\n' || c == '\r').parse_next(input)
}

fn parse_line<'a>(input: &mut &'a str) -> PResult<(&'a str, &'a str)> {
    (parse_line_text, multispace1).parse_next(input)
}
// move the input until newline is reached and return the preceding chars
fn next_line<'a>(input: &mut &'a [u8]) -> Result<&'a [u8]> {
    while let Some(next) = input.first() {

    }
    let out =
    let ret = {
        if let Some(next) = input.split(|&byte| byte == b'\n').next() {
            input
            next
        } else {
            input
        }
    };
    trim_leading_space(input);
    Ok(ret)
}

fn trim_leading_space<'a>(input: &mut &'a [u8]) {
    while let Some((first, rest)) = input.split_first() {
        if first.is_ascii_whitespace() {
            *input = rest;
        } else {
            break;
        }
    }
}

/// repeat the parse_line fn until the input is empty and collect to Vec
fn parse_lines<'a>(input: &mut &'a str) -> PResult<Vec<(&'a str, &'a str)>> {
    repeat(1.., parse_line).parse_next(input)
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
fn g1_parameter_parse<'a>(input: &mut &'a str) -> PResult<[&'a str; 5]> {
    let mut out = [""; 5];
    while let Ok((c, val)) = separated_pair(
        one_of::<_, _, InputError<_>>(['X', 'Y', 'Z', 'E', 'F']),
        winnow::combinator::empty,
        take_while(1.., is_number_char),
    )
    .parse_next(input)
    {
        match c {
            'X' => out[0] = val,
            'Y' => out[1] = val,
            'Z' => out[2] = val,
            'E' => out[3] = val,
            'F' => out[4] = val,
            _ => {}
        }
    }
    Ok(out)
}

/// Custom error type for integrating winnow errors
/// with the main application
#[derive(Debug, Default, PartialEq)]
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
pub fn parse_file(input: &Path) -> Result<GCodeModel, GCodeParseError> {
    let f = std::fs::File::open(input).map_err(|e| GCodeParseError {
        message: e.to_string(),
        span: 0..0,
        input: String::new(),
    })?;
    let extension = input.extension().and_then(|ext| ext.to_str());
    if extension != Some("gcode") {
        return Err(GCodeParseError {
            message: format!("Invalid file extension: {}", extension.unwrap_or("")),
            span: 0..0,
            input: String::new(),
        });
    }
    parse_gcode(std::io::BufReader::new(f))
}

pub fn parse_gcode(mut reader: BufReader<std::fs::File>) -> Result<GCodeModel, GCodeParseError> {
    let mut gcode = GCodeModel::default();
    let mut buffer = Vec::with_capacity(4096);
    while reader.read(&mut buffer).map_err(|e| GCodeParseError {
        message: e.to_string(),
        ..Default::default()
    })? > 0
    {
        let mut lines = buffer.drain(..).collect::<Vec<u8>>();
        let mut trail = lines.split_off(
            lines
                .iter()
                .rposition(|byte| *byte == b'\n')
                .unwrap_or(buffer.len()),
        );
        if trail.len() > 0 {
            buffer.append(&mut trail);
        }
        lines.split(|byte| *byte == b'\n').for_each(|l| {
            if let Ok(l) = str::from_utf8(l) {
                let (line, comments) = l.split_once(';').unwrap_or((l, ""));
                let string_copy = String::from(line);
                let line = line.split_whitespace().collect::<String>();
                let mut line = line.as_str();
                let command = match parse_word.parse_next(&mut line) {
                    Ok(("G", "1", mut rest)) => {
                        let g1 = g1_parameter_parse.parse_next(&mut rest).unwrap();
                        Command::G1 {
                            x: g1[0].parse().ok(),
                            y: g1[1].parse().ok(),
                            z: g1[2].parse().ok(),
                            e: g1[3].parse().ok(),
                            f: g1[4].parse().ok(),
                        }
                    }
                    Ok(("G", "28", _)) => Command::Home(string_copy),
                    Ok(("G", "90", _)) => Command::G90,
                    Ok(("G", "91", _)) => Command::G91,
                    Ok(("M", "82", _)) => Command::M82,
                    Ok(("M", "83", _)) => Command::M83,
                    _ => Command::Raw(string_copy),
                };
                gcode.lines.push(GCodeLine {
                    command,
                    comments: String::from(comments),
                });
            } else {
                gcode.lines.push(GCodeLine {
                    command: Command::Raw(String::from_utf8_lossy(l).to_string()),
                    comments: String::new(),
                });
            }
        });
    }

    Ok(gcode)
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
    */
