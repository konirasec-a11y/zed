use super::ModelingElementKind;
use crate::{
    Geometry, ModelingSection, RelationshipGraph, Style, UmlType, View, reroute_relationships,
};
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

pub(super) fn place(
    section: &mut ModelingSection,
    diagram_id: &str,
    element_id: &str,
    x: f64,
    y: f64,
    revision: u64,
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
            .copied().find(|kind| kind.model_kind() == element.kind
                || semantic_type.is_some() && UmlType::from_kind(kind.model_kind()) == semantic_type)
            .ok_or_else(|| format!("unsupported element View: {} has no existing geometry or registered creation kind", element.kind))?;
        (initial_geometry(kind, x, y), Style::default(), None)
    };
    let [identifier] = super::fresh_ids(section, ["mde-view"], revision)?;
    section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?
        .views
        .push(View {
            id: identifier,
            subject: Some(element_id.into()),
            label,
            geometry,
            style,
            z_index,
        });
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
        .views
        .iter()
        .find(|view| view.id == view_id)
        .ok_or("view not found in diagram")?;
    let is_element = view.subject.as_ref().is_some_and(|subject| {
        section
            .elements
            .iter()
            .any(|element| &element.id == subject)
    });
    let previous_paths = RelationshipGraph::new(section, diagram_id).paths();
    section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?
        .views
        .retain(|view| view.id != view_id);
    if is_element {
        let remaining_paths = RelationshipGraph::new(section, diagram_id).paths();
        // A second View of the same endpoint can keep an edge renderable. Do not
        // erase unrelated draft edges that were already missing a usable route.
        let hidden = previous_paths
            .keys()
            .filter(|identifier| !remaining_paths.contains_key(*identifier))
            .cloned()
            .collect::<HashSet<_>>();
        section
            .diagrams
            .iter_mut()
            .find(|diagram| diagram.id == diagram_id)
            .ok_or("diagram not found")?
            .views
            .retain(|view| {
                view.subject
                    .as_ref()
                    .is_none_or(|subject| !hidden.contains(subject))
            });
        reroute_relationships(section, diagram_id);
    }
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
                "properties": {"routing": "automatic"}}],
            "diagrams": [{"id": "one", "name": "One", "uml_di_type": "UMLClassDiagram", "views": [
                {"id": "a-first", "subject": "a", "geometry": {"kind": "rect", "x": 10.0, "y": 20.0, "width": 180.0, "height": 100.0}},
                {"id": "a-second", "subject": "a", "geometry": {"kind": "rect", "x": 10.0, "y": 220.0, "width": 180.0, "height": 100.0}},
                {"id": "b-first", "subject": "b", "geometry": {"kind": "rect", "x": 400.0, "y": 20.0, "width": 180.0, "height": 100.0}},
                {"id": "link-first", "subject": "link", "geometry": {"kind": "path", "start": {"x":190.0,"y":70.0},
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
