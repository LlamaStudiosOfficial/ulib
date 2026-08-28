// UI subsystem: parsing the .ulib markup, building a widget tree,
// laying out and rendering it into a pixel buffer, and hit-testing clicks.

use crate::font;
use crate::style::{self, Align, Style, StyleSheet};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Layout {
    HBox,
    VBox,
}

#[derive(Clone)]
pub enum Widget {
    Container { layout: Layout, children: Vec<Widget> },
    Label { text: String },
    Button { text: String, signal: Option<String> },
}

impl Widget {
    pub fn kind(&self) -> &'static str {
        match self {
            Widget::Container { layout, .. } => match layout {
                Layout::HBox => "hbox",
                Layout::VBox => "vbox",
            },
            Widget::Label { .. } => "label",
            Widget::Button { .. } => "button",
        }
    }

    pub fn selectors(&self) -> (&'static str, &'static str) {
        (self.kind(), "window")
    }
}

// ---------------------------------------------------------------------------
// Parsing the .ulib markup
// ---------------------------------------------------------------------------

pub struct Module {
    pub style_file: Option<String>,
    pub root: Widget,
}

struct Parser {
    tokens: Vec<String>,
    pos: usize,
}

fn tokenize(src: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '<' => {
                // Read the block tag: <HBOX> or </HBOX>
                let mut tag = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '>' {
                    tag.push(chars[i]);
                    i += 1;
                }
                i += 1; // skip '>'
                tokens.push(format!("<{}>", tag.trim().to_lowercase()));
            }
            '(' => {
                tokens.push("(".to_string());
                i += 1;
            }
            ')' => {
                tokens.push(")".to_string());
                i += 1;
            }
            ',' => {
                tokens.push(",".to_string());
                i += 1;
            }
            '"' => {
                // String literal.
                let mut s = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                    }
                    s.push(chars[i]);
                    i += 1;
                }
                i += 1; // skip closing quote
                tokens.push(s);
            }
            _ => {
                // Identifier until a delimiter.
                let mut s = String::new();
                while i < chars.len()
                    && !"()<,>".contains(chars[i])
                    && !chars[i].is_whitespace()
                {
                    s.push(chars[i]);
                    i += 1;
                }
                tokens.push(s);
            }
        }
    }
    Ok(tokens)
}

impl Parser {
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.pos).map(|s| s.as_str())
    }

    fn next(&mut self) -> Option<String> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// Parse a single widget (leaf or container).
    fn widget(&mut self) -> Result<Widget, String> {
        let Some(tok) = self.next() else {
            return Err("unexpected end of file".into());
        };

        if tok.starts_with('<') {
            let tag = tok.trim_start_matches('<').trim_end_matches('>').to_string();
            let layout = match tag.as_str() {
                "hbox" => Layout::HBox,
                "vbox" => Layout::VBox,
                other => return Err(format!("unknown container <{other}>")),
            };
            let mut children = Vec::new();
            loop {
                let Some(t) = self.peek() else {
                    return Err(format!("unclosed <{tag}>"));
                };
                if t.starts_with("</") {
                    self.next();
                    break;
                }
                children.push(self.widget()?);
            }
            Ok(Widget::Container { layout, children })
        } else if tok == "Style" {
            // Style() is handled at the top level in `parse_module`.
            Err("Style() is only allowed at the top of a .ulib file".into())
        } else if tok == "Label" {
            self.expect("(")?;
            let text = self.next().ok_or("expected label text")?;
            self.expect(")")?;
            Ok(Widget::Label { text })
        } else if tok == "Button" {
            self.expect("(")?;
            let text = self.next().ok_or("expected button text")?;
            self.expect(",")?;
            let signal = self.next().ok_or("expected button signal id")?;
            self.expect(")")?;
            Ok(Widget::Button {
                text,
                signal: Some(signal),
            })
        } else {
            Err(format!("unknown directive `{tok}`"))
        }
    }

    fn expect(&mut self, t: &str) -> Result<(), String> {
        match self.next() {
            Some(x) if x == t => Ok(()),
            other => Err(format!("expected `{t}`, got `{}`", other.unwrap_or_default())),
        }
    }
}

/// Parse module source into a widget tree plus an optional stylesheet path.
pub fn parse_module(src: &str) -> Result<Module, String> {
    let tokens = tokenize(src)?;
    let mut parser = Parser { tokens, pos: 0 };

    let mut style_file: Option<String> = None;
    let mut children: Vec<Widget> = Vec::new();

    while parser.pos < parser.tokens.len() {
        let tok = parser.peek().unwrap_or("").to_string();
        if tok == "Style" {
            parser.next();
            parser.expect("(")?;
            let path = parser.next().ok_or("expected style path")?;
            parser.expect(")")?;
            style_file = Some(path);
        } else {
            children.push(parser.widget()?);
        }
    }

    let root = Widget::Container {
        layout: Layout::VBox,
        children,
    };

    Ok(Module { style_file, root })
}

