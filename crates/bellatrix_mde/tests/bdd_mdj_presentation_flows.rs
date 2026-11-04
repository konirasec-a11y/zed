use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, export_mdj_section,
    import_mdj_document, prepare_model_document, project_model_section, validate_uml_model,
};
use bellatrix_mde_protocol::{ModelDocumentParts, command::Command};
use bellatrix_mde_protocol::{
    Style, TextAlignment, TextVerticalAlignment,
    projection::{InheritedStyleProperty, StyleInheritanceSource, StyleInheritanceStatus, View},
};
use serde_json::{Value, json};
use std::{error::Error, fs, path::PathBuf};

type FlowResult<T> = Result<T, Box<dyn Error>>;
const SOURCE: &[u8] = include_bytes!("../fixtures/bdd/staruml/presentation.mdj");
const SPECIFICATION: &str = include_str!("features/mdj_presentation_lifecycle.feature");

#[derive(serde::Deserialize)]
struct ModelResponse {
    model: Box<serde_json::value::RawValue>,
    #[serde(default)]
    diagnostics: Vec<MdjDiagnostic>,
}

#[derive(serde::Deserialize)]
struct MdjDiagnostic {
    path: String,
    message: String,
}

struct ImportedModel {
    project: Value,
    diagnostics: Vec<MdjDiagnostic>,
}

fn model_request(project: &Value) -> FlowResult<String> {
    Ok(format!(
        r#"{{"api_version":1,"model":{}}}"#,
        serde_json::to_string(project)?
    ))
}

fn import_mdj(bytes: &[u8]) -> FlowResult<ImportedModel> {
    let response: ModelResponse =
        serde_json::from_str(&import_mdj_document(bytes).map_err(std::io::Error::other)?)?;
    Ok(ImportedModel {
        project: serde_json::from_str(response.model.get())?,
        diagnostics: response.diagnostics,
    })
}

fn document_parts(project: &Value) -> FlowResult<ModelDocumentParts> {
    Ok(prepare_model_document(&model_request(project)?).map_err(std::io::Error::other)?)
}

fn export_mdj(project: &Value) -> FlowResult<Vec<u8>> {
    let document = document_parts(project)?;
    Ok(
        export_mdj_section(&document.id, &document.name, &document.section_json)
            .map_err(std::io::Error::other)?,
    )
}

fn encode_model(project: &Value, format: &str) -> FlowResult<Vec<u8>> {
    let document = document_parts(project)?;
    Ok(
        encode_model_section(&document.id, &document.name, &document.section_json, format)
            .map_err(std::io::Error::other)?,
    )
}

fn edit(project: &Value, command: Command) -> FlowResult<Value> {
    let document = document_parts(project)?;
    let section = apply_model_command(
        &document.id,
        &document.name,
        &document.section_json,
        1,
        command,
        false,
    )
    .map_err(std::io::Error::other)?;
    let bytes = encode_model_section(&document.id, &document.name, &section.to_string(), "json")
        .map_err(std::io::Error::other)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn decode_model(bytes: &[u8], format: &str) -> FlowResult<Value> {
    let response: ModelResponse = serde_json::from_str(
        &decode_model_document(bytes, format).map_err(std::io::Error::other)?,
    )?;
    Ok(serde_json::from_str(response.model.get())?)
}

struct SavedFlow(PathBuf);

impl SavedFlow {
    fn new(name: &str) -> FlowResult<Self> {
        let directory = std::env::temp_dir().join(format!(
            "bellatrix-mdj-style-bdd-{}-{name}",
            std::process::id()
        ));
        fs::create_dir(&directory)?;
        Ok(Self(directory))
    }

    fn reopen(&self, project: &Value, name: &str, format: &str) -> FlowResult<Value> {
        let extension = match format {
            "json" => "json",
            "protobuf" => "blx",
            _ => return Err("unrecognized persistence format".into()),
        };
        let path = self.0.join(format!("{name}.{extension}"));
        fs::write(&path, encode_model(project, format)?)?;
        let bytes = fs::read(path)?;
        let reopened = decode_model(&bytes, format)?;
        assert_eq!(&reopened, project);
        Ok(reopened)
    }

    fn export_reopen(&self, project: &Value, name: &str) -> FlowResult<Value> {
        let bytes = export_mdj(project)?;
        let path = self.0.join(format!("{name}.mdj"));
        fs::write(&path, &bytes)?;
        let reopened = import_mdj(&fs::read(path)?)?.project;
        assert_eq!(export_mdj(&reopened)?, bytes);
        assert_projection_equal(project, &reopened)?;
        Ok(reopened)
    }
}

impl Drop for SavedFlow {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "Could not clean MDJ style BDD directory {}: {error}",
                self.0.display()
            );
        }
    }
}

