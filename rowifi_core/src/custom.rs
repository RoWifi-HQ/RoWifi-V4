use regex::Regex;
use rowifi_models::{
    custom::{
        action, ActionInputSource, ActionMetadata, ActionType, Value, ValueType, Workflow,
        WorkflowNode,
    },
    roblox::id::{UniverseId, UserId},
};
use rowifi_roblox::{error::RobloxError, RobloxClient, UpdateDatastoreEntryArgs};
use std::{
    collections::{HashMap, VecDeque},
    sync::LazyLock,
};
use twilight_http::Client as TwilightClient;

static TEMPLATE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{(.*?)\}").unwrap());

pub struct ExecutionContext {
    pub nodes: Vec<WorkflowNodeExecution>,
}

pub struct WorkflowContext<'a> {
    pub bot: &'a TwilightClient,
    pub roblox: &'a RobloxClient,
}

pub struct WorkflowNodeExecution {
    pub id: usize,
    pub inputs: HashMap<String, Value>,
    pub outputs: HashMap<String, Value>,
}

#[derive(Debug)]
pub enum WorkflowExecutionError {
    NodeNotFound,
    InputNotFound,
    IncorrectInputType,
    Node {
        id: usize,
        err: WorkflowNodeExecutionError,
    },
}

#[derive(Debug)]
pub enum WorkflowNodeExecutionError {
    InputNotFound,
    OutputNotFound,
    InputTypeMismatch,
    IncorrectInputFormat,
    IncorrectOutputFormat,
    Discord(twilight_http::Error),
    Roblox(RobloxError),
}

