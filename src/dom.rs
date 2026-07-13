use std::collections::BTreeMap;
use std::fmt::{self, Write};

/// One node in Ferrum's document tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    pub children: Vec<Node>,
}

/// The node types supported by the first Ferrum milestone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Document,
    Element(ElementData),
    Text(String),
    Comment(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: BTreeMap<String, String>,
}

impl Node {
    pub fn document(children: Vec<Node>) -> Self {
        Self {
            kind: NodeKind::Document,
            children,
        }
    }

    pub fn element(tag_name: impl Into<String>, attributes: BTreeMap<String, String>) -> Self {
        Self {
            kind: NodeKind::Element(ElementData {
                tag_name: tag_name.into(),
                attributes,
            }),
            children: Vec::new(),
        }
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self {
            kind: NodeKind::Text(value.into()),
            children: Vec::new(),
        }
    }

    pub fn comment(value: impl Into<String>) -> Self {
        Self {
            kind: NodeKind::Comment(value.into()),
            children: Vec::new(),
        }
    }

    /// Produces a compact, deterministic tree representation for the CLI.
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        self.write_pretty(0, &mut output)
            .expect("writing to a String cannot fail");
        output
    }

    fn write_pretty(&self, depth: usize, output: &mut String) -> fmt::Result {
        let indent = "  ".repeat(depth);
        match &self.kind {
            NodeKind::Document => writeln!(output, "{indent}#document")?,
            NodeKind::Element(element) => {
                write!(output, "{indent}<{}", element.tag_name)?;
                for (name, value) in &element.attributes {
                    write!(output, " {name}=\"{value}\"")?;
                }
                writeln!(output, ">")?;
            }
            NodeKind::Text(value) => writeln!(output, "{indent}\"{}\"", value.escape_debug())?,
            NodeKind::Comment(value) => writeln!(output, "{indent}<!--{}-->", value.trim())?,
        }

        for child in &self.children {
            child.write_pretty(depth + 1, output)?;
        }
        Ok(())
    }
}
