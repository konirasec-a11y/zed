use bellatrix_mde::{
    commands::{Command, apply},
    *,
};

fn fixture() -> Result<Project, Box<dyn std::error::Error>> {
    let project: Project = serde_json::from_value(serde_json::json!({
        "id":"styles", "name":"Styles", "elements":[{"id":"a","kind":"uml.class","name":"A"},{"id":"b","kind":"uml.class","name":"B"}],
        "diagrams":[{"id":"d","name":"D","uml_di_type":"UMLClassDiagram","views":[
            {"id":"a-view","subject":"a","geometry":{"kind":"rect","x":0,"y":0,"width":100,"height":80}},
            {"id":"b-view","subject":"b","geometry":{"kind":"rect","x":300,"y":0,"width":100,"height":80}}
        ]}]
    }))?;
    Ok(apply(
        &project,
        &Command::CreateRelation { owner_id: None,
            diagram_id: "d".into(),
            kind: ModelingRelationKind::Association,
            from: "a".into(),
            to: "b".into(),
        },
        0,
    )?)
}

#[test]
fn route_commands_author_curves_isolate_views_and_roundtrip()
-> Result<(), Box<dyn std::error::Error>> {
    let mut project = fixture()?;
    let edge = project.diagrams[0]
        .views
        .last()
        .ok_or("missing edge")?
        .clone();
    let mut copy = edge.clone();
    copy.id = "copy".into();
    project.diagrams[0].views.push(copy.clone());
    let mut second = project.diagrams[0].clone();
    second.id = "other".into();
    for view in &mut second.views {
        view.id = format!("other-{}", view.id);
    }
    project.diagrams.push(second.clone());
    assert!(!project.relations[0].properties.contains_key("routing"));
    for routing in [
        RelationshipRouting::Straight,
        RelationshipRouting::Automatic,
        RelationshipRouting::Curve,
    ] {
        let updated = apply(
            &project,
            &Command::SetViewRouting {
                diagram_id: "d".into(),
                view_id: edge.id.clone(),
                routing,
            },
            1,
        )?;
        assert_eq!(updated.relations, project.relations);
        assert_eq!(updated.elements, project.elements);
        assert_eq!(updated.diagrams[1], second);
        assert_eq!(updated.diagrams[0].views.last(), Some(&copy));
        let view = updated.diagrams[0]
            .views
            .iter()
            .find(|view| view.id == edge.id)
            .ok_or("edge missing")?;
        assert_eq!(view.style.routing, Some(routing));
        let mut styled = view.style.clone();
        styled.stroke = Some("#aabbcc".into());
        let restyled = apply(
            &updated,
            &Command::SetViewStyle {
                diagram_id: "d".into(),
                view_id: edge.id.clone(),
                style: styled,
            },
            2,
        )?;
        let restyled_view = restyled.diagrams[0]
            .views
            .iter()
            .find(|item| item.id == edge.id)
            .ok_or("edge missing")?;
        assert_eq!(restyled_view.geometry, view.geometry);
        assert_eq!(restyled_view.style.routing, view.style.routing);
        assert_eq!(restyled.diagrams[0].views.last(), Some(&copy));
        assert_eq!(restyled.diagrams[1], second);
        if routing == RelationshipRouting::Curve {
            assert!(
                matches!(&view.geometry,Geometry::Path {segments,..} if matches!(segments.first(),Some(PathSegment::Cubic {..})))
            );
            let moved = apply(
                &updated,
                &Command::MoveView {
                    view_id: "a-view".into(),
                    dx: 0.0,
                    dy: 100.0,
                },
                2,
            )?;
            let moved_view = moved.diagrams[0]
                .views
                .iter()
                .find(|view| view.id == edge.id)
                .ok_or("edge missing")?;
            assert_ne!(moved_view.geometry, view.geometry);
            assert!(
                matches!(&moved_view.geometry,Geometry::Path {segments,..} if matches!(segments.first(),Some(PathSegment::Cubic {..})))
            );
            let section = ModelingSection::from_model(&moved);
            assert_eq!(
                RelationshipGraph::new(&section, "d").view_path(&edge.id),
                Some(moved_view.geometry.clone())
            );
        }
        for format in [Format::Json, Format::Protobuf] {
            assert_eq!(decode(&encode(&updated, format)?, format)?, updated);
        }
    }
    Ok(())
}

