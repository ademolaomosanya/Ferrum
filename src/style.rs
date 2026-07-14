//! Selector matching and cascade resolution.

use crate::css::{Rule, Selector, SimpleSelector, Specificity, Stylesheet, Value};
use crate::dom::{ElementData, Node, NodeKind};
use std::collections::BTreeMap;
use std::fmt::{self, Write};

pub type PropertyMap = BTreeMap<String, Value>;

/// A DOM node paired with the properties produced by Ferrum's cascade.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledNode<'a> {
    pub node: &'a Node,
    pub properties: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

impl<'a> StyledNode<'a> {
    pub fn property(&self, name: &str) -> Option<&Value> {
        self.properties.get(name)
    }

    /// Produces a deterministic view of the styled tree for the CLI and tests.
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        self.write_pretty(0, &mut output)
            .expect("writing to a String cannot fail");
        output
    }

    fn write_pretty(&self, depth: usize, output: &mut String) -> fmt::Result {
        let indent = "  ".repeat(depth);
        write!(output, "{indent}")?;
        match &self.node.kind {
            NodeKind::Document => output.push_str("#document"),
            NodeKind::Element(element) => write!(output, "<{}>", element.tag_name)?,
            NodeKind::Text(value) => write!(output, "\"{}\"", value.escape_debug())?,
            NodeKind::Comment(value) => write!(output, "<!--{}-->", value.trim())?,
        }
        if !self.properties.is_empty() {
            output.push_str(" {");
            for (index, (name, value)) in self.properties.iter().enumerate() {
                if index > 0 {
                    output.push_str("; ");
                }
                write!(output, "{name}: {value}")?;
            }
            output.push('}');
        }
        output.push('\n');

        for child in &self.children {
            child.write_pretty(depth + 1, output)?;
        }
        Ok(())
    }
}

/// Builds a styled tree using author rules from `stylesheet`.
pub fn style_tree<'a>(root: &'a Node, stylesheet: &Stylesheet) -> StyledNode<'a> {
    style_node(root, stylesheet, &PropertyMap::new())
}

fn style_node<'a>(
    node: &'a Node,
    stylesheet: &Stylesheet,
    parent_properties: &PropertyMap,
) -> StyledNode<'a> {
    let mut properties = inherited_properties(parent_properties);
    if let NodeKind::Element(element) = &node.kind {
        properties.extend(specified_values(element, stylesheet));
    }

    let children = node
        .children
        .iter()
        .map(|child| style_node(child, stylesheet, &properties))
        .collect();

    StyledNode {
        node,
        properties,
        children,
    }
}

fn inherited_properties(parent: &PropertyMap) -> PropertyMap {
    const INHERITED: &[&str] = &["color"];
    INHERITED
        .iter()
        .filter_map(|name| {
            parent
                .get(*name)
                .cloned()
                .map(|value| ((*name).to_owned(), value))
        })
        .collect()
}

/// Computes declarations for an element using specificity and source order.
pub fn specified_values(element: &ElementData, stylesheet: &Stylesheet) -> PropertyMap {
    let mut matching_rules: Vec<(Specificity, usize, &Rule)> = stylesheet
        .rules
        .iter()
        .enumerate()
        .filter_map(|(source_order, rule)| {
            rule.selectors
                .iter()
                .filter(|selector| matches_selector(element, selector))
                .map(Selector::specificity)
                .max()
                .map(|specificity| (specificity, source_order, rule))
        })
        .collect();

    matching_rules.sort_by_key(|(specificity, source_order, _)| (*specificity, *source_order));

    let mut values = PropertyMap::new();
    for (_, _, rule) in matching_rules {
        for declaration in &rule.declarations {
            values.insert(declaration.name.clone(), declaration.value.clone());
        }
    }
    values
}

pub fn matches_selector(element: &ElementData, selector: &Selector) -> bool {
    match selector {
        Selector::Simple(selector) => matches_simple_selector(element, selector),
    }
}

