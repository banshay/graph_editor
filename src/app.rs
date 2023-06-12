use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Deref;

use eframe::egui::{self, Context, DragValue, Key, Ui};
use eframe::{Frame, Storage};
use instant::Instant;
use log::info;

use egui_node_graph::*;

use crate::app::node::structs::*;
use crate::app::wzrd_node_graph::*;

mod node;
pub mod wzrd_node_graph;

use crate::app::node::WzrdNodes;
#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
extern "C" {
    #[wasm_bindgen(js_name = "updateDocument")]
    pub fn update_document(document: &str);
}

impl Default for WzrdValueType {
    fn default() -> Self {
        Self::Float { value: 0.0 }
    }
}

impl DataTypeTrait<WzrdGraphState> for WzrdNodeDataType {
    fn data_type_color(&self, user_state: &mut WzrdGraphState) -> ecolor::Color32 {
        match self {
            WzrdNodeDataType::Number => ecolor::Color32::from_rgb(0, 0, 255),
            WzrdNodeDataType::Any => ecolor::Color32::from_rgb(205, 205, 205),
        }
    }

    fn name(&self) -> Cow<str> {
        match self {
            WzrdNodeDataType::Number => Cow::Borrowed("number"),
            WzrdNodeDataType::Any => Cow::Borrowed("any"),
        }
    }
}

impl NodeTemplateTrait for WzrdNode {
    type NodeData = WzrdNodeData;
    type DataType = WzrdNodeDataType;
    type ValueType = WzrdValueType;
    type UserState = WzrdGraphState;

    fn node_finder_label(&self, user_state: &mut Self::UserState) -> Cow<str> {
        Cow::Borrowed(&self.label)
    }

    fn node_graph_label(&self, user_state: &mut Self::UserState) -> String {
        self.node_finder_label(user_state).into()
    }

    fn user_data(&self, user_state: &mut Self::UserState) -> Self::NodeData {
        WzrdNodeData {
            template: self.clone(),
        }
    }

    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
        node_id: NodeId,
    ) {
        for input in self.inputs.clone() {
            graph.add_input_param(
                node_id,
                input.name,
                input.data_type,
                input
                    .initial_value
                    .unwrap_or(WzrdValueType::Integer { value: 0 }),
                InputParamKind::ConnectionOnly,
                true,
            );
        }

        for output in self.outputs.clone() {
            graph.add_output_param(node_id, output.name, output.data_type);
        }
    }
}

impl WidgetValueTrait for WzrdValueType {
    type Response = WzrdResponse;
    type UserState = WzrdGraphState;
    type NodeData = WzrdNodeData;

    fn value_widget(
        &mut self,
        param_name: &str,
        node_id: NodeId,
        ui: &mut egui::Ui,
        user_state: &mut Self::UserState,
        node_data: &Self::NodeData,
    ) -> Vec<Self::Response> {
        match self {
            WzrdValueType::Integer { value } => {
                ui.label(param_name);
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(value));
                });
            }
            WzrdValueType::Float { value } => {
                ui.label(param_name);
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(value));
                });
            }
            WzrdValueType::String { value } => {
                ui.label(param_name);
            }
            _ => {}
        }

        Vec::new()
    }
}

impl UserResponseTrait for WzrdResponse {}

impl NodeDataTrait for WzrdNodeData {
    type Response = WzrdResponse;
    type UserState = WzrdGraphState;
    type DataType = WzrdNodeDataType;
    type ValueType = WzrdValueType;

    fn bottom_ui(
        &self,
        ui: &mut Ui,
        node_id: NodeId,
        graph: &Graph<Self, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
    ) -> Vec<NodeResponse<Self::Response, Self>>
    where
        WzrdResponse: UserResponseTrait,
    {
        let mut responses = vec![];

        responses
    }

    fn top_bar_ui(
        &self,
        ui: &mut Ui,
        node_id: NodeId,
        graph: &Graph<Self, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
    ) -> Vec<NodeResponse<Self::Response, Self>>
    where
        Self::Response: UserResponseTrait,
    {
        vec![]
    }

    fn output_ui(
        &self,
        ui: &mut Ui,
        _node_id: NodeId,
        _graph: &Graph<Self, Self::DataType, Self::ValueType>,
        _user_state: &mut Self::UserState,
        param_name: &str,
    ) -> Vec<NodeResponse<Self::Response, Self>>
    where
        Self::Response: UserResponseTrait,
    {
        ui.label(param_name);
        vec![]
    }

    fn can_delete(
        &self,
        _node_id: NodeId,
        _graph: &Graph<Self, Self::DataType, Self::ValueType>,
        _user_state: &mut Self::UserState,
    ) -> bool {
        false
    }
}

const EXTERNAL_UPDATE_COOLDOWN_MS: u128 = 1000;
impl eframe::App for WzrdNodeGraph {
    #[cfg(feature = "persistence")]
    fn save(&mut self, _storage: &mut dyn Storage) {
        eframe::set_value(_storage, PERSISTENCE_KEY, &self.state);
    }

    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
            })
        });

        let graph_response: WzrdGraphResponse = egui::CentralPanel::default()
            .show(ctx, |ui| {
                self.state
                    .draw_graph_editor(ui, self.node_templates.clone(), &mut self.user_state)
            })
            .inner;

        ctx.input(|i| {
            if i.key_released(Key::Delete) {
                for node_id in self.state.selected_nodes.iter() {
                    let (node, disc_events) = self.state.graph.remove_node(*node_id);
                    // Pass the disconnection responses first so user code can perform cleanup
                    // before node removal response.
                    self.state.node_positions.remove(*node_id);
                    // Make sure to not leave references to old nodes hanging
                    self.state.node_order.retain(|id| *id != *node_id);
                }
            }
        });

        ctx.input(|i| {
            if i.key_released(Key::I) {
                let mut cache: NodeCache = HashMap::new();
                let graph = self.evaluate_graph(&mut cache);
                info!("{graph}");
            }
        });

        if *self.format_requested.lock().unwrap() {
            *self.format_requested.lock().unwrap() = false;
            self.format_graph();
        }

        #[cfg(target_arch = "wasm32")]
        {
            fn call_external_update(graph: &mut WzrdNodeGraph) {
                let mut cache: NodeCache = HashMap::new();
                let graph = graph.evaluate_graph(&mut cache);
                update_document(&graph);
            }

            if graph_response.node_responses.len() > 0 || self.last_event.is_none() {
                self.last_event = Some(Instant::now());
            }

            if let (Some(last_update), Some(last_event_instant)) =
                (self.last_update, self.last_event)
            {
                let duration_since_last_update_ms =
                    Instant::now().duration_since(last_update).as_millis();
                match (
                    duration_since_last_update_ms.cmp(&EXTERNAL_UPDATE_COOLDOWN_MS),
                    last_event_instant.cmp(&last_update),
                ) {
                    (Ordering::Greater, Ordering::Greater) => {
                        info!(
                            "proper case: graph_responses size: {:}, duration since {:}",
                            graph_response.node_responses.len(),
                            duration_since_last_update_ms
                        );
                        self.last_update = Some(Instant::now());

                        call_external_update(self);
                    }
                    _ => {}
                };
            } else {
                self.last_update = Some(Instant::now());
                // call_external_update(self);
            }
        }
    }
}
