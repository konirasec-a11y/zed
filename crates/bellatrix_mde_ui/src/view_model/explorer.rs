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

pub(crate) fn selection_occurrence(
    section: &ModelingProjection,
    subject: &str,
    diagram: Option<&str>,
    preferred_view: Option<&str>,
) -> Option<(String, ViewOccurrence)> {
    // A semantic identity can have several diagram occurrences. Select the
    // exact diagram/view row rather than the first row with the same subject.
    if let Some(diagram) = diagram.and_then(|id| section.diagrams.iter().find(|item| item.id == id))
    {
        let views = diagram
            .all_views()
            .filter(|view| view.subject.as_deref() == Some(subject))
            .collect::<Vec<_>>();
        let chosen = preferred_view
            .and_then(|preferred| views.iter().copied().find(|view| view.id == preferred))
            .or_else(|| match views.as_slice() {
                [view] => Some(*view),
                _ => None,
            });
        if !views.is_empty() && chosen.is_none() {
            return None;
        }
        if let Some(view) = chosen {
            let row = section.explorer.added.iter().find(|row| {
                row.view.as_ref().is_some_and(|occurrence| {
                    occurrence.diagram_id == diagram.id && occurrence.view_id == view.id
                })
            })?;
            return Some((
                row.key.clone(),
                ViewOccurrence {
                    diagram_id: diagram.id.clone(),
                    view_id: view.id.clone(),
                    member_path: Vec::new(),
                },
            ));
        }
    }
    let candidates = section
        .explorer
        .added
        .iter()
        .filter(|row| {
            !row.diagram
                && row
                    .view
                    .as_ref()
                    .and_then(|view| view.subject_id.as_deref())
                    .unwrap_or(row.element_id.as_str())
                    == subject
        })
        .filter_map(|row| {
            let occurrence = section.explorer.parent_occurrences.get(&row.key)?;
            (Some(occurrence.diagram_id.as_str()) == diagram).then_some((row, occurrence))
        })
        .collect::<Vec<_>>();
    let has_preferred = candidates
        .iter()
        .any(|(_, occurrence)| Some(occurrence.view_id.as_str()) == preferred_view);
    let mut candidates = candidates.into_iter().filter(|(_, occurrence)| {
        !has_preferred || Some(occurrence.view_id.as_str()) == preferred_view
    });
    let (row, occurrence) = candidates.next()?;
    // Multiple renderings are not interchangeable. Keep model-only selection when ambiguous.
    if candidates.any(|(_, candidate)| candidate != occurrence) {
        return None;
    }
    Some((row.key.clone(), occurrence.clone()))
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
