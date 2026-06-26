pub mod display;
pub mod layout;
pub mod parser;
pub mod preprocess;
pub mod render;
pub mod sexpr;
pub mod svgs;

pub use display::{DefaultDisplay, DisplayContext, DisplayResult, KeyDisplay};
pub use layout::{GridCell, GridLayout};
pub use parser::{DefSrc, Layer, Model, parse};
pub use preprocess::{PreprocessError, preprocess};
pub use render::{CSS, render_fragment, render_full_html};

use std::path::Path;

/// Full pipeline: read file (with includes), parse, render HTML document.
pub fn render_file(path: &Path, platform: &str) -> Result<String, Error> {
    let source = preprocess(path).map_err(Error::Preprocess)?;
    let model = parse(&source, platform).map_err(Error::Parse)?;
    Ok(render_full_html(&model, &DefaultDisplay))
}

#[derive(Debug)]
pub enum Error {
    Preprocess(PreprocessError),
    Parse(sexpr::ParseError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Preprocess(e) => write!(f, "{}", e),
            Error::Parse(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}
