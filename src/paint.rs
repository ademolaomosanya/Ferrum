//! Display-list construction and dependency-free raster painting.

use crate::css::{Color, Value};
use crate::layout::{LayoutBox, Rect};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayCommand {
    SolidColor {
        color: Color,
        rect: Rect,
    },
    Text {
        color: Color,
        rect: Rect,
        text: String,
    },
}

/// Converts layout boxes into back-to-front painting commands.
pub fn build_display_list(root: &LayoutBox) -> Vec<DisplayCommand> {
    let mut commands = Vec::new();
    render_layout_box(root, &mut commands);
    commands
}

fn render_layout_box(layout_box: &LayoutBox, commands: &mut Vec<DisplayCommand>) {
    render_background(layout_box, commands);
    render_borders(layout_box, commands);
    render_text(layout_box, commands);
    for child in &layout_box.children {
        render_layout_box(child, commands);
    }
}

fn render_text(layout_box: &LayoutBox, commands: &mut Vec<DisplayCommand>) {
    let Some(text) = &layout_box.text else {
        return;
    };
    let color = color_value(layout_box, &["color"]).unwrap_or(Color {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 255,
    });
    commands.push(DisplayCommand::Text {
        color,
        rect: layout_box.dimensions.content,
        text: text.clone(),
    });
}

fn render_background(layout_box: &LayoutBox, commands: &mut Vec<DisplayCommand>) {
    let Some(color) = color_value(layout_box, &["background-color", "background"]) else {
        return;
    };
    commands.push(DisplayCommand::SolidColor {
        color,
        rect: layout_box.dimensions.border_box(),
    });
}

fn render_borders(layout_box: &LayoutBox, commands: &mut Vec<DisplayCommand>) {
    let dimensions = layout_box.dimensions;
    let border_box = dimensions.border_box();
    let fallback = color_value(layout_box, &["border-color", "color"]);

    let edges = [
        (
            dimensions.border.left,
            Rect {
                x: border_box.x,
                y: border_box.y,
                width: dimensions.border.left,
                height: border_box.height,
            },
            "border-left-color",
        ),
        (
            dimensions.border.right,
            Rect {
                x: border_box.x + border_box.width - dimensions.border.right,
                y: border_box.y,
                width: dimensions.border.right,
                height: border_box.height,
            },
            "border-right-color",
        ),
        (
            dimensions.border.top,
            Rect {
                x: border_box.x,
                y: border_box.y,
                width: border_box.width,
                height: dimensions.border.top,
            },
            "border-top-color",
        ),
        (
            dimensions.border.bottom,
            Rect {
                x: border_box.x,
                y: border_box.y + border_box.height - dimensions.border.bottom,
                width: border_box.width,
                height: dimensions.border.bottom,
            },
            "border-bottom-color",
        ),
    ];

    for (width, rect, property) in edges {
        if width <= 0.0 {
            continue;
        }
        let color = color_value(layout_box, &[property]).or(fallback);
        if let Some(color) = color {
            commands.push(DisplayCommand::SolidColor { color, rect });
        }
    }
}

