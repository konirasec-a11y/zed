use bellatrix_mde::{
    Format, Project, Style, TextAlignment, TextVerticalAlignment, View,
    commands::{Command, apply},
    decode, encode, mdj_conversion, validate_uml_model,
};
use serde_json::{Value, json};
use std::{error::Error, fs, path::PathBuf};

type FlowResult<T> = Result<T, Box<dyn Error>>;
const SOURCE: &[u8] = include_bytes!("../fixtures/bdd/staruml/presentation.mdj");
const SPECIFICATION: &str = include_str!("features/mdj_presentation_lifecycle.feature");

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

    fn reopen(&self, project: &Project, name: &str, format: Format) -> FlowResult<Project> {
        let extension = match format {
            Format::Json => "json",
            Format::Protobuf => "blx",
        };
        let path = self.0.join(format!("{name}.{extension}"));
        fs::write(&path, encode(project, format)?)?;
        let bytes = fs::read(path)?;
        let reopened = decode(&bytes, format)?;
        assert_eq!(&reopened, project);
        Ok(reopened)
    }

    fn export_reopen(&self, project: &Project, name: &str) -> FlowResult<Project> {
        let bytes = mdj_conversion::export(project)?;
        let path = self.0.join(format!("{name}.mdj"));
        fs::write(&path, &bytes)?;
        let reopened = mdj_conversion::import(&fs::read(path)?)?.project;
        assert_eq!(mdj_conversion::export(&reopened)?, bytes);
        assert_projection_equal(project, &reopened);
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

fn view<'a>(project: &'a Project, id: &str) -> FlowResult<&'a View> {
    project
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| view.id == id)
        .ok_or_else(|| format!("missing View {id}").into())
}

fn findings(project: &Project) -> FlowResult<Value> {
    Ok(serde_json::from_str(
        &validate_uml_model(&json!({"api_version": 1, "model": project}).to_string())
            .map_err(std::io::Error::other)?,
    )?)
}

fn restyle(project: &Project, id: &str, style: Style) -> FlowResult<Project> {
    Ok(apply(
        project,
        &Command::SetViewStyle {
            diagram_id: "diagram".into(),
            view_id: id.into(),
            style,
        },
        1,
    )
    .map_err(std::io::Error::other)?)
}

