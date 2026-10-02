use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::router::RouterError;
use crate::{DecodeError, Frame, NodePath};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Item,
    Float,
    Enum,
    PropertyList,
    List,
    Action,
    Control,
    Array,
    /// Firmware 2.x's signal chain, `root\app\router`: one row of positions
    /// and connectors (see [`crate::router`]).
    Router,
    Unknown(String),
}

impl NodeKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Item => "item",
            Self::Float => "float",
            Self::Enum => "enum",
            Self::PropertyList => "plist",
            Self::List => "list",
            Self::Action => "action",
            Self::Control => "ctrl",
            Self::Array => "array",
            Self::Router => "router",
            Self::Unknown(value) => value,
        }
    }
}

impl From<String> for NodeKind {
    fn from(value: String) -> Self {
        match value.as_str() {
            "item" => Self::Item,
            "float" => Self::Float,
            "enum" => Self::Enum,
            "plist" => Self::PropertyList,
            "list" => Self::List,
            "action" => Self::Action,
            "ctrl" => Self::Control,
            "array" => Self::Array,
            "router" => Self::Router,
            _ => Self::Unknown(value),
        }
    }
}

impl Serialize for NodeKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for NodeKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(String::deserialize(deserializer)?.into())
    }
}

/// Forward-compatible metadata returned by `browse` or `read`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeDescription {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<NodeKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(rename = "def", default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<Value>>,
    #[serde(rename = "ref", default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gzip: Option<bool>,
    #[serde(rename = "move", default, skip_serializing_if = "Option::is_none")]
    pub movable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl NodeDescription {
    /// Validate a prospective live value against the schema the device itself
    /// advertised. Unknown and command-like nodes remain non-writable.
    pub fn validate_value(&self, value: &Value) -> Result<(), NodeValueError> {
        match self.kind.as_ref() {
            Some(NodeKind::Float) => {
                let number = value.as_f64().filter(|number| number.is_finite()).ok_or(
                    NodeValueError::WrongType {
                        expected: "a finite number",
                    },
                )?;
                if self.min.is_some_and(|minimum| number < minimum)
                    || self.max.is_some_and(|maximum| number > maximum)
                {
                    return Err(NodeValueError::OutOfRange {
                        value: number,
                        min: self.min,
                        max: self.max,
                    });
                }
            }
            Some(NodeKind::Enum | NodeKind::Array) => {
                let options = self
                    .options
                    .as_ref()
                    .or(self.items.as_ref())
                    .ok_or(NodeValueError::MissingOptions)?;
                if !options.contains(value) {
                    return Err(NodeValueError::NotAnOption {
                        value: value.clone(),
                        options: options.clone(),
                    });
                }
            }
            Some(NodeKind::PropertyList) => {
                if !value.is_string() {
                    return Err(NodeValueError::WrongType {
                        expected: "a list-item name string",
                    });
                }
            }
            Some(NodeKind::Item) => {
                let shape = self.value.as_ref().or(self.default.as_ref());
                if let Some(shape) = shape {
                    let same_shape = matches!(
                        (shape, value),
                        (Value::Null, Value::Null)
                            | (Value::Bool(_), Value::Bool(_))
                            | (Value::Number(_), Value::Number(_))
                            | (Value::String(_), Value::String(_))
                            | (Value::Array(_), Value::Array(_))
                            | (Value::Object(_), Value::Object(_))
                    );
                    if !same_shape {
                        return Err(NodeValueError::WrongType {
                            expected: "the node's current JSON value type",
                        });
                    }
                }
            }
            Some(NodeKind::Router) => {
                let row = crate::Router::from_value(value).map_err(NodeValueError::Router)?;
                // The pedal ignores a row of any other length, so one is never
                // sent: it would look written and change nothing.
                let current = self.value.as_ref().or(self.default.as_ref());
                if let Some(current) =
                    current.and_then(|current| crate::Router::from_value(current).ok())
                {
                    if current.cells() != row.cells() {
                        return Err(NodeValueError::Router(RouterError::WrongLength {
                            expected: current.cells(),
                            actual: row.cells(),
                        }));
                    }
                }
            }
            Some(NodeKind::List | NodeKind::Action | NodeKind::Control | NodeKind::Unknown(_))
            | None => return Err(NodeValueError::NotWritable),
        }
        Ok(())
    }
}

