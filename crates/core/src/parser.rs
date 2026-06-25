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
}

#[derive(Debug)]
pub enum Layer {
    Full {
        name: String,
        keys: Vec<String>,
    },
    Sparse {
        name: String,
        map: HashMap<String, String>,
    },
}

pub fn parse(source: &str, platform: &str) -> Result<Model, ParseError> {
    let sexps = parse_sexpr(source)?;
    let mut aliases: HashMap<String, String> = HashMap::new();
    let mut src_keys: Option<Vec<String>> = None;
    let mut src_spans: Option<Vec<crate::sexpr::Span>> = None;
    let mut layers: Vec<Layer> = Vec::new();

    walk_top(&sexps, platform, &mut |sexp| {
        if let Sexp::List { items, .. } = sexp
            && let Some(Sexp::Atom { text: head, .. }) = items.first()
        {
            match head.as_str() {
                "defalias" => collect_aliases(items, source, &mut aliases),
                "defsrc" => {
                    let (keys, spans) = collect_src(items, source);
                    src_keys = Some(keys);
                    src_spans = Some(spans);
                }
                "deflayer" => {
                    if let Some(layer) = collect_deflayer(items, source) {
                        layers.push(layer);
                    }
                }
                "deflayermap" => {
                    if let Some(layer) = collect_deflayermap(items, source) {
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
        src: DefSrc { keys, layout },
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

fn collect_deflayer(items: &[Sexp], source: &str) -> Option<Layer> {
    // (deflayer NAME key key ...)
    let name = items.get(1)?.as_atom()?.to_string();
    let keys: Vec<String> = items.iter().skip(2).map(|s| s.as_text(source)).collect();
    Some(Layer::Full { name, keys })
}

fn collect_deflayermap(items: &[Sexp], source: &str) -> Option<Layer> {
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
    Some(Layer::Sparse { name, map })
}
