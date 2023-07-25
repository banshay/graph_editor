pub mod structs;

use egui_node_graph::NodeTemplateIter;
use enum_iterator::{all, Sequence};
use lazy_static::lazy_static;
use std::collections::HashMap;
use structs::*;

impl WzrdNodeTemplates {
    pub fn create_node(
        &mut self,
        label: &str,
        template: Option<String>,
        inputs: Vec<WzrdType>,
        outputs: Vec<WzrdType>,
    ) -> &WzrdNode {
        let new_node = WzrdNode {
            template,
            label: label.into(),
            inputs,
            outputs,
        };
        self.0.push(new_node);
        &self.0[self.0.len() - 1]
    }
}

impl NodeTemplateIter for WzrdNodeTemplates {
    type Item = WzrdNode;

    fn all_kinds(&self) -> Vec<Self::Item> {
        self.0.clone()
    }
}

#[derive(Eq, Hash, PartialEq, Sequence, Clone)]
pub enum WzrdNodes {
    Constant,
    Add,
    Multiply,
    Output,
    Variable,
    If,
}

lazy_static! {
    static ref NODE_MAP: HashMap<WzrdNodes, WzrdNode> = all::<WzrdNodes>()
        .map(|node_enum| (node_enum.clone(), node_enum.new()))
        .collect();
    static ref NODE_LABEL_MAP: HashMap<String, WzrdNode> = all::<WzrdNodes>()
        .map(|node_enum| node_enum.new())
        .map(|node| (node.label.clone(), node))
        .collect();
}

impl WzrdNodes {
    pub fn node(&self) -> WzrdNode {
        NODE_MAP[self].clone()
    }

    pub fn find_node(label: &str) -> Option<WzrdNode> {
        NODE_LABEL_MAP.get(label).map(|node| node.clone())
    }

    fn new(&self) -> WzrdNode {
        match self {
            WzrdNodes::Constant => WzrdNode {
                template: None,
                label: "Constant".into(),
                inputs: vec![WzrdType {
                    name: "value".into(),
                    data_type: WzrdValueType::Any,
                    order: 1,
                }],
                outputs: vec![WzrdType {
                    name: "out".into(),
                    data_type: WzrdValueType::Any,
                    order: 1,
                }],
            },
            WzrdNodes::Variable => WzrdNode {
                template: None,
                label: "Variable".into(),
                inputs: vec![],
                outputs: vec![],
            },
            WzrdNodes::Add => WzrdNode {
                template: Some("($0+$1)".into()),
                label: "+".to_string(),
                inputs: vec![
                    WzrdType {
                        name: "value1".into(),
                        data_type: WzrdValueType::Number { value: 0 },
                        order: 1,
                    },
                    WzrdType {
                        name: "value2".into(),
                        data_type: WzrdValueType::Number { value: 0 },
                        order: 2,
                    },
                ],
                outputs: vec![WzrdType {
                    name: "out".into(),
                    data_type: WzrdValueType::Any,
                    order: 1,
                }],
            },
            WzrdNodes::Multiply => WzrdNode {
                template: Some("($0*$1)".into()),
                label: "*".to_string(),
                inputs: vec![
                    WzrdType {
                        name: "value1".into(),
                        data_type: WzrdValueType::Number { value: 0 },
                        order: 1,
                    },
                    WzrdType {
                        name: "value2".into(),
                        data_type: WzrdValueType::Number { value: 0 },
                        order: 2,
                    },
                ],
                outputs: vec![WzrdType {
                    name: "out".into(),
                    data_type: WzrdValueType::Any,
                    order: 1,
                }],
            },
            WzrdNodes::Output => WzrdNode {
                template: Some("return $0".into()),
                label: "output".into(),
                inputs: vec![],
                outputs: vec![],
            },
            WzrdNodes::If => WzrdNode {
                label: "If".into(),
                template: Some(
                    "if ($0) then
  $1
else
  $2
end"
                    .into(),
                ),
                inputs: vec![
                    WzrdType {
                        name: "condition".into(),
                        data_type: WzrdValueType::Expression { value: "".into() },
                        order: 1,
                    },
                    WzrdType {
                        name: "then".into(),
                        data_type: WzrdValueType::Any,
                        order: 2,
                    },
                    WzrdType {
                        name: "else".into(),
                        data_type: WzrdValueType::Any,
                        order: 3,
                    },
                ],
                outputs: vec![WzrdType {
                    name: "".into(),
                    data_type: WzrdValueType::Any,
                    order: 1,
                }],
            },
        }
    }
}

pub fn create_std_nodes() -> Vec<WzrdNode> {
    let mut stds = vec![];

    stds.push(WzrdNodes::Add.node());
    stds.push(WzrdNodes::Multiply.node());
    stds.push(WzrdNodes::If.node());

    stds
}
