//! Engine-MDE adapter from typed contextual actions to the reusable UML
//! visuals. All icon layouts and controls are in bellatrix_ui_uml; only this
//! adapter knows the selected MDE type and its Cordis command.
use crate::MdeWorkbenchItem;
use bellatrix_mde_cordis::ModelingMemberKind;
use bellatrix_mde_model::Element;
use bellatrix_mde_modeling::{
    QuickEditAvailability, QuickEditControl, quick_edit_for,
};
use bellatrix_ui_uml::{UmlQuickAction, UmlQuickEdit, UmlVisibilityPicker};
use gpui::Entity;
use ui::IconName;

/// These components are deliberately absent outside editing mode.
/// Pending domain actions do not create fake buttons on the Canvas.
pub(crate) fn quick_edit_for_element(
    element: &Element,
    owner: Entity<MdeWorkbenchItem>,
) -> (Option<UmlQuickEdit>, Option<UmlVisibilityPicker>) {
    let descriptors = quick_edit_for(&element.kind);
    let actions: Vec<UmlQuickAction> = descriptors.iter()
        .filter(|action| {
            action.availability == QuickEditAvailability::Implemented
                && action.control == QuickEditControl::Button
        })
        .filter_map(|action| {
            let icon = match action.id {
                "uml.add-attribute" => IconName::SquarePlus,
                "uml.add-operation" => IconName::Code,
                _ => return None,
            };
            Some(UmlQuickAction {
                id: action.id,
                label: action.title,
                icon,
            })
        })
        .collect();
    let controls = if actions.is_empty() {
        None
    } else {
        let owner = owner.clone();
        Some(UmlQuickEdit::new(actions, move |id, _window, cx| {
            let kind = match id {
                "uml.add-attribute" => ModelingMemberKind::Attribute,
                "uml.add-operation" => ModelingMemberKind::Operation,
                _ => return,
            };
            owner.update(cx, |item, cx| item.add_member_to_selected(kind, cx));
        }))
    };

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

    (controls, visibility)
}
