use bellatrix_mde::{Project, Style};

#[test]
fn built_in_samples_inherit_ide_appearance() -> Result<(), Box<dyn std::error::Error>> {
    for source in [
        include_str!("../fixtures/todo.json"),
        include_str!("../fixtures/skeleton.json"),
    ] {
        let project: Project = serde_json::from_str(source)?;
        assert!(!project.diagrams.is_empty());
        for view in project.diagrams.iter().flat_map(|diagram| &diagram.views) {
            let appearance = Style {
                routing: view.style.routing,
                corner_radius: view.style.corner_radius,
                dash_pattern: view.style.dash_pattern.clone(),
                ..Style::default()
            };
            assert_eq!(
                view.style, appearance,
                "{} must inherit the IDE theme",
                view.id
            );
        }
    }
    Ok(())
}

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
    assert_eq!(apply(&updated, &command, 2)?, updated);
    Ok(())
}