// ---------------------------------------------------------------------------
// Layout & rendering
// ---------------------------------------------------------------------------

/// A laid out widget with its pixel rectangle.
pub struct Placed {
    pub widget: Widget,
    pub rect: (u32, u32, u32, u32), // x, y, w, h
    pub style: Style,
}

/// Lay out a widget tree into a list of placed widgets with resolved styles.
pub fn layout(root: &Widget, sheet: &StyleSheet, width: u32, height: u32) -> Vec<Placed> {
    let mut placed = Vec::new();
    layout_into(root, sheet, 0, 0, width, height, &mut placed);
    placed
}

fn layout_into(
    widget: &Widget,
    sheet: &StyleSheet,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    out: &mut Vec<Placed>,
) {
    let (sel, win_sel) = widget.selectors();
    let style = style::resolve(sheet, &[sel, win_sel]);
    let pad = style.padding;
    let margin = style.margin;

    match widget {
        Widget::Container { layout, children } => {
            let inner_x = x + margin;
            let inner_y = y + margin;
            let inner_w = w.saturating_sub(2 * margin);
            let inner_h = h.saturating_sub(2 * margin);

            let content_x = inner_x + pad;
            let content_y = inner_y + pad;
            let content_w = inner_w.saturating_sub(2 * pad);
            let content_h = inner_h.saturating_sub(2 * pad);

            let n = children.len().max(1) as u32;
            let mut child_index = 0u32;
            for child in children {
                match layout {
                    Layout::VBox => {
                        let child_h = content_h / n;
                        layout_into(
                            child,
                            sheet,
                            content_x,
                            content_y + child_index * child_h,
                            content_w,
                            child_h,
                            out,
                        );
                    }
                    Layout::HBox => {
                        let child_w = content_w / n;
                        layout_into(
                            child,
                            sheet,
                            content_x + child_index * child_w,
                            content_y,
                            child_w,
                            content_h,
                            out,
                        );
                    }
                }
                child_index += 1;
            }

            out.push(Placed {
                widget: widget.clone(),
                rect: (x, y, w, h),
                style,
            });
        }
        _ => {
            out.push(Placed {
                widget: widget.clone(),
                rect: (x, y, w, h),
                style,
            });
        }
    }
}

/// Background color for a widget (buttons get a tint).
pub fn rgb_from_u32(v: u32) -> (u8, u8, u8) {
    (((v >> 16) & 0xff) as u8, ((v >> 8) & 0xff) as u8, (v & 0xff) as u8)
}

