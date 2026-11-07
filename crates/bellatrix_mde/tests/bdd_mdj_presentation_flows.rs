use bellatrix_mde::contracts::protocol::command::Command;
use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, export_mdj_section,
    import_mdj_document, prepare_model_document,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

struct Document(Value);

impl Document {
    fn response(response: String) -> Result<Self> {
        let mut response: Value = serde_json::from_str(&response)?;
        let model = response
            .get_mut("model")
            .ok_or("missing model response")?
            .take();
        assert_eq!(model.get("schemaVersion"), Some(&json!(2)));
        assert!(model.pointer("/model/registry/entries").is_some());
        Ok(Self(model))
    }

    fn import(source: &[u8]) -> Result<Self> {
        Self::response(import_mdj_document(source).map_err(std::io::Error::other)?)
    }

    fn parts(&self) -> Result<bellatrix_mde::contracts::protocol::ModelDocumentParts> {
        Ok(
            prepare_model_document(&json!({"api_version": 1, "model": self.0}).to_string())
                .map_err(std::io::Error::other)?,
        )
    }

    fn command(&self, command: Command) -> Result<Self> {
        let parts = self.parts()?;
        let section = apply_model_command(&parts.id, &parts.name, &parts.section_json, 1, command)
            .map_err(std::io::Error::other)?;
        let bytes = encode_model_section(&parts.id, &parts.name, &section.to_string(), "json")
            .map_err(std::io::Error::other)?;
        Self::response(decode_model_document(&bytes, "json").map_err(std::io::Error::other)?)
    }

    fn export(&self) -> Result<Vec<u8>> {
        let parts = self.parts()?;
        Ok(
            export_mdj_section(&parts.id, &parts.name, &parts.section_json)
                .map_err(std::io::Error::other)?,
        )
    }

    fn reopen(&self, directory: &std::path::Path, format: &str) -> Result<Self> {
        let parts = self.parts()?;
        let bytes = encode_model_section(&parts.id, &parts.name, &parts.section_json, format)
            .map_err(std::io::Error::other)?;
        let path = directory.join(if format == "json" {
            "saved.json"
        } else {
            "saved.blx"
        });
        fs::write(&path, bytes)?;
        let reopened = Self::response(
            decode_model_document(&fs::read(path)?, format).map_err(std::io::Error::other)?,
        )?;
        assert_eq!(reopened.0, self.0);
        Ok(reopened)
    }
}

fn scenario(name: &str, run: impl FnOnce(&std::path::Path) -> Result) -> Result {
    let directory = std::env::temp_dir().join(format!("mdj-bdd-{}-{name}", std::process::id()));
    fs::create_dir(&directory)?;
    let outcome = run(&directory);
    fs::remove_dir_all(directory)?;
    outcome
}

use bellatrix_mde::contracts::protocol::{Style, TextAlignment, TextVerticalAlignment};

const SOURCE: &[u8] = include_bytes!("../fixtures/bdd/staruml/presentation.mdj");
const REQUIREMENTS: &str = include_str!("features/mdj_presentation_lifecycle.feature");

impl Document {
    fn view(&self, id: &str) -> Result<&Value> {
        fn find<'a>(value: &'a Value, id: &str) -> Option<&'a Value> {
            match value {
                Value::Object(fields) if fields.get("id").and_then(Value::as_str) == Some(id) => {
                    Some(value)
                }
                Value::Object(fields) => fields.values().find_map(|value| find(value, id)),
                Value::Array(values) => values.iter().find_map(|value| find(value, id)),
                _ => None,
            }
        }
        let diagrams = self
            .0
            .pointer("/model/diagrams")
            .ok_or("missing diagrams")?;
        find(diagrams, id).ok_or_else(|| format!("missing persisted View {id}").into())
    }

    fn style(&self, id: &str) -> Result<Style> {
        Ok(serde_json::from_value(
            self.view(id)?
                .get("style")
                .ok_or("missing authored Style")?
                .clone(),
        )?)
    }

    fn restyle(&self, id: &str, style: Style) -> Result<Self> {
        self.command(Command::SetViewStyle {
            diagram_id: "diagram".into(),
            view_id: id.into(),
            style,
        })
    }

    fn export_reopen(&self, directory: &std::path::Path, expected: &Value) -> Result<Self> {
        let bytes = self.export()?;
        assert_eq!(serde_json::from_slice::<Value>(&bytes)?, *expected);
        let path = directory.join("exported.mdj");
        fs::write(&path, &bytes)?;
        let reopened = Self::import(&fs::read(path)?)?;
        assert_eq!(reopened.export()?, bytes);
        for id in ["class-view", "class-copy", "name-label", "edge-view"] {
            assert_eq!(reopened.style(id)?, self.style(id)?);
        }
        for field in ["registry", "composition", "hierarchy"] {
            assert_eq!(reopened.0["model"].get(field), self.0["model"].get(field));
        }
        Ok(reopened)
    }
}