/// An ordered snapshot of browsed nodes. Ordering follows the device's reply,
/// which is meaningful for editor presentation and preset serialization.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTree {
    nodes: Vec<(NodePath, NodeDescription)>,
}

impl NodeTree {
    pub fn from_frame(frame: &Frame) -> Result<Self, NodeError> {
        let mut nodes = Vec::with_capacity(frame.records().len());
        for record in frame.records() {
            let path = NodePath::new(record.subject())?;
            let description = serde_json::from_value(record.value().clone()).map_err(|source| {
                NodeError::Description {
                    path: path.clone(),
                    source,
                }
            })?;
            nodes.push((path, description));
        }
        Ok(Self { nodes })
    }

    pub fn nodes(&self) -> &[(NodePath, NodeDescription)] {
        &self.nodes
    }

    pub fn get(&self, path: &NodePath) -> Option<&NodeDescription> {
        self.nodes
            .iter()
            .find_map(|(candidate, node)| (candidate == path).then_some(node))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error(transparent)]
    Path(#[from] crate::CommandError),
    #[error("invalid node description for {path}: {source}")]
    Description {
        path: NodePath,
        source: serde_json::Error,
    },
    #[error(transparent)]
    Decode(#[from] DecodeError),
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum NodeValueError {
    #[error("node is not safely writable")]
    NotWritable,
    #[error("value must be {expected}")]
    WrongType { expected: &'static str },
    #[error("numeric value {value} is outside {min:?}..{max:?}")]
    OutOfRange {
        value: f64,
        min: Option<f64>,
        max: Option<f64>,
    },
    #[error("enum node has no advertised options")]
    MissingOptions,
    #[error("{value} is not one of the advertised options {options:?}")]
    NotAnOption { value: Value, options: Vec<Value> },
    #[error(transparent)]
    Router(RouterError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_types_and_fields_survive_schema_evolution() {
        let frame =
            Frame::parse(br#"root\future:{"type":"matrix","desc":"Future","new_flag":true}"#)
                .unwrap();
        let tree = NodeTree::from_frame(&frame).unwrap();
        let node = &tree.nodes()[0].1;
        assert_eq!(node.kind, Some(NodeKind::Unknown("matrix".into())));
        assert_eq!(node.extra["new_flag"], true);
    }

    /// Firmware 2.x's chain is a node of its own kind, written as one row as
    /// long as the pedal's.
    #[test]
    fn the_router_is_written_as_one_row_as_long_as_the_pedals() {
        let frame = Frame::parse(
            br#"root\app\router:{"type":"router","desc":"router","value":[["root\\app\\gate","s",""]],"fixed":[[false,false,true]]}"#,
        )
        .unwrap();
        let tree = NodeTree::from_frame(&frame).unwrap();
        let node = &tree.nodes()[0].1;
        assert_eq!(node.kind, Some(NodeKind::Router));
        assert_eq!(serde_json::to_value(&node.kind).unwrap(), "router");
        assert!(node.extra.contains_key("fixed"));
        assert!(node
            .validate_value(&serde_json::json!([["", "p", "root\\app\\amp"]]))
            .is_ok());
        for wrong in [
            serde_json::json!([["root\\app\\gate"]]),
            serde_json::json!([["", "s", "", "s", ""]]),
            serde_json::json!(["", "s", ""]),
            serde_json::json!([["", 1, ""]]),
            serde_json::json!("root\\app\\gate"),
        ] {
            assert!(node.validate_value(&wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn float_and_enum_values_are_checked_against_device_schema() {
        let frame = Frame::parse(
            b"root\\gain:{\"type\":\"float\",\"value\":0.5,\"min\":0.0,\"max\":1.0}\r\nroot\\mode:{\"type\":\"enum\",\"value\":\"A\",\"options\":[\"A\",\"B\"]}",
        )
        .unwrap();
        let tree = NodeTree::from_frame(&frame).unwrap();
        assert!(tree.nodes()[0]
            .1
            .validate_value(&serde_json::json!(0.75))
            .is_ok());
        assert!(tree.nodes()[0]
            .1
            .validate_value(&serde_json::json!(2.0))
            .is_err());
        assert!(tree.nodes()[1]
            .1
            .validate_value(&serde_json::json!("B"))
            .is_ok());
        assert!(tree.nodes()[1]
            .1
            .validate_value(&serde_json::json!("C"))
            .is_err());
    }
}
