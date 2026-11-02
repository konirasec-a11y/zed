//! Headless graph operations and deterministic artifact planning.
mod graph;
mod inspector;
mod index;
mod ownership;
mod path;
mod plan;
mod publish;
mod scope;

pub use graph::{Graph, GraphError};
pub use inspector::{ElementInspector, InspectorMember, InspectorProperty, PropertyEditor, inspect_element, set_inspected_property};
pub use index::{GraphIndex, GraphLookupError};
pub use path::{PathError, physical_path, validate_segment};
pub use plan::{Artifact, Diagnostic, Plan, PlanError};
pub use publish::{publish_new, publish_replace};
pub use scope::{Scope, ScopeError, select_elements};
