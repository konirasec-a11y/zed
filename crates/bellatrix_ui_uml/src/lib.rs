//! UML visual elements and in-place editors for the native Zed GPUI runtime.
//! Geometry and capability contracts are headless; this crate only renders them.
mod class;
mod figures;
mod inline_edit;
mod quick_edit;
mod relationship;
pub use class::UmlClassifier;
pub use figures::UmlFigure;
pub use inline_edit::UmlInlineNameEditor;
pub use quick_edit::{UmlQuickAction, UmlQuickEdit, UmlVisibilityPicker};
pub use relationship::{UmlLineSegment, UmlRelationGlyph};
