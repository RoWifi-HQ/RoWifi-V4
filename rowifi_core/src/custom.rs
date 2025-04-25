use regex::Regex;
use rowifi_database::{Database, DatabaseError};
use rowifi_models::{
    custom::{
        action, ActionInputSource, ActionMetadata, ActionType, Value, ValueType, Workflow,
        WorkflowNode,
    },
    id::{GuildId, UserId},
    roblox::id::{UniverseId, UserId as RobloxUserId},
    user::RoUser,
};
use rowifi_roblox::{
    error::{ErrorKind, RobloxError},
    RobloxClient, UpdateDatastoreEntryArgs,
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet, VecDeque},
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
    pub database: &'a Database,
    pub guild_id: GuildId,
}

pub struct WorkflowNodeExecution {
    pub id: usize,
    pub inputs: HashMap<String, Value>,
    pub outputs: HashMap<String, Value>,
}

pub struct WorkflowExecutionNodeResult {
    pub outcome: String,
}

#[derive(Debug)]
pub enum WorkflowExecutionError {
    NodeNotFound {
        id: usize,
    },
    NodeTypeNotFound {
        kind: ActionType,
    },
    InputNotFound {
        id: usize,
        name: String,
    },
    OutputNotFound {
        id: usize,
        name: String,
        source_id: usize,
        source_name: String,
    },
    IncorrectInputType,
    Node {
        id: usize,
        err: WorkflowNodeExecutionError,
    },
}

pub struct ValidationContext {
    pub nodes: Vec<WorkflowNodeValidation>,
}

