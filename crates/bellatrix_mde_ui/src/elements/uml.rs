//! UML visual elements and in-place editors for the native Zed GPUI runtime.
//! Domain decisions arrive as read-only projections; interaction state stays in the UI.
mod class;
mod figure_geometry;
mod figures;
mod inline_edit;
mod interface_presentation;
mod quick_edit;
mod relationship;
mod timing;
pub use class::{UmlClassifier, ViewAppearance};
pub use figure_geometry::{TemplateSignatureLayout, template_signature_layout};
pub use figures::UmlFigure;
pub use inline_edit::UmlInlineNameEditor;
pub use interface_presentation::render_interface_icon;
pub use quick_edit::{UmlQuickAction, UmlQuickActionIcon, UmlQuickEdit, UmlVisibilityPicker};
pub use relationship::{UmlLineSegment, UmlRelationGlyph};

mod visual_definition;
pub use visual_definition::UmlVisualDefinition;
