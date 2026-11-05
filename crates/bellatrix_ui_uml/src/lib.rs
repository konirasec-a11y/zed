//! UML visual elements for GPUI, with optional legacy in-place editors.
//! Domain decisions arrive as read-only projections; interaction state stays in the UI.
mod class;
mod figure_geometry;
mod figures;
#[cfg(feature = "legacy_editors")]
mod inline_edit;
mod interface_presentation;
#[cfg(feature = "legacy_editors")]
mod quick_edit;
mod relationship;
mod timing;
pub use class::{UmlClassifier, ViewAppearance};
pub use figure_geometry::{TemplateSignatureLayout, template_signature_layout};
pub use figures::UmlFigure;
#[cfg(feature = "legacy_editors")]
pub use inline_edit::UmlInlineNameEditor;
pub use interface_presentation::render_interface_icon;
#[cfg(feature = "legacy_editors")]
pub use quick_edit::{UmlQuickAction, UmlQuickEdit, UmlVisibilityPicker};
pub use relationship::{UmlLineSegment, UmlRelationGlyph};
