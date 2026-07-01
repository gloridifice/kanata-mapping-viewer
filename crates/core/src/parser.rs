use crate::layout::GridLayout;
use crate::sexpr::{ParseError, Sexp, parse as parse_sexpr};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Model {
    pub src: DefSrc,
    pub aliases: HashMap<String, String>,
    pub layers: Vec<Layer>,
}

#[derive(Debug)]
pub struct DefSrc {
    pub keys: Vec<String>,
    pub layout: GridLayout,
    pub title: Option<String>,
    pub desc: Option<String>,
}

#[derive(Debug)]
pub enum Layer {
    Full {
        name: String,
        keys: Vec<String>,
        title: Option<String>,
        desc: Option<String>,
    },
    Sparse {
        name: String,
        map: HashMap<String, String>,
        title: Option<String>,
        desc: Option<String>,
    },
}

pub fn parse(source: &str, platform: &str) -> Result<Model, ParseError> {
    let sexps = parse_sexpr(source)?;
    let mut aliases: HashMap<String, String> = HashMap::new();
    let mut src_keys: Option<Vec<String>> = None;
    let mut src_spans: Option<Vec<crate::sexpr::Span>> = None;
    let mut src_title: Option<String> = None;
    let mut src_desc: Option<String> = None;
    let mut layers: Vec<Layer> = Vec::new();

    walk_top(&sexps, platform, &mut |sexp| {
        if let Sexp::List { items, span } = sexp
            && let Some(Sexp::Atom { text: head, .. }) = items.first()
        {
            let (title, desc) = extract_meta(source, span.start);
            match head.as_str() {
                "defalias" => collect_aliases(items, source, &mut aliases),
                "defsrc" => {
                    let (keys, spans) = collect_src(items, source);
                    src_keys = Some(keys);
                    src_spans = Some(spans);
                    src_title = title;
                    src_desc = desc;
                }
                "deflayer" => {
                    if let Some(layer) = collect_deflayer(items, source, title, desc) {
                        layers.push(layer);
                    }
                }
                "deflayermap" => {
                    if let Some(layer) = collect_deflayermap(items, source, title, desc) {
                        layers.push(layer);
                    }
                }
                _ => {}
            }
        }
    });

    let keys = src_keys.unwrap_or_default();
    let spans = src_spans.unwrap_or_default();
    let layout = crate::layout::compute_layout(source, &spans);

    Ok(Model {
        src: DefSrc {
            keys,
            layout,
            title: src_title,
            desc: src_desc,
        },
        aliases,
        layers,
    })
}

/// Walk top-level forms. Descends into `(platform (X) ...)` only when X == platform.
fn walk_top(items: &[Sexp], platform: &str, f: &mut impl FnMut(&Sexp)) {
    for item in items {
        if let Sexp::List { items: sub, .. } = item
            && let Some(Sexp::Atom { text: head, .. }) = sub.first()
            && head == "platform"
        {
            if let Some(Sexp::List { items: plat, .. }) = sub.get(1)
                && let Some(Sexp::Atom { text: name, .. }) = plat.first()
                && name == platform
            {
                walk_top(&sub[2..], platform, f);
            }
            continue;
        }
        f(item);
    }
}

fn collect_aliases(items: &[Sexp], source: &str, aliases: &mut HashMap<String, String>) {
    let mut i = 1; // skip "defalias"
    while i + 1 < items.len() {
        let name = items[i].as_atom().map(|s| s.to_string());
        let def = items[i + 1].as_text(source);
        if let Some(name) = name {
            aliases.insert(name, def);
        }
        i += 2;
    }
}

fn collect_src(items: &[Sexp], source: &str) -> (Vec<String>, Vec<crate::sexpr::Span>) {
    let mut keys = Vec::new();
    let mut spans = Vec::new();
    for item in items.iter().skip(1) {
        keys.push(item.as_text(source));
        spans.push(item.span());
    }
    (keys, spans)
}

fn collect_deflayer(
    items: &[Sexp],
    source: &str,
    title: Option<String>,
    desc: Option<String>,
) -> Option<Layer> {
    // (deflayer NAME key key ...)
    let name = items.get(1)?.as_atom()?.to_string();
    let keys: Vec<String> = items.iter().skip(2).map(|s| s.as_text(source)).collect();
    Some(Layer::Full { name, keys, title, desc })
}

