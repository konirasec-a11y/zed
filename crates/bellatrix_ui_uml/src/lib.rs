//! UML visual elements and in-place editors for the native Zed GPUI runtime.
//! Domain Elements/Relations remain in bellatrix_mde_model; this crate owns
//! only presentation, hit targets, inline editing and contextual icon controls.
mod class;
mod inline_edit;
mod quick_edit;
mod figures;
mod relationship;
pub use figures::UmlFigure;
pub use relationship::UmlRelationGlyph;

pub use class::UmlClassCard;
pub use inline_edit::UmlInlineNameEditor;
pub use quick_edit::{UmlQuickAction, UmlQuickEdit, UmlVisibilityPicker};
