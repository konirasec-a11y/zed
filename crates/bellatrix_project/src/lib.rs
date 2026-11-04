//! Project domain state: Cordis owns composability; this crate owns document semantics.
mod file;
mod plugin;
mod store;

pub use bellatrix_mde_contracts::project::*;
pub use plugin::ProjectPlugin;
pub use store::ProjectService;