fn matches_simple_selector(element: &ElementData, selector: &SimpleSelector) -> bool {
    if selector
        .tag_name
        .as_ref()
        .is_some_and(|tag_name| tag_name != &element.tag_name)
    {
        return false;
    }

    if selector
        .id
        .as_ref()
        .is_some_and(|id| element.attributes.get("id") != Some(id))
    {
        return false;
    }

    let classes = element
        .attributes
        .get("class")
        .map_or_else(Vec::new, |value| value.split_whitespace().collect());
    selector
        .classes
        .iter()
        .all(|class| classes.contains(&class.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{css, html};

    fn element_from(source: &str) -> Node {
        html::parse(source)
            .expect("valid HTML should parse")
            .children
            .remove(0)
    }

    #[test]
    fn matches_compound_selectors() {
        let node = element_from(r#"<main id="app" class="shell featured"></main>"#);
        let NodeKind::Element(element) = &node.kind else {
            panic!("expected an element");
        };
        let stylesheet = css::parse("main#app.shell.featured { display: block }")
            .expect("valid CSS should parse");

        assert!(matches_selector(element, &stylesheet.rules[0].selectors[0]));
    }

    #[test]
    fn rejects_a_selector_when_any_class_is_missing() {
        let node = element_from(r#"<div class="card"></div>"#);
        let NodeKind::Element(element) = &node.kind else {
            panic!("expected an element");
        };
        let stylesheet =
            css::parse(".card.featured { color: red }").expect("valid CSS should parse");

        assert!(!matches_selector(
            element,
            &stylesheet.rules[0].selectors[0]
        ));
    }

    #[test]
    fn specificity_wins_over_later_source_order() {
        let node = element_from(r#"<p id="intro" class="lead"></p>"#);
        let NodeKind::Element(element) = &node.kind else {
            panic!("expected an element");
        };
        let stylesheet =
            css::parse("#intro { color: #111 } .lead { color: #222 } p { color: #333 }")
                .expect("valid CSS should parse");

        assert_eq!(
            specified_values(element, &stylesheet)["color"],
            Value::Color(crate::css::Color {
                red: 17,
                green: 17,
                blue: 17,
                alpha: 255,
            })
        );
    }

    #[test]
    fn later_rule_wins_when_specificity_is_equal() {
        let node = element_from(r#"<p class="lead"></p>"#);
        let NodeKind::Element(element) = &node.kind else {
            panic!("expected an element");
        };
        let stylesheet = css::parse(".lead { display: block } .lead { display: inline }")
            .expect("valid CSS should parse");

        assert_eq!(
            specified_values(element, &stylesheet)["display"],
            Value::Keyword("inline".into())
        );
    }

    #[test]
    fn color_is_inherited_but_width_is_not() {
        let document = html::parse("<main><p>Hello</p></main>").expect("valid HTML should parse");
        let stylesheet =
            css::parse("main { color: #123; width: 400px }").expect("valid CSS should parse");
        let styled = style_tree(&document, &stylesheet);
        let paragraph = &styled.children[0].children[0];
        let text = &paragraph.children[0];

        assert!(paragraph.property("color").is_some());
        assert!(paragraph.property("width").is_none());
        assert!(text.property("color").is_some());
    }

    #[test]
    fn pretty_print_shows_cascaded_properties() {
        let document = html::parse(r#"<main id="app"><h1>Hello</h1></main>"#)
            .expect("valid HTML should parse");
        let stylesheet = css::parse("main { color: #123 } #app { display: block }")
            .expect("valid CSS should parse");

        assert_eq!(
            style_tree(&document, &stylesheet).pretty_print(),
            concat!(
                "#document\n",
                "  <main> {color: #112233ff; display: block}\n",
                "    <h1> {color: #112233ff}\n",
                "      \"Hello\" {color: #112233ff}\n",
            )
        );
    }
}
