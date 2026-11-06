use crate::ModelingSection;
use bellatrix_mde_contracts::protocol::command::{
    ModelingElementKind, ModelingMemberKind, StructuralDiagramKind,
};
use bellatrix_mde_contracts::protocol::explorer::*;
use std::collections::BTreeMap;
pub(crate) fn project(section: &ModelingSection) -> Result<ExplorerProjection, String> {
    let added = project_explorer(section, ExplorerSortOrder::Added);
    let name = project_explorer(section, ExplorerSortOrder::Name);
    let mut member_creation = BTreeMap::new();
    let mut contextual_diagrams = BTreeMap::new();
    let mut model_creation = BTreeMap::new();
    for element in &section.elements {
        let members = member_creation_kinds(section, &element.id);
        member_creation.insert(element.id.clone(), members);
        contextual_diagrams.insert(
            element.id.clone(),
            contextual_diagram_kinds(section, &element.id),
        );
        model_creation.insert(
            element.id.clone(),
            model_creation_kinds(section, Some(&element.id)),
        );
    }
    let mut parent_occurrences = BTreeMap::new();
    let mut type_labels = BTreeMap::new();
    for rows in [&added, &name] {
        for row in rows {
            type_labels
                .entry(row.kind.clone())
                .or_insert_with(|| explorer_type_label(&row.kind, false));
            let subject = row
                .view
                .as_ref()
                .and_then(|view| view.subject_id.as_deref())
                .unwrap_or(&row.element_id);
            if let Some(occurrence) =
                parent_occurrence_from_rows(section, Some(subject), &row.key, rows)
            {
                parent_occurrences
                    .entry(row.key.clone())
                    .or_insert(occurrence);
            }
        }
    }
    Ok(ExplorerProjection {
        added,
        name,
        member_creation,
        contextual_diagrams,
        model_creation,
        project_model_creation: model_creation_kinds(section, None),
        project_diagrams: project_diagram_kinds(),
        parent_occurrences,
        type_labels,
    })
}

fn member_creation_kinds(
    section: &ModelingSection,
    subject: &str,
) -> Vec<(ModelingMemberKind, &'static str)> {
    use crate::{UmlCapability, UmlType};
    let Some(owner_type) = section
        .elements
        .iter()
        .find(|element| element.id == subject)
        .and_then(|element| UmlType::from_kind(&element.kind))
    else {
        return Vec::new();
    };
    [
        (
            ModelingMemberKind::Attribute,
            UmlCapability::Attribute,
            "Attribute",
        ),
        (
            ModelingMemberKind::Operation,
            UmlCapability::Operation,
            "Operation",
        ),
        (
            ModelingMemberKind::Reception,
            UmlCapability::Reception,
            "Reception",
        ),
        (
            ModelingMemberKind::Parameter,
            UmlCapability::Parameter,
            "Parameter",
        ),
    ]
    .into_iter()
    .filter(|(_, capability, _)| owner_type.supports(*capability))
    .map(|(kind, _, label)| (kind, label))
    .collect()
}

fn contextual_diagram_kinds(
    section: &ModelingSection,
    subject: &str,
) -> Vec<StructuralDiagramKind> {
    let Some(element) = section
        .elements
        .iter()
        .find(|element| element.id == subject)
    else {
        return Vec::new();
    };
    let metaclass = crate::uml::root::object_metaclass(&element.kind);
    crate::StructuralDiagramKind::ALL_DIAGRAMS
        .into_iter()
        .filter(|kind| {
            if let Some(expected) = kind.context_metaclass() {
                metaclass.is_some_and(|actual| crate::uml::metamodel::conforms_to(actual, expected))
            } else if *kind == crate::StructuralDiagramKind::CompositeStructure {
                // Instance classifier evidence is not exposed by this menu projection yet.
                // Do not infer it from raw properties or offer an unverified context.
                crate::UmlType::from_kind(&element.kind)
                    .is_some_and(crate::UmlType::is_structured_classifier)
            } else {
                matches!(element.kind.as_str(), "uml.package" | "uml.model")
            }
        })
        .map(project_diagram_kind)
        .collect()
}