fn view(project: &Value, id: &str) -> FlowResult<View> {
    let document = document_parts(project)?;
    let projection = project_model_section(&document.id, &document.name, &document.section_json)
        .map_err(std::io::Error::other)?;
    projection
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| view.id == id)
        .cloned()
        .ok_or_else(|| format!("missing View {id}").into())
}

fn raw_view<'a>(project: &'a Value, id: &str) -> FlowResult<&'a Value> {
    fn find<'a>(views: &'a [Value], id: &str) -> Option<&'a Value> {
        views.iter().find_map(|view| {
            if view["id"] == id {
                Some(view)
            } else {
                view.get("children")
                    .and_then(Value::as_array)
                    .and_then(|children| find(children, id))
            }
        })
    }
    project["diagrams"]
        .as_array()
        .ok_or("missing diagrams")?
        .iter()
        .find_map(|diagram| {
            diagram["views"]
                .as_array()
                .and_then(|views| find(views, id))
        })
        .ok_or_else(|| format!("missing persisted View {id}").into())
}

fn findings(project: &Value) -> FlowResult<Value> {
    Ok(serde_json::from_str(
        &validate_uml_model(&json!({"api_version": 1, "model": project}).to_string())
            .map_err(std::io::Error::other)?,
    )?)
}

fn restyle(project: &Value, id: &str, style: Style) -> FlowResult<Value> {
    edit(
        project,
        Command::SetViewStyle {
            diagram_id: "diagram".into(),
            view_id: id.into(),
            style,
        },
    )
}

fn assert_projection_equal(left: &Value, right: &Value) -> FlowResult<()> {
    let mut left = left.clone();
    let mut right = right.clone();
    left.as_object_mut()
        .ok_or("canonical model must be an object")?
        .remove("mdj_provenance");
    right
        .as_object_mut()
        .ok_or("canonical model must be an object")?
        .remove("mdj_provenance");
    assert_eq!(left, right);
    Ok(())
}

