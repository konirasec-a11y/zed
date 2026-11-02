//! Quick Edit is the context-sensitive command surface for a selected MDE
//! element. It is distinct from the Property Editor (long-lived attribute
//! form) and from the Zed Project Panel (files and worktrees).
//!
//! IDs and ordering below are grounded in the maintainers' StarUML Quick Edit
//! HTML and the StarUML UML quickedits/uml.json reference. We do not copy its
//! JavaScript, templates, visual assets, or command implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickEditControl {
    Input,
    Visibility,
    Button,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickEditAvailability {
    /// A tested/implemented domain command must back the UI control.
    Implemented,
    /// Retain the documented reference action, but do not render as clickable.
    NeedsDomainContract,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickEditSide { Main, Left, Right }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuickEditAction {
    pub id: &'static str,
    pub title: &'static str,
    pub control: QuickEditControl,
    pub side: QuickEditSide,
    pub row: u8,
    pub availability: QuickEditAvailability,
    pub shortcut: Option<&'static str>,
}
use QuickEditAvailability::{Implemented as Ready, NeedsDomainContract as Pending};
use QuickEditControl::{Input, Visibility, Button};
use QuickEditSide::{Main, Left, Right};

const fn act(id:&'static str,title:&'static str,control:QuickEditControl,
    side:QuickEditSide,row:u8,availability:QuickEditAvailability,
    shortcut:Option<&'static str>) -> QuickEditAction {
    QuickEditAction { id,title,control,side,row,availability,shortcut }
}

// `uml.name-expression` is text `<< stereotype >> name` in the
// reference. Bellatrix currently implements the *name* part; stereotype is
// edited through the typed Property Editor until the complete expression
// parser is available.
const CLASS_ACTIONS: &[QuickEditAction] = &[
    act("uml.name-expression","Name expression",Input,Main,0,Ready,None),
    act("uml.visibility","Visibility",Visibility,Left,1,Ready,None),
    act("uml.add-note","Add Note",Button,Left,2,Pending,None),
    act("uml.add-constraint","Add Constraint",Button,Left,2,Pending,None),
    act("uml.add-attribute","Add Attribute",Button,Right,1,Ready,Some("Ctrl+Enter")),
    act("uml.add-operation","Add Operation",Button,Right,1,Ready,Some("Ctrl+Shift+Enter")),
    act("uml.add-template-parameter","Add Template Parameter",Button,Right,1,Pending,None),
    act("uml.add-reception","Add Reception",Button,Right,1,Pending,None),
    act("uml.add-subclass","Add Sub-Class",Button,Right,2,Pending,None),
    act("uml.add-superclass","Add Super-Class",Button,Right,2,Pending,None),
    act("uml.add-provided-interface","Add Provided Interface",Button,Right,2,Pending,None),
    act("uml.add-required-interface","Add Required Interface",Button,Right,2,Pending,None),
    act("uml.add-associated-class","Add Associated Class",Button,Right,3,Pending,None),
    act("uml.add-aggregated-class","Add Aggregated Class",Button,Right,3,Pending,None),
    act("uml.add-composited-class","Add Composited Class",Button,Right,3,Pending,None),
    act("uml.add-port","Add Port",Button,Right,4,Pending,None),
    act("uml.add-part","Add Part",Button,Right,4,Pending,None),
];
const INTERFACE_ACTIONS: &[QuickEditAction] = &[
    act("uml.name-expression","Name expression",Input,Main,0,Ready,None),
    act("uml.visibility","Visibility",Visibility,Left,1,Ready,None),
    act("uml.add-note","Add Note",Button,Left,2,Pending,None),
    act("uml.add-constraint","Add Constraint",Button,Left,2,Pending,None),
    act("uml.add-attribute","Add Attribute",Button,Right,1,Ready,Some("Ctrl+Enter")),
    act("uml.add-operation","Add Operation",Button,Right,1,Ready,Some("Ctrl+Shift+Enter")),
    act("uml.add-reception","Add Reception",Button,Right,1,Pending,None),
    act("uml.add-subinterface","Add Sub-Interface",Button,Right,2,Pending,None),
    act("uml.add-superinterface","Add Super-Interface",Button,Right,2,Pending,None),
    act("uml.add-realizing-class","Add Realizing Class",Button,Right,2,Pending,None),
];
const ATTRIBUTE_ACTIONS: &[QuickEditAction] = &[
    act("uml.attribute-expression","Attribute expression",Input,Main,0,Pending,None),
    act("uml.visibility","Visibility",Visibility,Left,1,Ready,None),
    act("uml.add-compartment-item","Add Attribute",Button,Right,1,Pending,None),
    act("uml.delete-compartment-item","Delete Attribute",Button,Right,1,Pending,None),
    act("uml.move-up-compartment-item","Move Attribute Up",Button,Right,1,Pending,None),
    act("uml.move-down-compartment-item","Move Attribute Down",Button,Right,1,Pending,None),
];
const OPERATION_ACTIONS: &[QuickEditAction] = &[
    act("uml.operation-expression","Operation expression",Input,Main,0,Pending,None),
    act("uml.visibility","Visibility",Visibility,Left,1,Ready,None),
    act("uml.add-compartment-item","Add Operation",Button,Right,1,Pending,None),
    act("uml.delete-compartment-item","Delete Operation",Button,Right,1,Pending,None),
    act("uml.move-up-compartment-item","Move Operation Up",Button,Right,1,Pending,None),
    act("uml.move-down-compartment-item","Move Operation Down",Button,Right,1,Pending,None),
];

/// A type chooses its own commands. Unknown kinds must not silently receive
/// Class actions or a fake generic editor.
pub fn quick_edit_for(kind: &str) -> &'static [QuickEditAction] {
    match kind {
        "rust.struct" | "uml.class" => CLASS_ACTIONS,
        "rust.trait" | "uml.interface" => INTERFACE_ACTIONS,
        "rust.field" | "uml.attribute" => ATTRIBUTE_ACTIONS,
        "rust.function" | "uml.operation" => OPERATION_ACTIONS,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn class_actions_match_staruml_reference_order_and_sections() {
        let a = quick_edit_for("uml.class");
        assert_eq!(a.len(), 17);
        assert_eq!(a[0].id, "uml.name-expression");
        assert_eq!(a[1].id, "uml.visibility");
        assert!(a.iter().any(|action| action.id=="uml.add-attribute" &&
            action.availability==Ready && action.shortcut==Some("Ctrl+Enter")));
        assert!(a.iter().any(|action| action.id=="uml.add-composited-class" &&
            action.side==Right && action.row==3 && action.availability==Pending));
        assert!(a.iter().any(|action| action.id=="uml.add-part" &&
            action.side==Right && action.row==4));
        let ids: HashSet<_> = a.iter().map(|action| action.id).collect();
        assert_eq!(a.len(), ids.len());
    }
    #[test]
    fn specialized_kinds_have_distinct_actions() {
        let field = quick_edit_for("rust.field");
        assert!(field.iter().any(|a| a.id=="uml.attribute-expression"));
        assert!(!field.iter().any(|a| a.id=="uml.add-subclass"));
        assert!(quick_edit_for("fs.file").is_empty());
        assert_eq!(quick_edit_for("rust.struct"), quick_edit_for("uml.class"));
        assert_ne!(quick_edit_for("rust.struct"), quick_edit_for("rust.trait"));
    }
    #[test]
    fn unavailable_commands_never_claim_implementation() {
        let class=quick_edit_for("rust.struct");
        for id in ["uml.add-note","uml.add-constraint","uml.add-subclass",
            "uml.add-template-parameter","uml.add-provided-interface","uml.add-port"] {
            assert!(class.iter().any(|a| a.id==id && a.availability==Pending));
        }
    }
}
