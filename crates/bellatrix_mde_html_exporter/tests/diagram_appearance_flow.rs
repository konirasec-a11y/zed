use bellatrix_mde::{
    Format, Geometry, LineMode, Project, Style, TextAlignment, TextVerticalAlignment,
    commands::{Command, apply},
    decode, encode,
};

#[test]
fn styled_diagram_survives_save_reload_and_html_export() -> Result<(), Box<dyn std::error::Error>> {
    let project: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/skeleton.json"))?;
    let diagram = project.diagrams.first().ok_or("diagram missing")?;
    let view = diagram
        .views
        .iter()
        .find(|view| matches!(view.geometry, Geometry::Rect { .. }))
        .ok_or("rectangular view missing")?;
    let styled = apply(
        &project,
        &Command::SetViewStyle {
            diagram_id: diagram.id.clone(),
            view_id: view.id.clone(),
            style: Style {
                text_align: Some(TextAlignment::Right),
                text_vertical_align: Some(TextVerticalAlignment::Bottom),
                line_mode: Some(LineMode::Dotted),
                show_shadow: Some(true),
                word_wrap: Some(true),
                ..view.style.clone()
            },
        },
        1,
    )?;
    let restored = decode(&encode(&styled, Format::Protobuf)?, Format::Protobuf)?;
    let export = bellatrix_mde_html_exporter::plan(&restored)?;
    let svg = export
        .artifacts()
        .filter(|artifact| {
            artifact
                .path
                .extension()
                .is_some_and(|extension| extension == "svg")
        })
        .filter_map(|artifact| artifact.contents.as_ref())
        .map(|bytes| String::from_utf8_lossy(bytes))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(svg.contains("filter=\"url(#view-shadow)\""));
    assert!(svg.contains("stroke-dasharray=\"1,3\""));
    assert!(svg.contains("text-anchor=\"end\" dominant-baseline=\"text-after-edge\""));
    assert!(svg.contains("href=\"../elements/id-"));
    assert_eq!(styled.elements, project.elements);
    assert_eq!(styled.relations, project.relations);
    Ok(())
}