fn collect_deflayermap(
    items: &[Sexp],
    source: &str,
    title: Option<String>,
    desc: Option<String>,
) -> Option<Layer> {
    // (deflayermap (NAME) src-key action src-key action ...)
    let name_list = items.get(1)?;
    let name = match name_list {
        Sexp::List { items: ni, .. } => ni.first()?.as_atom()?.to_string(),
        Sexp::Atom { text, .. } => text.clone(),
    };
    let mut map = HashMap::new();
    let mut i = 2;
    while i + 1 < items.len() {
        let key = items[i].as_atom().map(|s| s.to_string());
        let action = items[i + 1].as_text(source);
        if let Some(key) = key {
            map.insert(key, action);
        }
        i += 2;
    }
    Some(Layer::Sparse { name, map, title, desc })
}

/// Extract `;; name: ...` / `;; desc: ...` metadata from `;;` comment lines
/// immediately preceding the form that starts at `form_start`.
fn extract_meta(source: &str, form_start: usize) -> (Option<String>, Option<String>) {
    let prefix = &source[..form_start];
    // Lines strictly above the line containing `form_start`.
    let above = match prefix.rfind('\n') {
        Some(i) => &prefix[..i],
        None => return (None, None),
    };
    let mut title = None;
    let mut desc = None;
    for line in above.lines().rev() {
        let trimmed = line.trim_start();
        let rest = match trimmed.strip_prefix(";;") {
            Some(r) => r,
            None => break,
        };
        let rest = rest.trim_start();
        if let Some(val) = rest.strip_prefix("name:") {
            if title.is_none() {
                title = Some(val.trim().to_string());
            }
        } else if let Some(val) = rest.strip_prefix("desc:")
            && desc.is_none()
        {
            desc = Some(val.trim().to_string());
        }
    }
    (title, desc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_name_and_desc() {
        let src = "(defalias a b)\n\n;; name: Base\n;; desc: hello world\n(deflayer base\n a b)\n";
        let start = src.find("(deflayer base").unwrap();
        let (title, desc) = extract_meta(src, start);
        assert_eq!(title.as_deref(), Some("Base"));
        assert_eq!(desc.as_deref(), Some("hello world"));
    }

    #[test]
    fn meta_only_name() {
        let src = ";; name: Game\n(deflayer game x y)\n";
        let start = src.find("(deflayer game").unwrap();
        let (title, desc) = extract_meta(src, start);
        assert_eq!(title.as_deref(), Some("Game"));
        assert_eq!(desc, None);
    }

    #[test]
    fn meta_none_when_no_comment() {
        let src = "(defalias a b)\n(deflayer base a b)\n";
        let start = src.find("(deflayer base").unwrap();
        let (title, desc) = extract_meta(src, start);
        assert_eq!(title, None);
        assert_eq!(desc, None);
    }

    #[test]
    fn meta_stops_at_blank_line() {
        let src = ";; name: Ignored\n\n;; name: Real\n(deflayer base a b)\n";
        let start = src.find("(deflayer base").unwrap();
        let (title, _) = extract_meta(src, start);
        assert_eq!(title.as_deref(), Some("Real"));
    }

    #[test]
    fn meta_ignores_other_comments() {
        let src = ";; some note\n;; name: Capital\n;; another note\n;; desc: d\n(deflayer capital a)\n";
        let start = src.find("(deflayer capital").unwrap();
        let (title, desc) = extract_meta(src, start);
        assert_eq!(title.as_deref(), Some("Capital"));
        assert_eq!(desc.as_deref(), Some("d"));
    }

    #[test]
    fn meta_indented_comments() {
        let src = "  ;; name: Indented\n  (deflayer base a b)\n";
        let start = src.find("(deflayer base").unwrap();
        let (title, _) = extract_meta(src, start);
        assert_eq!(title.as_deref(), Some("Indented"));
    }

    #[test]
    fn parse_captures_layer_meta() {
        let src = "(defsrc a b c)\n;; name: My Layer\n;; desc: a description\n(deflayer base a b c)\n";
        let model = parse(src, "win").unwrap();
        assert_eq!(model.layers.len(), 1);
        match &model.layers[0] {
            Layer::Full { name, title, desc, .. } => {
                assert_eq!(name, "base");
                assert_eq!(title.as_deref(), Some("My Layer"));
                assert_eq!(desc.as_deref(), Some("a description"));
            }
            other => panic!("expected Full, got {other:?}"),
        }
    }

    #[test]
    fn parse_captures_defsrc_meta() {
        let src = ";; name: Source\n;; desc: the source\n(defsrc a b c)\n";
        let model = parse(src, "win").unwrap();
        assert_eq!(model.src.title.as_deref(), Some("Source"));
        assert_eq!(model.src.desc.as_deref(), Some("the source"));
    }
}
