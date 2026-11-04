//! Explorer filtering and selection over the cached Cordis projection.
pub use bellatrix_mde_cordis::ExplorerRow;
use bellatrix_mde_cordis::{
    ModelingElementKind, ModelingMemberKind, StructuralDiagramKind, ViewOccurrence,
    projection::ModelingProjection,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExplorerSortOrder {
    Added,
    Name,
}

pub(crate) fn member_creation_kinds(
    section: &ModelingProjection,
    subject: &str,
) -> Vec<(ModelingMemberKind, &'static str)> {
    section
        .explorer
        .member_creation
        .get(subject)
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn contextual_diagram_kinds(
    section: &ModelingProjection,
    subject: &str,
) -> Vec<StructuralDiagramKind> {
    section
        .explorer
        .contextual_diagrams
        .get(subject)
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn filter_explorer(rows: Vec<ExplorerRow>, query: &str) -> Vec<ExplorerRow> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return rows;
    }
    let words = query.split_whitespace().collect::<Vec<_>>();
    let mut retained = std::collections::HashSet::new();
    let mut ancestors = Vec::new();
    let mut matching_depth = None;
    for (index, row) in rows.iter().enumerate() {
        ancestors.truncate(row.depth);
        if matching_depth.is_some_and(|depth| row.depth <= depth) {
            matching_depth = None;
        }
        let text = format!(
            "{} {} {} {}",
            row.label,
            row.kind,
            row.element_id,
            row.view
                .as_ref()
                .and_then(|view| view.subject_id.as_deref())
                .unwrap_or("")
        )
        .to_lowercase();
        if matching_depth.is_some() || words.iter().all(|word| text.contains(*word)) {
            retained.extend(ancestors.iter().copied());
            retained.insert(index);
            if matching_depth.is_none() {
                matching_depth = Some(row.depth);
            }
        }
        ancestors.push(index);
    }
    rows.into_iter()
        .enumerate()
        .filter_map(|(index, row)| retained.contains(&index).then_some(row))
        .collect()
}

pub(crate) fn project_explorer(
    section: &ModelingProjection,
    sort: ExplorerSortOrder,
) -> Vec<ExplorerRow> {
    match sort {
        ExplorerSortOrder::Added => section.explorer.added.clone(),
        ExplorerSortOrder::Name => section.explorer.name.clone(),
    }
}

pub(crate) fn explorer_type_label(
    section: &ModelingProjection,
    kind: &str,
    relationship: bool,
) -> String {
    if relationship {
        return "Relationship".into();
    }
    section
        .explorer
        .type_labels
        .get(kind)
        .cloned()
        .unwrap_or_else(|| "Element".into())
}

pub(crate) fn model_creation_kinds(
    section: &ModelingProjection,
    owner: Option<&str>,
) -> Vec<ModelingElementKind> {
    match owner {
        Some(owner) => section
            .explorer
            .model_creation
            .get(owner)
            .cloned()
            .unwrap_or_default(),
        None => section.explorer.project_model_creation.clone(),
    }
}

pub(crate) fn project_diagram_kinds(section: &ModelingProjection) -> Vec<StructuralDiagramKind> {
    section.explorer.project_diagrams.clone()
}

pub(crate) fn parent_occurrence(
    section: &ModelingProjection,
    subject: Option<&str>,
    row_key: &str,
) -> Option<ViewOccurrence> {
    let subject = subject?;
    let row = section
        .explorer
        .added
        .iter()
        .find(|row| row.key == row_key)?;
    if row.diagram
        || row
            .view
            .as_ref()
            .and_then(|view| view.subject_id.as_deref())
            .unwrap_or(row.element_id.as_str())
            != subject
    {
        return None;
    }
    section.explorer.parent_occurrences.get(row_key).cloned()
}

pub(crate) fn parent_occurrence_from_rows(
    section: &ModelingProjection,
    subject: Option<&str>,
    row_key: &str,
    rows: &[ExplorerRow],
) -> Option<ViewOccurrence> {
    let subject = subject?;
    let row = rows.iter().find(|row| row.key == row_key)?;
    if row.diagram
        || row
            .view
            .as_ref()
            .and_then(|view| view.subject_id.as_deref())
            .unwrap_or(row.element_id.as_str())
            != subject
    {
        return None;
    }
    parent_occurrence(section, Some(subject), row_key)
}