#[test]
fn authored_style_survives_command_save_reopen_and_mdj_export() -> FlowResult<()> {
    assert!(
        SPECIFICATION.contains(
            "Scenario: Preserve and edit authored style through native persistence and MDJ"
        )
    );
    println!(
        "Given two occurrences of a Class, a nested label and an edge with authored MDJ style"
    );
    let imported = import_mdj(SOURCE)?;
    let original = imported.project;
    let baseline = findings(&original)?;
    let original_view = view(&original, "class-view")?;
    assert_eq!(original_view.style.fill.as_deref(), Some("#123456"));
    assert_eq!(original_view.style.stroke.as_deref(), Some("#654321"));
    assert_eq!(original_view.style.text_color.as_deref(), Some("#112233FF"));
    assert_eq!(original_view.style.font_family.as_deref(), Some("Inter"));
    assert_eq!(original_view.style.font_size, Some(13.5));
    assert_eq!(original_view.style.font_weight, Some(700));
    assert_eq!(original_view.style.font_italic, Some(true));
    assert_eq!(original_view.style.show_shadow, Some(false));
    let residue = raw_view(&original, "class-view")?
        .pointer("/presentation/source/residual")
        .and_then(Value::as_object)
        .ok_or("missing source residue")?;
    for field in ["fillColor", "lineColor", "fontColor", "font", "showShadow"] {
        assert!(!residue.contains_key(field));
    }
    assert_eq!(view(&original, "class-copy")?.style, Style::default());
    let label = view(&original, "name-label")?;
    assert_eq!(label.style.text_align, Some(TextAlignment::Left));
    assert_eq!(
        label.style.text_vertical_align,
        Some(TextVerticalAlignment::Top)
    );
    assert_eq!(label.style.font_underline, Some(true));
    assert_eq!(label.style.word_wrap, Some(false));
    let assert_inherited_label =
        |project: &Value, fill: &str, size: f64, weight: u16, italic: bool| -> FlowResult<()> {
            let label = view(project, "name-label")?;
            let inheritance = label.presentation.style_inheritance.as_ref()
                .ok_or("missing projected style inheritance")?;
            assert_eq!(inheritance.status, StyleInheritanceStatus::Resolved);
            assert_eq!(
                inheritance.source,
                StyleInheritanceSource::Reference("class-view".into())
            );
            assert!(inheritance.diagnostics.is_empty());
            for property in [
                InheritedStyleProperty::Fill,
                InheritedStyleProperty::Stroke,
                InheritedStyleProperty::TextColor,
                InheritedStyleProperty::FontFamily,
                InheritedStyleProperty::FontSize,
                InheritedStyleProperty::FontWeight,
                InheritedStyleProperty::FontItalic,
            ] {
                assert_eq!(
                    inheritance.origins.get(&property).map(String::as_str),
                    Some("class-view"),
                    "unexpected inheritance origin for {property:?}"
                );
            }
            assert_eq!(label.render_style.fill.as_deref(), Some(fill));
            assert_eq!(label.render_style.stroke.as_deref(), Some("#654321"));
            assert_eq!(label.render_style.text_color.as_deref(), Some("#112233FF"));
            assert_eq!(label.render_style.font_family.as_deref(), Some("Inter"));
            assert_eq!(label.render_style.font_size, Some(size));
            assert_eq!(label.render_style.font_weight, Some(weight));
            assert_eq!(label.render_style.font_italic, Some(italic));
            assert_eq!(label.style.font_family.as_deref(), Some("Inter"));
            assert_eq!(label.style.font_size, Some(11.0));
            assert_eq!(label.style.font_weight, Some(400));
            assert_eq!(label.style.font_italic, Some(false));
            assert_eq!(label.style.text_color.as_deref(), Some("#ABCDEF"));
            Ok(())
        };
    assert_inherited_label(&original, "#123456", 13.5, 700, true)?;
    let saved = SavedFlow::new("authored")?;
    let source_path = saved.0.join("source.mdj");
    fs::write(&source_path, SOURCE)?;
    for (name, format) in [("json", "json"), ("binary", "protobuf")] {
        println!("When I save and reopen the unchanged {name} contract");
        let reopened = saved.reopen(&original, &format!("original-{name}"), format)?;
        assert_eq!(export_mdj(&reopened)?, SOURCE);
        assert_inherited_label(&reopened, "#123456", 13.5, 700, true)?;
        println!("When I edit one occurrence through SetViewStyle and reopen the exported MDJ");
        let mut style = view(&reopened, "class-view")?.style.clone();
        style.fill = Some("#FEDCBA".into());
        style.font_size = Some(18.25);
        style.font_weight = Some(400);
        style.font_italic = Some(false);
        style.show_shadow = Some(true);
        let edited = restyle(&reopened, "class-view", style)?;
        assert_inherited_label(&edited, "#FEDCBA", 18.25, 400, false)?;
        assert_eq!(edited.get("elements"), original.get("elements"));
        assert_eq!(edited.get("relations"), original.get("relations"));
        assert_eq!(edited.get("composition"), original.get("composition"));
        assert_eq!(edited.get("hierarchy"), original.get("hierarchy"));
        assert_eq!(
            raw_view(&edited, "class-copy")?,
            raw_view(&original, "class-copy")?
        );
        assert_eq!(
            raw_view(&edited, "name-label")?,
            raw_view(&original, "name-label")?
        );
        assert_eq!(findings(&edited)?, baseline);
        let edited = saved.reopen(&edited, &format!("edited-{name}"), format)?;
        let reopened = saved.export_reopen(&edited, &format!("edited-{name}"))?;
        assert_inherited_label(&reopened, "#FEDCBA", 18.25, 400, false)?;
        let document: Value = serde_json::from_slice(&export_mdj(&reopened)?)?;
        let original_document: Value = serde_json::from_slice(SOURCE)?;
        assert_eq!(
            document["pluginPayload"],
            original_document["pluginPayload"]
        );
        let exported_view = &document["ownedElements"][0]["ownedElements"][1]["ownedViews"][0];
        assert_eq!(exported_view["fillColor"], "#FEDCBA");
        assert_eq!(exported_view["font"], "Inter;18.25;0");
        assert_eq!(
            exported_view["vendorStyle"],
            json!({"effects": ["glow", null]})
        );

        println!("When I edit an edge color and nested label presentation without changing UML");
        let mut edge_style = view(&reopened, "edge-view")?.style.clone();
        edge_style.stroke = Some("#102030".into());
        let nested = restyle(&reopened, "edge-view", edge_style)?;
        let mut label_style = view(&nested, "name-label")?.style;
        label_style.text_align = Some(TextAlignment::Right);
        label_style.text_vertical_align = Some(TextVerticalAlignment::Bottom);
        label_style.font_underline = Some(false);
        label_style.word_wrap = Some(true);
        let nested = restyle(&nested, "name-label", label_style)?;
        let nested = saved.reopen(&nested, &format!("nested-{name}"), format)?;
        let nested = saved.export_reopen(&nested, &format!("nested-{name}"))?;
        assert_inherited_label(&nested, "#FEDCBA", 18.25, 400, false)?;
        assert_eq!(findings(&nested)?, baseline);
        assert_eq!(
            view(&nested, "edge-view")?.style.stroke.as_deref(),
            Some("#102030")
        );
        let label = view(&nested, "name-label")?;
        assert_eq!(label.style.text_align, Some(TextAlignment::Right));
        assert_eq!(
            label.style.text_vertical_align,
            Some(TextVerticalAlignment::Bottom)
        );
        assert_eq!(label.style.font_underline, Some(false));
        assert_eq!(label.style.word_wrap, Some(true));

        println!("When I remove a complete authored font and fill without injecting defaults");
        let mut cleared_style = view(&reopened, "class-view")?.style.clone();
        cleared_style.fill = None;
        cleared_style.font_family = None;
        cleared_style.font_size = None;
        cleared_style.font_weight = None;
        cleared_style.font_italic = None;
        let cleared = restyle(&reopened, "class-view", cleared_style)?;
        let cleared = saved.reopen(&cleared, &format!("cleared-{name}"), format)?;
        let cleared = saved.export_reopen(&cleared, &format!("cleared-{name}"))?;
        let document: Value = serde_json::from_slice(&export_mdj(&cleared)?)?;
        let fields = document["ownedElements"][0]["ownedElements"][1]["ownedViews"][0]
            .as_object()
            .ok_or("missing View object")?;
        assert!(!fields.contains_key("fillColor"));
        assert!(!fields.contains_key("font"));
        assert_eq!(fs::read(&source_path)?, SOURCE);
    }
    Ok(())
}

