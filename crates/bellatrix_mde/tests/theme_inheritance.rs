use bellatrix_mde::{Format, Project, Style, decode, encode};

#[test]
fn restoring_inheritance_preserves_saved_geometry_and_other_views()
-> Result<(), Box<dyn std::error::Error>> {
    use bellatrix_mde::{
        RelationshipRouting,
        commands::{Command, apply},
    };

    let mut original: Project = serde_json::from_str(include_str!("../fixtures/todo.json"))?;
    let diagram = original.diagrams.first_mut().ok_or("missing diagram")?;
    let view = diagram
        .views
        .iter_mut()
        .find(|view| view.id == "view-dep")
        .ok_or("missing edge")?;
    view.style.stroke = Some("#123456".into());
    view.style.routing = Some(RelationshipRouting::Curve);
    let restored_style = Style {
        routing: view.style.routing,
        ..Style::default()
    };
    let command = Command::SetViewStyle {
        diagram_id: diagram.id.clone(),
        view_id: "view-dep".into(),
        style: restored_style.clone(),
    };
    let updated = apply(&original, &command, 1)?;
    let mut expected = original.clone();
    expected
        .diagrams
        .first_mut()
        .ok_or("missing diagram")?
        .views
        .iter_mut()
        .find(|view| view.id == "view-dep")
        .ok_or("missing edge")?
        .style = restored_style;
    assert_eq!(updated, expected);
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&updated, format)?, format)?;
        assert_eq!(reopened, expected);
        assert_eq!(apply(&reopened, &command, 2)?, reopened);
    }
    Ok(())
}
