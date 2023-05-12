use crate::app::WzrdNodeGraph;
use eframe::egui::Visuals;

mod app;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use eframe::egui::Visuals;

    eframe::run_native(
        "Wzrd Node Graph",
        eframe::NativeOptions::default(),
        Box::new(|creation_context| {
            creation_context.egui_ctx.set_visuals(Visuals::dark());
            #[cfg(feature = "persistence")]
            {
                Box::new(WzrdNodeGraph::new(creation_context))
            }

            #[cfg(not(feature = "persistence"))]
            Box::new(WzrdNodeGraph::default())
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "WzrdGraphEditor",
            web_options,
            Box::new(|cc| Box::new(WzrdNodeGraph::new(cc))),
        )
        // .await
        .expect("failed to start eframe");
    });
}