fn source_fields<'a>(
    source: &'a mut Value,
    id: &str,
) -> Result<&'a mut serde_json::Map<String, Value>> {
    fn find<'a>(value: &'a mut Value, id: &str) -> Option<&'a mut serde_json::Map<String, Value>> {
        match value {
            Value::Object(fields) => {
                if fields.get("_id").and_then(Value::as_str) == Some(id) {
                    Some(fields)
                } else {
                    fields.values_mut().find_map(|value| find(value, id))
                }
            }
            Value::Array(values) => values.iter_mut().find_map(|value| find(value, id)),
            _ => None,
        }
    }
    find(source, id).ok_or_else(|| format!("missing source fields {id}").into())
}

fn imported_styles(document: &Document) -> Result {
    let style = document.style("class-view")?;
    assert_eq!(style.fill.as_deref(), Some("#123456"));
    assert_eq!(style.stroke.as_deref(), Some("#654321"));
    assert_eq!(style.text_color.as_deref(), Some("#112233FF"));
    assert_eq!(style.font_family.as_deref(), Some("Inter"));
    assert_eq!(style.font_size, Some(13.5));
    assert_eq!(style.font_weight, Some(700));
    assert_eq!(style.font_italic, Some(true));
    assert_eq!(style.show_shadow, Some(false));
    assert_eq!(document.style("class-copy")?, Style::default());
    let label = document.style("name-label")?;
    assert_eq!(label.text_align, Some(TextAlignment::Left));
    assert_eq!(label.text_vertical_align, Some(TextVerticalAlignment::Top));
    assert_eq!(label.font_underline, Some(true));
    assert_eq!(label.word_wrap, Some(false));
    let residue = document
        .view("class-view")?
        .pointer("/presentation/source/residual")
        .and_then(Value::as_object)
        .ok_or("missing residue")?;
    for field in ["fillColor", "lineColor", "fontColor", "font", "showShadow"] {
        assert!(!residue.contains_key(field));
    }
    assert_eq!(
        document.0.pointer("/model/registry/entries/class/type"),
        Some(&json!("Class"))
    );
    Ok(())
}

#[test]
fn preserve_and_edit_authored_style_through_native_and_mdj_files() -> Result {
    assert!(
        REQUIREMENTS.contains(
            "Scenario: Preserve and edit authored style through native persistence and MDJ"
        )
    );
    scenario("authored-style", |directory| {
        let source_path = directory.join("source.mdj");
        fs::write(&source_path, SOURCE)?;
        let original = Document::import(&fs::read(&source_path)?)?;
        for format in ["json", "protobuf"] {
            let document = original.reopen(directory, format)?;
            imported_styles(&document)?;
            assert_eq!(document.export()?, SOURCE);
            let mut expected: Value = serde_json::from_slice(SOURCE)?;
            let mut style = document.style("class-view")?;
            style.fill = Some("#FEDCBA".into());
            style.font_size = Some(18.25);
            style.font_weight = Some(400);
            style.font_italic = Some(false);
            style.show_shadow = Some(true);
            let edited = document
                .restyle("class-view", style)?
                .reopen(directory, format)?;
            for field in ["registry", "composition", "hierarchy"] {
                assert_eq!(edited.0["model"].get(field), original.0["model"].get(field));
            }
            for id in ["class-copy", "name-label"] {
                assert_eq!(edited.view(id)?, original.view(id)?);
            }
            let fields = source_fields(&mut expected, "class-view")?;
            fields.insert("fillColor".into(), json!("#FEDCBA"));
            fields.insert("font".into(), json!("Inter;18.25;0"));
            fields.insert("showShadow".into(), json!(true));
            let edited = edited.export_reopen(directory, &expected)?;

            let mut edge = edited.style("edge-view")?;
            edge.stroke = Some("#102030".into());
            let mut label = edited.style("name-label")?;
            label.text_align = Some(TextAlignment::Right);
            label.text_vertical_align = Some(TextVerticalAlignment::Bottom);
            label.font_underline = Some(false);
            label.word_wrap = Some(true);
            let nested = edited
                .restyle("edge-view", edge)?
                .restyle("name-label", label)?
                .reopen(directory, format)?;
            source_fields(&mut expected, "edge-view")?.insert("lineColor".into(), json!("#102030"));
            let fields = source_fields(&mut expected, "name-label")?;
            fields.insert("horizontalAlignment".into(), json!(1));
            fields.insert("verticalAlignment".into(), json!(4));
            fields.insert("underline".into(), json!(false));
            fields.insert("wordWrap".into(), json!(true));
            let nested = nested.export_reopen(directory, &expected)?;
            assert_eq!(
                nested.0.pointer("/model/registry"),
                original.0.pointer("/model/registry")
            );

            let mut cleared = nested.style("class-view")?;
            cleared.fill = None;
            cleared.font_family = None;
            cleared.font_size = None;
            cleared.font_weight = None;
            cleared.font_italic = None;
            let cleared = nested
                .restyle("class-view", cleared)?
                .reopen(directory, format)?;
            let fields = source_fields(&mut expected, "class-view")?;
            fields.remove("fillColor");
            fields.remove("font");
            cleared.export_reopen(directory, &expected)?;
            assert_eq!(fs::read(&source_path)?, SOURCE);
        }
        Ok(())
    })
}

