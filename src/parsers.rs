use winnow::{
    ascii::{multispace0, till_line_ending}, combinator::{preceded, rest, separated_pair}, error::InputError, stream::Range, token::{literal, one_of, take, take_till, take_until, take_while}, PResult, Parser
};
use std::collections::HashMap;
use crate::{GCodeModel, GCodeLine};


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
enum SupportedCommands {
    G1,
    G28,
    G90,
    G91,
    M82,
    M83
}
impl<'a> SupportedCommands {
    fn get_key(&self) -> &str {
        match self {
            SupportedCommands::G1 => "G1",
            SupportedCommands::G28 => "G28",
            SupportedCommands::G90 => "G90",
            SupportedCommands::G91 => "G91",
            SupportedCommands::M82 => "M82",
            SupportedCommands::M83 => "M83"
        }
    }
    fn get_parser(&self) -> fn(&'a mut &'a str) -> PResult<GCodeLine> {
        match self {
            SupportedCommands::G1 => g1_parse,
            _ => unimplemented!()
        }
    }
}


fn outer_parser(input: &str) -> PResult<GCodeModel> {
    let gcode = GCodeModel::default();
    let input = winnow::Located::new(input);
    // split a file into lines and remove all whitespace
    while let Ok ((line, span)) = parse_line_with_span(input) {
        let (line, comments) = parse_comments(line)?;
        if let Ok(processed) = parse_line(line) {
            gcode.lines.push(processed);
        } else {
            gcode.lines.push(GCodeLine::Unprocessed(Id(0), line.to_string()));
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
fn g1_parameter_parse<'a>(input: &mut &'a str) -> PResult<HashMap<char, String>> {
    let mut out = HashMap::new();
    while let Ok((c, val)) = separated_pair(
        one_of::<_, _, InputError<_>>(['X', 'Y', 'Z', 'E', 'F']),
        winnow::combinator::empty,
        take_while(1.., |c| is_number_char(c)).parse_to(),
    )
    .parse_next(input)
    {
        out.insert(c, val);
    }
    Ok(out)
}

fn g1_parse_test() {}

// Function that takes a processed G1 command and returns parameters
fn g1_parse<'a>(input: &'a mut &'a str) -> PResult<G1> {
    let ((span, _, params, _, comments),) = ((
        clear_whitespace,
        literal("G1"),
        g1_parameter_parse,
        literal(';'),
        rest,
    ),)
        .parse_next(input)?;
    let comments = {
        if comments.is_empty() {
            None
        } else {
            Some(String::from(comments))
        }
    };
    let (x, y, z, e, f) = (
        params.get(&'X').copied(),
        params.get(&'Y').copied(),
        params.get(&'Z').copied(),
        params.get(&'E').copied(),
        params.get(&'F').copied(),
    );
    Ok(G1 {
        x,
        y,
        z,
        e,
        f,
        comments,
        span: String::new(),
    })
}

fn line_parse<'a>(input: &'a mut &'a str, parsed: &mut Parsed) -> GCodeLine {
    let id = parsed.id_counter.get();
    let g1 = g1_parse(input);
    if let Ok(g1) = g1 {
        GCodeLine::Unprocessed(Id(0), String::new())
    } else {
        GCodeLine::Unprocessed(Id(0), String::new())
    }
}
