use crate::sexpr::{parse as parse_sexpr, Sexp};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DisplayResult {
    pub label: String,
    pub tooltip: Option<String>,
    pub classes: Vec<&'static str>,
}

pub struct DisplayContext<'a> {
    pub aliases: &'a HashMap<String, String>,
}

pub trait KeyDisplay {
    fn display(&self, token: &str, ctx: &DisplayContext) -> DisplayResult;
}

pub struct DefaultDisplay;

impl KeyDisplay for DefaultDisplay {
    fn display(&self, token: &str, ctx: &DisplayContext) -> DisplayResult {
        if let Some(name) = token.strip_prefix('@') {
            let tooltip = ctx.aliases.get(name).cloned();
            return DisplayResult {
                label: token.to_string(),
                tooltip,
                classes: vec!["alias"],
            };
        }
        if token.starts_with('(') {
            if let Some(res) = display_sexpr(token) {
                return res;
            }
            return DisplayResult {
                label: token.to_string(),
                tooltip: None,
                classes: vec!["sexpr"],
            };
        }
        DisplayResult {
            label: token.to_string(),
            tooltip: None,
            classes: vec![],
        }
    }
}

fn display_sexpr(token: &str) -> Option<DisplayResult> {
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
        return Some(DisplayResult {
            label,
            tooltip: None,
            classes: vec!["unicode"],
        });
    }
    Some(DisplayResult {
        label: token.to_string(),
        tooltip: None,
        classes: vec!["sexpr"],
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
