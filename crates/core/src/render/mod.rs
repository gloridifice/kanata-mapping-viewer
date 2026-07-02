use crate::render::keyboard::KeyDataProcessor;
use crate::parser::Model;
use crate::render::layer::LayerRenderer;

pub mod keyboard;
mod layer;

pub const CSS: &str = include_str!("../../assets/style.css");

/// Extract inner content from an SVG string (between the opening <svg...> and closing </svg>).
fn svg_inner(svg: &str) -> &str {
    let start = svg.find('>').map(|i| i + 1).unwrap_or(0);
    let end = svg.rfind("</svg>").unwrap_or(svg.len());
    &svg[start..end]
}

pub fn render_fragment(model: &Model, display: &dyn KeyDataProcessor) -> String {
    let mut html = String::new();
    html.push_str("<style>.key-bg{position:absolute;inset:0;width:100%;height:100%;pointer-events:none;z-index:0;}.key>*:not(.key-bg){position:relative;z-index:1;}.key.passthrough .key-bg{opacity:0.35;}.key.alias .key-bg{filter:hue-rotate(140deg);}.key.unicode .key-bg{filter:hue-rotate(280deg);}.key.sexpr .key-bg{filter:hue-rotate(220deg);}</style>");
    html.push_str("<div class=\"viewer\">");

    // defsrc keyboard
    html.push_str(&LayerRenderer::from_defsrc(&model.src, model, display).build_html());

    for layer in &model.layers {
        html.push_str(&LayerRenderer::from_layer(layer, model, display).build_html());
    }

    html.push_str("</div>");
    html
}

pub fn render_full_html(
    model: &Model,
    display: &dyn KeyDataProcessor,
    is_dev_mode: bool,
) -> String {
    let fragment = render_fragment(model, display);
    let style = if is_dev_mode {
        r#"<link rel="stylesheet" href="./crates/core/assets/style.css">"#.to_string()
    } else {
        format!(r#"<style>{}</style>"#, CSS)
    };
    format!(
        r#"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        {style}
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
        style = &style,
        body = fragment
    )
}
