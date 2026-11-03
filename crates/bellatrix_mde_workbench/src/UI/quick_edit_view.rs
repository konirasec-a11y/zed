//! Map domain action descriptors to controls; no action semantics belong to a shape.
use crate::ui::MdeWorkbenchItem;
use bellatrix_mde::commands::ModelingMemberKind;
use bellatrix_mde::{
    Element, QuickEditAction, QuickEditAvailability, QuickEditControl, QuickEditSide,
    UmlQuickCreate, quick_edit_for,
};
use bellatrix_project::ProjectId;
use bellatrix_ui_uml::{UmlQuickAction, UmlQuickEdit, UmlVisibilityPicker};
use gpui::Entity;
use ui::IconName;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParameterAction {
    Add,
    Delete,
    Move { up: bool },
}

impl ParameterAction {
    pub(crate) fn from_id(id: &str) -> Option<Self> {
        match id {
            "uml.add-parameter" => Some(Self::Add),
            "uml.delete-parameter" => Some(Self::Delete),
            "uml.move-parameter-up" => Some(Self::Move { up: true }),
            "uml.move-parameter-down" => Some(Self::Move { up: false }),
            _ => None,
        }
    }

    pub(crate) fn icon(self) -> IconName {
        match self {
            Self::Add => IconName::SquarePlus,
            Self::Delete => IconName::Trash,
            Self::Move { up: true } => IconName::ArrowUp,
            Self::Move { up: false } => IconName::ArrowDown,
        }
    }
}

pub(crate) fn parameter_actions(kind: &str) -> Vec<(QuickEditAction, ParameterAction)> {
    quick_edit_for(kind)
        .into_iter()
        .filter(|action| {
            action.availability == QuickEditAvailability::Implemented
                && action.control == QuickEditControl::Button
        })
        .filter_map(|action| ParameterAction::from_id(action.id).map(|kind| (action, kind)))
        .collect()
}

pub(crate) fn quick_edit_for_element(
    element: &Element,
    owner: Entity<MdeWorkbenchItem>,
    project: ProjectId,
    revision: u64,
) -> (
    Option<UmlQuickEdit>,
    Option<UmlQuickEdit>,
    Option<UmlVisibilityPicker>,
) {
    let descriptors = quick_edit_for(&element.kind);
    let actions: Vec<(QuickEditSide, UmlQuickAction)> = descriptors
        .iter()
        .filter(|action| {
            action.availability == QuickEditAvailability::Implemented
                && action.control == QuickEditControl::Button
        })
        .filter_map(|action| {
            let icon = if let Some(action) = ParameterAction::from_id(action.id) {
                action.icon()
            } else {
                match action.id {
                    "uml.add-attribute" => IconName::SquarePlus,
                    "uml.add-operation" => IconName::Code,
                    "uml.add-reception" => IconName::Bell,
                    "uml.add-note" => IconName::Notepad,
                    "uml.add-constraint" => IconName::Warning,
                    "uml.add-subinterface" | "uml.add-subclass" => IconName::ArrowDown,
                    "uml.add-superinterface" | "uml.add-superclass" => IconName::ArrowUp,
                    "uml.add-realizing-class" => IconName::GitBranchPlus,
                    "uml.add-template-parameter" => IconName::Box,
                    "uml.add-provided-interface" => IconName::Circle,
                    "uml.add-required-interface" => IconName::CircleHelp,
                    "uml.add-associated-class" => IconName::Link,
                    "uml.add-aggregated-class" => IconName::GitBranch,
                    "uml.add-composited-class" => IconName::GitCommit,
                    "uml.add-port" => IconName::SquareDot,
                    "uml.add-part" => IconName::BoxOpen,
                    _ => return None,
                }
            };
            Some((
                action.side,
                UmlQuickAction {
                    id: action.id,
                    label: action.title,
                    icon,
                    row: action.row,
                    shortcut: action.shortcut,
                },
            ))
        })
        .collect();
    let make_actions = |side: QuickEditSide, role: &'static str| {
        let actions = actions
            .iter()
            .filter(|(candidate, _)| *candidate == side)
            .map(|(_, action)| action.clone())
            .collect::<Vec<_>>();
        if actions.is_empty() {
            return None;
        }
        let owner = owner.clone();
        let project = project.clone();
        let element_id = element.id.clone();
        Some(
            UmlQuickEdit::new(actions, move |id, _, cx| {
                if let Some(action) = ParameterAction::from_id(id) {
                    owner.update(cx, |item, cx| {
                        item.run_parameter_action(
                            action,
                            project.clone(),
                            element_id.clone(),
                            revision,
                            cx,
                        );
                    });
                    return;
                }
                match id {
                    "uml.add-attribute" => owner.update(cx, |item, cx| {
                        item.add_member_to_selected(ModelingMemberKind::Attribute, cx)
                    }),
                    "uml.add-operation" => owner.update(cx, |item, cx| {
                        item.add_member_to_selected(ModelingMemberKind::Operation, cx)
                    }),
                    "uml.add-reception" => owner.update(cx, |item, cx| {
                        item.add_member_to_selected(ModelingMemberKind::Reception, cx)
                    }),
                    id => {
                        let action = match id {
                            "uml.add-note" => UmlQuickCreate::Note,
                            "uml.add-constraint" => UmlQuickCreate::Constraint,
                            "uml.add-subinterface" | "uml.add-subclass" => UmlQuickCreate::Subtype,
                            "uml.add-superinterface" | "uml.add-superclass" => {
                                UmlQuickCreate::Supertype
                            }
                            "uml.add-realizing-class" => UmlQuickCreate::RealizingClass,
                            "uml.add-template-parameter" => UmlQuickCreate::TemplateParameter,
                            "uml.add-provided-interface" => UmlQuickCreate::ProvidedInterface,
                            "uml.add-required-interface" => UmlQuickCreate::RequiredInterface,
                            "uml.add-associated-class" => UmlQuickCreate::AssociatedClass,
                            "uml.add-aggregated-class" => UmlQuickCreate::AggregatedClass,
                            "uml.add-composited-class" => UmlQuickCreate::CompositedClass,
                            "uml.add-port" => UmlQuickCreate::Port,
                            "uml.add-part" => UmlQuickCreate::Part,
                            _ => return,
                        };
                        owner.update(cx, |item, cx| item.run_quick_action(action, cx));
                    }
                }
            })
            .side(role),
        )
    };
    let right = make_actions(QuickEditSide::Right, "right");
    let left = make_actions(QuickEditSide::Left, "left");
    let visibility = descriptors
        .iter()
        .any(|action| {
            action.id == "uml.visibility"
                && action.availability == QuickEditAvailability::Implemented
        })
        .then(|| {
            let current = element
                .properties
                .get("visibility")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("public");
            UmlVisibilityPicker::new(current, move |value, _, cx| {
                owner.update(cx, |item, cx| {
                    item.set_selected_property(
                        "properties.visibility".into(),
                        serde_json::Value::String(value.into()),
                        cx,
                    )
                });
            })
        });
    (right, left, visibility)
}
