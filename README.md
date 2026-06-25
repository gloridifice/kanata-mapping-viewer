# Kanata Mapping Viwer

Reads [kanata](https://github.com/jtroo/kanata) `.kbd` config files and generates a single self-contained static HTML file: each `defsrc` / `deflayer` is rendered as a keyboard diagram, with layers stacked vertically. Supports `deflayermap` sparse layers, `(include ...)`, `(platform (win) ...)` platform branches, `@alias` tooltips, and `(unicode x)` parsing.

Output is pure static HTML + CSS (tooltips use `:hover` popovers) — no JavaScript, no external dependencies, openable directly in a browser or embeddable elsewhere.

## Usage

Clone this project.

```
cargo run -- <input.kbd> [-o output.html] [--platform win|linux|macos] [-h]
```

| Argument              | Description                                                                                                    |
| --------------------- | -------------------------------------------------------------------------------------------------------------- |
| `<input.kbd>`         | Entry config file. `(include ...)` is resolved recursively (paths relative to the including file's directory). |
| `-o, --output <path>` | Output HTML path. Omit to write to stdout.                                                                     |
| `--platform <name>`   | Selects the `(platform (name) ...)` branch. Defaults to `win`.                                                 |
| `-h, --help`          | Show help.                                                                                                     |

Examples:

```sh
cargo run -- kanata/koiro.kbd -o out.html
cargo run -- kanata/koiro.kbd --platform linux -o out.html
# pipe to a file on Windows
cargo run -- kanata/koiro.kbd | Out-File -Encoding utf8 out.html
```

The generated HTML is a single file with CSS inlined at compile time via `include_str!`.

## Build

Requires Rust 1.94+ (edition 2024).

| Task          | Command                                                               |
| ------------- | --------------------------------------------------------------------- |
| Build CLI     | `cargo build -p kanata-viewer-cli` → `target/debug/kanata-viewer.exe` |
| Release build | `cargo build --release -p kanata-viewer-cli`                          |
| Tests         | `cargo test -p kanata-viewer-core`                                    |
| Lint          | `cargo clippy --workspace -- -D warnings`                             |

## Architecture / Maintenance

### Workspace layout

```
kanata-viewer/
├── Cargo.toml                 workspace root
├── crates/
│   ├── core/                  pure logic library
│   │   ├── assets/
│   │   │   └── style.css      single CSS source (inlined into HTML via include_str!)
│   │   └── src/
│   │       ├── lib.rs         facade: re-exports + render_file() end-to-end
│   │       ├── preprocess.rs  (include ...) text splicing, recursive + cycle detection
│   │       ├── sexpr.rs       S-expression tokenizer, each node carries a Span (byte range)
│   │       ├── parser.rs      sexpr → Model, with platform branch filtering
│   │       ├── layout.rs      computes grid (row/col/colspan) from defsrc key spans
│   │       ├── display.rs     KeyDisplay trait + DefaultDisplay
│   │       └── render.rs      Model → HTML fragment / full document
│   └── cli/                   binary: arg parsing → core → write file/stdout
└── kanata/                    sample configs
```

### Data flow

```
.kbd file
  │
  ▼ preprocess::preprocess(path)        recursive include → single string
  │
  ▼ sexpr::parse(source)                Vec<Sexp>, each node carries a Span
  │
  ▼ parser::parse(source, platform)     filter platform branches, extract
  │                                     defsrc/deflayer/deflayermap/defalias
  │                                     → Model { src, aliases, layers }
  │
  ▼ layout::compute_layout(source, src_spans)
  │                                     grid positions from each key's source location
  │
  ▼ render::render_fragment(model, &display)
  │                                     for each key, call display.display() → label/tooltip
  │                                     place via grid-column / grid-row / span, emit HTML
  │
  ▼ render::render_full_html(...)       wrap with <!DOCTYPE> + <style> (CSS via include_str!)
```

### Grid rules (`layout.rs`)

- Each key's `(line, char_col)` in source is normalized to `(row, col)`.
- Within a row: `colspan = next key's col − this key's col`; last key in row has colspan 1.
- So multi-space alignment in `defsrc` directly produces wide keys.
- `deflayer` / `deflayermap` reuse the src's `GridLayout`, filling content by index (full) or by key name (sparse).

### Key display (`display.rs`)

The `KeyDisplay` trait is the extension point:

```rust
pub trait KeyDisplay {
    fn display(&self, token: &str, ctx: &DisplayContext) -> DisplayResult;
}

pub struct DisplayResult {
    pub label: String,
    pub tooltip: Option<String>,
    pub classes: Vec<&'static str>,   // joined into class="key ..." at render time
}
```

`DefaultDisplay` strategy:

| Key form                        | label          | tooltip                                            | class     |
| ------------------------------- | -------------- | -------------------------------------------------- | --------- |
| bare atom: `a`, `ret`, `-`, ... | as-is          | —                                                  | (none)    |
| `@alias`                        | `@alias` as-is | alias definition text (nested `@...` not expanded) | `alias`   |
| `(unicode >)`                   | `>`            | —                                                  | `unicode` |
| `(unicode r#"""#)`              | `"`            | —                                                  | `unicode` |
| other `(sexpr ...)`             | as-is          | —                                                  | `sexpr`   |

To swap display strategy: implement `KeyDisplay` and pass it to `render::render_fragment(model, &your_display)`. `render_file()` currently hardcodes `DefaultDisplay`; change it to accept a trait object if needed.

### `deflayermap` sparse layers

Keys not listed in the map → fall back to the same-position `defsrc` key with the `passthrough` class (dashed gray in CSS). Listed keys render normally.

### `include` preprocessing (`preprocess.rs`)

- C-style text splicing: recursively replaces `(include path)` with that file's content, paths relative to the current file's directory.
- Cycle detection via `canonicalize`.
- `;;` comments are preserved during splicing so commented-out `(include ...)` is not mistaken for real one.

### Platform branches (`parser.rs`)

`(platform (win) ...)` only takes effect under `--platform win`. `walk_top` recurses into matching branches and skips others. `defalias` / `defsrc` / `deflayer` / `deflayermap` are collected at any nesting level.

### CSS

`crates/core/assets/style.css` is the single source. `render.rs` inlines it at compile time via `include_str!("../assets/style.css")`. Edit this one file and `cargo build` to update styles.

### Common maintenance tasks

| Task                                               | Where                                                                                                          |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Change key label/tooltip strategy                  | `crates/core/src/display.rs` (implement a new `KeyDisplay`)                                                    |
| Change keyboard visuals                            | `crates/core/assets/style.css`                                                                                 |
| Support a new top-level form (e.g. `deflocalkeys`) | `crates/core/src/parser.rs`, the `match head.as_str()` arm                                                     |
| Change grid/colspan rules                          | `crates/core/src/layout.rs`                                                                                    |
| Change include search-path strategy                | `crates/core/src/preprocess.rs`                                                                                |
| Add a CLI argument                                 | `crates/cli/src/main.rs`                                                                                       |
| Embed core as a library in another tool            | `crates/core` is a pure lib; `render_file()` / `parse()` / `render_full_html()` are all independently callable |
