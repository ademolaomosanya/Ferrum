//! End-to-end page loading and persistent interaction sessions.

use crate::css::Stylesheet;
use crate::dom::{Node, NodeKind};
use crate::layout::{LayoutBox, Rect};
use crate::paint::Canvas;
use crate::script::{ElementState, PageState, ScriptRuntime};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedPage {
    pub title: String,
    pub script_result: String,
    pub canvas: Canvas,
    pub hit_regions: Vec<HitRegion>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HitRegion {
    pub element_id: String,
    pub rect: Rect,
}

impl RenderedPage {
    /// Returns the frontmost ID-bearing element at window coordinates.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&str> {
        self.hit_regions
            .iter()
            .rev()
            .find(|region| contains(region.rect, x, y))
            .map(|region| region.element_id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserError {
    pub message: String,
}

impl fmt::Display for BrowserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for BrowserError {}

/// Owns a parsed page and its long-lived JavaScript context.
pub struct BrowserSession {
    document: Node,
    stylesheet: Stylesheet,
    initial_state: PageState,
    event_paths: BTreeMap<String, Vec<String>>,
    runtime: ScriptRuntime,
    width: u32,
    height: u32,
}

impl BrowserSession {
    pub fn new(
        html_source: &str,
        css_source: &str,
        script_source: &str,
        width: u32,
        height: u32,
    ) -> Result<Self, BrowserError> {
        let document = crate::html::parse(html_source).map_err(error)?;
        let stylesheet = crate::css::parse(css_source).map_err(error)?;
        let mut elements = BTreeMap::new();
        let mut event_paths = BTreeMap::new();
        collect_elements_and_paths(&document, &[], &mut elements, &mut event_paths);
        let initial_state = PageState {
            title: find_element(&document, "title")
                .map(text_content)
                .unwrap_or_else(|| "Ferrum".into()),
            elements,
        };
        let runtime = ScriptRuntime::new(script_source, &initial_state).map_err(error)?;
        Ok(Self {
            document,
            stylesheet,
            initial_state,
            event_paths,
            runtime,
            width,
            height,
        })
    }

    pub fn render(&mut self) -> Result<RenderedPage, BrowserError> {
        let outcome = self.runtime.outcome().map_err(error)?;
        let mut document = self.document.clone();
        let mut stylesheet = self.stylesheet.clone();

        for (id, updated) in &outcome.page.elements {
            let Some(original) = self.initial_state.elements.get(id) else {
                continue;
            };
            if updated.text_content != original.text_content
                && let Some(element) = find_element_by_id_mut(&mut document, id)
            {
                element.children = vec![Node::text(&updated.text_content)];
            }
            if updated.background != original.background && !updated.background.trim().is_empty() {
                let script_styles =
                    crate::css::parse(&format!("#{id} {{ background: {}; }}", updated.background))
                        .map_err(|parse_error| BrowserError {
                            message: format!(
                                "JavaScript produced invalid background CSS: {parse_error}"
                            ),
                        })?;
                stylesheet.rules.extend(script_styles.rules);
            }
        }

        let styled = crate::style::style_tree(&document, &stylesheet);
        let layout = crate::layout::layout_tree(&styled, self.width as f32, self.height as f32);
        let mut hit_regions = Vec::new();
        collect_hit_regions(&layout, &mut hit_regions);
        Ok(RenderedPage {
            title: outcome.page.title,
            script_result: outcome.result,
            canvas: crate::paint::paint(&layout, self.width, self.height),
            hit_regions,
        })
    }

    /// Dispatches a click to the target and its ID-bearing ancestors, then repaints.
    pub fn click(&mut self, target_id: &str) -> Result<RenderedPage, BrowserError> {
        let path = self
            .event_paths
            .get(target_id)
            .cloned()
            .unwrap_or_else(|| vec![target_id.to_owned()]);
        self.runtime
            .dispatch_click(target_id, &path)
            .map_err(error)?;
        self.render()
    }
}

pub fn render_with_script(
    html_source: &str,
    css_source: &str,
    script_source: &str,
    width: u32,
    height: u32,
) -> Result<RenderedPage, BrowserError> {
    BrowserSession::new(html_source, css_source, script_source, width, height)?.render()
}

fn collect_elements_and_paths(
    node: &Node,
    ancestors: &[String],
    elements: &mut BTreeMap<String, ElementState>,
    paths: &mut BTreeMap<String, Vec<String>>,
) {
    let mut child_ancestors = ancestors.to_vec();
    if let NodeKind::Element(element) = &node.kind
        && let Some(id) = element.attributes.get("id")
    {
        elements.insert(
            id.clone(),
            ElementState {
                text_content: text_content(node),
                background: String::new(),
            },
        );
        let mut path = vec![id.clone()];
        path.extend(ancestors.iter().rev().cloned());
        paths.insert(id.clone(), path);
        child_ancestors.push(id.clone());
    }
    for child in &node.children {
        collect_elements_and_paths(child, &child_ancestors, elements, paths);
    }
}

fn collect_hit_regions(layout: &LayoutBox, regions: &mut Vec<HitRegion>) {
    if let Some(element_id) = &layout.element_id {
        regions.push(HitRegion {
            element_id: element_id.clone(),
            rect: layout.dimensions.border_box(),
        });
    }
    for child in &layout.children {
        collect_hit_regions(child, regions);
    }
}

fn contains(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

fn find_element<'a>(node: &'a Node, tag_name: &str) -> Option<&'a Node> {
    if matches!(
        &node.kind,
        NodeKind::Element(element) if element.tag_name == tag_name
    ) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_element(child, tag_name))
}

fn find_element_by_id_mut<'a>(node: &'a mut Node, id: &str) -> Option<&'a mut Node> {
    if matches!(
        &node.kind,
        NodeKind::Element(element)
            if element.attributes.get("id").is_some_and(|value| value == id)
    ) {
        return Some(node);
    }
    node.children
        .iter_mut()
        .find_map(|child| find_element_by_id_mut(child, id))
}