pub async fn execute_workflow(
    workflow: &Workflow,
    workflow_context: &WorkflowContext<'_>,
    execution_context: &mut ExecutionContext,
    args: &HashMap<String, Value>,
) -> Result<(), WorkflowExecutionError> {
    let start = workflow
        .nodes
        .iter()
        .find(|n| n.kind == ActionType::Start)
        .ok_or(WorkflowExecutionError::NodeNotFound)?;

    let mut queue = VecDeque::new();
    queue.push_back(start);
    while let Some(node) = queue.pop_front() {
        for input in &node.inputs {
            let input_value = match &input.source {
                ActionInputSource::Static(v) => v.clone(),
                ActionInputSource::Action {
                    action_id,
                    output_name,
                } => {
                    let output = execution_context
                        .nodes
                        .iter()
                        .find(|n| n.id == *action_id)
                        .ok_or(WorkflowExecutionError::NodeNotFound)?
                        .outputs
                        .get(output_name)
                        .ok_or(WorkflowExecutionError::InputNotFound)?;
                    output.clone()
                }
                ActionInputSource::External(_v) => {
                    let input_value = args
                        .get(&input.name)
                        .ok_or(WorkflowExecutionError::InputNotFound)?;
                    input_value.clone()
                }
            };
            let execution_node = execution_context
                .nodes
                .iter_mut()
                .find(|n| n.id == node.id)
                .unwrap();
            execution_node
                .inputs
                .insert(input.name.clone(), input_value);
        }

        let execution_node = execution_context
            .nodes
            .iter_mut()
            .find(|n| n.id == node.id)
            .unwrap();

        execute_node(node, workflow_context, execution_node)
            .await
            .map_err(|err| WorkflowExecutionError::Node { id: node.id, err })?;

        for next in &node.next {
            let n = workflow
                .nodes
                .iter()
                .find(|n| n.id == *next)
                .ok_or(WorkflowExecutionError::NodeNotFound)?;
            queue.push_back(n);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn execute_node(
    node: &WorkflowNode,
    workflow_context: &WorkflowContext<'_>,
    execution_node: &mut WorkflowNodeExecution,
) -> Result<(), WorkflowNodeExecutionError> {
    match &node.metadata {
        ActionMetadata::Start => {
            for input in &execution_node.inputs {
                execution_node
                    .outputs
                    .insert(input.0.clone(), input.1.clone());
            }
        }
        ActionMetadata::SendMessage(action::SendMessage { message, channel }) => {
            let mut new_message = message.clone();
            for template_match in TEMPLATE_REGEX.find_iter(message) {
                let slug = template_match.as_str();
                if let Some(input) = execution_node.inputs.get(&slug[1..slug.len() - 1]) {
                    new_message.replace_range(template_match.range(), &input.to_string());
                }
            }
            workflow_context
                .bot
                .create_message(channel.0)
                .content(&new_message)
                .await?;
        }
        ActionMetadata::JoinString => {
            let mut final_string = String::new();
            for input in &node.inputs {
                final_string.push_str(&execution_node.inputs.get(&input.name).unwrap().to_string());
            }
            let output = node
                .outputs
                .first()
                .ok_or(WorkflowNodeExecutionError::OutputNotFound)?;
            execution_node
                .outputs
                .insert(output.name.clone(), Value::String(final_string));
        }
        ActionMetadata::GetDatastoreEntry => {
            let universe_id = match execution_node
                .inputs
                .get("universe_id")
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?
            {
                #[allow(clippy::cast_sign_loss)]
                Value::Number(n) => UniverseId((*n) as u64),
                Value::String(_) => return Err(WorkflowNodeExecutionError::InputTypeMismatch),
            };
            let datastore_id = execution_node
                .inputs
                .get("datastore_id")
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?
                .to_string();
            let entry_id = execution_node
                .inputs
                .get("entry_id")
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?
                .to_string();

            let entry = workflow_context
                .roblox
                .get_datastore_entry(universe_id, &datastore_id, &entry_id, None)
                .await?;

            for output in &node.outputs {
                let mut current = &entry.value;
                for path in output.name.split('.') {
                    current = current
                        .get(path)
                        .ok_or(WorkflowNodeExecutionError::IncorrectOutputFormat)?;
                }
                let output_value = match output.value {
                    ValueType::Number => Value::Number(
                        current
                            .as_i64()
                            .ok_or(WorkflowNodeExecutionError::IncorrectOutputFormat)?,
                    ),
                    ValueType::String => Value::String(
                        current
                            .as_str()
                            .ok_or(WorkflowNodeExecutionError::IncorrectOutputFormat)?
                            .to_string(),
                    ),
                };
                execution_node
                    .outputs
                    .insert(output.name.clone(), output_value);
            }
        }
        ActionMetadata::Add => {
            let mut sum = 0i64;
            for input in &node.inputs {
                let value = execution_node.inputs.get(&input.name).unwrap();
                match value {
                    Value::Number(n) => {
                        sum = sum.wrapping_add(*n);
                    }
                    Value::String(_) => return Err(WorkflowNodeExecutionError::InputTypeMismatch),
                }
            }
            let output = node
                .outputs
                .first()
                .ok_or(WorkflowNodeExecutionError::OutputNotFound)?;
            execution_node
                .outputs
                .insert(output.name.clone(), Value::Number(sum));
        }
        ActionMetadata::UpdateDatastoreEntry => {
            let universe_id = match execution_node
                .inputs
                .get("universe_id")
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?
            {
                #[allow(clippy::cast_sign_loss)]
                Value::Number(n) => UniverseId((*n) as u64),
                Value::String(_) => return Err(WorkflowNodeExecutionError::InputTypeMismatch),
            };
            let datastore_id = execution_node
                .inputs
                .get("datastore_id")
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?
                .to_string();
            let entry_id = execution_node
                .inputs
                .get("entry_id")
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?
                .to_string();

            let mut entry = workflow_context
                .roblox
                .get_datastore_entry(universe_id, &datastore_id, &entry_id, None)
                .await?;

            for input in &node.inputs {
                if matches!(
                    input.name.as_str(),
                    "universe_id" | "datastore_id" | "entry_id"
                ) {
                    continue;
                }

                let mut current = &mut entry.value;
                for path in input.name.split('.') {
                    current = current
                        .get_mut(path)
                        .ok_or(WorkflowNodeExecutionError::IncorrectInputFormat)?;
                }
                let new_value = execution_node.inputs.get(&input.name).unwrap();
                *current = match new_value {
                    Value::Number(n) => serde_json::json!(*n),
                    Value::String(s) => serde_json::json!(*s),
                };
            }

            let new_entry = workflow_context
                .roblox
                .update_datastore_entry(
                    universe_id,
                    &datastore_id,
                    &entry_id,
                    UpdateDatastoreEntryArgs {
                        value: entry.value,
                        users: entry
                            .users
                            .into_iter()
                            .map(|u| UserId(u.parse().unwrap()))
                            .collect(),
                        attributes: Some(entry.attributes),
                    },
                )
                .await?;

            for output in &node.outputs {
                let mut current = &new_entry.value;
                for path in output.name.split('.') {
                    current = current
                        .get(path)
                        .ok_or(WorkflowNodeExecutionError::IncorrectOutputFormat)?;
                }
                let output_value = match output.value {
                    ValueType::Number => Value::Number(
                        current
                            .as_i64()
                            .ok_or(WorkflowNodeExecutionError::IncorrectOutputFormat)?,
                    ),
                    ValueType::String => Value::String(
                        current
                            .as_str()
                            .ok_or(WorkflowNodeExecutionError::IncorrectOutputFormat)?
                            .to_string(),
                    ),
                };
                execution_node
                    .outputs
                    .insert(output.name.clone(), output_value);
            }
        }
    }

    Ok(())
}

impl From<twilight_http::Error> for WorkflowNodeExecutionError {
    fn from(err: twilight_http::Error) -> Self {
        Self::Discord(err)
    }
}

impl From<RobloxError> for WorkflowNodeExecutionError {
    fn from(err: RobloxError) -> Self {
        Self::Roblox(err)
    }
}
