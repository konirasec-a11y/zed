//! Headless graph operations and deterministic artifact planning.
mod graph;
mod inspector;
mod quick_edit;
mod index;
mod ownership;
mod path;
mod plan;
mod publish;
mod scope;
mod uml_types;
mod uml_geometry;

pub use graph::{Graph, GraphError};
pub use inspector::{ElementInspector, InspectorMember, InspectorProperty, PropertyEditor, inspect_element, set_inspected_property};
pub use quick_edit::{QuickEditAction, QuickEditAvailability, QuickEditControl, QuickEditSide, quick_edit_for};
pub use index::{GraphIndex, GraphLookupError};
pub use path::{PathError, physical_path, validate_segment};
pub use plan::{Artifact, Diagnostic, Plan, PlanError};
pub use publish::{publish_new, publish_replace};
pub use scope::{Scope, ScopeError, select_elements};
pub use uml_types::{UmlCapability, UmlQuickCommand, UmlType};
pub use uml_geometry::{ClassifierContents, InterfaceLayout, FoldedNoteLayout, classifier_contents, interface_layout, folded_note_layout};
