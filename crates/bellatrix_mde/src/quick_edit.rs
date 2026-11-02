//! Shared action definitions with type-specific composition, independent of the renderer.
use crate::UmlType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickEditControl { Input, Visibility, Button }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickEditAvailability { Implemented, NeedsDomainContract }
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
const fn act(id: &'static str, title: &'static str, control: QuickEditControl,
    side: QuickEditSide, row: u8, availability: QuickEditAvailability,
    shortcut: Option<&'static str>) -> QuickEditAction {
    QuickEditAction { id, title, control, side, row, availability, shortcut }
}
const NAME: QuickEditAction = act("uml.name-expression", "Name expression", Input, Main, 0, Ready, None);
const VISIBILITY: QuickEditAction = act("uml.visibility", "Visibility", Visibility, Left, 1, Ready, None);
const NOTE: QuickEditAction = act("uml.add-note", "Add Note", Button, Left, 2, Ready, None);
const CONSTRAINT: QuickEditAction = act("uml.add-constraint", "Add Constraint", Button, Left, 2, Ready, None);
const ATTRIBUTE: QuickEditAction = act("uml.add-attribute", "Add Attribute", Button, Right, 1, Ready, Some("Ctrl+Enter"));
const OPERATION: QuickEditAction = act("uml.add-operation", "Add Operation", Button, Right, 1, Ready, Some("Ctrl+Shift+Enter"));
const RECEPTION: QuickEditAction = act("uml.add-reception", "Add Reception", Button, Right, 1, Ready, None);

const CLASS_ACTIONS: &[QuickEditAction] = &[
    NAME, VISIBILITY, NOTE, CONSTRAINT, ATTRIBUTE, OPERATION,
    act("uml.add-template-parameter", "Add Template Parameter", Button, Right, 1, Ready, None),
    RECEPTION,
    act("uml.add-subclass", "Add Sub-Class", Button, Right, 2, Ready, None),
    act("uml.add-superclass", "Add Super-Class", Button, Right, 2, Ready, None),
    act("uml.add-provided-interface", "Add Provided Interface", Button, Right, 2, Ready, None),
    act("uml.add-required-interface", "Add Required Interface", Button, Right, 2, Ready, None),
    act("uml.add-associated-class", "Add Associated Class", Button, Right, 3, Ready, None),
    act("uml.add-aggregated-class", "Add Aggregated Class", Button, Right, 3, Ready, None),
    act("uml.add-composited-class", "Add Composited Class", Button, Right, 3, Ready, None),
    act("uml.add-port", "Add Port", Button, Right, 4, Ready, None),
    act("uml.add-part", "Add Part", Button, Right, 4, Ready, None),
];
const INTERFACE_ACTIONS: &[QuickEditAction] = &[
    NAME, VISIBILITY, NOTE, CONSTRAINT, ATTRIBUTE, OPERATION, RECEPTION,
    act("uml.add-subinterface", "Add Sub-Interface", Button, Right, 2, Ready, None),
    act("uml.add-superinterface", "Add Super-Interface", Button, Right, 2, Ready, None),
    act("uml.add-realizing-class", "Add Realizing Class", Button, Right, 2, Ready, None),
];
const CLASSIFIER_ACTIONS: &[QuickEditAction] = &[NAME, VISIBILITY, NOTE, CONSTRAINT, ATTRIBUTE, OPERATION];
const PACKAGE_ACTIONS: &[QuickEditAction] = &[NAME, NOTE, CONSTRAINT];
const ATTRIBUTE_ACTIONS: &[QuickEditAction] = &[
    act("uml.attribute-expression", "Attribute expression", Input, Main, 0, Pending, None),
    VISIBILITY,
    act("uml.add-compartment-item", "Add Attribute", Button, Right, 1, Pending, None),
    act("uml.delete-compartment-item", "Delete Attribute", Button, Right, 1, Pending, None),
    act("uml.move-up-compartment-item", "Move Attribute Up", Button, Right, 1, Pending, None),
    act("uml.move-down-compartment-item", "Move Attribute Down", Button, Right, 1, Pending, None),
];
const OPERATION_ACTIONS: &[QuickEditAction] = &[
    act("uml.operation-expression", "Operation expression", Input, Main, 0, Pending, None),
    VISIBILITY,
    act("uml.add-compartment-item", "Add Operation", Button, Right, 1, Pending, None),
    act("uml.delete-compartment-item", "Delete Operation", Button, Right, 1, Pending, None),
    act("uml.move-up-compartment-item", "Move Operation Up", Button, Right, 1, Pending, None),
    act("uml.move-down-compartment-item", "Move Operation Down", Button, Right, 1, Pending, None),
];
pub fn quick_edit_for(kind: &str) -> &'static [QuickEditAction] {
    match UmlType::from_kind(kind) {
        Some(UmlType::Class) => CLASS_ACTIONS,
        Some(UmlType::Interface) => INTERFACE_ACTIONS,
        Some(UmlType::Attribute) => ATTRIBUTE_ACTIONS,
        Some(UmlType::Operation) => OPERATION_ACTIONS,
        Some(UmlType::Package | UmlType::Model | UmlType::Subsystem) => PACKAGE_ACTIONS,
        Some(kind) if kind.is_classifier() => CLASSIFIER_ACTIONS,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UmlQuickCommand;
    #[test]
    fn classes_and_interfaces_share_exactly_the_same_common_actions() {
        let class = quick_edit_for("uml.class");
        let interface = quick_edit_for("uml.interface");
        for shared in [NAME, VISIBILITY, NOTE, CONSTRAINT, ATTRIBUTE, OPERATION, RECEPTION] {
            assert!(class.contains(&shared));
            assert!(interface.contains(&shared));
            assert_eq!(shared.availability, Ready);
        }
        assert_eq!(class.len(), 17);
        assert_eq!(interface.len(), 10);
        assert!(class.iter().any(|action| action.id == "uml.add-subclass" && action.availability == Ready));
        assert!(!class.iter().any(|action| action.id == "uml.add-subinterface"));
        assert!(!interface.iter().any(|action| action.id == "uml.add-subclass"));
        assert!(!UmlQuickCommand::AddRealizingClass.accepts("uml.class"));
    }
    #[test]
    fn specialized_kinds_have_distinct_actions() {
        assert!(quick_edit_for("fs.file").is_empty());
        assert_eq!(quick_edit_for("rust.struct"), quick_edit_for("uml.class"));
        assert_eq!(quick_edit_for("rust.trait"), quick_edit_for("uml.interface"));
        assert!(quick_edit_for("uml.package").contains(&NOTE));
        assert!(!quick_edit_for("uml.package").contains(&OPERATION));
    }
    #[test]
    fn class_actions_have_specialized_rows_one_through_four() {
        let actions=quick_edit_for("rust.struct");
        assert_eq!(actions.len(),17);
        assert!(actions.iter().all(|a|a.availability==Ready));
        for (row,ids) in [
            (1,&["uml.add-attribute","uml.add-operation","uml.add-template-parameter","uml.add-reception"][..]),
            (2,&["uml.add-subclass","uml.add-superclass","uml.add-provided-interface","uml.add-required-interface"][..]),
            (3,&["uml.add-associated-class","uml.add-aggregated-class","uml.add-composited-class"][..]),
            (4,&["uml.add-port","uml.add-part"][..]),
        ] {
            assert_eq!(actions.iter().filter(|a|a.side==Right && a.row==row).map(|a|a.id).collect::<Vec<_>>(),ids.to_vec());
        }
    }
}