#[test]
fn retain_unsupported_source_style_and_reject_typed_conflicts() -> Result {
    assert!(
        REQUIREMENTS
            .contains("Scenario: Preserve unsupported source style without substituting defaults")
    );
    scenario("source-residue", |directory| {
        for font in [
            json!("?;13;0"),
            json!("Inter;13;4"),
            json!("broken"),
            Value::Null,
        ] {
            let mut expected: Value = serde_json::from_slice(SOURCE)?;
            let unsupported = [
                ("font", font),
                ("fillColor", Value::Null),
                ("lineColor", json!("red")),
                ("fontColor", json!("#ZZZZZZ")),
                ("underline", json!("yes")),
                ("wordWrap", Value::Null),
                ("showShadow", json!(1)),
                ("horizontalAlignment", json!(99)),
                ("verticalAlignment", json!(-1)),
            ];
            for (field, value) in &unsupported {
                source_fields(&mut expected, "class-view")?.insert((*field).into(), value.clone());
            }
            let source = serde_json::to_vec_pretty(&expected)?;
            let source_path = directory.join("source.mdj");
            fs::write(&source_path, &source)?;
            let response =
                import_mdj_document(&fs::read(&source_path)?).map_err(std::io::Error::other)?;
            let envelope: Value = serde_json::from_str(&response)?;
            let diagnostics = envelope
                .get("diagnostics")
                .and_then(Value::as_array)
                .ok_or("missing diagnostics")?;
            for (field, _) in &unsupported {
                assert!(
                    diagnostics.iter().any(|diagnostic| {
                        diagnostic
                            .get("path")
                            .and_then(Value::as_str)
                            .is_some_and(|path| path.ends_with(&format!("/ownedViews/0/{field}")))
                            && diagnostic
                                .get("message")
                                .and_then(Value::as_str)
                                .is_some_and(|message| message.contains("source residue"))
                    }),
                    "missing diagnostic for {field}"
                );
            }
            let imported = Document::response(response)?;
            for format in ["json", "protobuf"] {
                let document = imported.reopen(directory, format)?;
                assert_eq!(document.style("class-view")?, Style::default());
                for (field, value) in &unsupported {
                    assert_eq!(
                        document
                            .view("class-view")?
                            .pointer("/presentation/source/residual")
                            .and_then(|residue| residue.get(*field)),
                        Some(value)
                    );
                }
                assert_eq!(document.export()?, source);
                let mut renamed = Document(document.0.clone());
                renamed.0["name"] = json!("Renamed project");
                let mut renamed_source = expected.clone();
                renamed_source["name"] = json!("Renamed project");
                let renamed = renamed
                    .reopen(directory, format)?
                    .export_reopen(directory, &renamed_source)?;
                let mut conflict = renamed.style("class-view")?;
                conflict.fill = Some("#ABCDEF".into());
                let conflict = renamed
                    .restyle("class-view", conflict)?
                    .reopen(directory, format)?;
                let error = conflict
                    .export()
                    .err()
                    .ok_or("conflicting residue was exported")?;
                assert!(
                    error
                        .to_string()
                        .contains("mapped field fillColor conflicts with source residue")
                );
                assert_eq!(fs::read(&source_path)?, source);
            }
        }
        Ok(())
    })
}

#[test]
fn reject_unrepresentable_edits_and_repair_exact_replay() -> Result {
    assert!(REQUIREMENTS.contains(
        "Scenario: Reject unsupported style edits and recover without changing the source"
    ));
    scenario("style-repair", |directory| {
        let source_path = directory.join("source.mdj");
        fs::write(&source_path, SOURCE)?;
        let imported = Document::import(&fs::read(&source_path)?)?;
        for format in ["json", "protobuf"] {
            let document = imported.reopen(directory, format)?;
            let original_style = document.style("class-view")?;
            let mut opacity = original_style.clone();
            opacity.opacity = Some(0.5);
            let mut weight = original_style.clone();
            weight.font_weight = Some(600);
            let mut partial = original_style.clone();
            partial.font_size = None;
            for (style, message) in [
                (opacity, "cannot represent this edit losslessly"),
                (weight, "only normal (400) or bold (700)"),
                (partial, "requires family, size, weight and italic together"),
            ] {
                let edited = document
                    .restyle("class-view", style)?
                    .reopen(directory, format)?;
                let error = edited
                    .export()
                    .err()
                    .ok_or("unsupported edit was exported")?;
                assert!(error.to_string().contains(message), "{error}");
                let repaired = edited
                    .restyle("class-view", original_style.clone())?
                    .reopen(directory, format)?;
                assert_eq!(repaired.0, document.0);
                assert_eq!(repaired.export()?, SOURCE);
                assert_eq!(repaired.export()?, repaired.export()?);
                assert_eq!(fs::read(&source_path)?, SOURCE);
            }
        }
        Ok(())
    })
}
