pub mod command;
mod command_metadata;
pub mod explorer;
pub mod inspector;
pub mod presentation;
pub mod projection;
pub mod tools;
pub mod trace;

pub use command::*;
pub use tools::ToolCatalog;

/// Wire namespace for the universal model section in an integration document.
pub use crate::MODELING_NAMESPACE;

/// Canonical model identity and opaque section JSON, not a second semantic model.
#[derive(Clone, Debug)]
pub struct ModelDocumentParts {
    pub id: String,
    pub name: String,
    pub section_json: String,
}
