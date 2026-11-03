use super::{Command, ModelingElementKind, apply};
use crate::{Geometry, PathSegment, Point, Project};

fn fixture() -> serde_json::Result<Project> {
    serde_json::from_value(serde_json::json!({
        "id": "model", "name": "Shared element",
        "elements": [{"id": "class", "kind": "uml.class", "name": "Class"}],
        "diagrams": [
            {"id": "one", "name": "One", "uml_di_type": "UMLCompositeStructureDiagram", "model_elements": ["class"], "views": [
                {"id": "original-view", "subject": "class", "label": "Custom caption", "z_index": 4,
                    "style": {"fill": "#112233", "stroke_width": 2.0},
                    "geometry": {"kind": "rect", "x": 10.0, "y": 20.0, "width": 230.0, "height": 160.0}}
            ]},
            {"id": "two", "name": "Two", "uml_di_type": "UMLClassDiagram"}
        ]
    }))
}

fn place(diagram: &str, element: &str, x: f64, y: f64) -> Command {
    Command::PlaceElementView {
        diagram_id: diagram.into(),
        element_id: element.into(),
        x,
        y,
    }
}

#[test]
fn placing_existing_element_copies_view_without_copying_semantics_or_context()
-> Result<(), Box<dyn std::error::Error>> {
    let source = fixture()?;
    let updated = apply(&source, &place("two", "class", 400.0, 500.0), 0)?;
    assert_eq!(updated.elements, source.elements);
    assert_eq!(updated.relations, source.relations);
    assert_eq!(updated.diagrams.first(), source.diagrams.first());
    let original_view = source
        .diagrams
        .first()
        .and_then(|diagram| diagram.views.first())
        .ok_or("source view missing")?;
    let target = updated.diagrams.get(1).ok_or("target diagram missing")?;
    assert!(target.model_elements.is_empty());
    let copied = target.views.first().ok_or("placed view missing")?;
    assert_ne!(copied.id, original_view.id);
    assert_eq!(copied.subject, original_view.subject);
    assert_eq!(copied.label, original_view.label);
    assert_eq!(copied.style, original_view.style);
    assert_eq!(
        copied.geometry,
        Geometry::Rect {
            x: 400.0,
            y: 500.0,
            width: 230.0,
            height: 160.0
        }
    );
    let repeated = apply(&updated, &place("two", "class", 700.0, 500.0), 0)?;
    let views = &repeated.diagrams.get(1).ok_or("target missing")?.views;
    assert_eq!(views.len(), 2);
    assert_eq!(
        views
            .iter()
            .map(|view| &view.id)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        2
    );
    assert_eq!(repeated.elements, source.elements);
    Ok(())
}

#[test]
fn ellipse_and_polygon_geometry_translate_from_their_bounding_top_left()
-> Result<(), Box<dyn std::error::Error>> {
    for (geometry, expected) in [
        (
            Geometry::Ellipse {
                cx: 30.0,
                cy: 50.0,
                rx: 20.0,
                ry: 30.0,
            },
            Geometry::Ellipse {
                cx: 420.0,
                cy: 530.0,
                rx: 20.0,
                ry: 30.0,
            },
        ),
        (
            Geometry::Polygon {
                points: vec![
                    Point { x: 30.0, y: 20.0 },
                    Point { x: 10.0, y: 60.0 },
                    Point { x: 50.0, y: 60.0 },
                ],
            },
            Geometry::Polygon {
                points: vec![
                    Point { x: 420.0, y: 500.0 },
                    Point { x: 400.0, y: 540.0 },
                    Point { x: 440.0, y: 540.0 },
                ],
            },
        ),
    ] {
        let mut source = fixture()?;
        if let Some(view) = source
            .diagrams
            .first_mut()
            .and_then(|diagram| diagram.views.first_mut())
        {
            view.geometry = geometry;
        }
        let updated = apply(&source, &place("two", "class", 400.0, 500.0), 0)?;
        let placed = updated
            .diagrams
            .get(1)
            .and_then(|diagram| diagram.views.first())
            .ok_or("placed view missing")?;
        assert_eq!(placed.geometry, expected);
    }
    Ok(())
}

#[test]
fn never_viewed_element_uses_the_same_creation_geometry() -> Result<(), Box<dyn std::error::Error>>
{
    let mut source = fixture()?;
    for diagram in &mut source.diagrams {
        diagram.views.clear();
    }
    let updated = apply(&source, &place("two", "class", 400.0, 500.0), 0)?;
    let placed = updated
        .diagrams
        .get(1)
        .and_then(|diagram| diagram.views.first())
        .ok_or("placed view missing")?;
    assert_eq!(
        placed.geometry,
        super::views::initial_geometry(ModelingElementKind::Class, 400.0, 500.0)
    );
    assert_eq!(updated.elements, source.elements);
    Ok(())
}

#[test]
fn invalid_placements_are_atomic_and_do_not_invent_unsupported_geometry() -> serde_json::Result<()>
{
    let source = fixture()?;
    for command in [
        place("missing", "class", 0.0, 0.0),
        place("two", "missing", 0.0, 0.0),
        place("two", "class", f64::NAN, 0.0),
        place("two", "class", 0.0, f64::INFINITY),
    ] {
        let before = source.clone();
        assert!(apply(&source, &command, 0).is_err());
        assert_eq!(source, before);
    }
    let mut unsupported = fixture()?;
    for element in &mut unsupported.elements {
        element.kind = "uml.unsupported_figure".into();
    }
    for diagram in &mut unsupported.diagrams {
        diagram.views.clear();
    }
    let before = unsupported.clone();
    assert!(
        matches!(apply(&unsupported, &place("two", "class", 0.0, 0.0), 0), Err(error) if error.contains("unsupported element View"))
    );
    assert_eq!(unsupported, before);
    let mut path_source = fixture()?;
    if let Some(view) = path_source
        .diagrams
        .first_mut()
        .and_then(|diagram| diagram.views.first_mut())
    {
        view.geometry = Geometry::Path {
            start: Point { x: 0.0, y: 0.0 },
            segments: vec![PathSegment::Line {
                to: Point { x: 10.0, y: 10.0 },
            }],
        };
    }
    let before = path_source.clone();
    assert!(
        matches!(apply(&path_source, &place("two", "class", 0.0, 0.0), 0), Err(error) if error.contains("Path View"))
    );
    assert_eq!(path_source, before);
    Ok(())
}

#[test]
fn project_identity_is_reserved_when_allocating_a_view_id() -> Result<(), Box<dyn std::error::Error>>
{
    let mut source = fixture()?;
    source.id = "mde-view-1".into();
    let updated = apply(&source, &place("two", "class", 400.0, 500.0), 0)?;
    let view = updated
        .diagrams
        .get(1)
        .and_then(|diagram| diagram.views.first())
        .ok_or("placed view missing")?;
    assert_ne!(view.id, source.id);
    assert_eq!(updated.id, source.id);
    assert_eq!(updated.elements, source.elements);
    assert_eq!(updated.relations, source.relations);
    crate::Graph::new(&updated).validate()?;
    Ok(())
}
