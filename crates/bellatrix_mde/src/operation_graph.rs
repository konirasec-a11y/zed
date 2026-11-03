use crate::{Plan, PlanError};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use thiserror::Error;

/// Convenient constructors for the built-in, single-output operations.
#[derive(Clone, Debug)]
pub enum Operation {
    RootDirectory,
    Directory { name: String },
    RenderedContent { bytes: Vec<u8> },
    File { name: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortType {
    Directory,
    Content,
    File,
    Text,
}

/// Values exchanged in memory between operations. Directory and file paths are logical.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationValue {
    Directory(PathBuf),
    Content(Vec<u8>),
    File(PathBuf),
    Text(String),
}

impl OperationValue {
    pub fn port_type(&self) -> PortType {
        match self {
            Self::Directory(_) => PortType::Directory,
            Self::Content(_) => PortType::Content,
            Self::File(_) => PortType::File,
            Self::Text(_) => PortType::Text,
        }
    }
}

pub type OperationValues = BTreeMap<String, OperationValue>;
/// Host-registered implementations must be pure and deterministic. The registry
/// is not a sandbox and cannot enforce this contract on arbitrary Rust code.
pub type OperationImplementation = fn(
    &str,
    &OperationValues,
    &OperationValues,
    &mut Plan,
) -> Result<OperationValue, OperationGraphError>;

#[derive(Clone, Debug)]
pub struct InputPort {
    pub name: String,
    pub port_type: PortType,
}

/// A versioned operation has named inputs and exactly one output.
#[derive(Clone)]
pub struct OperationDefinition {
    pub kind: String,
    pub version: u32,
    pub inputs: Vec<InputPort>,
    pub output: PortType,
    pub execute: OperationImplementation,
}

#[derive(Debug, Error)]
pub enum OperationGraphError {
    #[error("duplicate operation: {0}")]
    DuplicateOperation(String),
    #[error("unknown operation: {0}")]
    UnknownOperation(String),
    #[error("duplicate operation definition {kind} version {version}")]
    DuplicateDefinition { kind: String, version: u32 },
    #[error("unknown operation definition {kind} version {version}")]
    UnknownDefinition { kind: String, version: u32 },
    #[error("duplicate input port {port} in operation definition {kind} version {version}")]
    DuplicatePort {
        kind: String,
        version: u32,
        port: String,
    },
    #[error("unknown input port {port} on operation {operation}")]
    UnknownPort { operation: String, port: String },
    #[error("input {operation}.{port} is already connected")]
    DuplicateInput { operation: String, port: String },
    #[error("input {operation}.{port} is missing")]
    MissingInput { operation: String, port: String },
    #[error("input {operation}.{port} expects {expected:?}, but {source_id} outputs {actual:?}")]
    TypeMismatch {
        operation: String,
        port: String,
        expected: PortType,
        source_id: String,
        actual: PortType,
    },
    #[error("operation {operation} returned {actual:?}; its definition declares {expected:?}")]
    OutputTypeMismatch {
        operation: String,
        expected: PortType,
        actual: PortType,
    },
    #[error("invalid operation graph: {0}")]
    InvalidGraph(String),
    #[error("invalid operation arguments: {0}")]
    InvalidArguments(String),
    #[error("operation dependencies contain a cycle; blocked operations: {0}")]
    Cycle(String),
    #[error(transparent)]
    Plan(#[from] PlanError),
}

struct Node {
    definition: (String, u32),
    parameters: OperationValues,
}

pub struct OperationGraph {
    definitions: BTreeMap<(String, u32), OperationDefinition>,
    operations: BTreeMap<String, Node>,
    inputs: BTreeMap<(String, String), String>,
}

impl Default for OperationGraph {
    fn default() -> Self {
        let definitions = builtin_definitions()
            .into_iter()
            .map(|definition| ((definition.kind.clone(), definition.version), definition))
            .collect();
        Self {
            definitions,
            operations: BTreeMap::new(),
            inputs: BTreeMap::new(),
        }
    }
}

impl OperationGraph {
    fn definition(&self, node: &Node) -> Result<&OperationDefinition, OperationGraphError> {
        self.definitions.get(&node.definition).ok_or_else(|| {
            OperationGraphError::UnknownDefinition {
                kind: node.definition.0.clone(),
                version: node.definition.1,
            }
        })
    }

    pub fn new() -> Self {
        Self::default()
    }

