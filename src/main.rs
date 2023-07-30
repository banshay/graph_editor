use eframe::egui::Visuals;
#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};
use log::info;
use std::ops::Deref;

use crate::app::wzrd_node_graph::WzrdNodeGraph;

mod app;

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
extern "C" {
    #[wasm_bindgen(js_name = "getFileContents")]
    pub fn get_file_contents() -> String;
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();
    use eframe::egui::Visuals;

    let app = WzrdNodeGraph::new();
    let format_requested = app.format_requested.clone();

    *format_requested.lock().unwrap() = true;

    eframe::run_native(
        "Wzrd Node Graph",
        eframe::NativeOptions::default(),
        Box::new(|creation_context| {
            creation_context.egui_ctx.set_visuals(Visuals::dark());
            #[cfg(feature = "persistence")]
            {
                let mut wzrd_graph = Box::new(app);
                let mut graph = wzrd_graph.state.graph.clone();
                let mut user_state = wzrd_graph.user_state.clone();
                wzrd_graph.initialize_graph(
                    &mut graph,
                    &mut user_state,
                    // "
                    // def main(a)
                    //    return (48*(11+a))
                    // end
                    //                 ",
                    "
                    def main(a)
                        return (a%2==0) ? (5+5) : (48*(11+a))
                    end
                                        ",
                    //                     "
                    // def main
                    //    (15*7)
                    //    return (48*(11+20))
                    // end
                    //                 ",
                );
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

pub struct GraphWrapper(WzrdNodeGraph);

#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    let web_options = eframe::WebOptions::default();

    let wrapper = GraphWrapper(WzrdNodeGraph::new());
    let format_requested = wrapper.0.format_requested.clone();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "WzrdGraphEditor",
            web_options,
            Box::new(|cc| {
                let mut wzrd_graph = Box::new(wrapper.0);
                let mut graph = wzrd_graph.state.graph.clone();
                let mut user_state = wzrd_graph.user_state.clone();
                wzrd_graph.initialize_graph(&mut graph, &mut user_state, &get_file_contents());
                wzrd_graph.user_state = user_state;
                wzrd_graph.state.graph = graph;

                wzrd_graph
            }),
        )
        .await
        .expect("failed to start eframe");
    });

    *format_requested.lock().unwrap() = true;
}