#[test]
fn unsupported_style_survives_unrelated_edit_and_blocks_conflicting_overwrite() -> FlowResult<()> {
    assert!(
        SPECIFICATION
            .contains("Scenario: Preserve unsupported source style without substituting defaults")
    );
    let saved = SavedFlow::new("unsupported-source")?;
    for (variant, font) in [
        ("partial", json!("?;13;0")),
        ("precision", json!("Inter;13.000000000000000001;0")),
        ("trailing-zeroes", json!("Inter;13.00;0")),
        ("exponent", json!("Inter;1.3e1;0")),
        ("unknown-flags", json!("Inter;13;4")),
        ("null", Value::Null),
    ] {
        println!("Given {variant} font, null and malformed visual fields");
        let mut document: Value = serde_json::from_slice(SOURCE)?;
        let fields = document["ownedElements"][0]["ownedElements"][1]["ownedViews"][0]
            .as_object_mut()
            .ok_or("missing View")?;
        for (field, value) in [
            ("font", font),
            ("fillColor", Value::Null),
            ("lineColor", json!("red")),
            ("fontColor", json!("#ZZZZZZ")),
            ("underline", json!("yes")),
            ("wordWrap", Value::Null),
            ("showShadow", json!(1)),
            ("horizontalAlignment", json!(99)),
            ("verticalAlignment", json!(-1)),
        ] {
            fields.insert(field.into(), value);
        }
        let source = serde_json::to_vec_pretty(&document)?;
        let source_path = saved.0.join(format!("source-{variant}.mdj"));
        fs::write(&source_path, &source)?;
        let imported = import_mdj(&fs::read(&source_path)?)?;
        assert_eq!(
            view(&imported.project, "class-view")?.style,
            Style::default()
        );
        for field in [
            "font",
            "fillColor",
            "lineColor",
            "fontColor",
            "underline",
            "wordWrap",
            "showShadow",
            "horizontalAlignment",
            "verticalAlignment",
        ] {
            assert!(imported.diagnostics.iter().any(|diagnostic| {
                diagnostic.path.ends_with(&format!("/ownedViews/0/{field}"))
                    && diagnostic.message.contains("source residue")
            }));
        }
        for (name, format) in [("json", "json"), ("binary", "protobuf")] {
            let original = saved.reopen(&imported.project, &format!("{variant}-{name}"), format)?;
            assert_eq!(export_mdj(&original)?, source);
            let mut renamed = original.clone();
            renamed["name"] = "Renamed without replacing source style".into();
            let renamed = saved.reopen(&renamed, &format!("renamed-{variant}-{name}"), format)?;
            let renamed = saved.export_reopen(&renamed, &format!("renamed-{variant}-{name}"))?;
            let mut exported: Value = serde_json::from_slice(&export_mdj(&renamed)?)?;
            exported["name"] = document["name"].clone();
            assert_eq!(exported, document);
            let mut style = view(&renamed, "class-view")?.style.clone();
            style.fill = Some("#ABCDEF".into());
            let conflicting = restyle(&renamed, "class-view", style)?;
            let conflicting =
                saved.reopen(&conflicting, &format!("conflict-{variant}-{name}"), format)?;
            let error = export_mdj(&conflicting)
                .err()
                .ok_or("residue must not be overwritten")?;
            assert!(
                error
                    .to_string()
                    .contains("mapped field fillColor conflicts with source residue")
            );
            assert_eq!(findings(&conflicting)?, findings(&original)?);
            assert_eq!(fs::read(&source_path)?, source);
            assert_eq!(export_mdj(&original)?, source);
        }
    }
    Ok(())
}