pub(crate) fn project_diagram_kind(kind: crate::StructuralDiagramKind) -> StructuralDiagramKind {
    match kind {
        crate::StructuralDiagramKind::Class => StructuralDiagramKind::Class,
        crate::StructuralDiagramKind::Object => StructuralDiagramKind::Object,
        crate::StructuralDiagramKind::Package => StructuralDiagramKind::Package,
        crate::StructuralDiagramKind::Component => StructuralDiagramKind::Component,
        crate::StructuralDiagramKind::CompositeStructure => {
            StructuralDiagramKind::CompositeStructure
        }
        crate::StructuralDiagramKind::Deployment => StructuralDiagramKind::Deployment,
        crate::StructuralDiagramKind::Profile => StructuralDiagramKind::Profile,
        crate::StructuralDiagramKind::UseCase => StructuralDiagramKind::UseCase,
        crate::StructuralDiagramKind::Activity => StructuralDiagramKind::Activity,
        crate::StructuralDiagramKind::StateMachine => StructuralDiagramKind::StateMachine,
        crate::StructuralDiagramKind::Sequence => StructuralDiagramKind::Sequence,
        crate::StructuralDiagramKind::Communication => StructuralDiagramKind::Communication,
        crate::StructuralDiagramKind::InteractionOverview => {
            StructuralDiagramKind::InteractionOverview
        }
        crate::StructuralDiagramKind::Timing => StructuralDiagramKind::Timing,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExplorerSortOrder {
    Added,
    Name,
}

fn semantic_key(id: &str) -> String {
    format!("semantic:{id}")
}

fn diagram_key(id: &str) -> String {
    format!("diagram:{id}")
}

fn project_explorer(section: &ModelingSection, sort: ExplorerSortOrder) -> Vec<ExplorerRow> {
    use std::collections::{HashMap, HashSet};
    let authored = project_authored_composition(section, sort);
    let components = crate::visual_components(section);
    let presentations = section
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .map(|view| (view.id.as_str(), view))
        .collect::<HashMap<_, _>>();
    // Source compartments and labels belong to their figure, not to the model
    // tree. Keep the complete visual inventory available to other consumers.
    let internal_parents = components
        .iter()
        .filter(|component| {
            component.parent_view_id.is_some()
                && presentations
                    .get(component.view_id.as_str())
                    .is_some_and(|view| view.presentation.internal)
        })
        .map(|component| {
            (
                component.view_id.as_str(),
                component.parent_view_id.as_deref(),
            )
        })
        .collect::<HashMap<_, _>>();
    let components = components
        .iter()
        .filter(|component| !internal_parents.contains_key(component.view_id.as_str()))
        // ERD document composition already represents its entities and relationships;
        // labels and compartments are canvas occurrences, not extra model children.
        .filter(|view| {
            !section.diagrams.iter().any(|diagram| {
                diagram.id == view.diagram_id
                    && diagram.uml_di_type == "ERDDiagram"
                    && section.composition.contains_key(&diagram.id)
            })
        })
        .map(|view| {
            let mut parent = view.parent_view_id.as_deref();
            let mut visited = HashSet::new();
            while let Some(parent_id) = parent {
                let Some(internal_parent) = internal_parents.get(parent_id) else {
                    break;
                };
                if !visited.insert(parent_id) {
                    parent = None;
                    break;
                }
                parent = *internal_parent;
            }
            let kind = if view.kind == "visual component" {
                presentations
                    .get(view.view_id.as_str())
                    .and_then(|view| view.presentation.notation_kind.as_deref())
                    .map(str::to_owned)
                    .unwrap_or_else(|| view.kind.clone())
            } else {
                view.kind.clone()
            };
            let missing_label = view.label.trim().is_empty()
                || (view.kind == "visual component"
                    && view.label == "visual component"
                    && presentations
                        .get(view.view_id.as_str())
                        .and_then(|view| view.label.as_deref())
                        .is_none_or(|label| label.trim().is_empty()));
            let label = if missing_label {
                view.subject_id
                    .as_deref()
                    .and_then(|subject| {
                        section
                            .elements
                            .iter()
                            .find(|element| element.id == subject)
                    })
                    .and_then(|element| imported_element_label(section, element))
                    .unwrap_or_else(|| explorer_type_label(&kind, false))
            } else {
                view.label.clone()
            };
            VisualComponent {
                diagram_id: view.diagram_id.clone(),
                view_id: view.view_id.clone(),
                subject_id: view.subject_id.clone(),
                parent_view_id: parent.map(str::to_owned),
                label,
                kind,
            }
        })
        .collect::<Vec<_>>();
    let represented = components
        .iter()
        .filter_map(|view| view.subject_id.as_deref())
        .collect::<HashSet<_>>();
    let mut semantic_children: HashMap<String, Vec<ExplorerRow>> = HashMap::new();
    let mut ancestors: Vec<&ExplorerRow> = Vec::new();
    for row in &authored {
        ancestors.truncate(row.depth);
        if let Some(parent) = ancestors.last() {
            if !row.diagram && !parent.diagram {
                semantic_children
                    .entry(parent.element_id.clone())
                    .or_default()
                    .push(row.clone());
            }
        }
        ancestors.push(row);
    }
    let semantic = crate::uml::root::Model::from_section(section);
    let mut diagram_ancestors = HashMap::new();
    let mut ancestors = Vec::new();
    for row in &authored {
        ancestors.truncate(row.depth);
        if row.diagram {
            diagram_ancestors.insert(row.element_id.as_str(), ancestors.clone());
        }
        ancestors.push(row.element_id.as_str());
    }
    // Coalescing a model element with its occurrences must not hide its real
    // owner. Otherwise a visibly empty Model can still delete those occurrences
    // through the legitimate UML ownership cascade.
    let owner_is_visible = |subject: &str| {
        let Some(owner) = semantic
            .element(subject)
            .and_then(|element| semantic.owner(element).ok().flatten())
        else {
            return false;
        };
        components
            .iter()
            .filter(|view| view.subject_id.as_deref() == Some(subject))
            .all(|view| {
                let mut parent = view.parent_view_id.as_deref();
                let mut visited = HashSet::new();
                while let Some(parent_id) = parent {
                    if !visited.insert(parent_id) {
                        return false;
                    }
                    let Some(parent_view) = components.iter().find(|candidate| {
                        candidate.diagram_id == view.diagram_id && candidate.view_id == parent_id
                    }) else {
                        return false;
                    };
                    if parent_view.subject_id.as_deref() == Some(owner.id()) {
                        return true;
                    }
                    parent = parent_view.parent_view_id.as_deref();
                }
                diagram_ancestors
                    .get(view.diagram_id.as_str())
                    .is_some_and(|ancestors| ancestors.contains(&owner.id()))
            })
    };
    let mut hidden = HashSet::new();
    for (index, row) in authored.iter().enumerate().filter(|(_, row)| !row.diagram) {
        if !represented.contains(row.element_id.as_str()) || !owner_is_visible(&row.element_id) {
            continue;
        }
        let branch = authored[index..]
            .iter()
            .take_while(|candidate| candidate.key == row.key || candidate.depth > row.depth)
            .collect::<Vec<_>>();
        if branch.iter().any(|candidate| {
            candidate.diagram
                || section.composition.contains_key(&candidate.element_id)
                || (represented.contains(candidate.element_id.as_str())
                    && !owner_is_visible(&candidate.element_id))
        }) {
            continue;
        }
        hidden.extend(branch.into_iter().map(|candidate| candidate.key.clone()));
    }
    fn append_members(
        subject: &str,
        parent_key: &str,
        diagram_id: &str,
        depth: usize,
        children: &HashMap<String, Vec<ExplorerRow>>,
        represented: &HashSet<&str>,
        path: &mut HashSet<String>,
        rows: &mut Vec<ExplorerRow>,
    ) {
        if !path.insert(subject.to_owned()) {
            return;
        }
        if let Some(members) = children.get(subject) {
            for member in members {
                if represented.contains(member.element_id.as_str()) {
                    continue;
                }
                let mut row = member.clone();
                row.key = format!("{parent_key}/member:{}", member.element_id);
                row.diagram_context = Some(diagram_id.to_owned());
                row.depth = depth;
                let key = row.key.clone();
                rows.push(row);
                append_members(
                    &member.element_id,
                    &key,
                    diagram_id,
                    depth + 1,
                    children,
                    represented,
                    path,
                    rows,
                );
            }
        }
        path.remove(subject);
    }
    fn append_view(
        view: &VisualComponent,
        depth: usize,
        components: &[&VisualComponent],
        parents: &HashMap<String, Option<String>>,
        children: &HashMap<String, Vec<ExplorerRow>>,
        represented: &HashSet<&str>,
        seen: &mut HashSet<String>,
        rows: &mut Vec<ExplorerRow>,
    ) {
        if !seen.insert(view.view_id.clone()) {
            return;
        }
        let key = format!("view:{}:{}", view.diagram_id, view.view_id);
        rows.push(ExplorerRow {
            key: key.clone(),
            diagram_context: Some(view.diagram_id.clone()),
            element_id: view
                .subject_id
                .clone()
                .unwrap_or_else(|| view.view_id.clone()),
            label: view.label.clone(),
            kind: view.kind.clone(),
            depth,
            diagram: false,
            view: Some(view.clone()),
        });
        if let Some(subject) = &view.subject_id {
            append_members(
                subject,
                &key,
                &view.diagram_id,
                depth + 1,
                children,
                represented,
                &mut HashSet::new(),
                rows,
            );
        }
        for child in components.iter().filter(|child| {
            parents.get(&child.view_id).and_then(Option::as_deref) == Some(view.view_id.as_str())
        }) {
            append_view(
                child,
                depth + 1,
                components,
                parents,
                children,
                represented,
                seen,
                rows,
            );
        }
    }
    let mut rows = Vec::new();
    for row in &authored {
        if hidden.contains(&row.key) {
            continue;
        }
        rows.push(row.clone());
        if !row.diagram {
            continue;
        }
        let mut views = components
            .iter()
            .filter(|view| view.diagram_id == row.element_id)
            .collect::<Vec<_>>();
        if sort == ExplorerSortOrder::Name {
            views.sort_by(|left, right| {
                left.label
                    .to_lowercase()
                    .cmp(&right.label.to_lowercase())
                    .then_with(|| left.view_id.cmp(&right.view_id))
            });
        }
        let represented = views
            .iter()
            .filter_map(|view| view.subject_id.as_deref())
            .collect::<HashSet<_>>();
        let mut parents = HashMap::new();
        for view in &views {
            let parent = view
                .parent_view_id
                .as_ref()
                .filter(|parent| views.iter().any(|candidate| &candidate.view_id == *parent))
                .cloned();
            // UML ownership does not imply UMLDI containment. Only the authored
            // view parent determines where an occurrence belongs in the diagram.
            parents.insert(view.view_id.clone(), parent);
        }
        let mut seen = HashSet::new();
        for view in views
            .iter()
            .filter(|view| parents.get(&view.view_id).is_none_or(Option::is_none))
        {
            append_view(
                view,
                row.depth + 1,
                &views,
                &parents,
                &semantic_children,
                &represented,
                &mut seen,
                &mut rows,
            );
        }
        // Malformed presentation cycles must not hide objects or loop forever.
        for view in &views {
            append_view(
                view,
                row.depth + 1,
                &views,
                &parents,
                &semantic_children,
                &represented,
                &mut seen,
                &mut rows,
            );
        }
    }
    place_diagrams_at_occurrences(section, rows)
}

fn place_diagrams_at_occurrences(
    section: &ModelingSection,
    rows: Vec<ExplorerRow>,
) -> Vec<ExplorerRow> {
    use std::collections::{HashMap, HashSet};
    let nodes = rows
        .iter()
        .map(|row| (row.key.clone(), row))
        .collect::<HashMap<_, _>>();
    let mut children: HashMap<Option<String>, Vec<String>> = HashMap::new();
    let mut ancestors: Vec<String> = Vec::new();
    for row in &rows {
        ancestors.truncate(row.depth);
        let mut parent = ancestors.last().cloned();
        if row.diagram && !section.composition.contains_key(&row.element_id) {
            if let Some(occurrence) = section
                .hierarchy
                .get(&row.element_id)
                .and_then(|placement| placement.occurrence.as_ref())
            {
                let mut target = format!("view:{}:{}", occurrence.diagram_id, occurrence.view_id);
                for member in &occurrence.member_path {
                    target.push_str(&format!("/member:{member}"));
                }
                // A stale occurrence stays visible at the root. Resolving an
                // occurrence must never reassign the semantic object's owner.
                parent = nodes.contains_key(&target).then_some(target);
            }
        }
        children.entry(parent).or_default().push(row.key.clone());
        ancestors.push(row.key.clone());
    }
    fn visit(
        key: &str,
        depth: usize,
        nodes: &HashMap<String, &ExplorerRow>,
        children: &HashMap<Option<String>, Vec<String>>,
        seen: &mut HashSet<String>,
        result: &mut Vec<ExplorerRow>,
    ) {
        if !seen.insert(key.to_owned()) {
            return;
        }
        let Some(row) = nodes.get(key) else {
            return;
        };
        let mut row = (**row).clone();
        row.depth = depth;
        result.push(row);
        if let Some(descendants) = children.get(&Some(key.to_owned())) {
            for child in descendants {
                visit(child, depth + 1, nodes, children, seen, result);
            }
        }
    }
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    if let Some(roots) = children.get(&None) {
        for root in roots {
            visit(root, 0, &nodes, &children, &mut seen, &mut result);
        }
    }
    // Invalid cycles remain inspectable; each row is emitted exactly once.
    for row in &rows {
        visit(&row.key, 0, &nodes, &children, &mut seen, &mut result);
    }
    result
}

fn explorer_type_label(kind: &str, relationship: bool) -> String {
    if let Some(label) = imported_kind_label(kind) {
        return label;
    }
    if relationship {
        return "Relationship".into();
    }
    let label = crate::semantic_identity(kind).type_label;
    if label == kind {
        "Element".into()
    } else {
        label.into()
    }
}

fn imported_kind_label(kind: &str) -> Option<String> {
    if let Some(label) = crate::mde_api_projection_tools::imported_type_label(kind) {
        return Some(label.into());
    }
    let kind = kind.strip_prefix("mdj.")?;
    let kind = kind.strip_prefix("UML").unwrap_or(kind);
    let mut label = String::new();
    let mut previous_lowercase = false;
    for character in kind.chars() {
        if character.is_uppercase() && previous_lowercase {
            label.push(' ');
        }
        label.push(character);
        previous_lowercase = character.is_lowercase();
    }
    Some(label)
}

fn association_end_label(
    references: &crate::uml::references::ReferenceIndex<'_>,
    element: &crate::Element,
) -> Option<String> {
    use crate::uml::{associations, root::ElementRef};
    if !ElementRef::Object(element).conforms_to("Property") {
        return None;
    }
    let association = match associations::association_of(references, &element.id) {
        Ok(Some(association)) => association,
        Ok(None) => return None,
        Err(error) => return Some(format!("Property (unresolved association: {error})")),
    };
    let ends = match associations::member_ends(references, association.id()) {
        Ok(ends) => ends,
        Err(error) => return Some(format!("Association end (unresolved: {error})")),
    };
    let position = ends.iter().position(|end| end.id == element.id)? + 1;
    let role = element.name.trim();
    let label = if role.is_empty() {
        format!("Association end {position}")
    } else {
        format!("Association end {position}: {role}")
    };
    match associations::property_type(references, element) {
        Ok(Some(target)) => {
            let name = target
                .named()
                .and_then(|named| named.name())
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .unwrap_or(target.id());
            Some(format!("{label}: {name}"))
        }
        Ok(None) => Some(format!("{label} (type not set)")),
        Err(error) => Some(format!("{label} (unresolved type: {error})")),
    }
}

fn imported_element_label(section: &ModelingSection, element: &crate::Element) -> Option<String> {
    let type_label = imported_kind_label(&element.kind)?;
    if !element.name.trim().is_empty() {
        return Some(element.name.clone());
    }
    let endpoint_label = |end: &crate::Element| {
        end.properties
            .get("reference")
            .and_then(serde_json::Value::as_object)
            .filter(|reference| reference.len() == 1)
            .and_then(|reference| reference.get("$ref"))
            .and_then(serde_json::Value::as_str)
            .map(|id| {
                section
                    .elements
                    .iter()
                    .find(|target| target.id == id)
                    .map(|target| target.name.trim())
                    .filter(|name| !name.is_empty())
                    .unwrap_or(id)
                    .to_owned()
            })
    };
    if element.kind == "mdj.ERDRelationship" {
        let endpoint = |slot: &str| {
            section
                .elements
                .iter()
                .find(|end| {
                    end.kind == "mdj.ERDRelationshipEnd"
                        && section.composition.get(&end.id).is_some_and(|placement| {
                            placement.slot == slot
                                && placement.parent_id(&end.id, &section.relations)
                                    == Some(element.id.as_str())
                        })
                })
                .and_then(endpoint_label)
                .unwrap_or_else(|| "Unresolved endpoint".into())
        };
        return Some(format!("({} — {})", endpoint("/end1"), endpoint("/end2")));
    }
    if element.kind == "mdj.ERDRelationshipEnd" {
        let slot = section
            .composition
            .get(&element.id)
            .map(|placement| placement.slot.as_str());
        let role = match slot {
            Some("/end1") => "End 1",
            Some("/end2") => "End 2",
            _ => type_label.as_str(),
        };
        return Some(
            endpoint_label(element)
                .map_or_else(|| role.into(), |target| format!("{role}: {target}")),
        );
    }
    let endpoint = |slot: &str| {
        element
            .properties
            .get(slot)
            .and_then(serde_json::Value::as_object)
            .filter(|reference| reference.len() == 1)
            .and_then(|reference| reference.get("$ref"))
            .and_then(serde_json::Value::as_str)
            .and_then(|id| section.elements.iter().find(|target| target.id == id))
            .map(|target| target.name.trim())
            .filter(|name| !name.is_empty())
    };
    if let (Some(source), Some(target)) = (endpoint("source"), endpoint("target")) {
        return Some(format!("{type_label}: {source} → {target}"));
    }
    Some(type_label)
}

fn model_creation_kinds(
    section: &ModelingSection,
    owner: Option<&str>,
) -> Vec<ModelingElementKind> {
    use bellatrix_mde_contracts::protocol::command::ModelingElementKind::*;
    if owner.is_some_and(|id| {
        !section.elements.iter().any(|element| {
            element.id == id && matches!(element.kind.as_str(), "uml.model" | "uml.package")
        })
    }) {
        return Vec::new();
    }
    vec![
        Model,
        Subsystem,
        Package,
        Profile,
        Class,
        Interface,
        Signal,
        DataType,
        PrimitiveType,
        Enumeration,
        Artifact,
        Component,
        Node,
        UseCase,
        Actor,
        Activity,
        StateMachine,
        Interaction,
    ]
}

fn project_diagram_kinds() -> Vec<StructuralDiagramKind> {
    crate::StructuralDiagramKind::ALL_DIAGRAMS
        .into_iter()
        .filter(|kind| {
            kind.context_metaclass().is_none()
                && *kind != crate::StructuralDiagramKind::CompositeStructure
        })
        .map(project_diagram_kind)
        .collect()
}

fn parent_occurrence_from_rows(
    section: &ModelingSection,
    subject: Option<&str>,
    row_key: &str,
    rows: &[ExplorerRow],
) -> Option<crate::ViewOccurrence> {
    let subject = subject?;
    let index = rows.iter().position(|row| row.key == row_key)?;
    let row = rows.get(index)?;
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
    let mut member_path = Vec::new();
    let mut depth = row.depth;
    for candidate in std::iter::once(row).chain(
        rows[..index]
            .iter()
            .rev()
            .filter(|candidate| candidate.depth < row.depth),
    ) {
        if candidate.key != row.key && candidate.depth >= depth {
            continue;
        }
        depth = candidate.depth;
        if candidate.diagram {
            return None;
        }
        if let Some(view) = &candidate.view {
            member_path.reverse();
            let occurrence = crate::ViewOccurrence {
                diagram_id: view.diagram_id.clone(),
                view_id: view.view_id.clone(),
                member_path,
            };
            return crate::validate_diagram_occurrence(section, subject, &occurrence)
                .is_ok()
                .then_some(occurrence);
        }
        member_path.push(candidate.element_id.clone());
    }
    None
}

/// Document objects remain document objects even when pictured by multiple views.
/// View navigation continues through the canvas; it is not semantic containment.
fn project_authored_composition(
    section: &ModelingSection,
    sort: ExplorerSortOrder,
) -> Vec<ExplorerRow> {
    use std::collections::{HashMap, HashSet};
    let semantic = crate::project_hierarchy(section);
    let references =
        crate::uml::references::ReferenceIndex::new(crate::uml::root::Model::from_section(section));
    let labels = semantic
        .iter()
        .map(|row| (row.element_id.clone(), row.label.clone()))
        .collect::<HashMap<_, _>>();
    let semantic_order = semantic
        .iter()
        .enumerate()
        .map(|(index, row)| (row.element_id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut native_parents = HashMap::new();
    let mut ancestors = Vec::new();
    for row in &semantic {
        ancestors.truncate(row.depth);
        native_parents.insert(row.element_id.clone(), ancestors.last().cloned());
        ancestors.push(row.element_id.clone());
    }
    let mut identities = semantic
        .iter()
        .map(|row| row.element_id.clone())
        .chain(section.diagrams.iter().map(|diagram| diagram.id.clone()))
        .collect::<Vec<_>>();
    identities.sort();
    identities.dedup();
    identities.retain(|id| {
        !section.elements.iter().any(|element| {
            element.id == *id
                && element.kind == "mdj.UMLImage"
                && section
                    .composition
                    .get(id)
                    .is_some_and(|placement| placement.slot == "/icon")
        })
    });
    let source_order = section
        .elements
        .iter()
        .map(|element| element.id.as_str())
        .chain(
            section
                .relations
                .iter()
                .map(|relation| relation.id.as_str()),
        )
        .chain(section.diagrams.iter().map(|diagram| diagram.id.as_str()))
        .enumerate()
        .map(|(order, id)| (id, order))
        .collect::<HashMap<_, _>>();
    let mut nodes = HashMap::new();
    for id in &identities {
        let row = if let Some(diagram) = section.diagrams.iter().find(|diagram| diagram.id == *id) {
            ExplorerRow {
                key: diagram_key(id),
                element_id: id.clone(),
                label: diagram.name.clone(),
                kind: diagram.uml_di_type.clone(),
                depth: 0,
                diagram: true,
                view: None,
                diagram_context: None,
            }
        } else if let Some(element) = section.elements.iter().find(|element| element.id == *id) {
            ExplorerRow {
                key: semantic_key(id),
                element_id: id.clone(),
                label: association_end_label(&references, element)
                    .or_else(|| imported_element_label(section, element))
                    .unwrap_or_else(|| {
                        labels
                            .get(id)
                            .filter(|label| !label.trim().is_empty())
                            .cloned()
                            .unwrap_or_else(|| explorer_type_label(&element.kind, false))
                    }),
                kind: element.kind.clone(),
                depth: 0,
                diagram: false,
                view: None,
                diagram_context: None,
            }
        } else if let Some(relation) = section.relations.iter().find(|relation| relation.id == *id)
        {
            ExplorerRow {
                key: semantic_key(id),
                element_id: id.clone(),
                label: crate::visual_consistency::relationship_display_label(section, relation),
                kind: crate::relationship_presentation(section, relation).notation,
                depth: 0,
                diagram: false,
                view: None,
                diagram_context: None,
            }
        } else {
            continue;
        };
        nodes.insert(id.clone(), row);
    }
    let mut children: HashMap<Option<String>, Vec<String>> = HashMap::new();
    for id in nodes.keys() {
        let parent = if let Some(placement) = section.composition.get(id) {
            placement
                .parent_id(id, &section.relations)
                .map(str::to_owned)
        } else if nodes[id].diagram {
            section.hierarchy.get(id).and_then(|placement| {
                // Occurrence parents are resolved after diagram views and their
                // semantic members have been projected, not in the model tree.
                placement
                    .occurrence
                    .is_none()
                    .then(|| placement.parent_id.clone())
                    .flatten()
            })
        } else {
            native_parents.get(id).cloned().flatten()
        }
        .filter(|parent| nodes.contains_key(parent));
        children.entry(parent).or_default().push(id.clone());
    }
    for siblings in children.values_mut() {
        siblings.sort_by(|left, right| {
            let left_composed = section.composition.contains_key(left);
            let right_composed = section.composition.contains_key(right);
            if left_composed != right_composed {
                return right_composed.cmp(&left_composed);
            }
            if !section.composition.contains_key(left) && !section.composition.contains_key(right) {
                let left_parameter =
                    crate::semantic_identity(&nodes[left].kind).type_label == "Parameter";
                let right_parameter =
                    crate::semantic_identity(&nodes[right].kind).type_label == "Parameter";
                if left_parameter != right_parameter {
                    return right_parameter.cmp(&left_parameter);
                }
                if left_parameter {
                    return semantic_order
                        .get(left.as_str())
                        .cmp(&semantic_order.get(right.as_str()))
                        .then_with(|| left.cmp(right));
                }
            }
            let ordering = match sort {
                ExplorerSortOrder::Name => nodes[left]
                    .label
                    .to_lowercase()
                    .cmp(&nodes[right].label.to_lowercase()),
                ExplorerSortOrder::Added => {
                    match (
                        section.composition.get(left),
                        section.composition.get(right),
                    ) {
                        (Some(left), Some(right)) => left
                            .slot
                            .cmp(&right.slot)
                            .then_with(|| left.order.cmp(&right.order)),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => source_order
                            .get(left.as_str())
                            .cmp(&source_order.get(right.as_str())),
                    }
                }
            };
            ordering.then_with(|| left.cmp(right))
        });
    }
    fn visit(
        id: &str,
        depth: usize,
        nodes: &HashMap<String, ExplorerRow>,
        children: &HashMap<Option<String>, Vec<String>>,
        seen: &mut HashSet<String>,
        rows: &mut Vec<ExplorerRow>,
    ) {
        if !seen.insert(id.to_owned()) {
            return;
        }
        let Some(row) = nodes.get(id) else {
            return;
        };
        let mut row = row.clone();
        row.depth = depth;
        rows.push(row);
        if let Some(children_ids) = children.get(&Some(id.to_owned())) {
            for child in children_ids {
                visit(child, depth + 1, nodes, children, seen, rows);
            }
        }
    }
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    if let Some(roots) = children.get(&None) {
        for root in roots {
            visit(root, 0, &nodes, &children, &mut seen, &mut rows);
        }
    }
    // Structural validation diagnoses invalid graphs; projection must still terminate.
    for id in &identities {
        visit(id, 0, &nodes, &children, &mut seen, &mut rows);
    }
    rows
}
