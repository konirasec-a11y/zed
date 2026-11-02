//! UML visual elements and in-place editors for the native Zed GPUI runtime.
//! Geometry and capability contracts are headless; this crate only renders them.
mod class;
mod interface;
pub use interface::UmlInterfaceBall;
mod inline_edit;
mod quick_edit;
mod figures;
mod relationship;
pub use figures::UmlFigure;
pub use relationship::{UmlLineSegment, UmlRelationGlyph};
pub use class::{UmlClassCard, UmlClassifier};
pub use inline_edit::UmlInlineNameEditor;
pub use quick_edit::{UmlQuickAction, UmlQuickEdit, UmlVisibilityPicker};
