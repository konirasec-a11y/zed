use crate::ModelingSection;

/// A view is an independent visual identity even when it represents a shared subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualComponent {
    pub diagram_id: String,
    pub view_id: String,
    pub subject_id: Option<String>,
    pub parent_view_id: Option<String>,
    pub label: String,
    pub kind: String,
}

// Both semantic and diagram projections need the same human-readable fallback.
pub(crate) fn relationship_display_label(
    section: &ModelingSection,
    relation: &crate::Relation,
) -> String {
    use crate::RelationshipMarker;
    let endpoint = |id: &str| {
        section
            .elements
            .iter()
            .find(|element| element.id == id)
            .map(|element| element.name.trim())
            .filter(|name| !name.is_empty())
            .unwrap_or(id)
            .to_owned()
    };
    let presentation = crate::relationship_presentation(section, relation);
    let relation_type = crate::semantic_identity(&presentation.notation).type_label;
    let name = presentation
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(|name| format!("{relation_type}: {name}"))
        .unwrap_or_else(|| relation_type.to_owned());
    let marker = |marker, source| match marker {
        RelationshipMarker::None => "",
        RelationshipMarker::OpenArrow => {
            if source {
                "<"
            } else {
                ">"
            }
        }
        RelationshipMarker::Triangle => {
            if source {
                "◁"
            } else {
                "▷"
            }
        }
        RelationshipMarker::FilledTriangle => {
            if source {
                "◀"
            } else {
                "▶"
            }
        }
        RelationshipMarker::Diamond => "◇",
        RelationshipMarker::FilledDiamond => "◆",
        RelationshipMarker::FilledCircle => "●",
        RelationshipMarker::OpenArrowAndCircle => {
            if source {
                "○<"
            } else {
                ">○"
            }
        }
        RelationshipMarker::FilledTriangleAndCircle => {
            if source {
                "○◀"
            } else {
                "▶○"
            }
        }
        RelationshipMarker::Containment => "⊕",
    };
    let line = match presentation.style.line_mode {
        crate::LineMode::Solid => "─",
        crate::LineMode::Dashed => "┄",
        crate::LineMode::Dotted => "··",
    };
    format!(
        "{} {}{line} {} {}{line}{} {}",
        endpoint(&relation.from),
        marker(presentation.style.source_marker, true),
        name,
        marker(presentation.style.middle_marker, false),
        marker(presentation.style.target_marker, false),
        endpoint(&relation.to)
    )
}

/// Derive the cross-view inventory from persisted diagram views, without creating model elements.
pub fn visual_components(section: &ModelingSection) -> Vec<VisualComponent> {
    let model = crate::uml::root::Model::from_section(section);
    let references = crate::uml::references::ReferenceIndex::new(model);
    let mut components = Vec::new();
    for diagram in &section.diagrams {
        let mut pending = diagram
            .views
            .iter()
            .rev()
            .map(|view| (view, None))
            .collect::<Vec<_>>();
        while let Some((view, parent_view_id)) = pending.pop() {
            pending.extend(
                view.children
                    .iter()
                    .rev()
                    .map(|child| (child, Some(view.id.clone()))),
            );
            let element = view
                .subject
                .as_deref()
                .and_then(|id| section.elements.iter().find(|element| element.id == id));
            let relation = view
                .subject
                .as_deref()
                .and_then(|id| section.relations.iter().find(|relation| relation.id == id));
            let kind = view
                .draft
                .as_ref()
                .map(|draft| draft.notation_kind.clone())
                .or_else(|| element.map(|element| element.kind.clone()))
                .or_else(|| {
                    relation.map(|relation| {
                        crate::relationship_presentation(section, relation).notation
                    })
                })
                .unwrap_or_else(|| "visual component".into());
            let label = view
                .label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
                .map(str::to_owned)
                .or_else(|| {
                    element.map(|element| {
                        let exposed = crate::template_parametered_element(section, element)
                            .unwrap_or(element);
                        crate::format_element_label_for_view(exposed, &references, &view.style)
                    })
                })
                .or_else(|| relation.map(|relation| relationship_display_label(section, relation)))
                .unwrap_or_else(|| kind.clone());
            components.push(VisualComponent {
                diagram_id: diagram.id.clone(),
                view_id: view.id.clone(),
                subject_id: view.subject.clone(),
                parent_view_id: parent_view_id.or_else(|| {
                    view.draft
                        .as_ref()
                        .and_then(|draft| draft.parent_view_id.clone())
                }),
                label,
                kind,
            });
        }
    }
    components
}