    /// Register trusted host code. Each definition has one output, not multiple outputs.
    pub fn register(&mut self, definition: OperationDefinition) -> Result<(), OperationGraphError> {
        let key = (definition.kind.clone(), definition.version);
        if self.definitions.contains_key(&key) {
            return Err(OperationGraphError::DuplicateDefinition {
                kind: key.0,
                version: key.1,
            });
        }
        let mut names = BTreeSet::new();
        for port in &definition.inputs {
            if !names.insert(&port.name) {
                return Err(OperationGraphError::DuplicatePort {
                    kind: key.0,
                    version: key.1,
                    port: port.name.clone(),
                });
            }
        }
        self.definitions.insert(key, definition);
        Ok(())
    }

    pub fn add(
        &mut self,
        id: impl Into<String>,
        operation: Operation,
    ) -> Result<(), OperationGraphError> {
        let (kind, parameters) = match operation {
            Operation::RootDirectory => ("root_directory", OperationValues::new()),
            Operation::Directory { name } => (
                "directory",
                [("name".into(), OperationValue::Text(name))].into(),
            ),
            Operation::RenderedContent { bytes } => (
                "rendered_content",
                [("bytes".into(), OperationValue::Content(bytes))].into(),
            ),
            Operation::File { name } => {
                ("file", [("name".into(), OperationValue::Text(name))].into())
            }
        };
        self.add_registered(id, kind, 1, parameters)
    }

    pub fn add_registered(
        &mut self,
        id: impl Into<String>,
        kind: &str,
        version: u32,
        parameters: OperationValues,
    ) -> Result<(), OperationGraphError> {
        let id = id.into();
        if self.operations.contains_key(&id) {
            return Err(OperationGraphError::DuplicateOperation(id));
        }
        let key = (kind.to_owned(), version);
        if !self.definitions.contains_key(&key) {
            return Err(OperationGraphError::UnknownDefinition {
                kind: key.0,
                version,
            });
        }
        self.operations.insert(
            id,
            Node {
                definition: key,
                parameters,
            },
        );
        Ok(())
    }

    /// Bind a target input to another operation's single output.
    pub fn connect(
        &mut self,
        source: &str,
        target: &str,
        port: &str,
    ) -> Result<(), OperationGraphError> {
        let source_node = self
            .operations
            .get(source)
            .ok_or_else(|| OperationGraphError::UnknownOperation(source.to_owned()))?;
        let target_node = self
            .operations
            .get(target)
            .ok_or_else(|| OperationGraphError::UnknownOperation(target.to_owned()))?;
        let source_definition = self.definition(source_node)?;
        let target_definition = self.definition(target_node)?;
        let expected = target_definition
            .inputs
            .iter()
            .find(|input| input.name == port)
            .map(|input| input.port_type)
            .ok_or_else(|| OperationGraphError::UnknownPort {
                operation: target.to_owned(),
                port: port.to_owned(),
            })?;
        let actual = source_definition.output;
        if expected != actual {
            return Err(OperationGraphError::TypeMismatch {
                operation: target.to_owned(),
                port: port.to_owned(),
                expected,
                source_id: source.to_owned(),
                actual,
            });
        }
        let key = (target.to_owned(), port.to_owned());
        if self.inputs.contains_key(&key) {
            return Err(OperationGraphError::DuplicateInput {
                operation: target.to_owned(),
                port: port.to_owned(),
            });
        }
        self.inputs.insert(key, source.to_owned());
        Ok(())
    }

