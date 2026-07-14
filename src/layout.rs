//! Block layout and CSS box-model geometry.
//!
//! Ferrum currently lays out block boxes in normal vertical flow. Inline
//! formatting, margin collapsing, min/max constraints, and positioned layout
//! are intentionally reserved for later milestones.

use crate::css::Value;
use crate::dom::NodeKind;
use crate::style::{PropertyMap, StyledNode};
use std::fmt::{self, Write};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn expanded_by(self, edges: EdgeSizes) -> Self {
        Self {
            x: self.x - edges.left,
            y: self.y - edges.top,
            width: self.width + edges.left + edges.right,
            height: self.height + edges.top + edges.bottom,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    pub fn padding_box(self) -> Rect {
        self.content.expanded_by(self.padding)
    }

    pub fn border_box(self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }

    pub fn margin_box(self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxType {
    Block,
    Inline,
    Anonymous,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutBox {
    pub dimensions: Dimensions,
    pub box_type: BoxType,
    pub node_name: String,
    pub element_id: Option<String>,
    pub text: Option<String>,
    pub properties: PropertyMap,
    pub children: Vec<LayoutBox>,
}

impl LayoutBox {
    /// Produces deterministic geometry output for inspection and tests.
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        self.write_pretty(0, &mut output)
            .expect("writing to a String cannot fail");
        output
    }

    fn write_pretty(&self, depth: usize, output: &mut String) -> fmt::Result {
        let kind = match self.box_type {
            BoxType::Block => "block",
            BoxType::Inline => "inline",
            BoxType::Anonymous => "anonymous",
        };
        let content = self.dimensions.content;
        writeln!(
            output,
            "{}{} {} x={:.1} y={:.1} width={:.1} height={:.1}",
            "  ".repeat(depth),
            kind,
            self.node_name,
            content.x,
            content.y,
            content.width,
            content.height,
        )?;
        for child in &self.children {
            child.write_pretty(depth + 1, output)?;
        }
        Ok(())
    }

    fn layout(&mut self, containing_block: Dimensions) {
        self.calculate_width(containing_block);
        self.calculate_position(containing_block);
        self.layout_children();
        self.calculate_height();
    }

    fn calculate_width(&mut self, containing_block: Dimensions) {
        let auto = Value::Keyword("auto".into());
        let zero = Value::Length(0.0, crate::css::Unit::Px);

        let width = self.property("width").unwrap_or(&auto).clone();
        let margin_left = self.lookup("margin-left", "margin", &zero).clone();
        let margin_right = self.lookup("margin-right", "margin", &zero).clone();
        let border_left = self
            .lookup("border-left-width", "border-width", &zero)
            .clone();
        let border_right = self
            .lookup("border-right-width", "border-width", &zero)
            .clone();
        let padding_left = self.lookup("padding-left", "padding", &zero).clone();
        let padding_right = self.lookup("padding-right", "padding", &zero).clone();

        let total = [
            &margin_left,
            &margin_right,
            &border_left,
            &border_right,
            &padding_left,
            &padding_right,
            &width,
        ]
        .iter()
        .map(|value| length_or_zero(value))
        .sum::<f32>();
        let underflow = containing_block.content.width - total;

        let width_auto = is_auto(&width);
        let left_auto = is_auto(&margin_left);
        let right_auto = is_auto(&margin_right);
        let (content_width, left_margin, right_margin) = match (width_auto, left_auto, right_auto) {
            (false, false, false) => (
                length_or_zero(&width),
                length_or_zero(&margin_left),
                length_or_zero(&margin_right) + underflow,
            ),
            (false, true, false) => (
                length_or_zero(&width),
                underflow,
                length_or_zero(&margin_right),
            ),
            (false, false, true) => (
                length_or_zero(&width),
                length_or_zero(&margin_left),
                underflow,
            ),
            (false, true, true) => (length_or_zero(&width), underflow / 2.0, underflow / 2.0),
            (true, left_auto, right_auto) if underflow >= 0.0 => (
                underflow,
                if left_auto {
                    0.0
                } else {
                    length_or_zero(&margin_left)
                },
                if right_auto {
                    0.0
                } else {
                    length_or_zero(&margin_right)
                },
            ),
            (true, left_auto, right_auto) => (
                0.0,
                if left_auto {
                    0.0
                } else {
                    length_or_zero(&margin_left)
                },
                if right_auto {
                    underflow
                } else {
                    length_or_zero(&margin_right) + underflow
                },
            ),
        };

        self.dimensions.content.width = content_width;
        self.dimensions.padding.left = length_or_zero(&padding_left);
        self.dimensions.padding.right = length_or_zero(&padding_right);
        self.dimensions.border.left = length_or_zero(&border_left);
        self.dimensions.border.right = length_or_zero(&border_right);
        self.dimensions.margin.left = left_margin;
        self.dimensions.margin.right = right_margin;
    }

    fn calculate_position(&mut self, containing_block: Dimensions) {
        let zero = Value::Length(0.0, crate::css::Unit::Px);
        self.dimensions.margin.top = length_or_zero(self.lookup("margin-top", "margin", &zero));
        self.dimensions.margin.bottom =
            length_or_zero(self.lookup("margin-bottom", "margin", &zero));
        self.dimensions.border.top =
            length_or_zero(self.lookup("border-top-width", "border-width", &zero));
        self.dimensions.border.bottom =
            length_or_zero(self.lookup("border-bottom-width", "border-width", &zero));
        self.dimensions.padding.top = length_or_zero(self.lookup("padding-top", "padding", &zero));
        self.dimensions.padding.bottom =
            length_or_zero(self.lookup("padding-bottom", "padding", &zero));

        self.dimensions.content.x = containing_block.content.x
            + self.dimensions.margin.left
            + self.dimensions.border.left
            + self.dimensions.padding.left;
        self.dimensions.content.y = containing_block.content.y
            + containing_block.content.height
            + self.dimensions.margin.top
            + self.dimensions.border.top
            + self.dimensions.padding.top;
    }

    fn layout_children(&mut self) {
        for child in &mut self.children {
            child.layout(self.dimensions);
            self.dimensions.content.height += child.dimensions.margin_box().height;
        }
    }

    fn calculate_height(&mut self) {
        if let Some(Value::Length(height, _)) = self.property("height") {
            self.dimensions.content.height = *height;
        } else if let Some(text) = &self.text {
            self.dimensions.content.height = (crate::text::wrap(text, self.dimensions.content.width)
                .len() as u32
                * crate::text::LINE_HEIGHT) as f32;
        }
    }

    fn property(&self, name: &str) -> Option<&Value> {
        self.properties.get(name)
    }

    fn lookup<'a>(&'a self, name: &str, shorthand: &str, default: &'a Value) -> &'a Value {
        self.property(name)
            .or_else(|| self.property(shorthand))
            .unwrap_or(default)
    }
}

/// Builds and lays out a block tree inside a viewport of the given size.
pub fn layout_tree(root: &StyledNode<'_>, viewport_width: f32, viewport_height: f32) -> LayoutBox {
    let mut root_box = build_layout_tree(root).unwrap_or_else(|| LayoutBox {
        dimensions: Dimensions::default(),
        box_type: BoxType::Anonymous,
        node_name: "#empty".into(),
        element_id: None,
        text: None,
        properties: PropertyMap::new(),
        children: Vec::new(),
    });
    let viewport = Dimensions {
        content: Rect {
            width: viewport_width,
            height: 0.0,
            ..Rect::default()
        },
        ..Dimensions::default()
    };
    root_box.layout(viewport);
    root_box.dimensions.content.height = root_box.dimensions.content.height.max(viewport_height);
    root_box
}

fn build_layout_tree(styled: &StyledNode<'_>) -> Option<LayoutBox> {
    let box_type = match display(styled) {
        Display::None => return None,
        Display::Block => BoxType::Block,
        Display::Inline => BoxType::Inline,
    };
    let (node_name, element_id, text) = match &styled.node.kind {
        NodeKind::Document => ("#document".into(), None, None),
        NodeKind::Element(element) => (
            format!("<{}>", element.tag_name),
            element.attributes.get("id").cloned(),
            None,
        ),
        NodeKind::Text(value) => ("#text".into(), None, Some(value.clone())),
        NodeKind::Comment(_) => return None,
    };
    let children = styled
        .children
        .iter()
        .filter_map(build_layout_tree)
        .collect();
    Some(LayoutBox {
        dimensions: Dimensions::default(),
        box_type,
        node_name,
        element_id,
        text,
        properties: styled.properties.clone(),
        children,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Display {
    Block,
    Inline,
    None,
}

fn display(styled: &StyledNode<'_>) -> Display {
    match styled.property("display") {
        Some(Value::Keyword(value)) if value == "none" => Display::None,
        Some(Value::Keyword(value)) if value == "inline" => Display::Inline,
        Some(Value::Keyword(value)) if value == "block" => Display::Block,
        _ => match &styled.node.kind {
            NodeKind::Text(_) => Display::Inline,
            NodeKind::Comment(_) => Display::None,
            _ => Display::Block,
        },
    }
}

fn is_auto(value: &Value) -> bool {
    matches!(value, Value::Keyword(keyword) if keyword == "auto")
}

fn length_or_zero(value: &Value) -> f32 {
    match value {
        Value::Length(length, _) => *length,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{css, html, style};

    fn layout(html_source: &str, css_source: &str, width: f32) -> LayoutBox {
        let document = html::parse(html_source).expect("valid HTML should parse");
        let stylesheet = css::parse(css_source).expect("valid CSS should parse");
        let styled = style::style_tree(&document, &stylesheet);
        layout_tree(&styled, width, 600.0)
    }

    #[test]
    fn centers_a_fixed_width_box_with_auto_margins() {
        let root = layout(
            r#"<main id="app"></main>"#,
            "#app { width: 600px; margin-left: auto; margin-right: auto; padding: 10px; border-width: 2px }",
            800.0,
        );
        let main = &root.children[0];

        assert_eq!(main.dimensions.content.width, 600.0);
        assert_eq!(main.dimensions.margin.left, 88.0);
        assert_eq!(main.dimensions.margin.right, 88.0);
        assert_eq!(main.dimensions.content.x, 100.0);
    }

    #[test]
    fn auto_width_fills_space_inside_fixed_edges() {
        let root = layout(
            "<main></main>",
            "main { margin-left: 10px; margin-right: 20px; padding: 5px }",
            200.0,
        );
        let main = &root.children[0];

        assert_eq!(main.dimensions.content.width, 160.0);
        assert_eq!(main.dimensions.margin.left, 10.0);
        assert_eq!(main.dimensions.margin.right, 20.0);
        assert_eq!(main.dimensions.content.x, 15.0);
    }

    #[test]
    fn stacks_children_vertically_and_expands_parent_height() {
        let root = layout(
            "<main><p></p><p></p></main>",
            "p { height: 20px; margin-top: 5px; margin-bottom: 7px }",
            400.0,
        );
        let main = &root.children[0];

        assert_eq!(main.children[0].dimensions.content.y, 5.0);
        assert_eq!(main.children[1].dimensions.content.y, 37.0);
        assert_eq!(main.dimensions.content.height, 64.0);
    }

    #[test]
    fn explicit_height_overrides_content_height() {
        let root = layout(
            "<main><p></p></main>",
            "main { height: 100px } p { height: 20px }",
            400.0,
        );
        assert_eq!(root.children[0].dimensions.content.height, 100.0);
    }

    #[test]
    fn text_uses_bitmap_font_metrics_and_wraps() {
        let root = layout("<p>one two three</p>", "p { width: 84px }", 100.0);
        let text = &root.children[0].children[0];

        assert_eq!(text.dimensions.content.height, 36.0);
        assert_eq!(root.children[0].dimensions.content.height, 36.0);
    }

    #[test]
    fn display_none_removes_a_subtree() {
        let root = layout(
            "<main><p></p><aside><p></p></aside></main>",
            "p { height: 10px } aside { display: none }",
            400.0,
        );
        assert_eq!(root.children[0].children.len(), 1);
    }

    #[test]
    fn box_helpers_expand_content_in_order() {
        let dimensions = Dimensions {
            content: Rect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 20.0,
            },
            padding: EdgeSizes {
                left: 5.0,
                right: 5.0,
                top: 2.0,
                bottom: 2.0,
            },
            border: EdgeSizes {
                left: 1.0,
                right: 1.0,
                top: 1.0,
                bottom: 1.0,
            },
            margin: EdgeSizes::default(),
        };
        assert_eq!(dimensions.padding_box().width, 110.0);
        assert_eq!(dimensions.border_box().width, 112.0);
    }
}
