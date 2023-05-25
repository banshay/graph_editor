use serde::{Deserialize, Serialize};

#[derive(Default, Clone)]
pub struct WzrdNodeTemplates(pub Vec<WzrdNode>);

#[derive(Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct WzrdType {
    pub name: String,
    pub data_type: WzrdNodeDataType,
}

#[derive(Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct WzrdNode {
    pub template: Option<String>,
    pub label: String,
    pub inputs: Vec<WzrdType>,
    pub outputs: Vec<WzrdType>,
}

#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct WzrdNodeData {
    pub template: WzrdNode,
}

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub enum WzrdNodeDataType {
    Object,
    String,
    Function,
    Number,
    Expression,
    Literal,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub enum WzrdValueType {
    // Object { value: Option<dyn Any> },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
}

#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct WzrdGraphState {}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WzrdResponse {}
