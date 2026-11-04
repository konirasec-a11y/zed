use bellatrix_mde_cordis::{
    MODELING_NAMESPACE, MdePlugin, ModelingConvertDocument, ModelingConvertDocumentRequested,
    ModelingDocumentFormat, ModelingPrepareDocument, ModelingPrepareDocumentRequested,
    ModelingRenderHtml, ModelingRenderHtmlRequested, ModelingSetViewStyle,
    ModelingSetViewStyleRequested,
};
use bellatrix_project::{ProjectCreate, ProjectCreateRequested, ProjectPlugin, ProjectService};
use cordis::{Context, Plugin, PreparedPlugin, Routing, event::QueryOutcome};
use serde_json::{Value, json};
use std::{error::Error, fmt::Display, sync::Arc};

fn answer<T, E: Display>(outcome: QueryOutcome<Result<T, E>>) -> Result<T, Box<dyn Error>> {
    match outcome {
        QueryOutcome::Answer(Ok(value)) => Ok(value),
        QueryOutcome::Answer(Err(error)) => Err(std::io::Error::other(error.to_string()).into()),
        QueryOutcome::Miss => Err("Cordis responder unavailable".into()),
    }
}

#[tokio::test]
async fn styled_diagram_survives_save_reload_and_html_export() -> Result<(), Box<dyn Error>> {
    let mut project: Value =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/skeleton.json"))?;
    let member_id = {
        let member = project
            .get_mut("elements")
            .and_then(Value::as_array_mut)
            .and_then(|elements| {
                elements.iter_mut().find(|element| {
                    element.get("id").and_then(Value::as_str) == Some("field-title")
                })
            })
            .and_then(Value::as_object_mut)
            .ok_or("member missing")?;
        member.insert("kind".into(), json!("uml.attribute"));
        let properties = member
            .entry("properties")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or("member properties missing")?;
        properties.insert("visibility".into(), json!("public"));
        properties.insert("isReadOnly".into(), json!(true));
        member
            .get("id")
            .and_then(Value::as_str)
            .ok_or("member identity missing")?
            .to_owned()
    };
    let diagram = project
        .get_mut("diagrams")
        .and_then(Value::as_array_mut)
        .and_then(|diagrams| diagrams.first_mut())
        .ok_or("diagram missing")?;
    let diagram_id = diagram
        .get("id")
        .and_then(Value::as_str)
        .ok_or("diagram identity missing")?
        .to_owned();
    let views = diagram
        .get_mut("views")
        .and_then(Value::as_array_mut)
        .ok_or("views missing")?;
    let mut member_view = views.first().cloned().ok_or("view missing")?;
    let view = member_view.as_object_mut().ok_or("view missing")?;
    view.insert("id".into(), json!("member-label-view"));
    view.insert("subject".into(), json!(member_id));
    view.insert("label".into(), Value::Null);
    views.push(member_view);
    let view = views
        .iter()
        .find(|view| {
            view.get("id").and_then(Value::as_str) == Some("member-label-view")
                && view
                    .get("geometry")
                    .and_then(|geometry| geometry.get("kind"))
                    .and_then(Value::as_str)
                    == Some("rect")
        })
        .ok_or("rectangular view missing")?;
    let mut style = view.get("style").cloned().unwrap_or_else(|| json!({}));
    let style_object = style.as_object_mut().ok_or("view style missing")?;
    for (name, value) in [
        ("text_align", json!("right")),
        ("text_vertical_align", json!("bottom")),
        ("line_mode", json!("dotted")),
        ("show_shadow", json!(true)),
        ("word_wrap", json!(true)),
    ] {
        style_object.insert(name.into(), value);
    }
    let uml = style_object
        .entry("uml")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("UML style missing")?;
    for name in ["show_visibility", "show_type", "show_property"] {
        uml.insert(name.into(), json!(false));
    }

    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let provider = context
        .spawn(PreparedPlugin::from_input(
            ProjectPlugin::new(Arc::clone(&service)),
            Arc::clone(&service),
        ))
        .await?;
    let plugin = MdePlugin;
    plugin.prepare(())?;
    let plugin = context
        .spawn(PreparedPlugin::from_input(plugin, ()))
        .await?;
    let prepared = answer(
        context
            .query::<ModelingPrepareDocument>(
                Routing::Unscoped,
                ModelingPrepareDocumentRequested {
                    bytes: serde_json::to_vec(&project)?,
                    format: ModelingDocumentFormat::ModelJson,
                },
            )
            .await?,
    )?;
    let document = answer(
        context
            .query::<ProjectCreate>(
                Routing::Unscoped,
                ProjectCreateRequested {
                    document: prepared.document,
                },
            )
            .await?,
    )?;
    let original = document
        .section(MODELING_NAMESPACE)
        .ok_or("section missing")?
        .clone();
    answer(
        context
            .query::<ModelingSetViewStyle>(
                Routing::Unscoped,
                ModelingSetViewStyleRequested {
                    project_id: document.id().clone(),
                    diagram_id,
                    view_id: "member-label-view".into(),
                    style: serde_json::from_value(style)?,
                    expected_revision: document.revision(),
                },
            )
            .await?,
    )?;
    let snapshot = service.snapshot(document.id())?;
    let section = snapshot
        .section(MODELING_NAMESPACE)
        .ok_or("section missing")?;
    let mut styled = section
        .as_object()
        .ok_or("section is not an object")?
        .clone();
    styled.insert("schema_version".into(), json!(1));
    styled.insert("id".into(), json!(snapshot.id().as_str()));
    styled.insert("name".into(), json!(snapshot.name()));
    let saved = answer(
        context
            .query::<ModelingConvertDocument>(
                Routing::Unscoped,
                ModelingConvertDocumentRequested {
                    bytes: serde_json::to_vec(&styled)?,
                    source_format: "json".into(),
                    destination_format: "protobuf".into(),
                },
            )
            .await?,
    )?;
    let restored = answer(
        context
            .query::<ModelingConvertDocument>(
                Routing::Unscoped,
                ModelingConvertDocumentRequested {
                    bytes: saved,
                    source_format: "protobuf".into(),
                    destination_format: "json".into(),
                },
            )
            .await?,
    )?;
    let rendered = answer(
        context
            .query::<ModelingRenderHtml>(
                Routing::Unscoped,
                ModelingRenderHtmlRequested {
                    bytes: restored,
                    format: "json".into(),
                },
            )
            .await?,
    )?;
    let export: Value = serde_json::from_str(&rendered)?;
    let svg = export
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or("artifacts missing")?
        .iter()
        .filter(|artifact| {
            artifact
                .get("path")
                .and_then(Value::as_str)
                .is_some_and(|path| path.ends_with(".svg"))
        })
        .filter_map(|artifact| artifact.get("contents").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(svg.contains(">title</text>"));
    assert!(!svg.contains("+title"));
    assert!(!svg.contains("{readOnly}"));
    assert!(svg.contains("filter=\"url(#view-shadow)\""));
    assert!(svg.contains("stroke-dasharray=\"1,3\""));
    assert!(svg.contains("text-anchor=\"end\" dominant-baseline=\"text-after-edge\""));
    assert!(svg.contains("href=\"../elements/id-"));
    assert_eq!(section.get("elements"), original.get("elements"));
    assert_eq!(section.get("relations"), original.get("relations"));
    plugin.dispose().await?;
    provider.dispose().await?;
    Ok(())
}