fn text_content(node: &Node) -> String {
    match &node.kind {
        NodeKind::Text(value) => value.clone(),
        _ => node.children.iter().map(text_content).collect(),
    }
}

fn error(error: impl fmt::Display) -> BrowserError {
    BrowserError {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::Color;

    #[test]
    fn javascript_changes_reach_the_rendered_page() {
        let page = render_with_script(
            "<html><head><title>Before</title></head><body><main id=\"app\">Old</main></body></html>",
            "html { background: #fff } #app { width: 40px; height: 40px }",
            "document.title = 'After'; document.querySelector('#app').style.background = '#123456';",
            100,
            100,
        )
        .expect("page should render");

        assert_eq!(page.title, "After");
        assert!(page.canvas.pixels.contains(&Color {
            red: 0x12,
            green: 0x34,
            blue: 0x56,
            alpha: 255,
        }));
    }

    #[test]
    fn invalid_script_stops_page_loading() {
        let error = BrowserSession::new("<main id=\"app\"></main>", "", "const = ;", 10, 10)
            .err()
            .expect("invalid script should fail");
        assert!(error.message.contains("JavaScript error"));
    }

    #[test]
    fn hit_testing_dispatches_and_preserves_click_state() {
        let html = "<main id=\"app\"><p id=\"status\">Waiting</p></main>";
        let css = "#app { width: 100px; height: 60px } #status { height: 20px }";
        let script = r#"
            let clicks = 0;
            const status = document.getElementById('status');
            status.addEventListener('click', event => {
                status.textContent = String(++clicks);
                event.target.style.background = '#ff0000';
            });
        "#;
        let mut session =
            BrowserSession::new(html, css, script, 120, 80).expect("session should start");
        let initial = session.render().expect("page should render");
        let status = initial
            .hit_regions
            .iter()
            .find(|region| region.element_id == "status")
            .expect("status should have a hit region");
        let clicked = initial
            .hit_test(status.rect.x + 1.0, status.rect.y + 1.0)
            .expect("status should be hit")
            .to_owned();
        session
            .click(&clicked)
            .expect("first click should rerender");
        let updated = session
            .click(&clicked)
            .expect("second click should rerender");

        assert!(updated.canvas.pixels.contains(&Color {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        }));
        assert_eq!(
            session.runtime.outcome().unwrap().page.elements["status"].text_content,
            "2"
        );
    }

    #[test]
    fn click_bubbles_through_dom_ancestors_only() {
        let html =
            "<main id=\"app\"><p id=\"status\">Waiting</p><aside id=\"other\"></aside></main>";
        let script = r#"
            const status = document.getElementById('status');
            const app = document.getElementById('app');
            const other = document.getElementById('other');
            status.addEventListener('click', () => status.textContent = 'target');
            app.addEventListener('click', () => status.textContent += '>parent');
            other.addEventListener('click', () => status.textContent = 'wrong');
        "#;
        let mut session = BrowserSession::new(html, "", script, 100, 100).unwrap();
        session.click("status").unwrap();

        assert_eq!(
            session.runtime.outcome().unwrap().page.elements["status"].text_content,
            "target>parent"
        );
    }
}
