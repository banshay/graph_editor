use eframe::egui::{self, Context, DragValue, TextStyle, Ui};
use eframe::epaint::{ahash::HashMap, Color32};
use eframe::{epaint, Frame, Storage};
use egui_node_graph::*;
use std::any::Any;
use std::borrow::Cow;

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct WzrdNodeData {
    template: WzrdNodeTemplate,
}

#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum WzrdNodeDataType {
    Object,
    String,
    Function,
    Number,
    Any,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum WzrdValueType {
    Scalar { value: f64 },
    // Object { value: Option<dyn Any> },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
}

impl Default for WzrdValueType {
    fn default() -> Self {
        Self::Scalar { value: 0.0 }
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum WzrdNodeTemplate {
    Addition,
    Integer,
    Float,
    Output,
}

#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct WzrdGraphState {}

type WzrdGraph = Graph<WzrdNodeData, WzrdNodeDataType, WzrdValueType>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WzrdResponse {}

impl DataTypeTrait<WzrdGraphState> for WzrdNodeDataType {
    fn data_type_color(&self, user_state: &mut WzrdGraphState) -> epaint::color::Color32 {
        match self {
            WzrdNodeDataType::Object => egui::Color32::from_rgb(255, 0, 0),
            WzrdNodeDataType::String => egui::Color32::from_rgb(255, 0, 255),
            WzrdNodeDataType::Function => egui::Color32::from_rgb(0, 0, 255),
            WzrdNodeDataType::Number => Color32::from_rgb(0, 0, 255),
            WzrdNodeDataType::Any => Color32::from_rgb(255, 255, 255),
        }
    }

    fn name(&self) -> Cow<str> {
        match self {
            WzrdNodeDataType::Object => Cow::Borrowed("object"),
            WzrdNodeDataType::String => Cow::Borrowed("string"),
            WzrdNodeDataType::Function => Cow::Borrowed("function"),
            WzrdNodeDataType::Number => Cow::Borrowed("number"),
            WzrdNodeDataType::Any => Cow::Borrowed("any"),
        }
    }
}

impl NodeTemplateTrait for WzrdNodeTemplate {
    type NodeData = WzrdNodeData;
    type DataType = WzrdNodeDataType;
    type ValueType = WzrdValueType;
    type UserState = WzrdGraphState;

    fn node_finder_label(&self, user_state: &mut Self::UserState) -> Cow<str> {
        Cow::Borrowed(match self {
            WzrdNodeTemplate::Addition => "Add",
            WzrdNodeTemplate::Integer => "Integer",
            WzrdNodeTemplate::Float => "Float",
            WzrdNodeTemplate::Output => "Output",
        })
    }
    fn node_graph_label(&self, user_state: &mut Self::UserState) -> String {
        self.node_finder_label(user_state).into()
    }

    fn user_data(&self, user_state: &mut Self::UserState) -> Self::NodeData {
        WzrdNodeData { template: *self }
    }

    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
        node_id: NodeId,
    ) {
        let input_number = |graph: &mut WzrdGraph, name: &str| {
            graph.add_input_param(
                node_id,
                name.to_string(),
                WzrdNodeDataType::Number,
                WzrdValueType::Integer { value: 0 },
                InputParamKind::ConnectionOrConstant,
                true,
            );
        };

        let output_number = |graph: &mut WzrdGraph, name: &str| {
            graph.add_output_param(node_id, name.to_string(), WzrdNodeDataType::Number);
        };

        let input_constant = |graph: &mut WzrdGraph, name: &str, default_value: WzrdValueType| {
            graph.add_input_param(
                node_id,
                "value".to_string(),
                WzrdNodeDataType::Number,
                default_value,
                InputParamKind::ConstantOnly,
                true,
            );
        };

        match self {
            WzrdNodeTemplate::Addition => {
                input_number(graph, "param1");
                input_number(graph, "param2");
                output_number(graph, "");
            }
            WzrdNodeTemplate::Integer => {
                input_constant(graph, "value", WzrdValueType::Integer { value: 0 });
                output_number(graph, "");
            }
            WzrdNodeTemplate::Float => {
                input_constant(graph, "value", WzrdValueType::Float { value: 0.0 });
                output_number(graph, "");
            }
            WzrdNodeTemplate::Output => {
                graph.add_input_param(
                    node_id,
                    "".to_string(),
                    WzrdNodeDataType::Any,
                    WzrdValueType::String {
                        value: "".to_string(),
                    },
                    InputParamKind::ConnectionOnly,
                    true,
                );
            }
        }
    }
}

pub struct AllWzrdNodeTemplates;
impl NodeTemplateIter for AllWzrdNodeTemplates {
    type Item = WzrdNodeTemplate;

    fn all_kinds(&self) -> Vec<Self::Item> {
        vec![
            WzrdNodeTemplate::Addition,
            WzrdNodeTemplate::Integer,
            WzrdNodeTemplate::Float,
            WzrdNodeTemplate::Output,
        ]
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
}

type WzrdEditorState = GraphEditorState<
    WzrdNodeData,
    WzrdNodeDataType,
    WzrdValueType,
    WzrdNodeTemplate,
    WzrdGraphState,
>;

#[derive(Default)]
pub struct WzrdNodeGraph {
    state: WzrdEditorState,
    user_state: WzrdGraphState,
}

#[cfg(feature = "persistence")]
const PERSISTENCE_KEY: &str = "egui_node_graph";

#[cfg(feature = "persistence")]
impl WzrdNodeGraph {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        let state = creation_context
            .storage
            .and_then(|storage| eframe::get_value(storage, PERSISTENCE_KEY))
            .unwrap_or_default();

        Self {
            state,
            user_state: WzrdGraphState::default(),
        }
    }
}

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

        let graph_response = egui::CentralPanel::default()
            .show(ctx, |ui| {
                self.state
                    .draw_graph_editor(ui, AllWzrdNodeTemplates, &mut self.user_state)
            })
            .inner;

        // for node_response in graph_response.node_responses {
        //     if let NodeResponse::User(user_event) = node_response {
        //         match user_event {
        //         }
        //     }
        // }
    }
}
