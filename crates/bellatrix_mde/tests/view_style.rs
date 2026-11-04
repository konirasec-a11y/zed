use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, project_model_section,
    validate_graph_model,
};
use bellatrix_mde_contracts::protocol::{command::*, projection::ModelingProjection};
use serde_json::{Value, json};

type FlowResult<T> = Result<T, Box<dyn std::error::Error>>;

fn section(project: &Value) -> FlowResult<(String, String, Value)> {
    let id = project["id"]
        .as_str()
        .ok_or("project identity missing")?
        .to_owned();
    let name = project["name"]
        .as_str()
        .ok_or("project name missing")?
        .to_owned();
    let mut section = project.as_object().ok_or("project missing")?.clone();
    for key in ["id", "name", "schema_version"] {
        section.remove(key);
    }
    Ok((id, name, Value::Object(section)))
}
fn apply(project: &Value, command: &Command, revision: u64) -> FlowResult<Value> {
    let (id, name, section) = section(project)?;
    let updated = apply_model_command(
        &id,
        &name,
        &section.to_string(),
        revision,
        command.clone(),
    )?;
    Ok(serde_json::from_slice(&encode_model_section(
        &id,
        &name,
        &updated.to_string(),
        "json",
    )?)?)
}
fn encode(project: &Value, format: &str) -> FlowResult<Vec<u8>> {
    let (id, name, section) = section(project)?;
    Ok(encode_model_section(
        &id,
        &name,
        &section.to_string(),
        format,
    )?)
}
fn decode(bytes: &[u8], format: &str) -> FlowResult<Value> {
    let response: Value = serde_json::from_str(&decode_model_document(bytes, format)?)?;
    Ok(response["model"].clone())
}
fn projection(project: &Value) -> FlowResult<ModelingProjection> {
    let (id, name, section) = section(project)?;
    Ok(project_model_section(&id, &name, &section.to_string())?)
}
fn views(project: &Value, diagram: usize) -> FlowResult<&Vec<Value>> {
    project["diagrams"][diagram]["views"]
        .as_array()
        .ok_or_else(|| "views missing".into())
}
fn view<'a>(project: &'a Value, id: &str) -> FlowResult<&'a Value> {
    views(project, 0)?
        .iter()
        .find(|view| view["id"] == id)
        .ok_or_else(|| "view missing".into())
}
fn style(view: &Value) -> FlowResult<Style> {
    Ok(serde_json::from_value(view["style"].clone())?)
}

