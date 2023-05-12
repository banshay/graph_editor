#[cfg(target_arch = "wasm32")]
mod app;

#[cfg(target_arch = "wasm32")]
pub use app::WzrdNodeGraph;

#[cfg(target_arch = "wasm32")]
use eframe::{
    wasm_bindgen::{self, prelude::*, JsValue},
    web::backend::AppRunnerRef,
    App,
};

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub struct WebHandle {
    handle: AppRunnerRef,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<WebHandle, JsValue> {
    eframe::start_web(
        canvas_id,
        eframe::WebOptions::default(),
        Box::new(|cc| Box::new(WzrdNodeGraph::new(cc))),
    )
    .map(|handle| WebHandle { handle })
}
