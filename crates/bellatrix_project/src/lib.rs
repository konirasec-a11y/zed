//! Project domain state: Cordis owns composability; this crate owns document semantics.
mod document;
mod error;
mod events;
mod file;
mod plugin;
mod store;

pub use document::{ProjectDocument, ProjectId};
pub use error::ProjectError;
pub use events::*;
pub use plugin::ProjectPlugin;
pub use store::{ProjectSaveTicket, ProjectService};
