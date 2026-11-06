use super::ModelingElementKind;
use crate::{Geometry, ModelingSection, RelationshipGraph, Style, UmlType, View};
use std::collections::HashSet;

pub(super) fn initial_geometry(kind: ModelingElementKind, x: f64, y: f64) -> Geometry {
    let (width, height) = kind.initial_size();
    Geometry::Rect {
        x,
        y,
        width,
        height,
    }
}

pub(super) fn initial_geometry_in_diagram(
    kind: ModelingElementKind,
    diagram: &crate::Diagram,
    x: f64,
    y: f64,
) -> Geometry {
    let mut geometry = initial_geometry(kind, x, y);
    if let Geometry::Rect { width, height, .. } = &mut geometry {
        (*width, *height) = kind.initial_size_in_diagram(diagram);
    }
    geometry
}

pub(super) fn element_draft_command(
    diagram: &crate::Diagram,
    kind: ModelingElementKind,
    name: Option<&str>,
    position: (f64, f64),
    owner_id: Option<&str>,
    diagnostic: Option<&str>,
) -> super::Command {
    let mut properties = kind.initial_properties();
    if let Some(reason) = diagnostic {
        properties.insert("semantic_creation_diagnostic".into(), reason.into());
    }
    if let Some(owner) = owner_id {
        properties.insert("requested_owner_id".into(), owner.into());
    }
    super::Command::CreateVisualDraft {
        diagram_id: diagram.id.clone(),
        notation_kind: kind.model_kind().into(),
        label: Some(name.unwrap_or(kind.default_name()).into()),
        geometry: initial_geometry_in_diagram(kind, diagram, position.0, position.1),
        properties,
        parent_view_id: None,
        source_view_id: None,
        target_view_id: None,
    }
}

pub(super) fn place(
    section: &mut ModelingSection,
    diagram_id: &str,
    element_id: &str,
    x: f64,
    y: f64,
    revision: u64,
    project_id: &str,
) -> Result<(), String> {
    if !x.is_finite() || !y.is_finite() {
        return Err("view position must be finite".into());
    }
    let element = section
        .elements
        .iter()
        .find(|element| element.id == element_id)
        .ok_or("element not found")?;
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let z_index = diagram
        .views
        .iter()
        .map(|view| view.z_index)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("diagram z-index overflow")?;
    let source = section
        .diagrams
        .iter()
        .flat_map(|diagram| &diagram.views)
        .find(|view| view.subject.as_deref() == Some(element_id));
    let (geometry, style, label) = if let Some(view) = source {
        let (left, top) = match &view.geometry {
            Geometry::Layout { .. } => {
                return Err("placing a View with unresolved layout is not supported".into());
            }
            Geometry::Rect { x, y, .. } => (*x, *y),
            Geometry::Ellipse { cx, cy, rx, ry } => (cx - rx, cy - ry),
            Geometry::Polygon { points } => {
                let first = points.first().ok_or("element View polygon has no bounds")?;
                points
                    .iter()
                    .fold((first.x, first.y), |(left, top), point| {
                        (left.min(point.x), top.min(point.y))
                    })
            }
            Geometry::Path { .. } => {
                return Err("cannot place an element from a relationship Path View".into());
            }
        };
        let mut geometry = view.geometry.clone();
        geometry.translate(x - left, y - top);
        (geometry, view.style.clone(), view.label.clone())
    } else {
        let semantic_type = UmlType::from_kind(&element.kind);
        let kind = ModelingElementKind::BASIC.iter().chain(ModelingElementKind::ADVANCED)
            .chain(ModelingElementKind::PACKAGES).chain(ModelingElementKind::ANNOTATIONS)
            .chain(ModelingElementKind::STRUCTURAL)
            .chain(ModelingElementKind::PROFILES).chain(ModelingElementKind::BEHAVIOR)
            .chain(ModelingElementKind::ACTIVITY_NODES).chain(ModelingElementKind::STATE_NODES).chain(ModelingElementKind::INTERACTION_NODES).chain(ModelingElementKind::DETAILS).chain(ModelingElementKind::PSEUDOSTATES)
            .copied().find(|kind| kind.model_kind() == element.kind
                || semantic_type.is_some() && UmlType::from_kind(kind.model_kind()) == semantic_type)
            .ok_or_else(|| format!("unsupported element View: {} has no existing geometry or registered creation kind", element.kind))?;
        (
            initial_geometry_in_diagram(kind, diagram, x, y),
            Style::default(),
            None,
        )
    };
    let [identifier] = super::fresh_ids(section, ["mde-view"], revision, project_id)?;
    section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?
        .views
        .push(View {
            children: Vec::new(),
            presentation: Default::default(),
            endpoints: None,
            timing: None,
            draft: None,
            id: identifier,
            subject: Some(element_id.into()),
            label,
            geometry,
            style,
            z_index,
        });
    Ok(())
}

