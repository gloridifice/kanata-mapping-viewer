use crate::display::{DisplayContext, DisplayResult, KeyDisplay};
use crate::parser::{DefSrc, Layer, Model};

pub const CSS: &str = include_str!("../assets/style.css");

/// Extract inner content from an SVG string (between the opening <svg...> and closing </svg>).
fn svg_inner(svg: &str) -> &str {
    let start = svg.find('>').map(|i| i + 1).unwrap_or(0);
    let end = svg.rfind("</svg>").unwrap_or(svg.len());
    &svg[start..end]
}

pub fn render_fragment(model: &Model, display: &dyn KeyDisplay) -> String {
    let mut html = String::new();
    html.push_str("<style>.key-bg{position:absolute;inset:0;width:100%;height:100%;pointer-events:none;z-index:0;}.key>*:not(.key-bg){position:relative;z-index:1;}.key.passthrough .key-bg{opacity:0.35;}.key.alias .key-bg{filter:hue-rotate(140deg);}.key.unicode .key-bg{filter:hue-rotate(280deg);}.key.sexpr .key-bg{filter:hue-rotate(220deg);}</style>");
    html.push_str("<div class=\"viewer\">");

    // defsrc keyboard
    render_keyboard("defsrc", &model.src, model, display, &mut html);

    for layer in &model.layers {
        render_layer(layer, &model.src, model, display, &mut html);
    }

    html.push_str("</div>");
    html
}

pub fn render_full_html(model: &Model, display: &dyn KeyDisplay) -> String {
    let fragment = render_fragment(model, display);
    format!(
        r#"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <link rel="stylesheet" href="./crates/core/assets/style.css">
        <title>Kanata Mapping Viewer</title>
        <link rel="preconnect" href="https://fonts.googleapis.com">
        <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
        <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:ital,wght@0,100..800;1,100..800&display=swap" rel="stylesheet">
    </head>
    <body>
        {body}
    </body>
</html>
"#,
        body = fragment
    )
}

fn render_layer(
    layer: &Layer,
    src: &DefSrc,
    model: &Model,
    display: &dyn KeyDisplay,
    html: &mut String,
) {
    match layer {
        Layer::Full { name, keys } => {
            // pad/truncate to src length
            let cells: Vec<CellContent> = (0..src.keys.len())
                .map(|i| CellContent {
                    text: keys.get(i).cloned().unwrap_or_default(),
                    passthrough: false,
                })
                .collect();
            render_keyboard_cells(name, &cells, &src.layout, model, display, html);
        }
        Layer::Sparse { name, map } => {
            let cells: Vec<CellContent> = src
                .keys
                .iter()
                .map(|k| {
                    if let Some(action) = map.get(k) {
                        CellContent {
                            text: action.clone(),
                            passthrough: false,
                        }
                    } else {
                        CellContent {
                            text: k.clone(),
                            passthrough: true,
                        }
                    }
                })
                .collect();
            render_keyboard_cells(name, &cells, &src.layout, model, display, html);
        }
    }
}

fn render_keyboard(
    name: &str,
    src: &DefSrc,
    model: &Model,
    display: &dyn KeyDisplay,
    html: &mut String,
) {
    let cells: Vec<CellContent> = src
        .keys
        .iter()
        .map(|k| CellContent {
            text: k.clone(),
            passthrough: false,
        })
        .collect();
    render_keyboard_cells(name, &cells, &src.layout, model, display, html);
}

struct CellContent {
    text: String,
    passthrough: bool,
}

fn render_keyboard_cells(
    name: &str,
    cells: &[CellContent],
    layout: &crate::layout::GridLayout,
    model: &Model,
    display: &dyn KeyDisplay,
    html: &mut String,
) {
    let ctx = DisplayContext {
        aliases: &model.aliases,
    };
    html.push_str("<section class=\"keyboard\">");
    html.push_str(&format!("<h3>{}</h3>", esc(name)));
    html.push_str("<div class=\"grid\">");

    for (i, cell) in cells.iter().enumerate() {
        let Some(gc) = layout.cells.get(i) else {
            continue;
        };
        let res = display.display(&cell.text, &ctx);
        render_key(gc, &res, cell.passthrough, html);
    }

    html.push_str("</div></section>");
}

fn render_key(
    gc: &crate::layout::GridCell,
    res: &DisplayResult,
    passthrough: bool,
    html: &mut String,
) {
    let mut classes = String::new();
    for c in &res.classes {
        classes.push_str(c);
        classes.push(' ');
    }
    if passthrough {
        classes.push_str("passthrough");
    }
    html.push_str(&format!(
        "<div class=\"key {classes}\" style=\"grid-row: {row};\">",
        classes = classes.trim(),
        row = gc.row + 1
    ));
    // html.push_str(&format!(
    //     "<div class=\"key {classes}\" style=\"grid-column: {col}; grid-row: {row};\">",
    //     classes = classes.trim(),
    //     col = gc.col + 1,
    //     row = gc.row + 1
    // ));
    // Inline the keycap SVG as the key background
    html.push_str(&format!(
        "<svg class=\"key-bg\" xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\" preserveAspectRatio=\"none\">{}",
        svg_inner(crate::svgs::KEY)
    ));
    html.push_str("</svg>");
    html.push_str(&format!("<span class=\"label\">{}</span>", esc(&res.label)));
    if let Some(tip) = &res.tooltip {
        html.push_str(&format!("<span class=\"tooltip\">{}</span>", esc(tip)));
    }
    html.push_str("</div>");
}

fn esc(s: &str) -> String {
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