#[test]
fn unsupported_edits_fail_after_reopening_and_repair_restores_exact_replay() -> FlowResult<()> {
    assert!(SPECIFICATION.contains(
        "Scenario: Reject unsupported style edits and recover without changing the source"
    ));
    let original = import_mdj(SOURCE)?.project;
    let saved = SavedFlow::new("unsupported-edits")?;
    let baseline = findings(&original)?;
    for (name, format) in [("json", "json"), ("binary", "protobuf")] {
        let original = saved.reopen(&original, &format!("original-{name}"), format)?;
        for (variant, expected_error) in [
            ("opacity", "cannot represent this edit losslessly"),
            ("weight", "only normal (400) or bold (700)"),
            (
                "partial",
                "requires family, size, weight and italic together",
            ),
        ] {
            println!("When I author unsupported {variant}, save and reopen {name}");
            let mut style = view(&original, "class-view")?.style.clone();
            match variant {
                "opacity" => style.opacity = Some(0.5),
                "weight" => style.font_weight = Some(600),
                "partial" => style.font_size = None,
                _ => return Err("unrecognized BDD variant".into()),
            }
            let edited = restyle(&original, "class-view", style)?;
            let edited = saved.reopen(&edited, &format!("{variant}-{name}"), format)?;
            let error = export_mdj(&edited)
                .err()
                .ok_or("unsupported edit must not export stale source")?;
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(findings(&edited)?, baseline);
            let repaired = restyle(
                &edited,
                "class-view",
                view(&original, "class-view")?.style.clone(),
            )?;
            let repaired =
                saved.reopen(&repaired, &format!("repaired-{variant}-{name}"), format)?;
            assert_eq!(repaired, original);
            assert_eq!(export_mdj(&repaired)?, SOURCE);
            assert_eq!(export_mdj(&repaired)?, export_mdj(&original)?);
        }
    }
    Ok(())
}
