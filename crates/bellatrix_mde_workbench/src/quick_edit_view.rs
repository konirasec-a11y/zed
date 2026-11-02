//! Engine-MDE adapter from typed contextual actions to the reusable UML
//! visuals. All icon layouts and controls are in bellatrix_ui_uml; only this
//! adapter knows the selected MDE type and its Cordis command.
use crate::MdeWorkbenchItem;
use bellatrix_mde_cordis::{ModelingMemberKind, InterfaceQuickAction};
use bellatrix_mde_model::Element;
use bellatrix_mde_modeling::{
    QuickEditAvailability, QuickEditControl, QuickEditSide, quick_edit_for,
};
use bellatrix_ui_uml::{UmlQuickAction, UmlQuickEdit, UmlVisibilityPicker};
use gpui::Entity;
use ui::IconName;

/// These components are deliberately absent outside editing mode.
/// Pending domain actions do not create fake buttons on the Canvas.
pub(crate) fn quick_edit_for_element(
    element: &Element,
    owner: Entity<MdeWorkbenchItem>,
) -> (Option<UmlQuickEdit>, Option<UmlQuickEdit>, Option<UmlVisibilityPicker>) {
    let descriptors = quick_edit_for(&element.kind);
    let actions: Vec<(QuickEditSide, UmlQuickAction)> = descriptors.iter()
        .filter(|action| {
            action.availability == QuickEditAvailability::Implemented
                && action.control == QuickEditControl::Button
        })
        .filter_map(|action| {
            let icon = match action.id {
                "uml.add-attribute" => IconName::SquarePlus,
                "uml.add-operation" => IconName::Code,
                "uml.add-reception" => IconName::Bell,
                "uml.add-note" => IconName::Notepad,
                "uml.add-constraint" => IconName::Warning,
                "uml.add-subinterface" => IconName::ArrowDown,
                "uml.add-superinterface" => IconName::ArrowUp,
                "uml.add-realizing-class" => IconName::GitBranchPlus,
                _ => return None,
            };
            Some((action.side, UmlQuickAction {
                id: action.id,
                label: action.title,
                icon,
                row: action.row,
                shortcut: action.shortcut,
            }))
        })
        .collect();
    let make_actions = |side: QuickEditSide, role: &'static str| {
        let side_actions = actions.iter().filter(|(s,_)| *s == side)
            .map(|(_,action)| action.clone()).collect::<Vec<_>>();
        if side_actions.is_empty() {return None;}
        let owner=owner.clone();
        Some(UmlQuickEdit::new(side_actions, move |id, _window, cx| {
            match id {
                "uml.add-attribute" => {
                    owner.update(cx, |item, cx|
                        item.add_member_to_selected(ModelingMemberKind::Attribute, cx));
                }
                "uml.add-operation" => {
                    owner.update(cx, |item, cx|
                        item.add_member_to_selected(ModelingMemberKind::Operation, cx));
                }
                id => {
                    let action=match id {
                        "uml.add-note" => InterfaceQuickAction::AddNote,
                        "uml.add-constraint" => InterfaceQuickAction::AddConstraint,
                        "uml.add-reception" => InterfaceQuickAction::AddReception,
                        "uml.add-subinterface" => InterfaceQuickAction::AddSubinterface,
                        "uml.add-superinterface" => InterfaceQuickAction::AddSuperinterface,
                        "uml.add-realizing-class" => InterfaceQuickAction::AddRealizingClass,
                        _ => return,
                    };
                    owner.update(cx, |item, cx|
                        item.run_interface_quick_action(action, cx));
                }
            }
        }).side(role))
    };
    let right=make_actions(QuickEditSide::Right,"right");
    let left=make_actions(QuickEditSide::Left,"left");

    let visibility = descriptors.iter().any(|action|
        action.id == "uml.visibility"
            && action.availability == QuickEditAvailability::Implemented
    ).then(|| {
        let current = element.properties.get("visibility")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("public");
        UmlVisibilityPicker::new(current, move |value, _window, cx| {
            owner.update(cx, |item, cx| {
                item.set_selected_property(
                    "properties.visibility".into(),
                    serde_json::Value::String(value.to_owned()),
                    cx,
                );
            });
        })
    });

    (right, left, visibility)
}
