pub mod structs;

use egui_node_graph::NodeTemplateIter;
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

pub fn create_std_nodes() -> Vec<WzrdNode> {
    let mut stds = vec![];

    stds.push(WzrdNode {
        template: Some("$0+$1".into()),
        label: "Add".into(),
        inputs: vec![
            WzrdType {
                name: "value1".into(),
                data_type: WzrdNodeDataType::Number,
            },
            WzrdType {
                name: "value2".into(),
                data_type: WzrdNodeDataType::Number,
            },
        ],
        outputs: vec![WzrdType {
            name: "out".into(),
            data_type: WzrdNodeDataType::Number,
        }],
    });

    stds
}
