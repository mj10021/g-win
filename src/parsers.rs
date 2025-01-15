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
    if input.len() < 2 {
        return Err("empty input");
    }
    let first = input[0];
    if !first.is_ascii_alphabetic() {
        return Err("first char is not alphabetic");
    }
    input = &mut input[1..];
    let mut out = String::new();
    let init = (input[0] as char).is_numeric();

    while let Some((first, rest)) = input.split_first_mut() {
        if init != (*first as char).is_numeric() {
            break;
        }
        out.push(*first as char);
        input = rest;
    }
    Ok((first as char, out))
}
