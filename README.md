# Ferrum

Ferrum is a small browser engine written in Rust to make the web platform easier to understand. It is being built in public as an MLH Fellowship project, one inspectable stage at a time: source bytes become a DOM, styles, layout boxes, a display list, and finally pixels.

> **Current milestone:** parse an HTML document and inspect its DOM tree from the terminal.

<img width="1536" height="1024" alt="Ferrum project artwork" src="https://github.com/user-attachments/assets/2f34f710-ef0c-418d-9de0-d37ea1515ad2" />

## Try it

Ferrum currently requires a recent stable Rust toolchain.

```sh
cargo run -- examples/hello.html
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
- A dependency-free CLI, unit tests, formatting, and strict linting

Ferrum's parser is intentionally a learning-sized subset, not yet a conforming implementation of the WHATWG HTML parsing algorithm. Calling that boundary out clearly lets the project grow through measurable milestones instead of hiding complexity.

## Roadmap

- [x] **Milestone 1 — DOM:** HTML input becomes an inspectable document tree
- [ ] **Milestone 2 — CSS:** parse selectors and declarations into a CSS object model
- [ ] **Milestone 3 — Style:** match selectors and compute styled nodes
- [ ] **Milestone 4 — Layout:** generate block and inline layout boxes
- [ ] **Milestone 5 — Paint:** turn the layout tree into a display list and raster image
- [ ] **Milestone 6 — Browser shell:** load a local page and display the rendered result

See [ARCHITECTURE.md](ARCHITECTURE.md) for component boundaries and [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow.

## Why Ferrum?

Most developers use browsers every day without seeing the pipeline behind them. Ferrum aims to be compact enough to read, rigorous enough to test, and visual enough to demo. Rust makes ownership and data flow explicit while giving the project room to explore performance and safe concurrency later.

## Project status

Ferrum is experimental and educational. It is not intended for browsing untrusted content or replacing a production browser engine.

## License

Ferrum is available under the [MIT License](LICENSE).
