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
    references: &crate::uml::references::ReferenceIndex<'_>,
) -> String {
    crate::format_relationship_label(&relation.kind, &relation.properties)
        .filter(|label| !label.trim().is_empty())
        .unwrap_or_else(|| {
            let endpoint = |id: &str| section.elements.iter().find(|element| element.id == id)
                .map(|element| {
                    let exposed = crate::template_parametered_element(section, element).unwrap_or(element);
                    crate::element_display_label_with_references(exposed, references)
                })
                .filter(|label| !label.trim().is_empty())
                .unwrap_or_else(|| "Unnamed element".into());
            format!("{} → {}", endpoint(&relation.from), endpoint(&relation.to))
        })
}

/// Derive the cross-view inventory from persisted diagram views, without creating model elements.
pub fn visual_components(section: &ModelingSection) -> Vec<VisualComponent> {
    let model = crate::uml::root::Model::from_section(section);
    let references = crate::uml::references::ReferenceIndex::new(model);
    let mut components = Vec::new();
    for diagram in &section.diagrams {
        for view in &diagram.views {
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
                .or_else(|| relation.map(|relation| relation.kind.clone()))
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
                .or_else(|| {
                    relation.map(|relation| relationship_display_label(section, relation, &references))
                })
                .unwrap_or_else(|| kind.clone());
            components.push(VisualComponent {
                diagram_id: diagram.id.clone(),
                view_id: view.id.clone(),
                subject_id: view.subject.clone(),
                parent_view_id: view
                    .draft
                    .as_ref()
                    .and_then(|draft| draft.parent_view_id.clone()),
                label,
                kind,
            });
        }
    }
    components
}
