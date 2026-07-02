use std::sync::Arc;

use crate::{
    DefSrc, Layer, Model,
    parser::{LayerVariant, MetaInfo},
    render::keyboard::{DisplayContext, KeyCell, KeyDataProcessor, KeyboardRenderer, esc},
};

/// Pre-built renderer for one keyboard section (defsrc or a layer).
///
/// Construct via [`from_defsrc`](Self::from_defsrc) or
/// [`from_layer`](Self::from_layer), then call [`build_html`](Self::build_html)
/// to emit the HTML.
pub struct LayerRenderer {
    heading: String,
    subtitle: Option<String>,
    desc: Option<String>,
    deprecate: bool,
    keyboard: KeyboardRenderer,
}

impl LayerRenderer {
    /// Build a renderer for the `defsrc` section.
    pub fn from_defsrc(src: &DefSrc, model: &Model, display: &dyn KeyDataProcessor) -> Self {
        let ctx = DisplayContext {
            aliases: &model.aliases,
        };
        let cells: Vec<KeyCell> = src
            .keys
            .iter()
            .map(|k| resolve_cell(k, false, display, &ctx))
            .collect();
        let heading = src.title.clone().unwrap_or_else(|| "defsrc".to_string());

        Self {
            heading,
            subtitle: None, // defsrc has no inherent name
            desc: src.desc.clone(),
            deprecate: false,
            keyboard: KeyboardRenderer {
                keys: cells,
                layout: Arc::new(src.layout.clone()),
            },
        }
    }

    /// Build a renderer for a layer (`deflayer` or `deflayermap`).
    pub fn from_layer(layer: &Layer, model: &Model, display: &dyn KeyDataProcessor) -> Self {
        let ctx = DisplayContext {
            aliases: &model.aliases,
        };
        let src = &model.src;

        let Layer {
            name,
            meta_info:
                MetaInfo {
                    title,
                    desc,
                    deprecate,
                },
            variant,
        } = layer;

        let cells: Vec<KeyCell> = match variant {
            LayerVariant::Full { keys } => {
                // Pad/truncate to src length.
                (0..src.keys.len())
                    .map(|i| {
                        let token = keys.get(i).map(|s| s.as_str()).unwrap_or("");
                        resolve_cell(token, false, display, &ctx)
                    })
                    .collect()
            }
            LayerVariant::Sparse { map } => src
                .keys
                .iter()
                .map(|k| {
                    if let Some(action) = map.get(k) {
                        resolve_cell(action, false, display, &ctx)
                    } else {
                        resolve_cell(k, true, display, &ctx)
                    }
                })
                .collect(),
        };

        let heading = title.clone().unwrap_or_else(|| name.clone());

        // When a comment title overrides the inherent layer name, show the
        // layer name as a subtitle so the kanata identifier is not lost.
        let subtitle = match title {
            Some(t) if t != name => Some(name.clone()),
            _ => None,
        };

        Self {
            heading,
            subtitle,
            desc: desc.clone(),
            deprecate: *deprecate,
            keyboard: KeyboardRenderer {
                keys: cells,
                layout: Arc::new(src.layout.clone()),
            },
        }
    }

    /// Emit the HTML for this section.
    ///
    /// Returns an empty string when `deprecate` is set.
    pub fn build_html(self) -> String {
        if self.deprecate {
            return String::new();
        }

        let mut html = String::new();
        html.push_str("<section class=\"keyboard\">");
        html.push_str(&format!("<h3>{}</h3>", esc(&self.heading)));

        if let Some(sub) = &self.subtitle {
            html.push_str(&format!("<p class=\"keyboard-subtitle\">{}</p>", esc(sub)));
        }
        if let Some(d) = &self.desc {
            html.push_str(&format!("<p class=\"keyboard-desc\">{}</p>", esc(d)));
        }

        html.push_str(&self.keyboard.build_html());
        html.push_str("</section>");
        html
    }
}

/// Resolve a raw action token through the [`KeyDataProcessor`] trait into a
/// fully-populated [`KeyCell`].
fn resolve_cell(
    token: &str,
    passthrough: bool,
    display: &dyn KeyDataProcessor,
    ctx: &DisplayContext,
) -> KeyCell {
    let mut cell = display.process(token, ctx);
    cell.passthrough = passthrough;
    cell
}
