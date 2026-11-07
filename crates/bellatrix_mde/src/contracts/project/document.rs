use super::ProjectError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};

fn schema_version() -> u32 {
    1
}

/// Project identity outlives its path, its window and any engine generation.
/// Imported identities are opaque, including `/`; never use them as filesystem paths or URLs.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    pub fn new(id: impl Into<String>) -> Result<Self, ProjectError> {
        let id = id.into();
        if id.is_empty()
            || id.len() > 128
            || !id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(byte, b'-' | b'_' | b'.' | b'+' | b'/' | b'=')
            })
        {
            return Err(ProjectError::InvalidId);
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ProjectId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        Self::new(id).map_err(serde::de::Error::custom)
    }
}

/// Persistent platform envelope. Engine sections are opaque to the platform.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectDocument {
    // A reopened identity must not accept queued work from the previous session.
    // This token is shared across revisions and never enters the persisted contract.
    #[serde(skip)]
    session: Arc<()>,
    #[serde(default = "schema_version")]
    schema_version: u32,
    id: ProjectId,
    // Imported documents may have no name; a display fallback must not alter their persisted data.
    name: String,
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    sections: BTreeMap<String, Value>,
}

impl ProjectDocument {
    pub fn new(id: ProjectId, name: impl Into<String>) -> Result<Self, ProjectError> {
        let doc = Self {
            session: Arc::new(()),
            schema_version: schema_version(),
            id,
            name: name.into(),
            revision: 0,
            sections: BTreeMap::new(),
        };
        doc.validate()?;
        Ok(doc)
    }

    pub fn session_identity(&self) -> Arc<()> {
        Arc::clone(&self.session)
    }

    /// Reset a detached document before admitting it to a runtime session.
    /// This cannot change an immutable snapshot already held by a service.
    pub fn begin_session(&mut self) {
        self.session = Arc::new(());
    }

    pub fn id(&self) -> &ProjectId {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn section(&self, namespace: &str) -> Option<&Value> {
        self.sections.get(namespace)
    }
    pub fn sections(&self) -> &BTreeMap<String, Value> {
        &self.sections
    }

    /// Construct an *unopened* document for import. Live edits use Cordis requests.
    pub fn with_section(
        mut self,
        namespace: impl Into<String>,
        value: Value,
    ) -> Result<Self, ProjectError> {
        let namespace = namespace.into();
        validate_namespace(&namespace)?;
        self.sections.insert(namespace, value);
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), ProjectError> {
        ProjectId::new(self.id.as_str())?;
        if self.schema_version != schema_version() {
            return Err(ProjectError::Schema(self.schema_version));
        }
        for namespace in self.sections.keys() {
            validate_namespace(namespace)?;
        }
        Ok(())
    }

    /// Advance a detached snapshot; only the owning service can publish it
    /// as live state after checking the expected revision and session.
    pub fn replace(&mut self, namespace: String, value: Value) -> Result<bool, ProjectError> {
        validate_namespace(&namespace)?;
        if self.sections.get(&namespace) == Some(&value) {
            return Ok(false);
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(ProjectError::RevisionOverflow)?;
        self.sections.insert(namespace, value);
        Ok(true)
    }
}

fn validate_namespace(value: &str) -> Result<(), ProjectError> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    {
        return Err(ProjectError::InvalidNamespace);
    }
    Ok(())
}
