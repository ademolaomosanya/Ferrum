# Contributing to Ferrum

Ferrum welcomes small, focused changes that make browser internals clearer.

## Development setup

Install a recent stable Rust toolchain, clone the repository, and run:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Run the current demo with:

```sh
cargo run -- examples/hello.html
```

## Making a change

1. Choose one pipeline stage or parser behavior.
2. Add a focused test that captures the expected behavior.
3. Keep public types documented and error messages actionable.
4. Run the formatter, tests, and linter before opening a pull request.
5. Update the README or architecture notes if the supported behavior changes.

Good first contributions include parser edge-case tests, clearer diagnostics, DOM traversal helpers, and example documents. Larger compatibility work should start with a short design issue so the intended standards behavior is explicit.

