// A tiny CSS subset for styling widgets.
//
// Supported selectors: `button`, `label`, `hbox`, `vbox`, `window`, `*`.
// Supported properties:
//   background: #rrggbb;
//   color: #rrggbb;         (text color)
//   border-color: #rrggbb;
//   border-size: <n>;
//   padding: <n>;
//   margin: <n>;
//   align: left | center | right;   (text alignment)
//
// Example:
//   button {
//       background: #2266aa;
//       color: #ffffff;
//   }

use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone)]
pub struct Style {
    pub background: u32,
    pub color: u32,
    pub border_color: u32,
    pub border_size: u32,
    pub padding: u32,
    pub margin: u32,
    pub align: Align,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            background: 0x000000,
            color: 0xffffff,
            border_color: 0x888888,
            border_size: 1,
            padding: 4,
            margin: 2,
            align: Align::Center,
        }
    }
}

pub type StyleSheet = HashMap<String, Style>;

fn parse_color(s: &str) -> Option<u32> {
    let s = s.trim();
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

fn parse_u32(s: &str) -> Option<u32> {
    s.trim().parse().ok()
}

fn parse_align(s: &str) -> Option<Align> {
    match s.trim() {
        "left" => Some(Align::Left),
        "center" => Some(Align::Center),
        "right" => Some(Align::Right),
        _ => None,
    }
}

fn set_style(style: &mut Style, prop: &str, value: &str) {
    let value = value.trim();
    match prop.trim() {
        "background" => {
            if let Some(c) = parse_color(value) {
                style.background = c;
            }
        }
        "color" => {
            if let Some(c) = parse_color(value) {
                style.color = c;
            }
        }
        "border-color" => {
            if let Some(c) = parse_color(value) {
                style.border_color = c;
            }
        }
        "border-size" => {
            if let Some(n) = parse_u32(value) {
                style.border_size = n;
            }
        }
        "padding" => {
            if let Some(n) = parse_u32(value) {
                style.padding = n;
            }
        }
        "margin" => {
            if let Some(n) = parse_u32(value) {
                style.margin = n;
            }
        }
        "align" => {
            if let Some(a) = parse_align(value) {
                style.align = a;
            }
        }
        _ => {}
    }
}

fn canonical(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Parse CSS text into a stylesheet keyed by selector.
pub fn parse(css: &str) -> StyleSheet {
    let mut sheet = StyleSheet::new();
    let mut selector: Option<String> = None;

    for line in css.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with("/*") {
            continue;
        }
        if line.contains('{') {
            let sel = line.split('{').next().unwrap_or("").trim();
            selector = Some(canonical(sel));
            // If the block closes on the same line, parse its declarations too.
            let after = line.splitn(2, '{').nth(1).unwrap_or("");
            parse_declarations(&mut sheet, &selector.as_ref().unwrap(), after);
            if after.contains('}') {
                selector = None;
            }
            continue;
        }
        if line.contains('}') {
            parse_declarations(&mut sheet, selector.as_deref().unwrap_or(""), line);
            selector = None;
            continue;
        }
        if let Some(sel) = &selector {
            parse_declarations(&mut sheet, sel, line);
        }
    }

    sheet
}

fn parse_declarations(sheet: &mut StyleSheet, selector: &str, chunk: &str) {
    // Split off the closing brace, then handle `decl;` groups.
    let body = match chunk.rfind('}') {
        Some(i) => &chunk[..i],
        None => chunk,
    };
    for decl in body.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        if let Some(colon) = decl.find(':') {
            let prop = decl[..colon].trim();
            let value = decl[colon + 1..].trim();
            let entry = sheet.entry(selector.to_string()).or_default();
            set_style(entry, prop, value);
        }
    }
}

/// Resolve the effective style for a widget by selector, falling back to defaults.
pub fn resolve(sheet: &StyleSheet, selectors: &[&str]) -> Style {
    let mut style = Style::default();
    // Base: universal selector.
    if let Some(s) = sheet.get("*") {
        apply(s, &mut style);
    }
    for sel in selectors {
        if let Some(s) = sheet.get(&canonical(sel)) {
            apply(s, &mut style);
        }
    }
    style
}

fn apply(src: &Style, dst: &mut Style) {
    // Since Style defaults are concrete values, we can't distinguish "unset"
    // without per-field Option. For the subset, we just copy defined fields.
    if configured(src.background) {
        dst.background = src.background;
    }
    if configured(src.color) {
        dst.color = src.color;
    }
    if configured(src.border_color) {
        dst.border_color = src.border_color;
    }
    if src.border_size != 0 {
        dst.border_size = src.border_size;
    }
    if src.padding != 0 {
        dst.padding = src.padding;
    }
    if src.margin != 0 {
        dst.margin = src.margin;
    }
}

fn configured(v: u32) -> bool {
    v != 0
}