pub struct WorkflowNodeValidation {
    pub id: usize,
    pub inputs: HashSet<String>,
    pub outputs: HashSet<String>,
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
    Database(DatabaseError),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum WorkflowNodeValidationError {
    InputNotFound { name: String },
    OutputNotFound,
    IncorrectInputs { actual: u32, expected: u32 },
    IncorrectOutputs { actual: u32, expected: u32 },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum WorkflowValidationError {
    NodeNotFound {
        id: usize,
    },
    NodeTypeNotFound {
        kind: ActionType,
    },
    OutputNotFound {
        id: usize,
        name: String,
        source_id: usize,
        source_name: String,
    },
    IncorrectInputType,
    Node {
        id: usize,
        err: WorkflowNodeValidationError,
    },
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
        .ok_or(WorkflowExecutionError::NodeTypeNotFound {
            kind: ActionType::Start,
        })?;

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
                        .ok_or(WorkflowExecutionError::NodeNotFound { id: *action_id })?
                        .outputs
                        .get(output_name)
                        .ok_or(WorkflowExecutionError::OutputNotFound {
                            id: node.id,
                            name: input.name.clone(),
                            source_id: *action_id,
                            source_name: output_name.clone(),
                        })?;
                    output.clone()
                }
                ActionInputSource::External(_v) => {
                    let input_value =
                        args.get(&input.name)
                            .ok_or(WorkflowExecutionError::InputNotFound {
                                id: node.id,
                                name: input.name.clone(),
                            })?;
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

        let result = execute_node(node, workflow_context, execution_node)
            .await
            .map_err(|err| WorkflowExecutionError::Node { id: node.id, err })?;

        let next_node = node.next.get(&result.outcome).copied();
        if let Some(next) = next_node {
            queue.push_back(
                workflow
                    .nodes
                    .iter()
                    .find(|n| n.id == next)
                    .ok_or(WorkflowExecutionError::NodeNotFound { id: next })?,
            );
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn execute_node(
    node: &WorkflowNode,
    workflow_context: &WorkflowContext<'_>,
    execution_node: &mut WorkflowNodeExecution,
) -> Result<WorkflowExecutionNodeResult, WorkflowNodeExecutionError> {
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
                            .map(|u| RobloxUserId(u.parse().unwrap()))
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
        ActionMetadata::GetRoWifiUser => {
            let (_, user_id) = execution_node
                .inputs
                .iter()
                .next()
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?;
            let user_id = match user_id {
                Value::Number(n) => UserId::new(*n as u64),
                Value::String(_) => return Err(WorkflowNodeExecutionError::IncorrectInputFormat),
            };

            let user = workflow_context
                .database
                .query_opt::<RoUser>("SELECT * FROM roblox_users WHERE user_id = $1", &[&user_id])
                .await?;
            if let Some(user) = user {
                execution_node.outputs.insert(
                    "discord_id".into(),
                    Value::Number(user.user_id.get() as i64),
                );
                let roblox_id = user
                    .linked_accounts
                    .get(&workflow_context.guild_id)
                    .copied()
                    .unwrap_or(user.default_account_id);
                execution_node
                    .outputs
                    .insert("roblox_id".into(), Value::Number(roblox_id.0 as i64));

                return Ok(WorkflowExecutionNodeResult {
                    outcome: "success".into(),
                });
            } else {
                return Ok(WorkflowExecutionNodeResult {
                    outcome: "failure".into(),
                });
            }
        }
        ActionMetadata::PublishUniverseMessage => {
            let universe_id = match execution_node
                .inputs
                .get("universe_id")
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?
            {
                #[allow(clippy::cast_sign_loss)]
                Value::Number(n) => UniverseId((*n) as u64),
                Value::String(_) => return Err(WorkflowNodeExecutionError::InputTypeMismatch),
            };
            let topic = execution_node
                .inputs
                .get("topic")
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?
                .to_string();
            let message = execution_node
                .inputs
                .get("message")
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?
                .to_string();

            if let Err(err) = workflow_context
                .roblox
                .publish_universe_message(universe_id, &topic, &message)
                .await
            {
                tracing::error!(err = ?err);
                return Ok(WorkflowExecutionNodeResult {
                    outcome: "failure".into(),
                });
            } else {
                return Ok(WorkflowExecutionNodeResult {
                    outcome: "success".into(),
                });
            }
        }
        ActionMetadata::GetIdFromUsername => {
            let (_, username) = execution_node
                .inputs
                .iter()
                .next()
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?;
            let username = match username {
                Value::Number(_) => return Err(WorkflowNodeExecutionError::IncorrectInputFormat),
                Value::String(s) => s,
            };
            let output_name = node
                .outputs
                .iter()
                .next()
                .ok_or(WorkflowNodeExecutionError::OutputNotFound)?;

            let roblox_user = workflow_context
                .roblox
                .get_users_from_usernames([username.as_str()].into_iter())
                .await?;
            if roblox_user.is_empty() {
                return Ok(WorkflowExecutionNodeResult {
                    outcome: "failure".into(),
                });
            }

            execution_node.outputs.insert(
                output_name.name.clone(),
                Value::Number(roblox_user[0].id.0 as i64),
            );
            return Ok(WorkflowExecutionNodeResult {
                outcome: "success".into(),
            });
        }
        ActionMetadata::GetUsernameFromId => {
            let (_, user_id) = execution_node
                .inputs
                .iter()
                .next()
                .ok_or(WorkflowNodeExecutionError::InputNotFound)?;
            let user_id = match user_id {
                Value::Number(n) => *n as u64,
                Value::String(_) => return Err(WorkflowNodeExecutionError::IncorrectInputFormat),
            };
            let output_name = node
                .outputs
                .iter()
                .next()
                .ok_or(WorkflowNodeExecutionError::OutputNotFound)?;

            let roblox_user = match workflow_context
                .roblox
                .get_user(RobloxUserId(user_id))
                .await
            {
                Ok(u) => u,
                Err(err) => {
                    if let ErrorKind::Response {
                        route: _,
                        status,
                        bytes: _,
                    } = err.kind()
                    {
                        if status.as_u16() == 404 {
                            return Ok(WorkflowExecutionNodeResult {
                                outcome: "failure".into(),
                            });
                        }
                    }
                    return Err(err.into());
                }
            };

            execution_node
                .outputs
                .insert(output_name.name.clone(), Value::String(roblox_user.name));
            return Ok(WorkflowExecutionNodeResult {
                outcome: "success".into(),
            });
        }
    }

    Ok(WorkflowExecutionNodeResult {
        outcome: "next".into(),
    })
}

pub fn validate_workflow(
    workflow: &Workflow,
    execution_context: &mut ValidationContext,
) -> Result<(), WorkflowValidationError> {
    let start = workflow
        .nodes
        .iter()
        .find(|n| n.kind == ActionType::Start)
        .ok_or(WorkflowValidationError::NodeTypeNotFound {
            kind: ActionType::Start,
        })?;

    let mut queue = VecDeque::new();
    queue.push_back(start);
    while let Some(node) = queue.pop_front() {
        for input in &node.inputs {
            match &input.source {
                ActionInputSource::Static(_) => {}
                ActionInputSource::Action {
                    action_id,
                    output_name,
                } => {
                    let Some(previous_node) =
                        execution_context.nodes.iter().find(|n| n.id == *action_id)
                    else {
                        return Err(WorkflowValidationError::NodeNotFound { id: *action_id });
                    };
                    if !previous_node.outputs.contains(output_name) {
                        return Err(WorkflowValidationError::OutputNotFound {
                            id: node.id,
                            name: input.name.clone(),
                            source_id: *action_id,
                            source_name: output_name.clone(),
                        });
                    }
                }
                ActionInputSource::External(_v) => {}
            };
            let execution_node = execution_context
                .nodes
                .iter_mut()
                .find(|n| n.id == node.id)
                .unwrap();
            execution_node.inputs.insert(input.name.clone());
        }

        let execution_node = execution_context
            .nodes
            .iter_mut()
            .find(|n| n.id == node.id)
            .unwrap();

        validate_node(node, execution_node)
            .map_err(|err| WorkflowValidationError::Node { id: node.id, err })?;

        for next in node.next.values() {
            queue.push_back(
                workflow
                    .nodes
                    .iter()
                    .find(|n| n.id == *next)
                    .ok_or(WorkflowValidationError::NodeNotFound { id: *next })?,
            );
        }
    }

    Ok(())
}

pub fn validate_node(
    node: &WorkflowNode,
    execution_node: &mut WorkflowNodeValidation,
) -> Result<(), WorkflowNodeValidationError> {
    match &node.metadata {
        ActionMetadata::Start => {
            for input in &execution_node.inputs {
                execution_node.outputs.insert(input.clone());
            }
        }
        ActionMetadata::SendMessage(_) => {}
        ActionMetadata::JoinString => {
            if node.inputs.is_empty() {
                return Err(WorkflowNodeValidationError::IncorrectInputs {
                    actual: 0,
                    expected: 1,
                });
            }
            if node.outputs.len() != 1 {
                return Err(WorkflowNodeValidationError::IncorrectOutputs {
                    actual: node.outputs.len() as u32,
                    expected: 1,
                });
            }
            execution_node.outputs.insert(node.outputs[0].name.clone());
        }
        ActionMetadata::GetDatastoreEntry => {
            if !execution_node.inputs.contains("universe_id") {
                return Err(WorkflowNodeValidationError::InputNotFound {
                    name: "universe_id".to_string(),
                });
            }
            if !execution_node.inputs.contains("datastore_id") {
                return Err(WorkflowNodeValidationError::InputNotFound {
                    name: "datastore_id".to_string(),
                });
            }
            if !execution_node.inputs.contains("entry_id") {
                return Err(WorkflowNodeValidationError::InputNotFound {
                    name: "entry_id".to_string(),
                });
            }

            for output in &node.outputs {
                execution_node.outputs.insert(output.name.clone());
            }
        }
        ActionMetadata::Add => {
            if node.inputs.is_empty() {
                return Err(WorkflowNodeValidationError::IncorrectInputs {
                    actual: 0,
                    expected: 1,
                });
            }
            if node.outputs.len() != 1 {
                return Err(WorkflowNodeValidationError::IncorrectOutputs {
                    actual: node.outputs.len() as u32,
                    expected: 1,
                });
            }
            execution_node.outputs.insert(node.outputs[0].name.clone());
        }
        ActionMetadata::UpdateDatastoreEntry => {
            if !execution_node.inputs.contains("universe_id") {
                return Err(WorkflowNodeValidationError::InputNotFound {
                    name: "universe_id".to_string(),
                });
            }
            if !execution_node.inputs.contains("datastore_id") {
                return Err(WorkflowNodeValidationError::InputNotFound {
                    name: "datastore_id".to_string(),
                });
            }
            if !execution_node.inputs.contains("entry_id") {
                return Err(WorkflowNodeValidationError::InputNotFound {
                    name: "entry_id".to_string(),
                });
            }

            if execution_node.inputs.len() <= 3 {
                return Err(WorkflowNodeValidationError::IncorrectInputs {
                    actual: execution_node.inputs.len() as u32,
                    expected: 4,
                });
            }

            for output in &node.outputs {
                execution_node.outputs.insert(output.name.clone());
            }
        }
        ActionMetadata::GetRoWifiUser => {
            if execution_node.inputs.len() != 1 {
                return Err(WorkflowNodeValidationError::IncorrectInputs {
                    actual: execution_node.inputs.len() as u32,
                    expected: 1,
                });
            }

            execution_node.outputs.insert("roblox_id".into());
            execution_node.outputs.insert("discord_id".into());
        }
        ActionMetadata::PublishUniverseMessage => {
            if !execution_node.inputs.contains("universe_id") {
                return Err(WorkflowNodeValidationError::InputNotFound {
                    name: "universe_id".to_string(),
                });
            }
            if !execution_node.inputs.contains("topic") {
                return Err(WorkflowNodeValidationError::InputNotFound {
                    name: "topic".to_string(),
                });
            }
            if !execution_node.inputs.contains("message") {
                return Err(WorkflowNodeValidationError::InputNotFound {
                    name: "message".to_string(),
                });
            }
        }
        ActionMetadata::GetIdFromUsername | ActionMetadata::GetUsernameFromId => {
            if execution_node.inputs.len() != 1 {
                return Err(WorkflowNodeValidationError::IncorrectInputs {
                    actual: execution_node.inputs.len() as u32,
                    expected: 1,
                });
            }

            if execution_node.outputs.len() != 1 {
                return Err(WorkflowNodeValidationError::IncorrectOutputs {
                    actual: execution_node.outputs.len() as u32,
                    expected: 1,
                });
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

impl From<DatabaseError> for WorkflowNodeExecutionError {
    fn from(err: DatabaseError) -> Self {
        Self::Database(err)
    }
}
