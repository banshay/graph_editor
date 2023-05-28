use crate::app::node::structs::{
    WzrdGraphState, WzrdNode, WzrdNodeData, WzrdNodeDataType, WzrdNodeTemplates, WzrdResponse,
    WzrdType, WzrdValueType,
};
use crate::app::node::{create_std_nodes, WzrdNodes};
use eframe::egui::Pos2;
use eframe::glow::STENCIL_TEST;
use egui_node_graph::{
    Graph, GraphEditorState, GraphResponse, Node, NodeId, NodeResponse, NodeTemplateTrait, OutputId,
};
use instant::Instant;
use lazy_static::lazy_static;
use lib_ruby_parser::{Parser, ParserOptions, ParserResult};
use log::{debug, info};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::convert::identity;
use std::env::current_exe;
use std::ops::Deref;

type RNode = lib_ruby_parser::Node;

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

#[derive(Debug, Clone)]
enum ParsedValueType {
    Int(i128),
    String(String),
}

#[derive(Debug, Clone)]
struct ParsedWzrdNode {
    wzrd_node: WzrdNode,
    value: Option<ParsedValueType>, // keep for debug purposes for now
    inputs: Vec<ParsedWzrdNode>,
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

    pub fn initialize_graph(
        &mut self,
        graph: &mut Graph<WzrdNodeData, WzrdNodeDataType, WzrdValueType>,
        user_state: &mut WzrdGraphState,
        code: &str,
    ) {
        let parser = Parser::new(code, ParserOptions::default());
        let ParserResult {
            ast, input, tokens, ..
        } = parser.do_parse();

        if let Some(node) = ast {
            debug!("whole ast {node:?}");
            let parsed_graph = self.transform_ast(node.deref());
            debug!("Parsed graph {parsed_graph:?}");
            if let Some(node) = parsed_graph {
                self.build_graph(graph, user_state, &node);
            }
        }
    }

    fn build_graph(
        &mut self,
        graph: &mut Graph<WzrdNodeData, WzrdNodeDataType, WzrdValueType>,
        user_state: &mut WzrdGraphState,
        parsed_node: &ParsedWzrdNode,
    ) -> Node<WzrdNodeData> {
        let new_node = graph.add_node(
            parsed_node.wzrd_node.label.clone(),
            parsed_node.wzrd_node.user_data(user_state),
            |graph, node_id| parsed_node.wzrd_node.build_node(graph, user_state, node_id),
        );

        self.state.node_order.push(new_node);
        self.state
            .node_positions
            .insert(new_node, Pos2 { x: 100.0, y: 100.0 });

        let input_nodes: Vec<Node<WzrdNodeData>> = parsed_node
            .inputs
            .iter()
            .map(|node| self.build_graph(graph, user_state, node))
            .collect();

        let current_node: Node<WzrdNodeData> = graph.nodes[new_node].clone();
        for (i, (_, input_id)) in current_node.inputs.iter().enumerate() {
            if let Some(input_node) = input_nodes.get(i) {
                if let Some((_, output_id)) = input_node.outputs.first() {
                    graph.add_connection(*output_id, *input_id);
                }
            }
        }

        current_node
    }

    fn transform_ast(&self, node: &RNode) -> Option<ParsedWzrdNode> {
        match node {
            RNode::Begin(begin) => {
                debug!("{{");
                let statements: Vec<ParsedWzrdNode> = begin
                    .statements
                    .iter()
                    .map(|node| self.transform_ast(node))
                    .filter_map(identity)
                    .collect();
                debug!("}}");

                if let Some(ret) = statements.first() {
                    Some(ret.to_owned())
                } else {
                    None
                }
            }
            RNode::Send(send) => {
                if let Some(recv) = &send.recv {
                    let receiver = self.transform_ast(recv.deref()).unwrap();
                    let args: Vec<ParsedWzrdNode> = send
                        .args
                        .iter()
                        .map(|arg| self.transform_ast(arg))
                        .filter_map(|opt| opt)
                        .collect();

                    debug!(
                        "{:?} {:?} args: {:?}",
                        receiver.value, send.method_name, args,
                    );

                    if let Some(wzrd_node) = WzrdNodes::find_node(&send.method_name) {
                        let mut inputs = vec![receiver];
                        inputs.append(&mut args.clone());

                        Some(ParsedWzrdNode {
                            wzrd_node,
                            value: None,
                            inputs,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            RNode::Int(int) => {
                let mut template = WzrdNodes::Constant.node();
                //is it bad to assume a constant has one input?
                if let Some(input) = template.inputs.first() {
                    let mut cloned = input.clone();
                    cloned.initial_value = Some(WzrdValueType::Integer {
                        value: int.value.parse().unwrap_or(0),
                    });
                    template.inputs = vec![cloned];
                }
                Some(ParsedWzrdNode {
                    wzrd_node: template,
                    inputs: vec![],
                    value: Some(ParsedValueType::Int(int.value.parse().unwrap_or(0))),
                })
            }
            _ => None,
        }
    }
}
