# Ferrum

Ferrum is a small browser engine written in Rust to make the web platform easier to understand. ,one inspectable stage at a time: source bytes become a DOM, styles, layout boxes, a display list, and finally pixels.

> **Current milestone:** route native mouse clicks through layout hit testing to JavaScript and repaint the page.


## Try it

Ferrum currently requires a recent stable Rust toolchain.

```sh
# Show every available Ferrum command and option.
cargo run -- --help

# Parse the sample HTML and print its DOM tree.
cargo run -- examples/hello.html

# Parse the sample CSS and print its normalized stylesheet.
cargo run -- css examples/theme.css

# Apply the stylesheet and print the styled DOM tree.
cargo run -- style examples/hello.html examples/theme.css

# Calculate and print the page's layout boxes and geometry.
cargo run -- layout examples/hello.html examples/theme.css

# Paint the HTML and CSS page to a ferrum.ppm image file.
cargo run -- paint examples/hello.html examples/theme.css ferrum.ppm

# Display the HTML and CSS page as a colored terminal preview.
cargo run -- browse examples/hello.html examples/theme.css

# Load linked CSS and JavaScript, then render the page to ferrum.ppm.
cargo run -- render examples/hello.html ferrum.ppm

# Load linked HTML, CSS, and JavaScript in an interactive graphical window.
cargo run -- window examples/hello.html
```

Ferrum's parsers are intentionally learning-sized subsets, not yet conforming implementations of the full HTML and CSS standards. HTML error recovery, selector combinators, at-rules, functions, and most CSS units remain future work. Calling those boundaries out clearly lets the project grow through measurable milestones instead of hiding complexity.

The JavaScript runtime executes modern ECMAScript, but Ferrum's DOM bridge is intentionally small: scripts can read or update `document.title`, select any element with an ID, replace its `textContent`, set its background, and register click handlers with `addEventListener`. One Boa context remains alive for the window session, so variables, closures, listeners, and DOM-facing state persist across clicks. Clicks dispatch only along the ID-bearing target-to-root path; `event.target`, `event.currentTarget`, bubbling, and `stopPropagation()` are supported. Ferrum does not yet implement listener removal, default actions, most browser Web APIs, networking, or node creation.

The `window` command resolves the first `<link rel="stylesheet">` and `<script src>` relative to the HTML file. Click a colored area in the included sample to see JavaScript change the page and window title. For debugging, the explicit form is also available: `ferrum window page.html page.css page.js`.

## Learning reference

Ferrum's progression and event-routing architecture are informed by [Web Browser Engineering](https://browser.engineering/). In particular, a native click is converted from window coordinates into a layout hit, mapped back to a DOM element, dispatched to JavaScript, and then sent through style, layout, and paint again. Ferrum implements these ideas independently in Rust and intentionally exposes a smaller platform surface.

## Why Ferrum?

Most developers use browsers every day without seeing the pipeline behind them. Ferrum aims to be compact enough to read, rigorous enough to test, and visual enough to demo. Rust makes ownership and data flow explicit while giving the project room to explore performance and safe concurrency later.

## Project status

Ferrum is experimental and educational. It is not intended for browsing untrusted content or replacing a production browser engine.