fn assert_projection_equal(left: &Project, right: &Project) {
    let mut left = left.clone();
    let mut right = right.clone();
    left.mdj_provenance = None;
    right.mdj_provenance = None;
    assert_eq!(left, right);
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
    let imported = mdj_conversion::import(SOURCE)?;
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
    let residue = &original_view
        .presentation
        .source
        .as_ref()
        .ok_or("missing source")?
        .residual;
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
    assert!(imported.diagnostics.iter().any(|diagnostic| {
        diagnostic.path.ends_with("/subViews/0/parentStyle")
            && diagnostic.message.contains("not resolved")
    }));
    let saved = SavedFlow::new("authored")?;
    let source_path = saved.0.join("source.mdj");
    fs::write(&source_path, SOURCE)?;
    for (name, format) in [("json", Format::Json), ("binary", Format::Protobuf)] {
        println!("When I save and reopen the unchanged {name} contract");
        let reopened = saved.reopen(&original, &format!("original-{name}"), format)?;
        assert_eq!(mdj_conversion::export(&reopened)?, SOURCE);
        println!("When I edit one occurrence through SetViewStyle and reopen the exported MDJ");
        let mut style = view(&reopened, "class-view")?.style.clone();
        style.fill = Some("#FEDCBA".into());
        style.font_size = Some(18.25);
        style.font_weight = Some(400);
        style.font_italic = Some(false);
        style.show_shadow = Some(true);
        let edited = restyle(&reopened, "class-view", style)?;
        assert_eq!(edited.elements, original.elements);
        assert_eq!(edited.relations, original.relations);
        assert_eq!(edited.composition, original.composition);
        assert_eq!(edited.hierarchy, original.hierarchy);
        assert_eq!(view(&edited, "class-copy")?, view(&original, "class-copy")?);
        assert_eq!(view(&edited, "name-label")?, view(&original, "name-label")?);
        assert_eq!(findings(&edited)?, baseline);
        let edited = saved.reopen(&edited, &format!("edited-{name}"), format)?;
        let reopened = saved.export_reopen(&edited, &format!("edited-{name}"))?;
        let document: Value = serde_json::from_slice(&mdj_conversion::export(&reopened)?)?;
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
        let mut nested = restyle(&reopened, "edge-view", edge_style)?;
        // Nested-view style commands are not yet supported by the authoring API.
        // Exercise the persisted contract without claiming that UI capability.
        let label = nested
            .diagrams
            .iter_mut()
            .flat_map(|diagram| &mut diagram.views)
            .find(|view| view.id == "class-view")
            .ok_or("missing parent View")?
            .children
            .iter_mut()
            .find(|view| view.id == "name-label")
            .ok_or("missing label")?;
        label.style.text_align = Some(TextAlignment::Right);
        label.style.text_vertical_align = Some(TextVerticalAlignment::Bottom);
        label.style.font_underline = Some(false);
        label.style.word_wrap = Some(true);
        let nested = saved.reopen(&nested, &format!("nested-{name}"), format)?;
        let nested = saved.export_reopen(&nested, &format!("nested-{name}"))?;
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
        let document: Value = serde_json::from_slice(&mdj_conversion::export(&cleared)?)?;
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
        let imported = mdj_conversion::import(&fs::read(&source_path)?)?;
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
        for (name, format) in [("json", Format::Json), ("binary", Format::Protobuf)] {
            let original = saved.reopen(&imported.project, &format!("{variant}-{name}"), format)?;
            assert_eq!(mdj_conversion::export(&original)?, source);
            let mut renamed = original.clone();
            renamed.name = "Renamed without replacing source style".into();
            let renamed = saved.reopen(&renamed, &format!("renamed-{variant}-{name}"), format)?;
            let renamed = saved.export_reopen(&renamed, &format!("renamed-{variant}-{name}"))?;
            let mut exported: Value = serde_json::from_slice(&mdj_conversion::export(&renamed)?)?;
            exported["name"] = document["name"].clone();
            assert_eq!(exported, document);
            let mut style = view(&renamed, "class-view")?.style.clone();
            style.fill = Some("#ABCDEF".into());
            let conflicting = restyle(&renamed, "class-view", style)?;
            let conflicting =
                saved.reopen(&conflicting, &format!("conflict-{variant}-{name}"), format)?;
            let error = mdj_conversion::export(&conflicting)
                .err()
                .ok_or("residue must not be overwritten")?;
            assert!(
                error
                    .to_string()
                    .contains("mapped field fillColor conflicts with source residue")
            );
            assert_eq!(findings(&conflicting)?, findings(&original)?);
            assert_eq!(fs::read(&source_path)?, source);
            assert_eq!(mdj_conversion::export(&original)?, source);
        }
    }
    Ok(())
}

#[test]
fn unsupported_edits_fail_after_reopening_and_repair_restores_exact_replay() -> FlowResult<()> {
    assert!(SPECIFICATION.contains(
        "Scenario: Reject unsupported style edits and recover without changing the source"
    ));
    let original = mdj_conversion::import(SOURCE)?.project;
    let saved = SavedFlow::new("unsupported-edits")?;
    let baseline = findings(&original)?;
    for (name, format) in [("json", Format::Json), ("binary", Format::Protobuf)] {
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
            let error = mdj_conversion::export(&edited)
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
            assert_eq!(mdj_conversion::export(&repaired)?, SOURCE);
            assert_eq!(
                mdj_conversion::export(&repaired)?,
                mdj_conversion::export(&original)?
            );
        }
    }
    Ok(())
}