    /// Validate dependencies and produce a deterministic in-memory plan.
    pub fn execute(&self) -> Result<Plan, OperationGraphError> {
        let mut remaining = BTreeMap::new();
        let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut ready = BTreeSet::new();
        for (id, node) in &self.operations {
            let inputs = &self.definition(node)?.inputs;
            remaining.insert(id.as_str(), inputs.len());
            if inputs.is_empty() {
                ready.insert(id.as_str());
            }
            for port in inputs {
                let source = self
                    .inputs
                    .get(&(id.clone(), port.name.clone()))
                    .ok_or_else(|| OperationGraphError::MissingInput {
                        operation: id.clone(),
                        port: port.name.clone(),
                    })?;
                dependents
                    .entry(source.as_str())
                    .or_default()
                    .push(id.as_str());
            }
        }
        let mut values: BTreeMap<String, OperationValue> = BTreeMap::new();
        let mut plan = Plan::new();
        while let Some(id) = ready.pop_first() {
            let node = self
                .operations
                .get(id)
                .ok_or_else(|| OperationGraphError::UnknownOperation(id.to_owned()))?;
            let definition = self.definition(node)?;
            let mut inputs = OperationValues::new();
            for port in &definition.inputs {
                let source = self
                    .inputs
                    .get(&(id.to_owned(), port.name.clone()))
                    .ok_or_else(|| OperationGraphError::MissingInput {
                        operation: id.to_owned(),
                        port: port.name.clone(),
                    })?;
                let value = values.get(source).ok_or_else(|| {
                    OperationGraphError::InvalidGraph(format!("dependency {source} has no value"))
                })?;
                inputs.insert(port.name.clone(), value.clone());
            }
            let value = (definition.execute)(id, &inputs, &node.parameters, &mut plan)?;
            let actual = value.port_type();
            if actual != definition.output {
                return Err(OperationGraphError::OutputTypeMismatch {
                    operation: id.to_owned(),
                    expected: definition.output,
                    actual,
                });
            }
            values.insert(id.to_owned(), value);
            remaining.remove(id);
            if let Some(targets) = dependents.get(id) {
                for target in targets {
                    if let Some(count) = remaining.get_mut(target) {
                        *count -= 1;
                        if *count == 0 {
                            ready.insert(*target);
                        }
                    }
                }
            }
        }
        if !remaining.is_empty() {
            return Err(OperationGraphError::Cycle(
                remaining.keys().copied().collect::<Vec<_>>().join(", "),
            ));
        }
        Ok(plan)
    }
}

fn argument<'a>(
    values: &'a OperationValues,
    name: &str,
    expected: PortType,
) -> Result<&'a OperationValue, OperationGraphError> {
    values
        .get(name)
        .filter(|value| value.port_type() == expected)
        .ok_or_else(|| {
            OperationGraphError::InvalidArguments(format!("{name} must be {expected:?}"))
        })
}

fn root(
    _: &str,
    _: &OperationValues,
    _: &OperationValues,
    _: &mut Plan,
) -> Result<OperationValue, OperationGraphError> {
    Ok(OperationValue::Directory(PathBuf::new()))
}
fn content(
    _: &str,
    _: &OperationValues,
    parameters: &OperationValues,
    _: &mut Plan,
) -> Result<OperationValue, OperationGraphError> {
    Ok(argument(parameters, "bytes", PortType::Content)?.clone())
}
fn directory(
    id: &str,
    inputs: &OperationValues,
    parameters: &OperationValues,
    plan: &mut Plan,
) -> Result<OperationValue, OperationGraphError> {
    let OperationValue::Directory(parent) = argument(inputs, "parent", PortType::Directory)? else {
        return Err(OperationGraphError::InvalidArguments(
            "parent must be Directory".into(),
        ));
    };
    let OperationValue::Text(name) = argument(parameters, "name", PortType::Text)? else {
        return Err(OperationGraphError::InvalidArguments(
            "name must be Text".into(),
        ));
    };
    let path = parent.join(name);
    plan.directory(&path, id)?;
    Ok(OperationValue::Directory(path))
}
fn file(
    id: &str,
    inputs: &OperationValues,
    parameters: &OperationValues,
    plan: &mut Plan,
) -> Result<OperationValue, OperationGraphError> {
    let OperationValue::Directory(directory) = argument(inputs, "directory", PortType::Directory)?
    else {
        return Err(OperationGraphError::InvalidArguments(
            "directory must be Directory".into(),
        ));
    };
    let OperationValue::Content(content) = argument(inputs, "content", PortType::Content)? else {
        return Err(OperationGraphError::InvalidArguments(
            "content must be Content".into(),
        ));
    };
    let OperationValue::Text(name) = argument(parameters, "name", PortType::Text)? else {
        return Err(OperationGraphError::InvalidArguments(
            "name must be Text".into(),
        ));
    };
    let path = directory.join(name);
    plan.file(&path, content.clone(), id)?;
    Ok(OperationValue::File(path))
}
fn builtin_definitions() -> Vec<OperationDefinition> {
    fn definition(
        kind: &str,
        inputs: &[(&str, PortType)],
        output: PortType,
        execute: OperationImplementation,
    ) -> OperationDefinition {
        OperationDefinition {
            kind: kind.into(),
            version: 1,
            inputs: inputs
                .iter()
                .map(|(name, port_type)| InputPort {
                    name: (*name).into(),
                    port_type: *port_type,
                })
                .collect(),
            output,
            execute,
        }
    }
    vec![
        definition("root_directory", &[], PortType::Directory, root),
        definition(
            "directory",
            &[("parent", PortType::Directory)],
            PortType::Directory,
            directory,
        ),
        definition("rendered_content", &[], PortType::Content, content),
        definition(
            "file",
            &[
                ("directory", PortType::Directory),
                ("content", PortType::Content),
            ],
            PortType::File,
            file,
        ),
    ]
}
