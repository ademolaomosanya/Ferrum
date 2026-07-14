# Ferrum

Ferrum is a small browser engine written in Rust to make the web platform easier to understand. ,one inspectable stage at a time: source bytes become a DOM, styles, layout boxes, a display list, and finally pixels.

> **Current milestone:** load and render a local page in Ferrum's terminal browser shell.

<img width="1536" height="1024" alt="Ferrum project artwork" src="https://github.com/user-attachments/assets/2f34f710-ef0c-418d-9de0-d37ea1515ad2" />

## Try it

Ferrum currently requires a recent stable Rust toolchain.

```sh
cargo run -- --help
cargo run -- examples/hello.html
cargo run -- css examples/theme.css
cargo run -- style examples/hello.html examples/theme.css
cargo run -- layout examples/hello.html examples/theme.css
cargo run -- paint examples/hello.html examples/theme.css ferrum.ppm
cargo run -- browse examples/hello.html examples/theme.css
```

You can also pipe a document through standard input:

```sh
printf '<main><h1>Hello, Ferrum!</h1></main>' | cargo run -- -
```

The output is a deterministic representation of the parsed tree:

```text
#document
  <main>
    <h1>
      "Hello, Ferrum!"
```

## What works today

- A DOM model for documents, elements, text, and comments
- Nested HTML elements and standard void elements
- Quoted, unquoted, and boolean attributes
- Doctypes, comments, Unicode text, and useful parse errors
- A CSS object model with rules, selectors, declarations, typed values, and specificity
- Type, class, ID, universal, compound, and selector-list parsing
- Keyword values, pixel lengths, and 3/4/6/8-digit hex colors
- Selector matching across DOM tag names, IDs, and whitespace-separated classes
- Cascade resolution using specificity and stylesheet source order
- Basic `color` inheritance and deterministic styled-tree output
- Block width calculation, auto margins, and normal vertical flow
- Content, padding, border, and margin box geometry
- Explicit heights, `display: none`, and deterministic layout-tree output
- Background and per-edge border display commands
- Clipped raster painting with source-over alpha blending
- Dependency-free binary PPM image output
- Wrapped text layout and a built-in 5×7 bitmap font
- A true-color terminal browser preview for local pages
- A dependency-free CLI, unit tests, formatting, and strict linting

Ferrum's parsers are intentionally learning-sized subsets, not yet conforming implementations of the full HTML and CSS standards. HTML error recovery, selector combinators, at-rules, functions, and most CSS units remain future work. Calling those boundaries out clearly lets the project grow through measurable milestones instead of hiding complexity.

## Verification

Ferrum's unit tests cover each rendering stage, while black-box CLI tests execute the compiled binary and verify successful parsing, diagnostics, metadata, and image output.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same checks and renders the example page as an end-to-end smoke test.

## Roadmap

- [x] **Milestone 1 — DOM:** HTML input becomes an inspectable document tree
- [x] **Milestone 2 — CSS:** parse selectors and declarations into a CSS object model
- [x] **Milestone 3 — Style:** match selectors and compute styled nodes
- [x] **Milestone 4 — Layout:** generate block layout boxes and geometry
- [x] **Milestone 5 — Paint:** turn the layout tree into a display list and raster image
- [x] **Milestone 6 — Browser shell:** load a local page and display the rendered result

See [ARCHITECTURE.md](ARCHITECTURE.md) for component boundaries and [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow.

## Why Ferrum?

Most developers use browsers every day without seeing the pipeline behind them. Ferrum aims to be compact enough to read, rigorous enough to test, and visual enough to demo. Rust makes ownership and data flow explicit while giving the project room to explore performance and safe concurrency later.

## Project status

Ferrum is experimental and educational. It is not intended for browsing untrusted content or replacing a production browser engine.

## License

Ferrum is available under the [MIT License](LICENSE).
