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
pub use class::UmlClassifier;
pub use figures::UmlFigure;
pub use figure_geometry::{TemplateSignatureLayout, template_signature_layout};
pub use inline_edit::UmlInlineNameEditor;
pub use quick_edit::{UmlQuickAction, UmlQuickEdit, UmlVisibilityPicker};
pub use relationship::{UmlLineSegment, UmlRelationGlyph};
