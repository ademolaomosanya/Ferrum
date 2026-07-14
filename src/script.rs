//! A small JavaScript host bridge for Ferrum pages.
//!
//! This is intentionally not a Web IDL or full DOM implementation. It exposes
//! the first useful scripting surface while keeping the boundary explicit.

use boa_engine::{Context, Source};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageState {
    pub title: String,
    pub elements: BTreeMap<String, ElementState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementState {
    pub text_content: String,
    pub background: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptOutcome {
    pub page: PageState,
    pub result: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEvent {
    pub event_type: String,
    pub target_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    pub message: String,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "JavaScript error: {}", self.message)
    }
}

impl Error for ScriptError {}

pub fn execute(source: &str, initial: PageState) -> Result<ScriptOutcome, ScriptError> {
    execute_with_event(source, initial, None)
}

pub fn execute_with_event(
    source: &str,
    initial: PageState,
    event: Option<&ScriptEvent>,
) -> Result<ScriptOutcome, ScriptError> {
    let elements = initial
        .elements
        .iter()
        .map(|(id, element)| {
            format!(
                "{}: {{ id: {}, textContent: {}, style: {{ background: {} }} }}",
                js_string(id),
                js_string(id),
                js_string(&element.text_content),
                js_string(&element.background),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let event = event.map_or_else(
        || "null".to_owned(),
        |event| {
            format!(
                "{{ type: {}, target: __ferrumElements[{}] || null }}",
                js_string(&event.event_type),
                js_string(&event.target_id),
            )
        },
    );
    let prelude = format!(
        r##"
        const __ferrumElements = {{ {} }};
        globalThis.document = {{
            title: {},
            readyState: "complete",
            __elements: __ferrumElements,
            querySelector(selector) {{
                return typeof selector === "string" && selector.startsWith("#")
                    ? this.__elements[selector.slice(1)] || null
                    : null;
            }},
            getElementById(id) {{
                return this.__elements[String(id)] || null;
            }}
        }};
        globalThis.window = globalThis;
        globalThis.event = {};
        "##,
        elements,
        js_string(&initial.title),
        event,
    );

    let mut context = Context::default();
    context
        .eval(Source::from_bytes(&prelude))
        .map_err(script_error)?;
    let result = context
        .eval(Source::from_bytes(source))
        .map_err(script_error)?;
    context.run_jobs().map_err(script_error)?;

    let result = result
        .to_string(&mut context)
        .map_err(script_error)?
        .to_std_string_escaped();
    let mut updated_elements = BTreeMap::new();
    for id in initial.elements.keys() {
        let key = js_string(id);
        updated_elements.insert(
            id.clone(),
            ElementState {
                text_content: evaluate_string(
                    &mut context,
                    &format!("String(document.__elements[{key}].textContent)"),
                )?,
                background: evaluate_string(
                    &mut context,
                    &format!("String(document.__elements[{key}].style.background || '')"),
                )?,
            },
        );
    }
    Ok(ScriptOutcome {
        page: PageState {
            title: evaluate_string(&mut context, "String(document.title)")?,
            elements: updated_elements,
        },
        result,
    })
}

fn evaluate_string(context: &mut Context, expression: &str) -> Result<String, ScriptError> {
    context
        .eval(Source::from_bytes(expression))
        .map_err(script_error)?
        .to_string(context)
        .map_err(script_error)
        .map(|value| value.to_std_string_escaped())
}

fn script_error(error: impl fmt::Display) -> ScriptError {
    ScriptError {
        message: error.to_string(),
    }
}

fn js_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{2028}' => output.push_str("\\u2028"),
            '\u{2029}' => output.push_str("\\u2029"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> PageState {
        let mut elements = BTreeMap::new();
        elements.insert(
            "app".into(),
            ElementState {
                text_content: "Initial text".into(),
                background: String::new(),
            },
        );
        elements.insert(
            "status".into(),
            ElementState {
                text_content: "Waiting".into(),
                background: String::new(),
            },
        );
        PageState {
            title: "Before".into(),
            elements,
        }
    }

    #[test]
    fn executes_javascript_and_returns_dom_mutations() {
        let outcome = execute(
            r##"
                document.title = "After";
                const app = document.querySelector("#app");
                app.textContent = "JavaScript rendered this";
                app.style.background = "#224466";
                document.getElementById("status").textContent = "Ready";
                6 * 7;
            "##,
            page(),
        )
        .expect("valid JavaScript should execute");

        assert_eq!(outcome.result, "42");
        assert_eq!(outcome.page.title, "After");
        assert_eq!(
            outcome.page.elements["app"].text_content,
            "JavaScript rendered this"
        );
        assert_eq!(outcome.page.elements["app"].background, "#224466");
        assert_eq!(outcome.page.elements["status"].text_content, "Ready");
    }

    #[test]
    fn reports_javascript_syntax_errors() {
        let error = execute("const = ;", page()).expect_err("invalid JavaScript should fail");
        assert!(error.to_string().contains("JavaScript error"));
    }

    #[test]
    fn dispatches_a_click_event_to_javascript() {
        let event = ScriptEvent {
            event_type: "click".into(),
            target_id: "status".into(),
        };
        let outcome = execute_with_event(
            "if (event && event.target.id === 'status') { event.target.textContent = 'Clicked'; }",
            page(),
            Some(&event),
        )
        .expect("click handler should execute");
        assert_eq!(outcome.page.elements["status"].text_content, "Clicked");
    }

    #[test]
    fn safely_embeds_page_strings_in_the_runtime() {
        let initial = PageState {
            title: "quotes \" and \\ slashes".into(),
            elements: BTreeMap::from([(
                "app".into(),
                ElementState {
                    text_content: "line one\nline two".into(),
                    background: String::new(),
                },
            )]),
        };
        let outcome = execute("document.title", initial.clone())
            .expect("escaped initial state should execute");
        assert_eq!(outcome.page.title, initial.title);
        assert_eq!(outcome.page.elements, initial.elements);
    }
}
