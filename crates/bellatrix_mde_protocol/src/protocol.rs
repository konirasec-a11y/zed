pub mod command;
pub mod presentation;
pub mod trace;
pub mod inspector;
pub mod tools;
pub mod explorer;
pub mod projection;

pub use command::*;
pub use tools::ToolCatalog;

/// Wire namespace for the universal model section in an integration document.
pub const MODELING_NAMESPACE: &str = "bellatrix.mde";

/// Canonical model identity and opaque section JSON, not a second semantic model.
#[derive(Clone, Debug)]
pub struct ModelDocumentParts {
    pub id: String,
    pub name: String,
    pub section_json: String,
}