fn fixture() -> FlowResult<Value> {
    let project = serde_json::json!({
        "id":"styles", "name":"Styles", "elements":[{"id":"a","kind":"uml.class","name":"A"},{"id":"b","kind":"uml.class","name":"B"}],
        "diagrams":[{"id":"d","name":"D","uml_di_type":"UMLClassDiagram","views":[
            {"id":"a-view","subject":"a","geometry":{"kind":"rect","x":0,"y":0,"width":100,"height":80}},
            {"id":"b-view","subject":"b","geometry":{"kind":"rect","x":300,"y":0,"width":100,"height":80}}
        ]}]
    });
    let project = decode(&serde_json::to_vec(&project)?, "json")?;
    Ok(apply(
        &project,
        &Command::CreateRelation {
            owner_id: None,
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
    let edge = views(&project, 0)?.last().ok_or("missing edge")?.clone();
    let edge_id = edge["id"]
        .as_str()
        .ok_or("edge identity missing")?
        .to_owned();
    let mut copy = edge.clone();
    copy["id"] = "copy".into();
    project["diagrams"][0]["views"]
        .as_array_mut()
        .ok_or("views missing")?
        .push(copy.clone());
    let mut second = project["diagrams"][0].clone();
    second["id"] = "other".into();
    for view in second["views"].as_array_mut().ok_or("views missing")? {
        view["id"] = format!(
            "other-{}",
            view["id"].as_str().ok_or("view identity missing")?
        )
        .into();
        if let Some(endpoints) = view.get_mut("endpoints").and_then(Value::as_object_mut) {
            for field in ["source_view_id", "target_view_id"] {
                if let Some(endpoint) = endpoints.get_mut(field)
                    && let Some(identifier) = endpoint.as_str()
                {
                    *endpoint = format!("other-{identifier}").into();
                }
            }
        }
    }
    project["diagrams"]
        .as_array_mut()
        .ok_or("diagrams missing")?
        .push(second.clone());
    assert!(
        project["relations"][0]["properties"]
            .get("routing")
            .is_none()
    );
    for routing in [
        RelationshipRouting::Straight,
        RelationshipRouting::Automatic,
        RelationshipRouting::Curve,
    ] {
        let updated = apply(
            &project,
            &Command::SetViewRouting {
                diagram_id: "d".into(),
                view_id: edge_id.clone(),
                routing,
            },
            1,
        )?;
        assert_eq!(updated["relations"], project["relations"]);
        assert_eq!(updated["elements"], project["elements"]);
        assert_eq!(updated["diagrams"][1], second);
        assert_eq!(views(&updated, 0)?.last(), Some(&copy));
        let view = view(&updated, &edge_id)?;
        assert_eq!(style(view)?.routing, Some(routing));
        let mut styled = style(view)?;
        styled.stroke = Some("#aabbcc".into());
        let restyled = apply(
            &updated,
            &Command::SetViewStyle {
                diagram_id: "d".into(),
                view_id: edge_id.clone(),
                style: styled,
            },
            2,
        )?;
        let restyled_view = self::view(&restyled, &edge_id)?;
        assert_eq!(restyled_view["geometry"], view["geometry"]);
        assert_eq!(style(restyled_view)?.routing, style(view)?.routing);
        assert_eq!(views(&restyled, 0)?.last(), Some(&copy));
        assert_eq!(restyled["diagrams"][1], second);
        if routing == RelationshipRouting::Curve {
            assert!(
                matches!(serde_json::from_value::<Geometry>(view["geometry"].clone())?,Geometry::Path {segments,..} if matches!(segments.first(),Some(PathSegment::Cubic {..})))
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
            let moved_view = self::view(&moved, &edge_id)?;
            assert_ne!(moved_view["geometry"], view["geometry"]);
            let geometry: Geometry = serde_json::from_value(moved_view["geometry"].clone())?;
            assert!(
                matches!(&geometry,Geometry::Path {segments,..} if matches!(segments.first(),Some(PathSegment::Cubic {..})))
            );
            let projected = projection(&moved)?;
            assert_eq!(
                projected
                    .diagrams
                    .iter()
                    .find(|diagram| diagram.id == "d")
                    .and_then(|diagram| diagram.views.iter().find(|view| view.id == edge_id))
                    .and_then(|view| view.routed_geometry.clone()),
                Some(geometry)
            );
        }
        for format in ["json", "protobuf"] {
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
    assert_eq!(self::style(&updated["diagrams"][0]["views"][0])?, style);
    assert_eq!(updated["relations"], original["relations"]);
    assert_eq!(updated["elements"], original["elements"]);
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
    let edge = views(&original, 0)?.last().ok_or("edge missing")?;
    let edge_id = edge["id"]
        .as_str()
        .ok_or("edge identity missing")?
        .to_owned();
    let mut custom = self::style(edge)?;
    custom.dash_pattern = vec![3.0, 2.0];
    custom.line_mode = Some(LineMode::Dashed);
    let styled = apply(
        &original,
        &Command::SetViewStyle {
            diagram_id: "d".into(),
            view_id: edge_id.clone(),
            style: custom,
        },
        1,
    )?;
    assert_eq!(styled["elements"], original["elements"]);
    assert_eq!(styled["relations"], original["relations"]);
    for mode in [LineMode::Solid, LineMode::Dotted, LineMode::Dashed] {
        let mut style = self::style(edge)?;
        style.line_mode = Some(mode);
        let styled = apply(
            &original,
            &Command::SetViewStyle {
                diagram_id: "d".into(),
                view_id: edge_id.clone(),
                style,
            },
            1,
        )?;
        assert_eq!(styled["relations"], original["relations"]);
        for format in ["json", "protobuf"] {
            assert_eq!(decode(&encode(&styled, format)?, format)?, styled);
        }
    }
    assert_eq!(original, fixture()?);
    Ok(())
}

#[test]
fn presentation_alignment_and_stack_are_view_local_and_ignore_unrelated_semantic_errors()
-> Result<(), Box<dyn std::error::Error>> {
    let mut project = fixture()?;
    let mut other = project["diagrams"][0].clone();
    other["id"] = "other-diagram".into();
    for view in other["views"].as_array_mut().ok_or("views missing")? {
        view["id"] = format!(
            "other-{}",
            view["id"].as_str().ok_or("view identity missing")?
        )
        .into();
        if let Some(endpoints) = view.get_mut("endpoints").and_then(Value::as_object_mut) {
            for field in ["source_view_id", "target_view_id"] {
                if let Some(endpoint) = endpoints.get_mut(field)
                    && let Some(identifier) = endpoint.as_str()
                {
                    *endpoint = format!("other-{identifier}").into();
                }
            }
        }
    }
    project["diagrams"]
        .as_array_mut()
        .ok_or("diagrams missing")?
        .push(other.clone());
    project["relations"]
        .as_array_mut()
        .ok_or("relations missing")?
        .push(json!({
            "id":"bad-ownership", "kind":"owns", "from":"a", "to":"a", "properties":{}
        }));
    let findings: Value = serde_json::from_str(&validate_graph_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    assert_eq!(findings["has_errors"], true);
    let original_elements = project["elements"].clone();
    let original_relations = project["relations"].clone();
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
        let projected = projection(&project)?;
        let mut ordered: Vec<_> = projected.diagrams[0].views.iter().collect();
        ordered.sort_by_key(|view| view.z_index);
        assert_eq!(
            ordered.iter().position(|view| view.id == "a-view"),
            Some(expected)
        );
    }
    assert_eq!(project["elements"], original_elements);
    assert_eq!(project["relations"], original_relations);
    assert_eq!(project["diagrams"][1], other);
    assert_eq!(self::style(&project["diagrams"][0]["views"][0])?, style);
    for format in ["json", "protobuf"] {
        assert_eq!(decode(&encode(&project, format)?, format)?, project);
    }
    Ok(())
}
