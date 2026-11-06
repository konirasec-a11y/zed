//! Map domain action descriptors to controls; no action semantics belong to a shape.
use crate::elements::uml::{UmlQuickAction, UmlQuickEdit, UmlVisibilityPicker};
use crate::workbench_ui::MdeWorkbenchItem;
use bellatrix_mde_cordis::{
    ModelingMemberKind, QuickEditAction, QuickEditAvailability, QuickEditControl, QuickEditSide,
    UmlQuickCreate,
    projection::{Element, ModelingProjection, Relation},
};
use bellatrix_project::ProjectId;
use gpui::Entity;
use ui::IconName;

#[derive(Clone)]
pub(crate) struct QuickEditTarget {
    pub project: ProjectId,
    pub diagram: String,
    pub element: String,
    pub view: Option<String>,
    pub revision: u64,
}

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

pub(crate) fn parameter_actions(
    actions: &[QuickEditAction],
) -> Vec<(QuickEditAction, ParameterAction)> {
    actions
        .iter()
        .copied()
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
    diagram: String,
    view: Option<String>,
    revision: u64,
) -> (
    Option<UmlQuickEdit>,
    Option<UmlQuickEdit>,
    Option<UmlVisibilityPicker>,
) {
    let target = QuickEditTarget {
        project,
        diagram,
        element: element.id.clone(),
        view,
        revision,
    };
    let descriptors = &element.quick_actions;
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
                    visual: crate::elements::uml::UmlVisualDefinition::for_quick_action(action.id),
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
        let target = target.clone();
        Some(
            UmlQuickEdit::new(actions, move |id, _, cx| {
                if owner
                    .update(cx, |item, cx| item.quick_edit_snapshot(&target, cx))
                    .is_none()
                {
                    return;
                }
                if let Some(action) = ParameterAction::from_id(id) {
                    owner.update(cx, |item, cx| {
                        item.run_parameter_action(
                            action,
                            target.project.clone(),
                            target.element.clone(),
                            target.revision,
                            cx,
                        );
                    });
                    return;
                }
                match id {
                    "uml.add-attribute" | "uml.add-operation" | "uml.add-reception" => {
                        let kind = match id {
                            "uml.add-attribute" => ModelingMemberKind::Attribute,
                            "uml.add-operation" => ModelingMemberKind::Operation,
                            _ => ModelingMemberKind::Reception,
                        };
                        owner.update(cx, |item, cx| {
                            let result =
                                item.source
                                    .add_member(&target.element, kind, target.revision);
                            item.report_command_result(result, cx);
                        });
                    }
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
                        owner.update(cx, |item, cx| item.run_quick_action(action, &target, cx));
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
            let current = element.visibility.as_deref().unwrap_or("public");
            UmlVisibilityPicker::new(current, move |value, _, cx| {
                owner.update(cx, |item, cx| {
                    if item.quick_edit_snapshot(&target, cx).is_none() {
                        return;
                    }
                    let result = item.source.set_property(
                        &target.element,
                        "properties.visibility",
                        serde_json::Value::String(value.into()),
                        target.revision,
                    );
                    item.report_command_result(result, cx);
                });
            })
        });
    (right, left, visibility)
}

pub(crate) fn quick_edit_for_relationship(
    section: &ModelingProjection,
    relation: &Relation,
    owner: Entity<MdeWorkbenchItem>,
    project: ProjectId,
    revision: u64,
    read_only: bool,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> Option<gpui::AnyElement> {
    use crate::workbench_ui::WorkbenchPalette;
    use bellatrix_mde_cordis::PropertyEditor;
    use gpui::{MouseButton, div, prelude::*, px, svg};
    use ui::{ContextMenu, DropdownMenu, Icon, IconButton, Label, Tooltip, prelude::*};

    let inspection = section.inspect_element(&relation.id)?;
    let type_label = inspection.type_label.as_str();
    let palette = WorkbenchPalette::current(cx);
    let mut controls = Vec::new();
    for (id, role, icon) in [
        (
            &relation.from,
            relation.source_role.as_str(),
            IconName::ArrowLeft,
        ),
        (
            &relation.to,
            relation.target_role.as_str(),
            IconName::ArrowRight,
        ),
    ] {
        let endpoint = section.elements.iter().find(|element| element.id == *id);
        let name = endpoint
            .map(|element| element.name.as_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(id);
        let tooltip = format!("Select {role}: {name}");
        let target = owner.clone();
        let endpoint_id = id.clone();
        controls.push(
            IconButton::new(format!("relationship-endpoint-{role}-{id}"), icon)
                .aria_label(tooltip.clone())
                .tooltip(Tooltip::text(tooltip))
                .disabled(endpoint.is_none())
                .on_click(move |_, _, cx| {
                    target.update(cx, |item, cx| item.select_element(endpoint_id.clone(), cx));
                    cx.stop_propagation();
                })
                .into_any_element(),
        );
    }
    if !read_only {
        for property in inspection.properties.iter().filter(|property| {
            property.editable
                && property.group != "Other properties"
                && property.path != "properties.navigable"
        }) {
            let options: Vec<(String, serde_json::Value)> = match &property.editor {
                PropertyEditor::Choice(options) => options
                    .iter()
                    .map(|option| (option.clone(), serde_json::Value::String(option.clone())))
                    .collect(),
                PropertyEditor::Boolean => [false, true]
                    .into_iter()
                    .map(|value| (value.to_string(), serde_json::Value::Bool(value)))
                    .collect(),
                _ => continue,
            };
            let icon = match property.path.as_str() {
                "properties.visibility" => IconName::Eye,
                "properties.end1Aggregation" | "properties.end2Aggregation" => IconName::GitCommit,
                _ => IconName::Settings,
            };
            let target = owner.clone();
            let project = project.clone();
            let element_id = relation.id.clone();
            let path = property.path.clone();
            let current = property.value.clone();
            let menu = ContextMenu::build(window, cx, move |menu, _, _| {
                options.into_iter().fold(menu, |menu, (label, value)| {
                    let label = if value == current {
                        format!("✓ {label}")
                    } else {
                        label
                    };
                    let target = target.clone();
                    let project = project.clone();
                    let element_id = element_id.clone();
                    let path = path.clone();
                    menu.entry(label, None, move |_, cx| {
                        target.update(cx, |item, cx| {
                            if item.source.is_read_only() {
                                return;
                            }
                            if item.project_id() != &project
                                || item.selected_element() != Some(element_id.as_str())
                            {
                                item.feedback =
                                    "Relationship edit not sent: the active selection changed"
                                        .into();
                                cx.notify();
                                return;
                            }
                            // Keep the revision displayed by the menu; stale choices must not
                            // overwrite newer endpoint or relationship edits.
                            let result = item.source.set_property(
                                &element_id,
                                &path,
                                value.clone(),
                                revision,
                            );
                            item.report_command_result(result, cx);
                        });
                    })
                })
            });
            let current = property
                .value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| property.value.to_string());
            let tooltip = format!("{}: {current}", property.label);
            controls.push(
                DropdownMenu::new_with_element(
                    format!("relationship-quick-{}", property.path),
                    Icon::new(icon).into_any_element(),
                    menu,
                )
                .aria_label(property.label.clone())
                .trigger_tooltip(Tooltip::text(tooltip))
                .into_any_element(),
            );
        }
    }
    Some(
        div()
            .id(format!("relationship-quick-edit-{}", relation.id))
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .w_full()
            .min_w_0()
            .rounded_md()
            .bg(palette.surface)
            .border_1()
            .border_color(palette.border)
            .text_color(palette.text)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .id("relationship-quick-type")
                    .tooltip(Tooltip::text(type_label.to_owned()))
                    .flex()
                    .min_w_0()
                    .items_center()
                    .gap_2()
                    .child(
                        svg()
                            .data(
                                crate::elements::uml::UmlVisualDefinition::for_kind(
                                    &inspection.presentation_kind,
                                    &inspection.type_label,
                                    true,
                                )
                                .icon(),
                            )
                            .size(px(16.0))
                            .flex_none()
                            .text_color(palette.muted),
                    )
                    .child(Label::new(type_label.to_owned()).truncate()),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .children(controls),
            )
            .into_any_element(),
    )
}
