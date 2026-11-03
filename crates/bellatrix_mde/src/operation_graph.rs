use crate::{Plan, PlanError};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use thiserror::Error;

/// Operations over logical artifacts. Content is rendered by the caller before
/// building the graph; evaluating this graph performs no filesystem access.
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
}

impl Operation {
    fn output_type(&self) -> PortType {
        match self {
            Self::RootDirectory | Self::Directory { .. } => PortType::Directory,
            Self::RenderedContent { .. } => PortType::Content,
            Self::File { .. } => PortType::File,
        }
    }
    fn inputs(&self) -> &'static [(&'static str, PortType)] {
        match self {
            Self::RootDirectory | Self::RenderedContent { .. } => &[],
            Self::Directory { .. } => &[("parent", PortType::Directory)],
            Self::File { .. } => &[
                ("directory", PortType::Directory),
                ("content", PortType::Content),
            ],
        }
    }
}

#[derive(Debug, Error)]
pub enum OperationGraphError {
    #[error("duplicate operation: {0}")]
    DuplicateOperation(String),
    #[error("unknown operation: {0}")]
    UnknownOperation(String),
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
    #[error("operation dependencies contain a cycle; blocked operations: {0}")]
    Cycle(String),
    #[error(transparent)]
    Plan(#[from] PlanError),
}

#[derive(Default)]
pub struct OperationGraph {
    operations: BTreeMap<String, Operation>,
    inputs: BTreeMap<(String, String), String>,
}

#[derive(Clone)]
enum Value {
    Directory(PathBuf),
    Content(Vec<u8>),
    File,
}

impl OperationGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(
        &mut self,
        id: impl Into<String>,
        operation: Operation,
    ) -> Result<(), OperationGraphError> {
        let id = id.into();
        if self.operations.contains_key(&id) {
            return Err(OperationGraphError::DuplicateOperation(id));
        }
        self.operations.insert(id, operation);
        Ok(())
    }

    /// Bind a target input to the single output of another operation.
    pub fn connect(
        &mut self,
        source: &str,
        target: &str,
        port: &str,
    ) -> Result<(), OperationGraphError> {
        let source_operation = self
            .operations
            .get(source)
            .ok_or_else(|| OperationGraphError::UnknownOperation(source.to_owned()))?;
        let target_operation = self
            .operations
            .get(target)
            .ok_or_else(|| OperationGraphError::UnknownOperation(target.to_owned()))?;
        let expected = target_operation
            .inputs()
            .iter()
            .find(|(name, _)| *name == port)
            .map(|(_, kind)| *kind)
            .ok_or_else(|| OperationGraphError::UnknownPort {
                operation: target.to_owned(),
                port: port.to_owned(),
            })?;
        let actual = source_operation.output_type();
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

    /// Validate the operation dependencies and produce a deterministic in-memory plan.
    pub fn execute(&self) -> Result<Plan, OperationGraphError> {
        let mut remaining = BTreeMap::new();
        let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut ready = BTreeSet::new();
        for (id, operation) in &self.operations {
            let inputs = operation.inputs();
            remaining.insert(id.as_str(), inputs.len());
            if inputs.is_empty() {
                ready.insert(id.as_str());
            }
            for (port, _) in inputs {
                let source = self
                    .inputs
                    .get(&(id.clone(), (*port).to_owned()))
                    .ok_or_else(|| OperationGraphError::MissingInput {
                        operation: id.clone(),
                        port: (*port).to_owned(),
                    })?;
                // Count each port, even if several ports share a source.
                dependents
                    .entry(source.as_str())
                    .or_default()
                    .push(id.as_str());
            }
        }
        let mut values = BTreeMap::new();
        let mut plan = Plan::new();
        while let Some(id) = ready.pop_first() {
            let input =
                |port: &str| -> &Value { &values[&self.inputs[&(id.to_owned(), port.to_owned())]] };
            let value = match &self.operations[id] {
                Operation::RootDirectory => Value::Directory(PathBuf::new()),
                Operation::RenderedContent { bytes } => Value::Content(bytes.clone()),
                Operation::Directory { name } => {
                    let Value::Directory(parent) = input("parent") else {
                        unreachable!("validated port type")
                    };
                    let path = parent.join(name);
                    plan.directory(&path, id)?;
                    Value::Directory(path)
                }
                Operation::File { name } => {
                    let Value::Directory(directory) = input("directory") else {
                        unreachable!("validated port type")
                    };
                    let Value::Content(content) = input("content") else {
                        unreachable!("validated port type")
                    };
                    plan.file(directory.join(name), content.clone(), id)?;
                    Value::File
                }
            };
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
