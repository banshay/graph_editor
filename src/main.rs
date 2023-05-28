use eframe::egui::Visuals;

use crate::app::wzrd_node_graph::WzrdNodeGraph;

mod app;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();
    use eframe::egui::Visuals;

    eframe::run_native(
        "Wzrd Node Graph",
        eframe::NativeOptions::default(),
        Box::new(|creation_context| {
            creation_context.egui_ctx.set_visuals(Visuals::dark());
            #[cfg(feature = "persistence")]
            {
                let mut wzrd_graph = Box::new(WzrdNodeGraph::default());
                let mut graph = wzrd_graph.state.graph.clone();
                let mut user_state = wzrd_graph.user_state.clone();
                wzrd_graph.initialize_graph(&mut graph, &mut user_state, "(48*(11+20))");
                wzrd_graph.user_state = user_state;
                wzrd_graph.state.graph = graph;
                wzrd_graph
            }

            #[cfg(not(feature = "persistence"))]
            Box::new(WzrdNodeGraph::default())
        }),
    )
    .unwrap();
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
            Box::new(|cc| Box::new(WzrdNodeGraph::default())),
        )
        .await
        .expect("failed to start eframe");
    });
}
