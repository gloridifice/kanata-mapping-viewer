use crate::display::{DisplayContext, DisplayResult, KeyDisplay};
use crate::parser::{DefSrc, Layer, Model};

pub const CSS: &str = include_str!("../assets/style.css");

pub fn render_fragment(model: &Model, display: &dyn KeyDisplay) -> String {
    let mut html = String::new();
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
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>kanata-viewer</title>\n<style>\n{css}</style>\n</head>\n<body>\n{body}\n</body>\n</html>\n",
        css = CSS,
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
    html.push_str(&format!(
        "<div class=\"grid\" style=\"grid-template-columns: repeat({}, 1fr); grid-template-rows: repeat({}, auto);\">",
        layout.n_cols.max(1),
        layout.n_rows.max(1)
    ));

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
        "<div class=\"key {classes}\" style=\"grid-column: {col} / span {span}; grid-row: {row};\">",
        classes = classes.trim(),
        col = gc.col + 1,
        span = gc.colspan.max(1),
        row = gc.row + 1
    ));
    html.push_str(&format!(
        "<span class=\"label\">{}</span>",
        esc(&res.label)
    ));
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
