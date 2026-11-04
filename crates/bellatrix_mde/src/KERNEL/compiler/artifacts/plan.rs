use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub element: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Artifact {
    pub path: PathBuf,
    pub contents: Option<Vec<u8>>,
    pub origin: String,
}

#[derive(Default, Debug)]
pub struct Plan {
    artifacts: BTreeMap<String, Artifact>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Error)]
pub enum PlanError {
    #[error("invalid relative artifact path: {0}")]
    Path(String),
    #[error("artifact collision at {path}: {existing} vs {incoming}")]
    Collision {
        path: String,
        existing: String,
        incoming: String,
    },
}

impl Plan {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn artifacts(&self) -> impl Iterator<Item = &Artifact> {
        self.artifacts.values()
    }
    pub fn count(&self) -> usize {
        self.artifacts.len()
    }
    pub fn warn(&mut self, code: &str, message: impl Into<String>, element: Option<&str>) {
        self.diagnostics.push(Diagnostic {
            code: code.to_owned(),
            message: message.into(),
            element: element.map(str::to_owned),
        });
    }
    pub fn insert(
        &mut self,
        path: PathBuf,
        contents: Option<Vec<u8>>,
        origin: impl Into<String>,
    ) -> Result<(), PlanError> {
        let origin = origin.into();
        let mut parts = Vec::new();
        for part in path.components() {
            match part {
                Component::Normal(seg) => {
                    let segment = seg.to_string_lossy().to_string();
                    crate::validate_segment(&segment)
                        .map_err(|_| PlanError::Path(path.display().to_string()))?;
                    parts.push(segment);
                }
                _ => return Err(PlanError::Path(path.display().to_string())),
            }
        }
        if parts.is_empty()
            || (parts.len() == 1 && parts[0].eq_ignore_ascii_case(".bellatrix-generated.json"))
        {
            return Err(PlanError::Path(path.display().to_string()));
        }
        // Enforce case-insensitive collision safety for Windows too.
        let key = parts.join("/").to_lowercase();
        if let Some(existing) = self.artifacts.get(&key) {
            return Err(PlanError::Collision {
                path: path.display().to_string(),
                existing: existing.origin.clone(),
                incoming: origin,
            });
        }
        for (other_key, other) in &self.artifacts {
            if (key.starts_with(&format!("{other_key}/")) && other.contents.is_some())
                || (other_key.starts_with(&format!("{key}/")) && contents.is_some())
            {
                return Err(PlanError::Collision {
                    path: path.display().to_string(),
                    existing: other.origin.clone(),
                    incoming: origin,
                });
            }
        }
        self.artifacts.insert(
            key,
            Artifact {
                path,
                contents,
                origin,
            },
        );
        Ok(())
    }
    pub fn file(
        &mut self,
        path: impl AsRef<Path>,
        bytes: Vec<u8>,
        origin: &str,
    ) -> Result<(), PlanError> {
        self.insert(path.as_ref().to_path_buf(), Some(bytes), origin)
    }
    pub fn directory(&mut self, path: impl AsRef<Path>, origin: &str) -> Result<(), PlanError> {
        self.insert(path.as_ref().to_path_buf(), None, origin)
    }
}
