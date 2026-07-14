//! CSS data types and a parser for Ferrum's first styling milestone.
//!
//! The parser intentionally supports a useful subset of CSS rather than the
//! complete CSS Syntax specification. Its public model is designed to feed the
//! style system without coupling selector matching to parsing.

use std::error::Error;
use std::fmt::{self, Write};

#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

impl Stylesheet {
    /// Produces a normalized stylesheet representation for inspection.
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        for (rule_index, rule) in self.rules.iter().enumerate() {
            if rule_index > 0 {
                output.push('\n');
            }
            for (selector_index, selector) in rule.selectors.iter().enumerate() {
                if selector_index > 0 {
                    output.push_str(", ");
                }
                write!(output, "{selector}").expect("writing to a String cannot fail");
            }
            output.push_str(" {\n");
            for declaration in &rule.declarations {
                writeln!(output, "  {}: {};", declaration.name, declaration.value)
                    .expect("writing to a String cannot fail");
            }
            output.push_str("}\n");
        }
        output
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    Simple(SimpleSelector),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
}

/// `(id selectors, class selectors, type selectors)`.
pub type Specificity = (usize, usize, usize);

impl Selector {
    pub fn specificity(&self) -> Specificity {
        match self {
            Self::Simple(selector) => (
                usize::from(selector.id.is_some()),
                selector.classes.len(),
                usize::from(selector.tag_name.is_some()),
            ),
        }
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simple(selector) => {
                if let Some(tag_name) = &selector.tag_name {
                    formatter.write_str(tag_name)?;
                } else if selector.id.is_none() && selector.classes.is_empty() {
                    formatter.write_str("*")?;
                }
                if let Some(id) = &selector.id {
                    write!(formatter, "#{id}")?;
                }
                for class in &selector.classes {
                    write!(formatter, ".{class}")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Length(f32, Unit),
    Color(Color),
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keyword(keyword) => formatter.write_str(keyword),
            Self::Length(length, Unit::Px) => write!(formatter, "{length}px"),
            Self::Color(color) => write!(
                formatter,
                "#{:02x}{:02x}{:02x}{:02x}",
                color.red, color.green, color.blue, color.alpha
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Px,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

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
            "CSS parse error at byte {}: {}",
            self.offset, self.message
        )
    }
}

impl Error for ParseError {}

pub fn parse(source: &str) -> Result<Stylesheet, ParseError> {
    Parser::new(source).parse_stylesheet()
}

struct Parser<'a> {
    source: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    fn parse_stylesheet(mut self) -> Result<Stylesheet, ParseError> {
        let mut rules = Vec::new();
        self.skip_ignored()?;
        while !self.is_eof() {
            rules.push(self.parse_rule()?);
            self.skip_ignored()?;
        }
        Ok(Stylesheet { rules })
    }

    fn parse_rule(&mut self) -> Result<Rule, ParseError> {
        let selectors = self.parse_selectors()?;
        self.skip_ignored()?;
        self.expect('{')?;
        let declarations = self.parse_declarations()?;
        Ok(Rule {
            selectors,
            declarations,
        })
    }

    fn parse_selectors(&mut self) -> Result<Vec<Selector>, ParseError> {
        let mut selectors = Vec::new();
        loop {
            self.skip_ignored()?;
            selectors.push(Selector::Simple(self.parse_simple_selector()?));
            self.skip_ignored()?;
            match self.peek() {
                Some(',') => {
                    self.advance();
                    self.skip_ignored()?;
                    if self.peek() == Some('{') || self.is_eof() {
                        return Err(ParseError::new(
                            self.position,
                            "expected a selector after ','",
                        ));
                    }
                }
                Some('{') => break,
                Some(character) => {
                    return Err(ParseError::new(
                        self.position,
                        format!("unsupported selector syntax near '{character}'"),
                    ));
                }
                None => {
                    return Err(ParseError::new(
                        self.position,
                        "expected '{' after selector",
                    ));
                }
            }
        }
        Ok(selectors)
    }

    fn parse_simple_selector(&mut self) -> Result<SimpleSelector, ParseError> {
        let start = self.position;
        let mut selector = SimpleSelector {
            tag_name: None,
            id: None,
            classes: Vec::new(),
        };

        if self.peek() == Some('*') {
            self.advance();
        } else if self.peek().is_some_and(is_identifier_start) {
            selector.tag_name = Some(self.consume_identifier()?.to_ascii_lowercase());
        }

        loop {
            match self.peek() {
                Some('#') => {
                    self.advance();
                    if selector.id.is_some() {
                        return Err(ParseError::new(
                            self.position,
                            "a simple selector can contain only one ID",
                        ));
                    }
                    selector.id = Some(self.consume_identifier()?);
                }
                Some('.') => {
                    self.advance();
                    selector.classes.push(self.consume_identifier()?);
                }
                _ => break,
            }
        }

        if start == self.position {
            return Err(ParseError::new(start, "expected a selector"));
        }
        Ok(selector)
    }

    fn parse_declarations(&mut self) -> Result<Vec<Declaration>, ParseError> {
        let mut declarations = Vec::new();
        loop {
            self.skip_ignored()?;
            if self.consume_if("}") {
                return Ok(declarations);
            }
            if self.is_eof() {
                return Err(ParseError::new(
                    self.position,
                    "expected '}' after declarations",
                ));
            }

            let name = self.consume_identifier()?.to_ascii_lowercase();
            self.skip_ignored()?;
            self.expect(':')?;
            self.skip_ignored()?;
            let value = self.parse_value()?;
            declarations.push(Declaration { name, value });
            self.skip_ignored()?;

            if self.consume_if(";") {
                continue;
            }
            if self.is_eof() {
                return Err(ParseError::new(
                    self.position,
                    "expected '}' after declarations",
                ));
            }
            if self.peek() != Some('}') {
                return Err(ParseError::new(
                    self.position,
                    "expected ';' or '}' after declaration",
                ));
            }
        }
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        let start = self.position;
        let mut raw = String::new();
        let mut segment_start = self.position;
        while self
            .peek()
            .is_some_and(|character| character != ';' && character != '}')
        {
            if self.starts_with("/*") {
                raw.push_str(&self.source[segment_start..self.position]);
                self.consume_comment()?;
                segment_start = self.position;
            } else {
                self.advance();
            }
        }
        raw.push_str(&self.source[segment_start..self.position]);

        let raw = raw.trim();
        if raw.is_empty() {
            return Err(ParseError::new(start, "expected a property value"));
        }
        parse_typed_value(raw).map_err(|message| ParseError::new(start, message))
    }

    fn consume_identifier(&mut self) -> Result<String, ParseError> {
        let start = self.position;
        if !self.peek().is_some_and(is_identifier_start) {
            return Err(ParseError::new(start, "expected an identifier"));
        }
        self.advance();
        while self.peek().is_some_and(is_identifier_character) {
            self.advance();
        }
        Ok(self.source[start..self.position].to_owned())
    }

    fn skip_ignored(&mut self) -> Result<(), ParseError> {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.advance();
            }
            if self.starts_with("/*") {
                self.consume_comment()?;
            } else {
                return Ok(());
            }
        }
    }

    fn consume_comment(&mut self) -> Result<(), ParseError> {
        let start = self.position;
        self.position += 2;
        let Some(end) = self.source[self.position..].find("*/") else {
            return Err(ParseError::new(start, "unterminated comment"));
        };
        self.position += end + 2;
        Ok(())
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

    fn is_eof(&self) -> bool {
        self.position >= self.source.len()
    }
}

