use crate::app::node::structs::{
    WzrdFunction, WzrdGraphState, WzrdNode, WzrdNodeData, WzrdNodeDataType, WzrdNodeTemplates,
    WzrdResponse, WzrdType, WzrdValueType,
};
use crate::app::node::{create_std_nodes, WzrdNodes};
use eframe::egui::accesskit::Role::Math;
use eframe::egui::{pos2, vec2, Pos2, Rect};
use eframe::glow::STENCIL_TEST;
use egui_node_graph::{
    Graph, GraphEditorState, GraphResponse, Node, NodeId, NodeRects, NodeResponse,
    NodeTemplateTrait, OutputId,
};
use instant::Instant;
use lazy_static::lazy_static;
use lib_ruby_parser::{Parser, ParserOptions, ParserResult};
use log::{debug, info};
use queues::{IsQueue, Queue};
use regex::Regex;
use slotmap::SecondaryMap;
use std::cmp::max;
use std::collections::{HashMap, HashSet, LinkedList};
use std::convert::identity;
use std::env::current_exe;
use std::ops::{Add, Deref};
use std::sync::{Arc, Mutex};

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
    pub function_stack: LinkedList<WzrdFunction>,

    pub format_requested: Arc<Mutex<bool>>,
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
    pub fn new() -> Self {
        let standard_nodes: Vec<WzrdNode> = create_std_nodes();

        Self {
            state: WzrdEditorState::default(),
            user_state: WzrdGraphState::default(),
            node_templates: WzrdNodeTemplates(standard_nodes),
            last_update: None,
            last_event: None,
            format_requested: Arc::new(Mutex::new(false)),
            function_stack: LinkedList::new(),
        }
    }

    pub fn evaluate_graph(&mut self, cache: &mut NodeCache) -> String {
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
                    None => match &node.user_data.template {
                        WzrdNode {
                            ref label, outputs, ..
                        } if label == "Variable" => outputs
                            .first()
                            .map(|output| output.name.clone())
                            .unwrap_or("".into()),
                        _ => input_values[0].clone(),
                    },
                })
            }
        }

        if let Some(id) = self.find_last_node() {
            let evaluator = Evaluator::new(&self.state.graph, cache);
            let code_body = evaluator
                .evaluate_node(id)
                .unwrap_or("error while calling evaluate node".into());

            if let Some(function_signature) = self.function_stack.pop_back() {
                let arguments = function_signature.arguments.join(", ");
                format!(
                    "
                    def {:}{:}
                        {code_body}
                    end
                    ",
                    function_signature.name,
                    if arguments.is_empty() {
                        String::from("")
                    } else {
                        format!("({arguments})")
                    }
                )
            } else {
                code_body
            }
        } else {
            "Could not evaluate Graph".into()
        }
    }

    pub fn find_last_node(&self) -> Option<NodeId> {
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
        last_node_id
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

    fn parse_arguments(&self, node: &RNode) -> Vec<String> {
        match node {
            RNode::Args(args) => args
                .args
                .iter()
                .flat_map(|arg| self.parse_arguments(&arg))
                .collect(),
            RNode::Arg(arg) => {
                vec![arg.name.clone()]
            }
            _ => vec![],
        }
    }

    fn transform_ast(&mut self, node: &RNode) -> Option<ParsedWzrdNode> {
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

                statements.first().map(|statement| statement.to_owned())
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
            RNode::Def(def) => {
                self.function_stack.push_back(WzrdFunction {
                    name: def.name.to_string(),
                    arguments: def
                        .args
                        .clone()
                        .map(|node| self.parse_arguments(node.deref()))
                        .unwrap_or(vec![]),
                });
                if let Some(body) = &def.body {
                    self.transform_ast(body.deref())
                } else {
                    None
                }
            }
            RNode::Lvar(lvar) => {
                let mut template = WzrdNodes::Variable.node();
                template.outputs = vec![WzrdType {
                    name: lvar.name.to_string(),
                    data_type: WzrdNodeDataType::Any,
                    initial_value: None,
                }];

                Some(ParsedWzrdNode {
                    wzrd_node: template,
                    inputs: vec![],
                    value: None,
                })
            }
            RNode::Return(ret) => {
                let arguments: Vec<ParsedWzrdNode> = ret
                    .args
                    .iter()
                    .map(|node| self.transform_ast(node))
                    .filter_map(identity)
                    .collect();

                let mut template = WzrdNodes::Output.node();
                template.inputs = vec![WzrdType {
                    name: "output".to_string(),
                    data_type: WzrdNodeDataType::Any,
                    initial_value: None,
                }];

                Some(ParsedWzrdNode {
                    wzrd_node: template,
                    inputs: arguments
                        .first()
                        .map(|arg| vec![arg.to_owned()])
                        .unwrap_or(vec![]),
                    value: None,
                })
            }
            _ => None,
        }
    }

    pub fn format_graph(&mut self) {
        const X_OFFSET: f32 = 50.0;
        const Y_OFFSET: f32 = 50.0;

        let mut new_positions: SecondaryMap<NodeId, Pos2> = SecondaryMap::new();

        fn build_outer_rect(input_rects: Vec<Rect>) -> (f32, f32) {
            let mut max_x = 0.0;
            let mut sum_y = 0.0;
            for rect in input_rects {
                let dimension = rect.max - rect.min;
                max_x = f32::max(max_x, dimension.x);
                sum_y = sum_y + Y_OFFSET + dimension.y;
            }
            (max_x, sum_y)
        }

        fn format(
            state: &WzrdEditorState,
            node: &Node<WzrdNodeData>,
            new_positions: &mut SecondaryMap<NodeId, Pos2>,
        ) {
            let input_rects: Vec<Rect> = node
                .inputs
                .iter()
                .filter_map(|(_, input_id)| state.graph.connections.get(*input_id))
                .map(|output_id| {
                    let output_param = &state.graph.outputs[*output_id];
                    state.node_rects[&output_param.node]
                })
                .collect();
            let outer_rect = build_outer_rect(input_rects);

            let current_node_position = new_positions[node.id];
            let current_node_rect = state.node_rects[&node.id];
            let current_node_baseline =
                current_node_position.y + ((current_node_rect.max - current_node_rect.min).y / 2.0);

            let first_node_y = current_node_baseline - (outer_rect.1 / 2.0);
            let first_node_x = current_node_position.x - X_OFFSET - outer_rect.0;
            let mut prev_node_y = first_node_y;

            for input in node
                .inputs
                .iter()
                .filter_map(|(_, input_id)| state.graph.connections.get(*input_id))
                .map(|output_id| {
                    let output_param = &state.graph.outputs[*output_id];
                    &state.graph.nodes[output_param.node]
                })
                .into_iter()
            {
                new_positions.insert(input.id, pos2(first_node_x, prev_node_y));
                prev_node_y = state.node_rects[&input.id].max.y + Y_OFFSET;
            }
        }

        if let Some(last_node_id) = self.find_last_node() {
            let last_node = &self.state.graph.nodes[last_node_id];
            new_positions.insert(last_node_id, pos2(0.0, 0.0));

            let mut queue: Queue<&Node<WzrdNodeData>> = Queue::new();
            queue
                .add(last_node)
                .expect("Unable to add node to queue for formatting.");

            while queue.size() > 0 {
                if let Ok(node) = queue.remove() {
                    format(&self.state, node, &mut new_positions);
                    node.inputs
                        .iter()
                        .filter_map(|(_, input_id)| self.state.graph.connection(*input_id))
                        .map(|output_id| {
                            let output = &self.state.graph.outputs[output_id];
                            &self.state.graph.nodes[output.node]
                        })
                        .for_each(|child_node| {
                            queue
                                .add(&child_node)
                                .expect("Unable to add node to queue for formatting.");
                        });
                } else {
                    break;
                }
            }
        }

        self.state.node_positions = new_positions;
        self.state.pan_zoom.pan = vec2(
            self.state.ui_rect.width() / 2.0,
            self.state.ui_rect.height() / 2.0,
        );
    }
}