fn pack(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn set_px(buf: &mut [u32], x: i32, y: i32, color: u32, bounds: (u32, u32)) {
    if x < 0 || y < 0 {
        return;
    }
    let (bw, bh) = bounds;
    let x = x as u32;
    let y = y as u32;
    if x >= bw || y >= bh {
        return;
    }
    buf[(y * bw + x) as usize] = color;
}

fn fill_rect(buf: &mut [u32], w: u32, h: u32, x: u32, y: u32, rw: u32, rh: u32, color: u32) {
    let (bw, bh) = (w, h);
    for yy in y..y + rh {
        for xx in x..x + rw {
            set_px(buf, xx as i32, yy as i32, color, (bw, bh));
        }
    }
    let _ = bh;
}

fn draw_text(buf: &mut [u32], w: u32, h: u32, x: u32, y: u32, text: &str, color: u32) {
    let mut cx = x;
    for ch in text.chars() {
        if let Some(rows) = font::glyph(ch) {
            for (row, pattern) in rows.iter().enumerate() {
                for col in 0..font::GLYPH_W {
                    if pattern & (1 << (4 - col)) != 0 {
                        set_px(buf, (cx + col) as i32, (y + row as u32) as i32, color, (w, h));
                    }
                }
            }
        } else {
            // Unsupported glyph: draw a space of equal width.
        }
        cx += font::GLYPH_W + font::GLYPH_GAP;
    }
}

/// Draw the placed widgets into a pixel buffer.
pub fn render(placed: &[Placed], buf: &mut [u32], width: u32, height: u32) {
    // Clear.
    for p in buf.iter_mut() {
        *p = 0x222222;
    }

    for item in placed {
        let (x, y, w, h) = item.rect;
        match &item.widget {
            Widget::Container { .. } => {
                fill_rect(buf, width, height, x, y, w, h, item.style.background);
            }
            Widget::Label { text } => {
                // Transparent background; draw text.
                let color = item.style.color;
                let tw = font::text_width(text);
                let align = item.style.align;
                let tx = match align {
                    Align::Left => x + item.style.padding,
                    Align::Center => x + (w.saturating_sub(tw)) / 2,
                    Align::Right => x + w.saturating_sub(tw + item.style.padding),
                };
                let ty = y + (h.saturating_sub(font::text_height())) / 2;
                draw_text(buf, width, height, tx, ty, text, color);
            }
            Widget::Button { text, .. } => {
                let (r, g, b) = rgb_from_u32(item.style.background);
                // Slightly lighter than container background.
                let base = pack(
                    (r as u32 + 24).min(255) as u8,
                    (g as u32 + 24).min(255) as u8,
                    (b as u32 + 24).min(255) as u8,
                );
                fill_rect(buf, width, height, x, y, w, h, base);
                // Border.
                let bs = item.style.border_size;
                for i in 0..bs {
                    let c = item.style.border_color;
                    // top & bottom
                    for xx in x..x + w {
                        set_px(buf, xx as i32, (y + i) as i32, c, (width, height));
                        set_px(buf, xx as i32, (y + h - 1 - i) as i32, c, (width, height));
                    }
                    // left & right
                    for yy in y..y + h {
                        set_px(buf, (x + i) as i32, yy as i32, c, (width, height));
                        set_px(buf, (x + w - 1 - i) as i32, yy as i32, c, (width, height));
                    }
                }
                // Text.
                let color = item.style.color;
                let tw = font::text_width(text);
                let tx = x + (w.saturating_sub(tw)) / 2;
                let ty = y + (h.saturating_sub(font::text_height())) / 2;
                draw_text(buf, width, height, tx, ty, text, color);
            }
        }
    }
}

/// Given a click position, return the signal name of the deepest button hit.
pub fn hit_test(placed: &[Placed], x: u32, y: u32) -> Option<String> {
    // Iterate over placed widgets; prefer the smallest (deepest) hit button.
    let mut best: Option<(u32, &Placed)> = None;
    for item in placed {
        let (rx, ry, rw, rh) = item.rect;
        if let Widget::Button { signal, .. } = &item.widget {
            if signal.is_some() {
                if x >= rx && x < rx + rw && y >= ry && y < ry + rh {
                    let area = rw * rh;
                    match best {
                        Some((ba, _)) if area < ba => best = Some((area, item)),
                        None => best = Some((area, item)),
                        _ => {}
                    }
                }
            }
        }
    }
    best.map(|(_, item)| match &item.widget {
        Widget::Button { signal, .. } => signal.clone().unwrap(),
        _ => unreachable!(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ui_file() {
        let src = r#"
Style(style.css)
Label("Wow text!")
<HBOX>
Button("Super button!!!!!",hbutton)
</HBOX>
<VBOX>
Button("Super button wow!!!!!!",hbutton)
</VBOX>
"#;
        let module = parse_module(src).expect("should parse");
        assert_eq!(module.style_file.as_deref(), Some("style.css"));
        let Widget::Container { layout, children } = &module.root else {
            panic!("expected root container");
        };
        assert_eq!(*layout, Layout::VBox);
        assert_eq!(children.len(), 3);
    }

    #[test]
    fn hit_tests_buttons() {
        let src = r#"
<HBOX>
Button("A",a)
Button("B",b)
</HBOX>
"#;
        let module = parse_module(src).unwrap();
        let sheet = StyleSheet::new();
        // 100x40 window. Default padding/margin (8/2) inset the content area;
        // with a single HBox child and two buttons, button A ~ (20,18..26),
        // button B ~ (50,18..26).
        let placed = layout(&module.root, &sheet, 100, 40);
        assert_eq!(hit_test(&placed, 25, 22).as_deref(), Some("a"));
        assert_eq!(hit_test(&placed, 65, 22).as_deref(), Some("b"));
        assert_eq!(hit_test(&placed, 5, 5).as_deref(), None);
    }

    #[test]
    fn resolves_css() {
        let css = r#"
button { background: #2266aa; color: #ffffff; }
"#;
        let sheet = style::parse(css);
        let s = style::resolve(&sheet, &["button", "window"]);
        assert_eq!(s.background, 0x2266aa);
        assert_eq!(s.color, 0xffffff);
    }
}
