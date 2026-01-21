use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::Window;

pub fn document() -> web_sys::Document {
    Window::get().document().expect("Document object not found")
}

pub struct Element {
    inner: web_sys::Element,
}

pub struct CanvasElement {
    inner: web_sys::HtmlCanvasElement,
}

impl Element {
    pub fn create(tag: &str) -> Self {
        let inner = document()
            .create_element(tag)
            .expect("Failed to create element");
        Self { inner }
    }

    pub fn by_id(id: &str) -> Option<Self> {
        let doc = document();
        let inner = doc.get_element_by_id(id)?;
        Some(Self { inner })
    }

    pub fn select(selector: &str) -> Option<Self> {
        let doc = document();
        let inner = doc.query_selector(selector).ok().flatten()?;
        Some(Self { inner })
    }

    fn html_element(&self) -> Option<web_sys::HtmlElement> {
        self.inner.dyn_ref::<web_sys::HtmlElement>().cloned()
    }

    pub fn set_inner_html(&self, content: &str) {
        self.inner.set_inner_html(content);
    }

    pub fn add_class(&self, class: &str) {
        let _ = self.inner.class_list().add_1(class);
    }

    pub fn remove_class(&self, class: &str) {
        let _ = self.inner.class_list().remove_1(class);
    }

    /// returns true if the class is now present, false if it was removed
    pub fn toggle_class(&self, class: &str) -> bool {
        self.inner.class_list().toggle(class).unwrap_or(false)
    }

    pub fn set_style(&self, property: &str, value: &str) {
        let style = self
            .html_element()
            .expect("Element is not an HtmlElement")
            .style();
        style.set_property(property, value).ok();
    }

    pub fn get_style(&self, property: &str) -> Option<String> {
        self.html_element()?
            .style()
            .get_property_value(property)
            .ok()
    }

    pub fn set_attribute(&self, name: &str, value: &str) {
        let _ = self.inner.set_attribute(name, value);
    }

    pub fn get_attribute(&self, name: &str) -> Option<String> {
        self.inner.get_attribute(name)
    }

    pub fn append(&self, child: &Element) {
        let _ = self.inner.append_child(&child.inner);
    }

    pub fn prepend(&self, child: &Element) {
        let _ = self.inner.prepend_with_node_1(&child.inner);
    }

    pub fn remove(&self) {
        self.inner.remove();
    }

    pub fn rect(&self) -> (f64, f64, f64, f64) {
        let rect = self.inner.get_bounding_client_rect();
        (rect.x(), rect.y(), rect.width(), rect.height())
    }

    pub fn first_child(&self) -> Option<Element> {
        self.inner
            .first_element_child()
            .map(|inner| Element { inner })
    }

    pub fn last_child(&self) -> Option<Element> {
        self.inner
            .last_element_child()
            .map(|inner| Element { inner })
    }

    pub fn next_sibling(&self) -> Option<Element> {
        self.inner
            .next_element_sibling()
            .map(|inner| Element { inner })
    }
}

impl CanvasElement {
    pub fn create(width: u32, height: u32) -> Self {
        let canvas = document()
            .create_element("canvas")
            .expect("Failed to create canvas element")
            .dyn_into::<HtmlCanvasElement>()
            .expect("Created element is not a canvas");

        canvas.set_width(width);
        canvas.set_height(height);
        Self { inner: canvas }
    }

    pub fn get_context_2d(&self) -> Option<CanvasRenderingContext2d> {
        let context = self
            .inner
            .get_context("2d")
            .ok()??
            .dyn_into::<CanvasRenderingContext2d>()
            .ok()?;
        Some(context)
    }
}

pub struct ElementCollection {
    elements: Vec<Element>,
}

impl ElementCollection {
    pub fn select(selector: &str) -> Self {
        let doc = document();
        let Some(node_list) = doc.query_selector_all(selector).ok() else {
            return Self {
                elements: Vec::new(),
            };
        };

        let mut elements = Vec::new();
        elements.reserve_exact(node_list.length() as usize);
        for i in 0..node_list.length() {
            if let Some(node) = node_list.item(i)
                && let Ok(elem) = node.dyn_into::<web_sys::Element>()
            {
                elements.push(Element { inner: elem });
            }
        }

        Self { elements }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Element> {
        self.elements.iter()
    }
}