fn parse_typed_value(raw: &str) -> Result<Value, String> {
    if let Some(hex) = raw.strip_prefix('#') {
        return parse_color(hex).map(Value::Color);
    }
    if let Some(number) = raw.strip_suffix("px") {
        let value = number
            .trim()
            .parse::<f32>()
            .map_err(|_| format!("invalid pixel length '{raw}'"))?;
        if !value.is_finite() {
            return Err(format!("invalid pixel length '{raw}'"));
        }
        return Ok(Value::Length(value, Unit::Px));
    }
    Ok(Value::Keyword(raw.to_owned()))
}

fn parse_color(hex: &str) -> Result<Color, String> {
    let expanded;
    let digits = match hex.len() {
        3 | 4 => {
            expanded = hex
                .chars()
                .flat_map(|character| [character, character])
                .collect::<String>();
            expanded.as_str()
        }
        6 | 8 => hex,
        _ => return Err(format!("invalid hex color '#{hex}'")),
    };

    let channel = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&digits[range], 16).map_err(|_| format!("invalid hex color '#{hex}'"))
    };
    Ok(Color {
        red: channel(0..2)?,
        green: channel(2..4)?,
        blue: channel(4..6)?,
        alpha: if digits.len() == 8 {
            channel(6..8)?
        } else {
            255
        },
    })
}

fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || matches!(character, '_' | '-')
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rules_selectors_and_declarations() {
        let stylesheet = parse(
            r#"
                main#app.shell, .fallback {
                    display: block;
                    width: 320px;
                    color: #1a2b3c;
                }
            "#,
        )
        .expect("valid CSS should parse");

        assert_eq!(stylesheet.rules.len(), 1);
        let rule = &stylesheet.rules[0];
        assert_eq!(rule.selectors.len(), 2);
        assert_eq!(rule.declarations.len(), 3);
        assert_eq!(rule.declarations[1].value, Value::Length(320.0, Unit::Px));
        assert_eq!(
            rule.declarations[2].value,
            Value::Color(Color {
                red: 26,
                green: 43,
                blue: 60,
                alpha: 255,
            })
        );
    }

    #[test]
    fn calculates_selector_specificity() {
        let stylesheet = parse("article#story.featured.lead { display: block }")
            .expect("valid CSS should parse");
        assert_eq!(stylesheet.rules[0].selectors[0].specificity(), (1, 2, 1));
    }

    #[test]
    fn parses_short_and_alpha_hex_colors() {
        assert_eq!(
            parse_typed_value("#0f08"),
            Ok(Value::Color(Color {
                red: 0,
                green: 255,
                blue: 0,
                alpha: 136,
            }))
        );
        assert_eq!(
            parse_typed_value("#11223344"),
            Ok(Value::Color(Color {
                red: 17,
                green: 34,
                blue: 51,
                alpha: 68,
            }))
        );
    }

    #[test]
    fn ignores_comments_and_accepts_a_trailing_semicolon() {
        let stylesheet = parse("/* theme */ * { color: /* ink */ na/* split */vy; }")
            .expect("valid CSS should parse");
        assert_eq!(
            stylesheet.rules[0].declarations[0].value,
            Value::Keyword("navy".into())
        );
    }

    #[test]
    fn reports_an_unclosed_rule() {
        let error = parse("p { color: red").expect_err("invalid CSS should fail");
        assert!(error.message.contains("expected '}'"));
    }

    #[test]
    fn pretty_print_normalizes_a_stylesheet() {
        let stylesheet =
            parse("h1, .title { color: #abc; width: 24px }").expect("valid CSS should parse");
        assert_eq!(
            stylesheet.pretty_print(),
            "h1, .title {\n  color: #aabbccff;\n  width: 24px;\n}\n"
        );
    }

    #[test]
    fn rejects_unsupported_descendant_selectors() {
        let error = parse("main p { color: red }").expect_err("unsupported CSS should fail");
        assert!(error.message.contains("unsupported selector syntax"));
    }
}
