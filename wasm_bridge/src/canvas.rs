use wasm_bindgen::JsCast;
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, OffscreenCanvas, OffscreenCanvasRenderingContext2d,
};

use crate::dom::document;

pub struct CanvasElement {
    inner: HtmlCanvasElement,
}

pub struct OffscreenCanvasElement {
    inner: OffscreenCanvas,
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

    pub fn set_size(&self, width: u32, height: u32) {
        self.inner.set_width(width);
        self.inner.set_height(height);
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

    pub fn to_data_url(&self) -> String {
        self.inner.to_data_url().unwrap_or_default()
    }
}

impl OffscreenCanvasElement {
    pub fn new(width: u32, height: u32) -> Self {
        let inner = OffscreenCanvas::new(width, height).expect("Failed to create OffscreenCanvas");
        Self { inner }
    }

    pub fn set_size(&self, width: u32, height: u32) {
        self.inner.set_width(width);
        self.inner.set_height(height);
    }

    pub fn get_context_2d(&self) -> Option<OffscreenCanvasRenderingContext2d> {
        let context = self
            .inner
            .get_context("2d")
            .ok()??
            .dyn_into::<OffscreenCanvasRenderingContext2d>()
            .ok()?;
        Some(context)
    }

    pub fn transfer_to_image_bitmap(&self) -> Result<web_sys::ImageBitmap, wasm_bindgen::JsValue> {
        self.inner.transfer_to_image_bitmap()
    }

    pub fn convert_to_blob(&self) -> js_sys::Promise {
        self.inner
            .convert_to_blob()
            .expect("Failed to convert to blob")
    }

    pub fn inner(&self) -> &OffscreenCanvas {
        &self.inner
    }
}
