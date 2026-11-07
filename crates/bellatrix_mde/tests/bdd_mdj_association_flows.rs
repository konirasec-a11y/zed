use bellatrix_mde::contracts::protocol::{command::Command, projection::ModelingProjection};
use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, export_mdj_section,
    import_mdj_document, prepare_model_document, project_model_section,
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

    fn projection(&self) -> Result<ModelingProjection> {
        let parts = self.parts()?;
        Ok(
            project_model_section(&parts.id, &parts.name, &parts.section_json)
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

fn source_object<'a>(value: &'a Value, id: &str) -> Result<&'a Value> {
    fn find<'a>(value: &'a Value, id: &str) -> Option<&'a Value> {
        match value {
            Value::Object(fields) if fields.get("_id").and_then(Value::as_str) == Some(id) => {
                Some(value)
            }
            Value::Object(fields) => fields.values().find_map(|value| find(value, id)),
            Value::Array(values) => values.iter().find_map(|value| find(value, id)),
            _ => None,
        }
    }
    find(value, id).ok_or_else(|| format!("missing MDJ object {id}").into())
}

use bellatrix_mde::contracts::protocol::NavigationKnowledge;

const SOURCE: &[u8] = include_bytes!("../fixtures/bdd/interchange/mdj-association-defaults.mdj");
const REQUIREMENTS: &str = include_str!("features/mdj_association_lifecycle.feature");

impl Document {
    fn property(&self, id: &str, path: &str, value: Value) -> Result<Self> {
        self.command(Command::SetProperty {
            element_id: id.into(),
            path: path.into(),
            value,
        })
    }

    fn navigation(&self, id: &str, navigation: NavigationKnowledge) -> Result<Self> {
        self.command(Command::SetAssociationEndNavigation {
            association_id: "association".into(),
            property_id: id.into(),
            navigation,
        })
    }

    fn contract(&self, id: &str, metaclass: &str) -> Result<&Value> {
        let entry = self
            .0
            .pointer("/model/registry/entries")
            .and_then(|entries| entries.get(id))
            .ok_or("missing registry entry")?;
        assert_eq!(entry.get("type"), Some(&json!(metaclass)));
        entry
            .get("data")
            .ok_or_else(|| "missing contract data".into())
    }

    fn query(&self, id: &str, path: &str) -> Result<Value> {
        let projection = self.projection()?;
        let inspector = projection.inspect_element(id).ok_or("missing inspector")?;
        let property = inspector
            .properties
            .iter()
            .find(|property| property.path == path)
            .ok_or_else(|| format!("missing query {id}.{path}"))?;
        Ok(property.value.clone())
    }
}

fn multiplicity(document: &Document, authored: bool) -> Result {
    for id in ["left-end", "right-end"] {
        let property = document.contract(id, "Property")?;
        assert_eq!(property.get("association"), Some(&json!("association")));
        assert!(property.get("multiplicity").is_none());
        for (field, metaclass, value) in [
            ("lowerValue", "LiteralInteger", "0"),
            ("upperValue", "LiteralUnlimitedNatural", "*"),
        ] {
            if authored && id == "left-end" {
                let reference = property
                    .get(field)
                    .and_then(Value::as_str)
                    .ok_or("missing bound")?;
                assert_eq!(
                    document.contract(reference, metaclass)?.get("value"),
                    Some(&json!(value))
                );
            } else {
                assert_eq!(property.get(field), Some(&Value::Null));
            }
        }
        assert_eq!(
            document.query(id, "properties.multiplicity")?,
            json!(if authored && id == "left-end" {
                "0..*"
            } else {
                "1"
            })
        );
    }
    let projection = document.projection()?;
    let relation = projection
        .relations
        .iter()
        .find(|relation| relation.id == "association")
        .ok_or("missing association projection")?;
    assert_eq!(
        relation.presentation.source_labels.multiplicity.as_deref(),
        authored.then_some("0..*")
    );
    assert!(relation.presentation.target_labels.multiplicity.is_none());
    let source: Value = serde_json::from_slice(&document.export()?)?;
    assert_eq!(
        source_object(&source, "left-end")?.get("multiplicity"),
        authored.then(|| json!("0..*")).as_ref()
    );
    assert_eq!(
        source_object(&source, "right-end")?.get("multiplicity"),
        Some(&json!(""))
    );
    Ok(())
}

