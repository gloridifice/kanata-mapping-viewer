use std::collections::HashMap;
use std::sync::Arc;

use crate::sexpr::{Sexp, parse as parse_sexpr};
use crate::{GridLayout, GridPos, render::svg_inner};

// ---------------------------------------------------------------------------
// KeyCell & KeyboardRenderer
// ---------------------------------------------------------------------------

pub struct KeyCell {
    pub text: String,
    pub tooltip: Option<String>,
    pub extra_classes: Vec<String>,
    pub passthrough: bool,
}

impl KeyCell {
    pub fn build_html(self, pos: &GridPos) -> String {
        let mut ret = String::new();
        let Self {
            text,
            tooltip,
            extra_classes,
            passthrough,
        } = self;

        let mut classes = String::new();
        for c in &extra_classes {
            classes.push_str(c);
            classes.push(' ');
        }
        if passthrough {
            classes.push_str("passthrough");
        }
        ret.push_str(&format!(
            "<div class=\"key {classes}\" style=\"grid-row: {row};\">",
            classes = classes.trim(),
            row = pos.row + 1
        ));
        ret.push_str(&format!(
        "<svg class=\"key-bg\" xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\" preserveAspectRatio=\"none\">{}",
        svg_inner(crate::svgs::KEY)
    ));
        ret.push_str("</svg>");
        ret.push_str(&format!("<span class=\"label\">{}</span>", esc(&text)));
        if let Some(tip) = &tooltip {
            ret.push_str(&format!("<span class=\"tooltip\">{}</span>", esc(tip)));
        }
        ret.push_str("</div>");

        ret
    }
}

pub struct KeyboardRenderer {
    pub keys: Vec<KeyCell>,
    pub layout: Arc<GridLayout>,
}

impl KeyboardRenderer {
    pub fn build_html(self) -> String {
        let mut ret = String::new();
        let Self { keys, layout } = self;
        ret.push_str("<div class=\"grid\">");

        for (i, cell) in keys.into_iter().enumerate() {
            let Some(grid_pos) = layout.cells.get(i) else {
                continue;
            };
            ret.push_str(&cell.build_html(grid_pos));
        }

        ret.push_str("</div>");
        ret
    }
}

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// KeyDataProcessor trait & default implementation
// ---------------------------------------------------------------------------

pub struct DisplayContext<'a> {
    pub aliases: &'a HashMap<String, String>,
}

pub trait KeyDataProcessor {
    fn process(&self, token: &str, ctx: &DisplayContext) -> KeyCell;
}

pub struct DefaultKeyDataProcessor;

impl KeyDataProcessor for DefaultKeyDataProcessor {
    fn process(&self, token: &str, ctx: &DisplayContext) -> KeyCell {
        if let Some(name) = token.strip_prefix('@') {
            let tooltip = ctx.aliases.get(name).cloned();
            return KeyCell {
                text: token.to_string(),
                tooltip,
                extra_classes: vec!["alias".to_string()],
                passthrough: false,
            };
        }
        if token.starts_with('(') {
            if let Some(res) = display_sexpr(token) {
                return res;
            }
            return KeyCell {
                text: token.to_string(),
                tooltip: None,
                extra_classes: vec!["sexpr".to_string()],
                passthrough: false,
            };
        }
        KeyCell {
            text: token.to_string(),
            tooltip: None,
            extra_classes: vec![],
            passthrough: false,
        }
    }
}

fn display_sexpr(token: &str) -> Option<KeyCell> {
    let top = parse_sexpr(token).ok()?;
    let Sexp::List { items, .. } = top.first()? else {
        return None;
    };
    let Sexp::Atom { text: head, .. } = items.first()? else {
        return None;
    };
    if head == "unicode" {
        let arg = items.get(1)?;
        let label = extract_raw_string_content(arg.as_text(token).as_str());
        return Some(KeyCell {
            text: label,
            tooltip: None,
            extra_classes: vec!["unicode".to_string()],
            passthrough: false,
        });
    }
    Some(KeyCell {
        text: token.to_string(),
        tooltip: None,
        extra_classes: vec!["sexpr".to_string()],
        passthrough: false,
    })
}

/// Extract display content from a raw string atom or bare atom.
/// `r#"""#` -> `"`, `r"x"` -> `x`, `"x"` -> `x`, `>` -> `>`.
fn extract_raw_string_content(text: &str) -> String {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix('r') {
        let hash_count = rest.bytes().take_while(|b| *b == b'#').count();
        let after_hashes = &rest[hash_count..];
        if let Some(inner_start) = after_hashes.strip_prefix('"') {
            let suffix = format!("\"{}", "#".repeat(hash_count));
            if let Some(inner) = inner_start.strip_suffix(&suffix) {
                return inner.to_string();
            }
        }
    }
    if let Some(inner) = t.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return inner.to_string();
    }
    t.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_plain() {
        assert_eq!(extract_raw_string_content(">"), ">");
    }

    #[test]
    fn unicode_raw_string() {
        assert_eq!(extract_raw_string_content("r#\"\"\"#"), "\"");
    }
}
