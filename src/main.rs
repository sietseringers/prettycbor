use anyhow::{anyhow, bail, Context, Result};
use clap::{ArgGroup, Parser};
use std::fmt::Write;
use std::io;
use std::io::{Read, Write as IoWrite};
use std::process::{Command, Stdio};

/// One indentation level is by default this much spaces.
const SPACE_COUNT: usize = 2;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(group(ArgGroup::new("mode").args(&["hex", "diag"])))]
struct CliInput {
    /// Let cbor2diag.rb parse embedded CBOR using its -e flag
    #[arg(short, long)]
    embedded: bool,

    /// Amount of spaces used for indentation
    #[arg(short, long, default_value_t = SPACE_COUNT)]
    indent: usize,

    /// Force parsing input as hexadecimal which is passed through cbor2diag.rb
    #[arg(short = 'x', long)]
    hex: bool,

    /// Force acting directly on the input
    #[arg(short, long)]
    diag: bool,

    /// Data to act on, either hexadecimal or diagnostic. If absent, stdin is read.{n}
    /// If neither --hex or --diag is given, the input is parsed as hexadecimal.{n}
    /// If that works, the result is passed through cbor2diag.rb and then acted upon.{n}
    /// If not, the input is acting upon directly.
    data: Option<String>,
}

fn main() -> Result<()> {
    let cli_input = CliInput::parse();

    let input_raw = match cli_input.data {
        Some(inp) => inp,
        None => {
            let mut stdin_buf = String::new();
            io::stdin().read_to_string(&mut stdin_buf)?;
            stdin_buf.into()
        }
    };

    if input_raw.len() == 0 {
        bail!("no input received, pass input either via stdin or command-line argument");
    }

    // Determine the input for the pretty printing as specified by the options
    let input: Vec<u8> = if cli_input.hex {
        cbor2diag(
            hex::decode(&input_raw).context("hexadecimal decoding failed")?,
            cli_input.embedded,
        )?
    } else if cli_input.diag {
        input_raw.into_bytes()
    } else {
        try_hex_cbor2diag(input_raw, cli_input.embedded)?
    };

    // Do our thing
    println!("{}", pretty_print(input.as_slice(), cli_input.indent));
    Ok(())
}

fn try_hex_cbor2diag(input_raw: String, embedded: bool) -> Result<Vec<u8>> {
    let input = match hex::decode(&input_raw) {
        Ok(j) => cbor2diag(j, embedded)?,
        Err(_) => input_raw.into_bytes(),
    };
    Ok(input)
}

const NO_CBOR2DIAG_ERR: &str = "failed to locate cbor2diag.rb.
Ensure cbor2diag.rb is installed (using \"gem install cbor-diag\") and present in your $PATH,
or input diagnostic CBOR instead (e.g. using https://https://cbor.me).";

fn cbor2diag(input: Vec<u8>, embedded: bool) -> Result<Vec<u8>> {
    let cbor2diag = which::which("cbor2diag.rb").context(NO_CBOR2DIAG_ERR)?;

    let args: &[&str] = if embedded { &["-e"] } else { &[] };
    let mut process = Command::new(cbor2diag)
        .args(args)
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
        bail!("cbor2diag.rb failed")
    } else {
        Ok(output.stdout)
    }
}

fn pretty_print(input: &[u8], space_count: usize) -> String {
    // Specify a capacity to try to avoid reallocation. The factor 2 is a little arbitrary
    // but should suffice in most cases.
    let mut output = String::with_capacity(input.len() * 2);

    let mut in_str = false;
    let mut indent_count = 0;

    for idx in 0..input.len() {
        let c = input[idx] as char;
        let prev = idx.checked_sub(1).map(|i| input[i] as char);
        let next = input.get(idx + 1).map(|b| *b as char);

        if c == '\"' && prev.map_or(true, |ch| ch != '\\') {
            in_str = !in_str;
        }

        if in_str {
            // If we're in a string, always just print it
            write_char(&mut output, c, in_str);
        } else {
            process_char(
                c,
                &mut output,
                &mut indent_count,
                space_count,
                prev,
                next,
                in_str,
            );
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
    in_str: bool,
) {
    if is_open(c) {
        write_char(output, c, in_str);
        *indent_count += 1;
        if next.map_or(false, |ch| !is_close(ch)) {
            newline(output, *indent_count, space_count);
        }
    } else if is_close(c) {
        *indent_count -= 1;
        if prev.map_or(false, |ch| !is_open(ch)) {
            newline(output, *indent_count, space_count);
        }
        write_char(output, c, in_str);
    } else if c == ',' {
        write_char(output, c, in_str);
        newline(output, *indent_count, space_count);
    } else {
        write_char(output, c, in_str);
    }
}

fn is_open(c: char) -> bool {
    c == '{' || c == '['
}

fn is_close(c: char) -> bool {
    c == '}' || c == ']'
}

fn write_char(output: &mut String, c: char, in_str: bool) {
    if !in_str && c == ' ' {
        return;
    }
    output.write_char(c).unwrap();
    if !in_str && c == ':' {
        output.write_char(' ').unwrap();
    }
}

fn newline(output: &mut String, indent_count: usize, space_count: usize) {
    output.write_char('\n').unwrap();
    output
        .write_str(&(" ".repeat(indent_count * space_count)))
        .unwrap();
}
