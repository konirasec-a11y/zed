//! Disposable evaluation records. Persistent modeling state is model::ModelingSection.
pub use crate::contracts::model::*;
mod project;
mod section;
pub mod semantic;
pub use project::Project;
pub(crate) use project::deserialize_hierarchy;
pub use section::ModelingSection;
pub use semantic::{Element, Relation};
