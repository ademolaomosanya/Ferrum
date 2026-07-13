//! A deliberately small HTML parser.
//!
//! This is not yet an implementation of the WHATWG parsing algorithm. It is a
//! well-tested first milestone that gives later tokenizer and tree-builder work
//! a clear home.

use crate::dom::Node;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl ParseError {
    fn new(offset: usize, message: impl Into<String>) -> Self {
        Self {
            offset,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "HTML parse error at byte {}: {}",
            self.offset, self.message
        )
    }
}

impl Error for ParseError {}

/// Parses an HTML fragment into a document node.
pub fn parse(source: &str) -> Result<Node, ParseError> {
    Parser::new(source).parse()
}

struct Parser<'a> {
    source: &'a str,
    position: usize,
    open_nodes: Vec<Node>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
            open_nodes: vec![Node::document(Vec::new())],
        }
    }

    fn parse(mut self) -> Result<Node, ParseError> {
        while !self.is_eof() {
            if self.starts_with("<!--") {
                self.parse_comment()?;
            } else if self.starts_with_case_insensitive("<!doctype") {
                self.consume_doctype()?;
            } else if self.starts_with("</") {
                self.parse_closing_tag()?;
            } else if self.starts_with("<") {
                self.parse_opening_tag()?;
            } else {
                self.parse_text();
            }
        }

        if self.open_nodes.len() != 1 {
            let tag = tag_name(self.open_nodes.last().expect("document node exists"));
            return Err(ParseError::new(
                self.position,
                format!("unclosed <{tag}> element"),
            ));
        }

        Ok(self.open_nodes.pop().expect("document node exists"))
    }

    fn parse_comment(&mut self) -> Result<(), ParseError> {
        let start = self.position;
        self.position += "<!--".len();
        let remainder = &self.source[self.position..];
        let Some(end) = remainder.find("-->") else {
            return Err(ParseError::new(start, "unterminated comment"));
        };
        let value = &remainder[..end];
        self.position += end + "-->".len();
        self.append(Node::comment(value));
        Ok(())
    }

    fn consume_doctype(&mut self) -> Result<(), ParseError> {
        let start = self.position;
        let Some(end) = self.source[self.position..].find('>') else {
            return Err(ParseError::new(start, "unterminated doctype"));
        };
        self.position += end + 1;
        Ok(())
    }

    fn parse_closing_tag(&mut self) -> Result<(), ParseError> {
        let start = self.position;
        self.position += 2;
        self.skip_whitespace();
        let closing_name = self.consume_name().to_ascii_lowercase();
        if closing_name.is_empty() {
            return Err(ParseError::new(start, "expected a closing tag name"));
        }
        self.skip_whitespace();
        self.expect('>')?;

        if self.open_nodes.len() == 1 {
            return Err(ParseError::new(
                start,
                format!("unexpected closing tag </{closing_name}>"),
            ));
        }

        let current_name = tag_name(self.open_nodes.last().expect("open element exists"));
        if current_name != closing_name {
            return Err(ParseError::new(
                start,
                format!("expected </{current_name}> but found </{closing_name}>"),
            ));
        }

        let completed = self.open_nodes.pop().expect("open element exists");
        self.append(completed);
        Ok(())
    }

    fn parse_opening_tag(&mut self) -> Result<(), ParseError> {
        let start = self.position;
        self.position += 1;
        let name = self.consume_name().to_ascii_lowercase();
        if name.is_empty() {
            return Err(ParseError::new(start, "expected a tag name after '<'"));
        }

        let mut attributes = BTreeMap::new();
        let self_closing = loop {
            self.skip_whitespace();
            if self.consume_if("/>") {
                break true;
            }
            if self.consume_if(">") {
                break false;
            }
            if self.is_eof() {
                return Err(ParseError::new(start, format!("unterminated <{name}> tag")));
            }

            let attribute_start = self.position;
            let attribute_name = self.consume_name().to_ascii_lowercase();
            if attribute_name.is_empty() {
                return Err(ParseError::new(
                    attribute_start,
                    "expected an attribute name",
                ));
            }
            self.skip_whitespace();
            let value = if self.consume_if("=") {
                self.skip_whitespace();
                self.consume_attribute_value()?
            } else {
                String::new()
            };
            attributes.insert(attribute_name, value);
        };

        let element = Node::element(&name, attributes);
        if self_closing || is_void_element(&name) {
            self.append(element);
        } else {
            self.open_nodes.push(element);
        }
        Ok(())
    }

    fn parse_text(&mut self) {
        let end = self.source[self.position..]
            .find('<')
            .map_or(self.source.len(), |offset| self.position + offset);
        let value = &self.source[self.position..end];
        self.position = end;
        if !value.trim().is_empty() {
            self.append(Node::text(value.trim()));
        }
    }

    fn consume_attribute_value(&mut self) -> Result<String, ParseError> {
        let Some(first) = self.peek() else {
            return Err(ParseError::new(
                self.position,
                "expected an attribute value",
            ));
        };
        if first == '\'' || first == '"' {
            self.position += first.len_utf8();
            let start = self.position;
            while self.peek().is_some_and(|character| character != first) {
                self.advance();
            }
            if self.is_eof() {
                return Err(ParseError::new(start, "unterminated quoted attribute"));
            }
            let value = self.source[start..self.position].to_owned();
            self.advance();
            Ok(value)
        } else {
            let start = self.position;
            while self.peek().is_some_and(|character| {
                !character.is_whitespace() && character != '>' && character != '/'
            }) {
                self.advance();
            }
            if start == self.position {
                return Err(ParseError::new(start, "expected an attribute value"));
            }
            Ok(self.source[start..self.position].to_owned())
        }
    }

    fn consume_name(&mut self) -> &'a str {
        let start = self.position;
        while self.peek().is_some_and(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':')
        }) {
            self.advance();
        }
        &self.source[start..self.position]
    }

    fn append(&mut self, node: Node) {
        self.open_nodes
            .last_mut()
            .expect("document node exists")
            .children
            .push(node);
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.advance();
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), ParseError> {
        if self.peek() == Some(expected) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::new(
                self.position,
                format!("expected '{expected}'"),
            ))
        }
    }

    fn consume_if(&mut self, value: &str) -> bool {
        if self.starts_with(value) {
            self.position += value.len();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) {
        if let Some(character) = self.peek() {
            self.position += character.len_utf8();
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.position..].chars().next()
    }

    fn starts_with(&self, value: &str) -> bool {
        self.source[self.position..].starts_with(value)
    }

    fn starts_with_case_insensitive(&self, value: &str) -> bool {
        self.source[self.position..]
            .get(..value.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(value))
    }

    fn is_eof(&self) -> bool {
        self.position >= self.source.len()
    }
}

