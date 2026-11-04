use bellatrix_mde_protocol::inspector::*;
pub(crate) fn project_inspector(
    section: &crate::ModelingSection,
    id: &str,
) -> Option<ElementInspector> {
    let inspector = crate::inspect_element(section, id)?;
    let type_label = crate::semantic_identity(&inspector.presentation_kind)
        .type_label
        .to_owned();
    let quick_actions = project_quick_actions(&inspector.kind);
    Some(ElementInspector {
        element_id: inspector.element_id,
        kind: inspector.kind,
        presentation_kind: inspector.presentation_kind,
        type_label,
        name: inspector.name,
        display_label: inspector.display_label,
        quick_actions,
        properties: inspector
            .properties
            .into_iter()
            .map(project_property)
            .collect(),
        reference_choices_by_path: inspector
            .reference_choices_by_path
            .into_iter()
            .map(|(path, choices)| {
                let choices = choices
                    .into_iter()
                    .map(|choice| InspectorReferenceChoice {
                        id: choice.id,
                        label: choice.label,
                    })
                    .collect();
                (path, choices)
            })
            .collect(),
        attributes: inspector
            .attributes
            .into_iter()
            .map(project_member)
            .collect(),
        operations: inspector
            .operations
            .into_iter()
            .map(project_member)
            .collect(),
        parameters: inspector
            .parameters
            .into_iter()
            .map(project_member)
            .collect(),
        receptions: inspector
            .receptions
            .into_iter()
            .map(project_member)
            .collect(),
        template_parameters: inspector
            .template_parameters
            .into_iter()
            .map(project_member)
            .collect(),
        ports: inspector.ports.into_iter().map(project_member).collect(),
        parts: inspector.parts.into_iter().map(project_member).collect(),
    })
}

pub(crate) fn project_quick_actions(kind: &str) -> Vec<QuickEditAction> {
    crate::quick_edit_for(kind)
        .into_iter()
        .map(|action| QuickEditAction {
            id: action.id,
            title: action.title,
            control: match action.control {
                crate::QuickEditControl::Input => QuickEditControl::Input,
                crate::QuickEditControl::Visibility => QuickEditControl::Visibility,
                crate::QuickEditControl::Button => QuickEditControl::Button,
            },
            side: match action.side {
                crate::QuickEditSide::Main => QuickEditSide::Main,
                crate::QuickEditSide::Left => QuickEditSide::Left,
                crate::QuickEditSide::Right => QuickEditSide::Right,
            },
            row: action.row,
            availability: match action.availability {
                crate::QuickEditAvailability::Implemented => QuickEditAvailability::Implemented,
                crate::QuickEditAvailability::NeedsDomainContract => {
                    QuickEditAvailability::NeedsDomainContract
                }
            },
            shortcut: action.shortcut,
        })
        .collect()
}

fn project_property(property: crate::InspectorProperty) -> InspectorProperty {
    let editor = match property.editor {
        crate::PropertyEditor::Text => PropertyEditor::Text,
        crate::PropertyEditor::Multiline => PropertyEditor::Multiline,
        crate::PropertyEditor::Boolean => PropertyEditor::Boolean,
        crate::PropertyEditor::Number => PropertyEditor::Number,
        crate::PropertyEditor::Choice(options) => {
            PropertyEditor::Choice(options.iter().map(|option| (*option).to_owned()).collect())
        }
        crate::PropertyEditor::Reference => PropertyEditor::Reference,
        crate::PropertyEditor::Stereotype => PropertyEditor::Stereotype,
        crate::PropertyEditor::ReadOnly => PropertyEditor::ReadOnly,
    };
    InspectorProperty {
        path: property.path,
        label: property.label,
        group: property.group.to_owned(),
        editor,
        value: property.value,
        editable: property.editable,
    }
}

fn project_member(member: crate::InspectorMember) -> InspectorMember {
    InspectorMember {
        id: member.id,
        name: member.name,
        display_label: member.display_label,
        kind: member.kind,
    }
}