pub(super) fn set_timing(
    section: &mut ModelingSection,
    view_id: &str,
    timing: &Option<crate::TimingPresentation>,
) -> Result<(), String> {
    if let Some(timing) = timing {
        timing.validate()?;
    }
    let view = section
        .diagrams
        .iter_mut()
        .find_map(|diagram| diagram.view_mut(view_id))
        .ok_or("view not found")?;
    view.timing = timing.clone();
    Ok(())
}

pub(super) fn remove(
    section: &mut ModelingSection,
    diagram_id: &str,
    view_id: &str,
) -> Result<(), String> {
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let view = diagram
        .all_views()
        .find(|view| view.id == view_id)
        .ok_or("view not found in diagram")?;
    if !view.children.is_empty() {
        return Err(format!(
            "Cannot remove View {view_id}: removing a View with children requires hierarchy-aware deletion"
        ));
    }
    let element_id = view
        .subject
        .as_ref()
        .filter(|subject| {
            section
                .elements
                .iter()
                .any(|element| &element.id == *subject)
        })
        .cloned();
    let graph = RelationshipGraph::new(section, diagram_id);
    let previously_drawable = diagram
        .all_views()
        .filter(|view| graph.view_path(&view.id).is_some())
        .map(|view| view.id.clone())
        .collect::<Vec<_>>();
    let mut staged = section.clone();
    let mut removed = HashSet::from([view_id.to_owned()]);
    let diagram = staged
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    remove_view_ids(&mut diagram.views, &removed);
    if let Some(element_id) = element_id {
        let graph = RelationshipGraph::new(&staged, diagram_id);
        // Preserve the existing policy: only previously drawable semantic edges
        // that lose their route disappear. Existing unrouteable drafts remain.
        let hidden = previously_drawable
            .into_iter()
            .filter(|identifier| graph.view_path(identifier).is_none())
            .collect::<HashSet<_>>();
        let diagram = staged
            .diagrams
            .iter_mut()
            .find(|diagram| diagram.id == diagram_id)
            .ok_or("diagram not found")?;
        if let Some(view) = diagram
            .all_views()
            .find(|view| hidden.contains(&view.id) && !view.children.is_empty())
        {
            return Err(format!(
                "Cannot remove View {view_id}: affected relationship View {} has children",
                view.id
            ));
        }
        remove_view_ids(&mut diagram.views, &hidden);
        removed.extend(hidden);
        crate::relationships::reroute_relationships_for_endpoint(
            &mut staged,
            diagram_id,
            &element_id,
        );
    }
    let removed_ids = removed.iter().map(String::as_str).collect();
    super::ensure_removed_view_dependencies(&staged, &removed_ids, &HashSet::new(), false, true)?;
    *section = staged;
    Ok(())
}