fn tag_name(node: &Node) -> &str {
    match &node.kind {
        crate::dom::NodeKind::Element(element) => &element.tag_name,
        _ => "#document",
    }
}

fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::NodeKind;

    #[test]
    fn parses_nested_elements_text_and_attributes() {
        let document =
            parse(r#"<!doctype html><main id="app"><h1>Hello</h1><p hidden>Ferrum</p></main>"#)
                .expect("valid HTML should parse");

        let main = &document.children[0];
        let NodeKind::Element(main_element) = &main.kind else {
            panic!("expected a main element");
        };
        assert_eq!(main_element.tag_name, "main");
        assert_eq!(main_element.attributes["id"], "app");
        assert_eq!(main.children.len(), 2);
        assert_eq!(
            main.children[0].children[0].kind,
            NodeKind::Text("Hello".into())
        );
    }

    #[test]
    fn supports_comments_void_elements_and_unicode() {
        let document =
            parse("<!-- greeting --><p>Hello, 世界<br>!</p>").expect("valid HTML should parse");
        assert!(matches!(document.children[0].kind, NodeKind::Comment(_)));
        assert_eq!(document.children[1].children.len(), 3);
    }

    #[test]
    fn reports_mismatched_closing_tags() {
        let error = parse("<main><p>text</main>").expect_err("invalid HTML should fail");
        assert!(error.message.contains("expected </p>"));
        assert_eq!(error.offset, 13);
    }

    #[test]
    fn pretty_print_is_deterministic() {
        let document = parse(r#"<button type="button" class="primary">Go</button>"#)
            .expect("valid HTML should parse");
        assert_eq!(
            document.pretty_print(),
            "#document\n  <button class=\"primary\" type=\"button\">\n    \"Go\"\n"
        );
    }
}
