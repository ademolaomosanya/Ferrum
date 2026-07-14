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
    let arguments: Vec<String> = env::args().skip(1).collect();
    match arguments.as_slice() {
        [flag] if flag == "--help" || flag == "-h" => {
            print_help();
            Ok(())
        }
        [flag] if flag == "--version" || flag == "-V" => {
            println!("ferrum {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        [mode, input] if mode == "css" => inspect_css(input),
        [mode, html_input, css_input] if mode == "style" => inspect_styles(html_input, css_input),
        [mode, html_input, css_input] if mode == "layout" => inspect_layout(html_input, css_input),
        [mode, html_input, css_input] if mode == "browse" => browse(html_input, css_input),
        [mode, html_input, css_input, output] if mode == "paint" => {
            paint_page(html_input, css_input, output)
        }
        [input] => inspect_html(input),
        _ => Err(usage()),
    }
}

fn inspect_html(input: &str) -> Result<(), String> {
    let source = read_source(input)?;
    let document = ferrum::html::parse(&source).map_err(|error| error.to_string())?;
    print!("{}", document.pretty_print());
    Ok(())
}

fn inspect_css(input: &str) -> Result<(), String> {
    let source = read_source(input)?;
    let stylesheet = ferrum::css::parse(&source).map_err(|error| error.to_string())?;
    print!("{}", stylesheet.pretty_print());
    Ok(())
}

fn inspect_styles(html_input: &str, css_input: &str) -> Result<(), String> {
    let (document, stylesheet) = parse_page(html_input, css_input)?;
    let styled = ferrum::style::style_tree(&document, &stylesheet);
    print!("{}", styled.pretty_print());
    Ok(())
}

fn inspect_layout(html_input: &str, css_input: &str) -> Result<(), String> {
    let (document, stylesheet) = parse_page(html_input, css_input)?;
    let styled = ferrum::style::style_tree(&document, &stylesheet);
    let layout = ferrum::layout::layout_tree(&styled, 800.0, 600.0);
    print!("{}", layout.pretty_print());
    Ok(())
}

fn paint_page(html_input: &str, css_input: &str, output: &str) -> Result<(), String> {
    let (document, stylesheet) = parse_page(html_input, css_input)?;
    let styled = ferrum::style::style_tree(&document, &stylesheet);
    let layout = ferrum::layout::layout_tree(&styled, 800.0, 600.0);
    let canvas = ferrum::paint::paint(&layout, 800, 600);
    canvas
        .save_ppm(output)
        .map_err(|error| format!("could not write {output}: {error}"))?;
    println!(
        "painted {}x{} pixels to {output}",
        canvas.width, canvas.height
    );
    Ok(())
}

fn browse(html_input: &str, css_input: &str) -> Result<(), String> {
    let (document, stylesheet) = parse_page(html_input, css_input)?;
    let styled = ferrum::style::style_tree(&document, &stylesheet);
    let layout = ferrum::layout::layout_tree(&styled, 800.0, 600.0);
    let canvas = ferrum::paint::paint(&layout, 800, 600);
    println!("Ferrum — {html_input} ({}x{})", canvas.width, canvas.height);
    print!("{}", canvas.ansi_preview(80));
    Ok(())
}

fn parse_page(
    html_input: &str,
    css_input: &str,
) -> Result<(ferrum::dom::Node, ferrum::css::Stylesheet), String> {
    if html_input == "-" && css_input == "-" {
        return Err("HTML and CSS cannot both read from stdin".into());
    }
    let html_source = read_source(html_input)?;
    let css_source = read_source(css_input)?;
    let document = ferrum::html::parse(&html_source).map_err(|error| error.to_string())?;
    let stylesheet = ferrum::css::parse(&css_source).map_err(|error| error.to_string())?;
    Ok((document, stylesheet))
}

fn read_source(input: &str) -> Result<String, String> {
    if input == "-" {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|error| format!("could not read stdin: {error}"))?;
        Ok(source)
    } else {
        fs::read_to_string(input).map_err(|error| format!("could not read {input}: {error}"))
    }
}

fn usage() -> String {
    "invalid arguments; run 'ferrum --help' for usage".into()
}

fn print_help() {
    println!(
        "Ferrum — an educational browser engine\n\n\
Usage:\n  \
ferrum <file.html | ->\n  \
ferrum css <file.css | ->\n  \
ferrum style <file.html | -> <file.css | ->\n  \
ferrum layout <file.html | -> <file.css | ->\n  \
ferrum paint <file.html | -> <file.css | -> <output.ppm>\n  \
ferrum browse <file.html | -> <file.css | ->\n\n\
Commands:\n  \
css      Parse and normalize a stylesheet\n  \
style    Inspect the styled DOM tree\n  \
layout   Inspect block-layout geometry\n  \
paint    Render an 800x600 binary PPM image\n  \
browse   Display an ANSI true-color terminal preview\n\n\
Options:\n  \
-h, --help       Print help\n  \
-V, --version    Print version"
    );
}
