use serde::{
    de::{Deserializer, Error as DeError, IgnoredAny, MapAccess, Visitor},
    Deserialize, Serialize,
};
use serde_json::value::RawValue;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Workflow {
    pub name: String,
    pub nodes: Vec<WorkflowNode>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkflowNode {
    pub id: usize,
    pub name: String,
    pub next: Vec<usize>,
    pub inputs: Vec<ActionInput>,
    pub outputs: Vec<ActionOutput>,
    pub kind: ActionType,
    pub metadata: ActionMetadata,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActionType {
    Start = 0,
    SendMessage = 1,
    JoinString = 2,
    GetDatastoreEntry = 3,
    Add = 4,
    UpdateDatastoreEntry = 5,
}

#[derive(Clone, Debug, Serialize)]
pub enum ActionMetadata {
    Start,
    SendMessage(action::SendMessage),
    JoinString,
    GetDatastoreEntry,
    Add,
    UpdateDatastoreEntry,
}

pub mod action {
    use serde::{Deserialize, Serialize};

    use crate::id::ChannelId;

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SendMessage {
        pub message: String,
        pub channel: ChannelId,
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Field {
    pub name: String,
    pub value: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActionInput {
    pub name: String,
    pub source: ActionInputSource,
    pub kind: ActionInputSourceType,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActionInputSourceType {
    Static,
    Action,
    External,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ActionInputSource {
    Static(Value),
    Action {
        action_id: usize,
        output_name: String,
    },
    External(ValueType),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionOutput {
    pub name: String,
    pub value: ValueType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Value {
    String(String),
    Number(i64),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ValueType {
    Number,
    String,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => f.write_str(&s),
            Self::Number(n) => f.write_str(&n.to_string()),
        }
    }
}

impl<'de> Deserialize<'de> for WorkflowNode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Debug, Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Id,
            Name,
            Next,
            Inputs,
            Outputs,
            Kind,
            Metadata,
        }

        struct WorkflowNodeVisitor;

        impl<'de> Visitor<'de> for WorkflowNodeVisitor {
            type Value = WorkflowNode;

            fn expecting(&self, f: &mut Formatter) -> FmtResult {
                f.write_str("struct WorkflowNode")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut id = None;
                let mut name = None;
                let mut next = None;
                let mut inputs = None;
                let mut outputs = None;
                let mut kind = None;
                let mut metadata = None::<Box<RawValue>>;

                loop {
                    let key = match map.next_key() {
                        Ok(Some(key)) => key,
                        Ok(None) => break,
                        Err(_) => {
                            map.next_value::<IgnoredAny>()?;

                            continue;
                        }
                    };

                    match key {
                        Field::Id => {
                            if id.is_some() {
                                return Err(DeError::duplicate_field("id"));
                            }

                            id = Some(map.next_value()?);
                        }
                        Field::Name => {
                            if name.is_some() {
                                return Err(DeError::duplicate_field("name"));
                            }

                            name = Some(map.next_value()?);
                        }
                        Field::Next => {
                            if next.is_some() {
                                return Err(DeError::duplicate_field("next"));
                            }

                            next = Some(map.next_value()?);
                        }
                        Field::Inputs => {
                            if inputs.is_some() {
                                return Err(DeError::duplicate_field("inputs"));
                            }

                            inputs = Some(map.next_value()?);
                        }
                        Field::Outputs => {
                            if outputs.is_some() {
                                return Err(DeError::duplicate_field("outputs"));
                            }

                            outputs = Some(map.next_value()?);
                        }
                        Field::Kind => {
                            if kind.is_some() {
                                return Err(DeError::duplicate_field("kind"));
                            }

                            kind = Some(map.next_value()?);
                        }
                        Field::Metadata => {
                            if metadata.is_some() {
                                return Err(DeError::duplicate_field("metadata"));
                            }

                            metadata = Some(map.next_value()?);
                        }
                    }
                }

                let id = id.ok_or_else(|| DeError::missing_field("id"))?;
                let name = name.ok_or_else(|| DeError::missing_field("name"))?;
                let next = next.ok_or_else(|| DeError::missing_field("next"))?;
                let inputs = inputs.ok_or_else(|| DeError::missing_field("inputs"))?;
                let outputs = outputs.ok_or_else(|| DeError::missing_field("outputs"))?;
                let kind = kind.ok_or_else(|| DeError::missing_field("kind"))?;
                let metadata = metadata.ok_or_else(|| DeError::missing_field("metadata"))?;

                let metadata = match kind {
                    ActionType::Start => ActionMetadata::Start,
                    ActionType::JoinString => ActionMetadata::JoinString,
                    ActionType::Add => ActionMetadata::Add,
                    ActionType::GetDatastoreEntry => ActionMetadata::GetDatastoreEntry,
                    ActionType::SendMessage => ActionMetadata::SendMessage(
                        action::SendMessage::deserialize(metadata.as_ref())
                            .map_err(|err| DeError::custom(err))?,
                    ),
                    ActionType::UpdateDatastoreEntry => ActionMetadata::UpdateDatastoreEntry,
                };

                Ok(WorkflowNode {
                    id,
                    name,
                    next,
                    inputs,
                    outputs,
                    kind,
                    metadata,
                })
            }
        }

        const FIELDS: &[&str] = &[
            "id", "name", "next", "inputs", "outputs", "kind", "metadata",
        ];

        deserializer.deserialize_struct("WorkflowNode", FIELDS, WorkflowNodeVisitor)
    }
}

impl<'de> Deserialize<'de> for ActionInput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Debug, Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Name,
            Source,
            Kind,
        }

        struct ActionInputVisitor;

        impl<'de> Visitor<'de> for ActionInputVisitor {
            type Value = ActionInput;

            fn expecting(&self, f: &mut Formatter) -> FmtResult {
                f.write_str("struct ActionInput")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut name = None;
                let mut source = None::<Box<RawValue>>;
                let mut kind = None;

                loop {
                    let key = match map.next_key() {
                        Ok(Some(key)) => key,
                        Ok(None) => break,
                        Err(_) => {
                            map.next_value::<IgnoredAny>()?;

                            continue;
                        }
                    };

                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(DeError::duplicate_field("name"));
                            }

                            name = Some(map.next_value()?);
                        }
                        Field::Source => {
                            if source.is_some() {
                                return Err(DeError::duplicate_field("source"));
                            }

                            source = Some(map.next_value()?);
                        }
                        Field::Kind => {
                            if kind.is_some() {
                                return Err(DeError::duplicate_field("kind"));
                            }

                            kind = Some(map.next_value()?);
                        }
                    }
                }

                let name = name.ok_or_else(|| DeError::missing_field("name"))?;
                let source = source.ok_or_else(|| DeError::missing_field("source"))?;
                let kind = kind.ok_or_else(|| DeError::missing_field("kind"))?;

                let source = match kind {
                    ActionInputSourceType::External => ActionInputSource::External(
                        ValueType::deserialize(source.as_ref())
                            .map_err(|err| DeError::custom(err))?,
                    ),
                    ActionInputSourceType::Static => ActionInputSource::Static(
                        Value::deserialize(source.as_ref()).map_err(|err| DeError::custom(err))?,
                    ),
                    ActionInputSourceType::Action => {
                        ActionInputSource::deserialize(source.as_ref())
                            .map_err(|err| DeError::custom(err))?
                    }
                };

                Ok(ActionInput { name, source, kind })
            }
        }

        const FIELDS: &[&str] = &["name", "kind", "source"];

        deserializer.deserialize_struct("ActionInput", FIELDS, ActionInputVisitor)
    }
}
