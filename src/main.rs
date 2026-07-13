use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("ferrum: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    let Some(input) = arguments.next() else {
        return Err("usage: ferrum <file.html | ->".into());
    };
    if arguments.next().is_some() {
        return Err("expected exactly one input path".into());
    }

    let source = if input == "-" {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|error| format!("could not read stdin: {error}"))?;
        source
    } else {
        fs::read_to_string(&input).map_err(|error| format!("could not read {input}: {error}"))?
    };

    let document = ferrum::html::parse(&source).map_err(|error| error.to_string())?;
    print!("{}", document.pretty_print());
    Ok(())
}
