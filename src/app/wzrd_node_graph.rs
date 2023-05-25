use crate::app::node::create_std_nodes;
use crate::app::node::structs::{
    WzrdGraphState, WzrdNode, WzrdNodeData, WzrdNodeDataType, WzrdNodeTemplates, WzrdResponse,
    WzrdValueType,
};
use egui_node_graph::{
    Graph, GraphEditorState, GraphResponse, Node, NodeId, NodeResponse, OutputId,
};
use instant::Instant;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[cfg(feature = "persistence")]
pub const PERSISTENCE_KEY: &str = "egui_node_graph";

pub type WzrdGraph = Graph<WzrdNodeData, WzrdNodeDataType, WzrdValueType>;
pub type WzrdGraphResponse = GraphResponse<WzrdResponse, WzrdNodeData>;

pub type WzrdEditorState =
    GraphEditorState<WzrdNodeData, WzrdNodeDataType, WzrdValueType, WzrdNode, WzrdGraphState>;

pub type NodeCache = HashMap<NodeId, String>;

#[derive(Default)]
pub struct WzrdNodeGraph {
    pub state: WzrdEditorState,
    pub user_state: WzrdGraphState,
    pub node_templates: WzrdNodeTemplates,
    pub last_update: Option<Instant>,
    pub last_event: Option<Instant>,
}

#[cfg(feature = "persistence")]
impl WzrdNodeGraph {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        let state = creation_context
            .storage
            .and_then(|storage| eframe::get_value(storage, PERSISTENCE_KEY))
            .unwrap_or_default();

        let standard_nodes: Vec<WzrdNode> = create_std_nodes();

        Self {
            state,
            user_state: WzrdGraphState::default(),
            node_templates: WzrdNodeTemplates(standard_nodes),
            last_update: None,
            last_event: None,
        }
    }

    pub fn evaluate_graph(&self, cache: &mut NodeCache) -> String {
        struct Evaluator<'a> {
            graph: &'a WzrdGraph,
            cache: &'a mut NodeCache,
        }

        impl<'a> Evaluator<'a> {
            fn new(graph: &'a WzrdGraph, cache: &'a mut NodeCache) -> Self {
                Self { graph, cache }
            }

            fn extract_arguments(&self, template: &'a str) -> HashSet<&'a str> {
                lazy_static! {
                    static ref ARGUMENT_REGEX: Regex = Regex::new(r"\$\d+").unwrap();
                }
                ARGUMENT_REGEX
                    .find_iter(template)
                    .map(|mat| mat.as_str())
                    .collect()
            }

            fn evaluate_node(&self, node_id: NodeId) -> anyhow::Result<String> {
                let node: &Node<WzrdNodeData> = &self.graph[node_id];
                let input_values: Vec<String> = node
                    .inputs
                    .clone()
                    .into_iter()
                    .map(|(input, input_id)| {
                        if let Some(other_output_id) = self.graph.connection(input_id) {
                            let other_node_id: NodeId = self.graph.get_output(other_output_id).node;
                            if let Some(cached_out) = self.cache.get(&other_node_id) {
                                cached_out.clone()
                            } else {
                                self.evaluate_node(self.graph[other_node_id].id)
                                    .expect("nothing returned from node evaluation")
                            }
                        } else {
                            //node has a constant value, so it's of WzrdValueType
                            match &self.graph.inputs[input_id].value {
                                WzrdValueType::String { value } => format!("\"{value}\""),
                                WzrdValueType::Integer { value } => {
                                    format!("{value}")
                                }
                                WzrdValueType::Float { value } => {
                                    format!("{value}")
                                }
                            }
                        }
                    })
                    .collect();

                // self.extract_argument(node.user_data.template.template)
                Ok(match &node.user_data.template.template {
                    Some(template) => {
                        let mut ret = String::from(template);
                        let arguments = self.extract_arguments(template);
                        for (i, arg) in arguments.iter().enumerate() {
                            ret = ret.replace(arg, &input_values[i]);
                        }
                        ret
                    }
                    None => input_values[0].clone(),
                })
            }
        }

        //find last node
        let mut last_node_id: Option<NodeId> = None;
        for (node_id, node) in self.state.graph.nodes.iter() {
            let output_ids: Vec<&OutputId> = node
                .outputs
                .iter()
                .map(|(_, output_id)| output_id)
                .collect();
            let mut connected_outputs = self.state.graph.connections.clone();
            connected_outputs.retain(|input, output_id| output_ids.contains(&&*output_id));
            debug!(
                "{:} has {:} connected outputs",
                node.label,
                connected_outputs.len()
            );
            if connected_outputs.len() == 0 {
                last_node_id = Some(node_id);
            }
        }

        if let Some(id) = last_node_id {
            let evaluator = Evaluator::new(&self.state.graph, cache);
            evaluator
                .evaluate_node(id)
                .unwrap_or("error while calling evaluate node".into())
        } else {
            "Could not evaluate Graph".into()
        }
    }
}