fn color_value(layout_box: &LayoutBox, names: &[&str]) -> Option<Color> {
    names
        .iter()
        .find_map(|name| match layout_box.properties.get(*name) {
            Some(Value::Color(color)) => Some(*color),
            _ => None,
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Color>,
}

impl Canvas {
    pub fn new(width: u32, height: u32, background: Color) -> Self {
        Self {
            width,
            height,
            pixels: vec![background; width as usize * height as usize],
        }
    }

    pub fn paint(&mut self, commands: &[DisplayCommand]) {
        for command in commands {
            match command {
                DisplayCommand::SolidColor { color, rect } => self.fill_rect(*rect, *color),
                DisplayCommand::Text { color, rect, text } => self.draw_text(text, *rect, *color),
            }
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.pixels[y as usize * self.width as usize + x as usize])
    }

    pub fn to_ppm_bytes(&self) -> Vec<u8> {
        let header = format!("P6\n{} {}\n255\n", self.width, self.height);
        let mut bytes = Vec::with_capacity(header.len() + self.pixels.len() * 3);
        bytes.extend_from_slice(header.as_bytes());
        for pixel in &self.pixels {
            bytes.extend_from_slice(&[pixel.red, pixel.green, pixel.blue]);
        }
        bytes
    }

    pub fn save_ppm(&self, path: impl AsRef<Path>) -> io::Result<()> {
        fs::write(path, self.to_ppm_bytes())
    }

    /// Downsamples the canvas into ANSI true-color terminal cells.
    pub fn ansi_preview(&self, columns: u32) -> String {
        let columns = columns.clamp(1, self.width.max(1));
        let cell_width = self.width.div_ceil(columns).max(1);
        let cell_height = cell_width * 2;
        let rows = self.height.div_ceil(cell_height);
        let mut output = String::new();

        for row in 0..rows {
            for column in 0..columns {
                let x_start = column * cell_width;
                let y_start = row * cell_height;
                let x_end = (x_start + cell_width).min(self.width);
                let y_end = (y_start + cell_height).min(self.height);
                let mut red = 0_u64;
                let mut green = 0_u64;
                let mut blue = 0_u64;
                let mut count = 0_u64;
                for y in y_start..y_end {
                    for x in x_start..x_end {
                        let pixel = self.pixels[y as usize * self.width as usize + x as usize];
                        red += u64::from(pixel.red);
                        green += u64::from(pixel.green);
                        blue += u64::from(pixel.blue);
                        count += 1;
                    }
                }
                if let (Some(red), Some(green), Some(blue)) = (
                    red.checked_div(count),
                    green.checked_div(count),
                    blue.checked_div(count),
                ) {
                    output.push_str(&format!("\x1b[48;2;{};{};{}m  ", red, green, blue,));
                }
            }
            output.push_str("\x1b[0m\n");
        }
        output
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let x_start = rect.x.floor().max(0.0) as u32;
        let y_start = rect.y.floor().max(0.0) as u32;
        let x_end = (rect.x + rect.width).ceil().clamp(0.0, self.width as f32) as u32;
        let y_end = (rect.y + rect.height).ceil().clamp(0.0, self.height as f32) as u32;

        for y in y_start.min(self.height)..y_end {
            for x in x_start.min(self.width)..x_end {
                let index = y as usize * self.width as usize + x as usize;
                self.pixels[index] = blend(color, self.pixels[index]);
            }
        }
    }

    fn draw_text(&mut self, text: &str, rect: Rect, color: Color) {
        for (line_index, line) in crate::text::wrap(text, rect.width).iter().enumerate() {
            let y = rect.y + line_index as f32 * crate::text::LINE_HEIGHT as f32;
            for (character_index, character) in line.chars().enumerate() {
                let x = rect.x + character_index as f32 * crate::text::ADVANCE as f32;
                for (row, bits) in crate::text::glyph(character).iter().enumerate() {
                    for column in 0..crate::text::GLYPH_WIDTH {
                        let mask = 1 << (crate::text::GLYPH_WIDTH - 1 - column);
                        if bits & mask == 0 {
                            continue;
                        }
                        self.fill_rect(
                            Rect {
                                x: x + (column * crate::text::SCALE) as f32,
                                y: y + (row as u32 * crate::text::SCALE) as f32,
                                width: crate::text::SCALE as f32,
                                height: crate::text::SCALE as f32,
                            },
                            color,
                        );
                    }
                }
            }
        }
    }
}

/// Paints a layout tree onto an opaque white canvas.
pub fn paint(root: &LayoutBox, width: u32, height: u32) -> Canvas {
    let default_background = Color {
        red: 255,
        green: 255,
        blue: 255,
        alpha: 255,
    };
    let canvas_background = if root.node_name == "#document" {
        root.children
            .iter()
            .find(|child| child.node_name == "<html>")
            .and_then(|html| color_value(html, &["background-color", "background"]))
            .unwrap_or(default_background)
    } else {
        default_background
    };
    let mut canvas = Canvas::new(width, height, canvas_background);
    canvas.paint(&build_display_list(root));
    canvas
}

fn blend(source: Color, destination: Color) -> Color {
    if source.alpha == 255 {
        return source;
    }
    if source.alpha == 0 {
        return destination;
    }
    let alpha = source.alpha as f32 / 255.0;
    let channel = |source: u8, destination: u8| {
        (source as f32 * alpha + destination as f32 * (1.0 - alpha)).round() as u8
    };
    Color {
        red: channel(source.red, destination.red),
        green: channel(source.green, destination.green),
        blue: channel(source.blue, destination.blue),
        alpha: 255,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{BoxType, Dimensions, EdgeSizes};
    use crate::style::PropertyMap;

    const RED: Color = Color {
        red: 255,
        green: 0,
        blue: 0,
        alpha: 255,
    };
    const BLUE: Color = Color {
        red: 0,
        green: 0,
        blue: 255,
        alpha: 255,
    };
    const WHITE: Color = Color {
        red: 255,
        green: 255,
        blue: 255,
        alpha: 255,
    };

    fn painted_box() -> LayoutBox {
        let mut properties = PropertyMap::new();
        properties.insert("background".into(), Value::Color(RED));
        properties.insert("border-color".into(), Value::Color(BLUE));
        LayoutBox {
            dimensions: Dimensions {
                content: Rect {
                    x: 2.0,
                    y: 2.0,
                    width: 4.0,
                    height: 4.0,
                },
                padding: EdgeSizes {
                    left: 1.0,
                    right: 1.0,
                    top: 1.0,
                    bottom: 1.0,
                },
                border: EdgeSizes {
                    left: 1.0,
                    right: 1.0,
                    top: 1.0,
                    bottom: 1.0,
                },
                margin: EdgeSizes::default(),
            },
            box_type: BoxType::Block,
            node_name: "<main>".into(),
            element_id: None,
            text: None,
            properties,
            children: Vec::new(),
        }
    }

    #[test]
    fn display_list_contains_a_background_and_four_borders() {
        assert_eq!(build_display_list(&painted_box()).len(), 5);
    }

    #[test]
    fn emits_and_paints_text_commands() {
        let mut text_box = painted_box();
        text_box.dimensions = Dimensions {
            content: Rect {
                width: crate::text::ADVANCE as f32,
                height: crate::text::LINE_HEIGHT as f32,
                ..Rect::default()
            },
            ..Dimensions::default()
        };
        text_box.text = Some("A".into());
        text_box.properties.clear();
        text_box
            .properties
            .insert("color".into(), Value::Color(RED));

        let commands = build_display_list(&text_box);
        assert!(matches!(commands.as_slice(), [DisplayCommand::Text { .. }]));
        let mut canvas = Canvas::new(12, 18, WHITE);
        canvas.paint(&commands);
        assert_eq!(canvas.pixel(2, 0), Some(RED));
    }

    #[test]
    fn paints_background_and_border_pixels() {
        let canvas = paint(&painted_box(), 10, 10);
        assert_eq!(canvas.pixel(0, 0), Some(BLUE));
        assert_eq!(canvas.pixel(1, 1), Some(RED));
        assert_eq!(canvas.pixel(2, 2), Some(RED));
        assert_eq!(canvas.pixel(8, 8), Some(WHITE));
    }

    #[test]
    fn propagates_the_html_background_to_the_canvas() {
        let mut html = painted_box();
        html.node_name = "<html>".into();
        html.properties
            .insert("background".into(), Value::Color(RED));
        let document = LayoutBox {
            dimensions: Dimensions::default(),
            box_type: BoxType::Block,
            node_name: "#document".into(),
            element_id: None,
            text: None,
            properties: PropertyMap::new(),
            children: vec![html],
        };

        assert_eq!(paint(&document, 10, 10).pixel(9, 9), Some(RED));
    }

    #[test]
    fn clips_commands_to_the_canvas() {
        let mut canvas = Canvas::new(2, 2, WHITE);
        canvas.paint(&[DisplayCommand::SolidColor {
            color: RED,
            rect: Rect {
                x: -10.0,
                y: -10.0,
                width: 11.0,
                height: 11.0,
            },
        }]);
        assert_eq!(canvas.pixel(0, 0), Some(RED));
        assert_eq!(canvas.pixel(1, 1), Some(WHITE));
    }

    #[test]
    fn blends_translucent_colors() {
        assert_eq!(
            blend(
                Color {
                    red: 255,
                    green: 0,
                    blue: 0,
                    alpha: 128,
                },
                BLUE,
            ),
            Color {
                red: 128,
                green: 0,
                blue: 127,
                alpha: 255,
            }
        );
    }

    #[test]
    fn encodes_a_binary_ppm() {
        let canvas = Canvas::new(2, 1, RED);
        let bytes = canvas.to_ppm_bytes();
        assert!(bytes.starts_with(b"P6\n2 1\n255\n"));
        assert_eq!(bytes.len(), b"P6\n2 1\n255\n".len() + 6);
    }

    #[test]
    fn creates_a_true_color_terminal_preview() {
        let canvas = Canvas::new(2, 2, RED);
        let preview = canvas.ansi_preview(1);
        assert!(preview.contains("\x1b[48;2;255;0;0m"));
        assert!(preview.ends_with("\x1b[0m\n"));
    }
}
