use crate::{GraphIndex, Project};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    Project,
    Elements(Vec<String>),
}

#[derive(Debug, Error)]
pub enum ScopeError {
    #[error("unknown selected element: {0}")]
    Unknown(String),
}

/// Descends only through ownership, declarations, and physical containment.
/// Dependency/call/reference edges never cause transitive generation.
pub fn select_elements(project: &Project, scope: &Scope) -> Result<HashSet<String>, ScopeError> {
    if matches!(scope, Scope::Project) {
        return Ok(project.elements.iter().map(|e| e.id.clone()).collect());
    }
    let Scope::Elements(ids) = scope else {
        unreachable!()
    };
    let index = GraphIndex::new(project);
    for id in ids {
        if !index.contains(id) {
            return Err(ScopeError::Unknown(id.clone()));
        }
    }
    Ok(index.reachable(
        &ids.iter().map(String::as_str).collect::<Vec<_>>(),
        &["owns", "declares", "physical_parent"],
    ))
}