fn remove_view_ids(views: &mut Vec<View>, removed: &HashSet<String>) {
    views.retain_mut(|view| {
        if removed.contains(&view.id) {
            return false;
        }
        remove_view_ids(&mut view.children, removed);
        true
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_draft(
    section: &mut ModelingSection,
    diagram_id: &str,
    notation_kind: &str,
    label: &Option<String>,
    geometry: &Geometry,
    properties: &std::collections::BTreeMap<String, serde_json::Value>,
    parent_view_id: &Option<String>,
    source_view_id: &Option<String>,
    target_view_id: &Option<String>,
    revision: u64,
    project_id: &str,
) -> Result<(), String> {
    if notation_kind.trim().is_empty() {
        return Err("visual notation kind must not be blank".into());
    }
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    for identifier in [parent_view_id, source_view_id, target_view_id]
        .into_iter()
        .flatten()
    {
        if !diagram.all_views().any(|view| &view.id == identifier) {
            return Err(format!(
                "referenced view not found in diagram: {identifier}"
            ));
        }
    }
    let z_index = diagram
        .views
        .iter()
        .map(|view| view.z_index)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("diagram z-index overflow")?;
    let [id] = super::fresh_ids(section, ["mde-view"], revision, project_id)?;
    let diagram = section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    diagram.views.push(View {
        children: Vec::new(),
        presentation: Default::default(),
        endpoints: (source_view_id.is_some() || target_view_id.is_some()).then(|| {
            crate::ViewEndpoints {
                source_view_id: source_view_id.clone(),
                target_view_id: target_view_id.clone(),
            }
        }),
        timing: None,
        id,
        subject: None,
        label: label.clone(),
        geometry: geometry.clone(),
        style: Style::default(),
        z_index,
        draft: Some(crate::VisualDraft {
            notation_kind: notation_kind.into(),
            properties: properties.clone(),
            parent_view_id: parent_view_id.clone(),
        }),
    });
    crate::reroute_visual_drafts(diagram);
    Ok(())
}

pub(super) fn set_endpoints(
    section: &mut ModelingSection,
    diagram_id: &str,
    view_id: &str,
    endpoints: &crate::ViewEndpoints,
) -> Result<(), String> {
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let view = diagram
        .all_views()
        .find(|view| view.id == view_id)
        .ok_or("view not found in diagram")?;
    if !matches!(view.geometry, Geometry::Path { .. }) {
        return Err("only relationship Path Views can have endpoints".into());
    }
    for identifier in [&endpoints.source_view_id, &endpoints.target_view_id]
        .into_iter()
        .flatten()
    {
        if identifier == view_id || !diagram.all_views().any(|view| &view.id == identifier) {
            return Err(format!("invalid endpoint View in diagram: {identifier}"));
        }
    }
    if let Some(relation) = view.subject.as_ref().and_then(|subject| {
        section
            .relations
            .iter()
            .find(|relation| &relation.id == subject)
    }) {
        crate::relationships::route_at_views(
            section,
            diagram_id,
            &relation.from,
            &relation.to,
            endpoints,
        )?;
    }
    let diagram = section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let view = diagram
        .view_mut(view_id)
        .ok_or("view not found in diagram")?;
    view.endpoints = Some(endpoints.clone());
    crate::reroute_visual_drafts(diagram);
    crate::reroute_relationships(section, diagram_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        Project,
        commands::{Command, apply},
    };

    fn fixture() -> serde_json::Result<Project> {
        let mut project: Project = serde_json::from_value(serde_json::json!({
            "id": "model", "name": "Shared subjects",
            "elements": [{"id": "a", "kind": "uml.class", "name": "A"},
                {"id": "b", "kind": "uml.class", "name": "B"}],
            "relations": [{"id": "link", "kind": "uml.association", "from": "a", "to": "b",
                "properties": {}}],
            "diagrams": [{"id": "one", "name": "One", "uml_di_type": "UMLClassDiagram", "views": [
                {"id": "a-first", "subject": "a", "geometry": {"kind": "rect", "x": 10.0, "y": 20.0, "width": 180.0, "height": 100.0}},
                {"id": "a-second", "subject": "a", "geometry": {"kind": "rect", "x": 10.0, "y": 220.0, "width": 180.0, "height": 100.0}},
                {"id": "b-first", "subject": "b", "geometry": {"kind": "rect", "x": 400.0, "y": 20.0, "width": 180.0, "height": 100.0}},
                {"id": "link-first", "subject": "link", "style": {"routing":"automatic"}, "geometry": {"kind": "path", "start": {"x":190.0,"y":70.0},
                    "segments": [{"kind":"line","to":{"x":400.0,"y":70.0}}]}}
            ]}]
        }))?;
        if let Some(first) = project.diagrams.first() {
            let mut second = first.clone();
            second.id = "two".into();
            second.name = "Two".into();
            for view in &mut second.views {
                view.id = format!("other-{}", view.id);
            }
            project.diagrams.push(second);
        }
        Ok(project)
    }

    fn remove(diagram: &str, view: &str) -> Command {
        Command::RemoveView {
            diagram_id: diagram.into(),
            view_id: view.into(),
        }
    }

    #[test]
    fn removing_one_copy_keeps_other_views_and_reanchors_relationship()
    -> Result<(), Box<dyn std::error::Error>> {
        let original = fixture()?;
        let updated = apply(&original, &remove("one", "a-first"), 0)?;
        assert_eq!(updated.elements, original.elements);
        assert_eq!(updated.relations, original.relations);
        assert_eq!(updated.diagrams.get(1), original.diagrams.get(1));
        let diagram = updated.diagrams.first().ok_or("diagram missing")?;
        assert!(!diagram.views.iter().any(|view| view.id == "a-first"));
        assert!(diagram.views.iter().any(|view| view.id == "a-second"));
        assert!(diagram.views.iter().any(|view| view.id == "link-first"));
        let old_edge = original
            .diagrams
            .first()
            .and_then(|diagram| diagram.views.iter().find(|view| view.id == "link-first"));
        let new_edge = diagram.views.iter().find(|view| view.id == "link-first");
        assert_ne!(
            new_edge.map(|view| &view.geometry),
            old_edge.map(|view| &view.geometry)
        );
        Ok(())
    }

    #[test]
    fn removing_last_endpoint_view_hides_only_local_relationship_views()
    -> Result<(), Box<dyn std::error::Error>> {
        let original = fixture()?;
        let updated = apply(&original, &remove("one", "b-first"), 0)?;
        assert_eq!(updated.elements, original.elements);
        assert_eq!(updated.relations, original.relations);
        assert_eq!(updated.diagrams.get(1), original.diagrams.get(1));
        let diagram = updated.diagrams.first().ok_or("diagram missing")?;
        assert!(
            diagram
                .views
                .iter()
                .all(|view| view.subject.as_deref() != Some("link"))
        );
        assert_eq!(diagram.views.len(), 2);
        Ok(())
    }

    #[test]
    fn removing_edge_view_keeps_semantic_relationship_and_other_copies()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut original = fixture()?;
        if let Some(diagram) = original.diagrams.first_mut()
            && let Some(edge) = diagram
                .views
                .iter()
                .find(|view| view.id == "link-first")
                .cloned()
        {
            let mut copy = edge;
            copy.id = "link-second".into();
            diagram.views.push(copy);
        }
        let updated = apply(&original, &remove("one", "link-first"), 0)?;
        let mut expected = original.clone();
        if let Some(diagram) = expected.diagrams.first_mut() {
            diagram.views.retain(|view| view.id != "link-first");
        }
        assert_eq!(updated, expected);
        Ok(())
    }

    fn manual_copies() -> Result<Project, Box<dyn std::error::Error>> {
        let mut project = fixture()?;
        let diagram = project.diagrams.first_mut().ok_or("diagram missing")?;
        for view in &mut diagram.views {
            view.style.routing = None;
        }
        let mut edge = diagram
            .views
            .iter()
            .find(|view| view.id == "link-first")
            .cloned()
            .ok_or("edge missing")?;
        edge.id = "link-second".into();
        if let crate::Geometry::Path { segments, .. } = &mut edge.geometry {
            segments.insert(
                0,
                crate::PathSegment::Line {
                    to: crate::Point { x: 300.0, y: 180.0 },
                },
            );
        }
        diagram.views.push(edge);
        Ok(project)
    }

    #[test]
    fn manual_view_projection_preserves_distinct_paths_and_is_diagram_scoped()
    -> Result<(), Box<dyn std::error::Error>> {
        let original = manual_copies()?;
        let section = crate::ModelingSection::from_model(&original);
        let graph = crate::RelationshipGraph::new(&section, "one");
        let first = graph.view_path("link-first").ok_or("first path missing")?;
        let second = graph
            .view_path("link-second")
            .ok_or("second path missing")?;
        assert_ne!(first, second);
        for identifier in ["missing", "link", "a-first", "other-link-first"] {
            assert!(graph.view_path(identifier).is_none(), "{identifier}");
        }
        assert!(
            crate::RelationshipGraph::new(&section, "missing")
                .view_path("link-first")
                .is_none()
        );
        let mut persisted = section.clone();
        crate::reroute_relationships(&mut persisted, "one");
        let diagram = persisted.diagrams.first().ok_or("diagram missing")?;
        for (identifier, expected) in [("link-first", first), ("link-second", second)] {
            assert_eq!(
                diagram
                    .views
                    .iter()
                    .find(|view| view.id == identifier)
                    .map(|view| &view.geometry),
                Some(&expected)
            );
        }
        assert_eq!(section, crate::ModelingSection::from_model(&original));
        Ok(())
    }

    #[test]
    fn endpoint_removal_checks_each_edge_view_and_preserves_existing_drafts()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut original = manual_copies()?;
        let diagram = original.diagrams.first_mut().ok_or("diagram missing")?;
        let edge = diagram
            .views
            .iter_mut()
            .find(|view| view.id == "link-first")
            .ok_or("edge missing")?;
        if let crate::Geometry::Path { segments, .. } = &mut edge.geometry {
            segments.clear();
        }
        let updated = apply(&original, &remove("one", "b-first"), 0)?;
        let diagram = updated.diagrams.first().ok_or("diagram missing")?;
        assert!(diagram.views.iter().any(|view| view.id == "link-first"));
        assert!(!diagram.views.iter().any(|view| view.id == "link-second"));
        assert_eq!(updated.elements, original.elements);
        assert_eq!(updated.relations, original.relations);
        assert_eq!(updated.diagrams.get(1), original.diagrams.get(1));
        Ok(())
    }

    #[test]
    fn removing_unrelated_node_preserves_each_manual_edge_view_geometry()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut original = manual_copies()?;
        let mut unrelated = original
            .elements
            .first()
            .cloned()
            .ok_or("element missing")?;
        unrelated.id = "unrelated".into();
        original.elements.push(unrelated);
        let diagram = original.diagrams.first_mut().ok_or("diagram missing")?;
        let mut view = diagram.views.first().cloned().ok_or("view missing")?;
        view.id = "unrelated-view".into();
        view.subject = Some("unrelated".into());
        diagram.views.push(view);
        let updated = apply(&original, &remove("one", "unrelated-view"), 0)?;
        let mut expected = original.clone();
        expected
            .diagrams
            .first_mut()
            .ok_or("diagram missing")?
            .views
            .retain(|view| view.id != "unrelated-view");
        assert_eq!(updated, expected);
        Ok(())
    }

    #[test]
    fn endpoint_removal_and_movement_preserve_each_manual_edge_shape()
    -> Result<(), Box<dyn std::error::Error>> {
        let original = manual_copies()?;
        for command in [
            remove("one", "a-first"),
            Command::MoveView {
                view_id: "a-first".into(),
                dx: 0.0,
                dy: 150.0,
            },
        ] {
            let updated = apply(&original, &command, 0)?;
            assert_eq!(updated.elements, original.elements);
            assert_eq!(updated.relations, original.relations);
            assert_eq!(updated.diagrams.get(1), original.diagrams.get(1));
            let diagram = updated.diagrams.first().ok_or("diagram missing")?;
            for (identifier, expected_segments) in [("link-first", 1), ("link-second", 2)] {
                let view = diagram
                    .views
                    .iter()
                    .find(|view| view.id == identifier)
                    .ok_or("edge missing")?;
                let crate::Geometry::Path { start, segments } = &view.geometry else {
                    return Err("edge must remain a path".into());
                };
                assert_eq!(segments.len(), expected_segments, "{identifier}");
                assert_ne!(
                    start.y, 70.0,
                    "endpoint must follow remaining or moved node"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn removal_rejects_wrong_diagram_and_missing_view_atomically() -> serde_json::Result<()> {
        let original = fixture()?;
        let before = original.clone();
        for command in [
            remove("missing", "a-first"),
            remove("two", "a-first"),
            remove("one", "missing"),
        ] {
            assert!(apply(&original, &command, 0).is_err());
            assert_eq!(original, before);
        }
        Ok(())
    }
}
