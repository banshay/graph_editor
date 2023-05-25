#[cfg(target_arch = "wasm32")]
mod app;

#[cfg(target_arch = "wasm32")]
pub use app::wzrd_node_graph::WzrdNodeGraph;

#[cfg(target_arch = "wasm32")]
use eframe::{
    egui::mutex::Mutex,
    wasm_bindgen::{self, prelude::*, JsValue},
    web::backend::AppRunnerRef,
    App,
};

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub struct WebHandle {
    handle: AppRunnerRef,
}

// #[wasm_bindgen]
// #[cfg(target_arch = "wasm32")]
// impl From<Arc<Mutex<AppRunner>>> for WebHandle {
//     fn from(value: Arc<Mutex<AppRunner>>) -> Self {
//         WebHandle {
//             handler: value
//         }
//     }
// }

// #[cfg(target_arch = "wasm32")]
// #[wasm_bindgen]
// pub async fn start(canvas_id: &str) -> Result<WebHandle, JsValue> {
//     Ok(WebHandle {
//         handle: eframe::start_web(
//             canvas_id,
//             eframe::WebOptions::default(),
//             Box::new(|cc| Box::new(WzrdNodeGraph::new(cc))),
//         )
//         .await?,
//     })
// }