#[test]
fn style_validation_is_atomic_and_preserves_semantics() -> Result<(), Box<dyn std::error::Error>> {
    let original = fixture()?;
    let style = Style {
        fill: Some("#112233".into()),
        stroke_width: Some(2.0),
        font_size: Some(16.0),
        opacity: Some(0.8),
        ..Style::default()
    };
    let command = |style| Command::SetViewStyle {
        diagram_id: "d".into(),
        view_id: "a-view".into(),
        style,
    };
    let updated = apply(&original, &command(style.clone()), 1)?;
    assert_eq!(updated.diagrams[0].views[0].style, style);
    assert_eq!(updated.relations, original.relations);
    assert_eq!(updated.elements, original.elements);
    for style in [
        Style {
            fill: Some("red".into()),
            ..Style::default()
        },
        Style {
            font_size: Some(f64::NAN),
            ..Style::default()
        },
        Style {
            stroke_width: Some(-1.0),
            ..Style::default()
        },
        Style {
            opacity: Some(2.0),
            ..Style::default()
        },
        Style {
            routing: Some(RelationshipRouting::Curve),
            ..Style::default()
        },
    ] {
        assert!(apply(&original, &command(style), 1).is_err());
    }
    assert!(
        apply(
            &original,
            &Command::SetViewRouting {
                diagram_id: "d".into(),
                view_id: "a-view".into(),
                routing: RelationshipRouting::Curve
            },
            1
        )
        .is_err()
    );
    let edge = original.diagrams[0].views.last().ok_or("edge missing")?;
    let mut custom = edge.style.clone();
    custom.dash_pattern = vec![3.0, 2.0];
    custom.line_mode = Some(LineMode::Dashed);
    let styled = apply(
        &original,
        &Command::SetViewStyle {
            diagram_id: "d".into(),
            view_id: edge.id.clone(),
            style: custom,
        },
        1,
    )?;
    assert_eq!(styled.elements, original.elements);
    assert_eq!(styled.relations, original.relations);
    for mode in [LineMode::Solid, LineMode::Dotted, LineMode::Dashed] {
        let mut style = edge.style.clone();
        style.line_mode = Some(mode);
        let styled = apply(
            &original,
            &Command::SetViewStyle {
                diagram_id: "d".into(),
                view_id: edge.id.clone(),
                style,
            },
            1,
        )?;
        assert_eq!(styled.relations, original.relations);
        for format in [Format::Json, Format::Protobuf] {
            assert_eq!(decode(&encode(&styled, format)?, format)?, styled);
        }
    }
    assert_eq!(original, fixture()?);
    Ok(())
}

#[test]
fn presentation_alignment_and_stack_are_view_local_and_ignore_unrelated_semantic_errors()
-> Result<(), Box<dyn std::error::Error>> {
    use bellatrix_mde::commands::ViewOrder;
    let mut project = fixture()?;
    let mut other = project.diagrams[0].clone();
    other.id = "other-diagram".into();
    for view in &mut other.views {
        view.id = format!("other-{}", view.id);
    }
    project.diagrams.push(other.clone());
    // A bad semantic ownership cycle must not prevent fixing presentation.
    project.relations.push(Relation {
        id: "bad-ownership".into(),
        kind: "owns".into(),
        from: "a".into(),
        to: "a".into(),
        properties: Default::default(),
    });
    assert!(Graph::new(&project).validate().is_err());
    let original_elements = project.elements.clone();
    let original_relations = project.relations.clone();
    let style = Style {
        text_align: Some(TextAlignment::Right),
        text_vertical_align: Some(TextVerticalAlignment::Bottom),
        uml: UmlStyle {
            stereotype_display: Some(StereotypeDisplay::Hidden),
            suppress_attributes: Some(true),
            suppress_operations: Some(true),
            suppress_receptions: Some(true),
            ..UmlStyle::default()
        },
        fill: Some("#112233".into()),
        ..Style::default()
    };
    project = apply(
        &project,
        &Command::SetViewStyle {
            diagram_id: "d".into(),
            view_id: "a-view".into(),
            style: style.clone(),
        },
        10,
    )?;
    for (order, expected) in [
        (ViewOrder::BringToFront, 2),
        (ViewOrder::SendBackward, 1),
        (ViewOrder::SendToBack, 0),
        (ViewOrder::BringForward, 1),
    ] {
        project = apply(
            &project,
            &Command::ReorderView {
                diagram_id: "d".into(),
                view_id: "a-view".into(),
                order,
            },
            11,
        )?;
        let mut ordered: Vec<_> = project.diagrams[0].views.iter().collect();
        ordered.sort_by_key(|view| view.z_index);
        assert_eq!(
            ordered.iter().position(|view| view.id == "a-view"),
            Some(expected)
        );
    }
    assert_eq!(project.elements, original_elements);
    assert_eq!(project.relations, original_relations);
    assert_eq!(project.diagrams[1], other);
    assert_eq!(project.diagrams[0].views[0].style, style);
    for format in [Format::Json, Format::Protobuf] {
        assert_eq!(decode(&encode(&project, format)?, format)?, project);
    }
    let mut malformed = project.clone();
    malformed.diagrams[0].width = f64::NAN;
    assert!(
        apply(
            &malformed,
            &Command::ReorderView {
                diagram_id: "d".into(),
                view_id: "a-view".into(),
                order: ViewOrder::BringToFront
            },
            12
        )
        .is_err()
    );
    Ok(())
}