fn navigation(document: &Document, known: bool) -> Result {
    for (id, chosen) in [("left-end", true), ("right-end", false)] {
        let answer = document.query(id, "uml.isNavigable")?;
        if known {
            assert_eq!(answer, json!(chosen));
        } else {
            assert!(
                answer
                    .as_str()
                    .is_some_and(|message| message.contains("unverified"))
            );
        }
        assert_eq!(
            document.query(id, "authoring.navigation")?,
            json!(if !known {
                "unverified"
            } else if chosen {
                "navigable"
            } else {
                "notNavigable"
            })
        );
    }
    let complete_set = document.query("association", "uml.navigableOwnedEnd")?;
    if known {
        assert_eq!(complete_set, json!([{"$ref": "left-end"}]));
        assert!(document.0.pointer("/model/authoringKnowledge").is_none());
    } else {
        assert!(
            complete_set
                .as_str()
                .is_some_and(|message| message.contains("unverified"))
        );
        assert_eq!(
            document
                .0
                .pointer("/model/authoringKnowledge/associations/association"),
            Some(&json!({"navigation_unverified": ["left-end", "right-end"]}))
        );
    }
    assert_eq!(
        document
            .contract("association", "Association")?
            .get("navigableOwnedEnd"),
        Some(&if known {
            json!(["left-end"])
        } else {
            json!([])
        })
    );
    let exported = document.export()?;
    let source: Value = serde_json::from_slice(&exported)?;
    assert_eq!(
        source_object(&source, "left-end")?.get("navigable"),
        known.then(|| json!("navigable")).as_ref()
    );
    assert_eq!(
        source_object(&source, "right-end")?.get("navigable"),
        Some(&json!(if known { "notNavigable" } else { "unspecified" }))
    );
    let reimported = Document::import(&exported)?;
    assert_eq!(
        reimported.0.pointer("/model/authoringKnowledge"),
        document.0.pointer("/model/authoringKnowledge")
    );
    Ok(())
}

#[test]
fn preserve_default_multiplicity_through_authoring_and_reopening() -> Result {
    assert!(REQUIREMENTS.contains(
        "Scenario: Preserve omitted and explicit empty multiplicities through edits and reopening"
    ));
    scenario("multiplicity", |directory| {
        let imported = Document::import(SOURCE)?;
        multiplicity(&imported, false)?;
        assert_eq!(imported.export()?, SOURCE);
        let provenance = imported
            .0
            .pointer("/model/mdjProvenance")
            .ok_or("missing source metadata")?
            .clone();
        for format in ["json", "protobuf"] {
            let renamed = imported
                .property("left", "name", json!("RenamedLeft"))?
                .reopen(directory, format)?;
            multiplicity(&renamed, false)?;
            let edited = renamed
                .property("left-end", "properties.multiplicity", json!("0..*"))?
                .reopen(directory, format)?;
            multiplicity(&edited, true)?;
            let restored = edited
                .property("left-end", "properties.multiplicity", json!(""))?
                .reopen(directory, format)?;
            multiplicity(&restored, false)?;
            assert_eq!(
                restored.0.pointer("/model/mdjProvenance"),
                Some(&provenance)
            );
        }
        Ok(())
    })
}

#[test]
fn preserve_unknown_navigation_and_reverse_explicit_decisions() -> Result {
    assert!(REQUIREMENTS.contains(
        "Scenario: Author navigation decisions without inventing defaults or losing source spelling"
    ));
    scenario("navigation", |directory| {
        let mut source: Value = serde_json::from_slice(SOURCE)?;
        let association = source
            .pointer_mut("/ownedElements/0/ownedElements/2")
            .ok_or("missing source association")?;
        association
            .get_mut("end1")
            .and_then(Value::as_object_mut)
            .ok_or("missing first end")?
            .remove("navigable");
        association["end2"]["navigable"] = json!("unspecified");
        let source = serde_json::to_vec_pretty(&source)?;
        let imported = Document::import(&source)?;
        navigation(&imported, false)?;
        assert_eq!(imported.export()?, source);
        for format in ["json", "protobuf"] {
            let renamed = imported.reopen(directory, format)?.property(
                "left",
                "name",
                json!("RenamedLeft"),
            )?;
            navigation(&renamed, false)?;
            let decided = renamed
                .navigation("left-end", NavigationKnowledge::Known(true))?
                .navigation("right-end", NavigationKnowledge::Known(false))?
                .reopen(directory, format)?;
            navigation(&decided, true)?;
            let restored = decided
                .navigation("right-end", NavigationKnowledge::Unverified)?
                .navigation("left-end", NavigationKnowledge::Unverified)?
                .reopen(directory, format)?;
            navigation(&restored, false)?;
            assert_eq!(restored.0, renamed.0);
        }
        Ok(())
    })
}
