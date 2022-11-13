use anyhow::{anyhow, bail, Context, Result};
use std::fmt::Write;
use std::io::{Read, Write as IoWrite};
use std::process::{Command, Stdio};
use std::{env, io};

/// One indentation level is this much spaces.
const SPACE_COUNT: usize = 2;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        bail!("{} arguments received, 0 or 1 expected", args.len() - 1);
    }

    let mut stdin_buf = String::new();
    let input_raw = if args.len() == 2 {
        args[1].clone()
    } else {
        io::stdin().read_to_string(&mut stdin_buf)?;
        stdin_buf
    };

    if input_raw.len() == 0 {
        bail!("no input received, pass input either via stdin or command-line argument");
    }

    // If input is valid hexadecimal, run cbor2diag.rb on it an use that. Otherwise, just use the
    // input directly.
    let i = hex::decode(&input_raw);
    let input = match i {
        Ok(j) => cbor2diag(j)?,
        Err(_) => input_raw.into_bytes(),
    };

    println!("{}", pretty_print(input.as_slice(), SPACE_COUNT));
    Ok(())
}

fn cbor2diag(input: Vec<u8>) -> Result<Vec<u8>> {
    let cbor2diag = which::which("cbor2diag.rb").context("failed to locate cbor2diag.rb")?;

    let mut process = Command::new(cbor2diag)
        .arg("-e")
        .stdin(Stdio::piped()) // Pipe through.
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = process
        .stdin
        .take()
        .ok_or(anyhow!("failed to open stdin"))?;
    std::thread::spawn(move || {
        stdin
            .write_all(input.as_slice())
            .expect("failed to write to stdin");
    });

    let output = process.wait_with_output()?;
    if !output.status.success() {
        bail!(
            "cbor2diag returned error: {}",
            String::from_utf8(output.stderr).unwrap()
        )
    } else {
        Ok(output.stdout)
    }
}

fn pretty_print(input: &[u8], space_count: usize) -> String {
    let mut output = String::with_capacity(input.len() * 2);
    let mut in_str = false;
    let mut indent_count = 0;
    let len = input.len();

    for idx in 0..len {
        let c = input[idx] as char;
        let prev = idx.checked_sub(1).map(|i| input[i] as char);
        let next = input.get(idx + 1).map(|b| *b as char);

        if c == '\"' && prev.map_or(true, |ch| ch != '\\') {
            in_str = !in_str;
        }

        if in_str {
            // If we're in a string, always just print it
            write_char(&mut output, c);
        } else {
            process_char(c, &mut output, &mut indent_count, space_count, prev, next);
        }
    }

    output
}

fn process_char(
    c: char,
    output: &mut String,
    indent_count: &mut usize,
    space_count: usize,
    prev: Option<char>,
    next: Option<char>,
) {
    if is_open(c) {
        write_char(output, c);
        *indent_count += 1;
        if next.map_or(false, |ch| !is_close(ch)) {
            newline(output, *indent_count, space_count);
        }
    } else if is_close(c) {
        *indent_count -= 1;
        if prev.map_or(false, |ch| !is_open(ch)) {
            newline(output, *indent_count, space_count);
        }
        write_char(output, c);
    } else if c == ',' {
        write_char(output, c);
        newline(output, *indent_count, space_count);
    } else {
        write_char(output, c);
    }
}

fn is_open(c: char) -> bool {
    c == '{' || c == '['
}

fn is_close(c: char) -> bool {
    c == '}' || c == ']'
}

fn write_char(output: &mut String, c: char) {
    if c == ' ' {
        return;
    }
    output.write_char(c).unwrap();
    if c == ':' {
        output.write_char(' ').unwrap();
    }
}

fn newline(output: &mut String, indent_count: usize, space_count: usize) {
    output.write_char('\n').unwrap();
    output
        .write_str(&(" ".repeat(indent_count * space_count)))
        .unwrap();
}
