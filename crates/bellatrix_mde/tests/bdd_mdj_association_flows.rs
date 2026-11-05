use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, export_mdj_section,
    import_mdj_document, prepare_model_document, project_model_section,
};
use bellatrix_mde_contracts::protocol::{ModelDocumentParts, command::Command};
use serde_json::{Value, json};
use std::{error::Error, fs, path::PathBuf};

type FlowResult<T> = Result<T, Box<dyn Error>>;

#[derive(serde::Deserialize)]
struct ModelResponse {
    model: Box<serde_json::value::RawValue>,
}

fn model_response(response: &str) -> FlowResult<Value> {
    let response: ModelResponse = serde_json::from_str(response)?;
    Ok(serde_json::from_str(response.model.get())?)
}

fn model_request(project: &Value) -> FlowResult<String> {
    Ok(format!(
        r#"{{"api_version":1,"model":{}}}"#,
        serde_json::to_string(project)?
    ))
}

fn import_mdj(bytes: &[u8]) -> FlowResult<Value> {
    model_response(&import_mdj_document(bytes).map_err(std::io::Error::other)?)
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
    )
    .map_err(std::io::Error::other)?;
    let bytes = encode_model_section(&document.id, &document.name, &section.to_string(), "json")
        .map_err(std::io::Error::other)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn decode_model(bytes: &[u8], format: &str) -> FlowResult<Value> {
    model_response(&decode_model_document(bytes, format).map_err(std::io::Error::other)?)
}

fn mdj_object<'a>(value: &'a Value, id: &str) -> Option<&'a serde_json::Map<String, Value>> {
    match value {
        Value::Object(object) => {
            if object.get("_id").and_then(Value::as_str) == Some(id) {
                Some(object)
            } else {
                object.values().find_map(|value| mdj_object(value, id))
            }
        }
        Value::Array(values) => values.iter().find_map(|value| mdj_object(value, id)),
        _ => None,
    }
}

struct SavedFlow(PathBuf);

impl SavedFlow {
    fn new(name: &str) -> FlowResult<Self> {
        let directory = std::env::temp_dir().join(format!(
            "bellatrix-mdj-association-bdd-{}-{name}",
            std::process::id()
        ));
        fs::create_dir(&directory)?;
        Ok(Self(directory))
    }

    fn reopen(&self, project: &Value, name: &str, format: &str) -> FlowResult<Value> {
        let path = self.0.join(name);
        fs::write(&path, encode_model(project, format)?)?;
        let reopened = decode_model(&fs::read(path)?, format)?;
        assert_eq!(&reopened, project);
        Ok(reopened)
    }
}

impl Drop for SavedFlow {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "Could not clean BDD directory {}: {error}",
                self.0.display()
            );
        }
    }
}

fn set_property(project: &Value, id: &str, path: &str, value: Value) -> FlowResult<Value> {
    edit(
        project,
        Command::SetProperty {
            element_id: id.into(),
            path: path.into(),
            value,
        },
    )
}

fn object<'a>(project: &'a Value, collection: &str, id: &str) -> FlowResult<&'a Value> {
    project[collection]
        .as_array()
        .ok_or("missing collection")?
        .iter()
        .find(|item| item["id"] == id)
        .ok_or_else(|| format!("missing {collection} item {id}").into())
}

fn assert_exported_multiplicities(project: &Value, left: Option<&str>) -> FlowResult<()> {
    let bytes = export_mdj(project)?;
    let document: Value = serde_json::from_slice(&bytes)?;
    let first = mdj_object(&document, "left-end").ok_or("missing first MDJ end")?;
    let second = mdj_object(&document, "right-end").ok_or("missing second MDJ end")?;
    assert_eq!(first.get("multiplicity").and_then(Value::as_str), left);
    assert_eq!(second.get("multiplicity"), Some(&json!("")));
    assert_eq!(first.get("navigable"), Some(&json!("notNavigable")));
    assert_eq!(second.get("navigable"), Some(&json!("navigable")));
    Ok(())
}

#[test]
fn omitted_multiplicity_survives_edit_save_reopen_and_restore() -> FlowResult<()> {
    assert!(include_str!("features/mdj_association_lifecycle.feature").contains(
        "Scenario: Preserve omitted and explicit empty multiplicities through edits and reopening"
    ));
    println!("Given explicit navigation with omitted and explicit empty MDJ multiplicities");
    let original = include_bytes!("../fixtures/bdd/interchange/mdj-association-defaults.mdj");
    let project = import_mdj(original)?;
    for id in ["left-end", "right-end"] {
        let end = object(&project, "elements", id)?;
        assert_eq!(end["kind"], "uml.attribute");
        assert_eq!(end["properties"].get("multiplicity"), Some(&json!("")));
    }
    let association = object(&project, "relations", "association")?;
    assert_eq!(association["kind"], "uml.association");
    assert_eq!(
        association["properties"].get("navigableOwnedEnd"),
        Some(&json!([{"$ref":"right-end"}]))
    );
    assert_eq!(export_mdj(&project)?, original.as_slice());
    let saved = SavedFlow::new("multiplicity")?;
    println!("When a class is renamed and the document is saved, closed and reopened");
    let renamed = set_property(&project, "left", "name", json!("RenamedLeft"))?;
    for (name, format) in [("renamed.json", "json"), ("renamed.blx", "protobuf")] {
        let reopened = saved.reopen(&renamed, name, format)?;
        assert_exported_multiplicities(&reopened, None)?;
    }
    println!("When a formerly omitted multiplicity is authored and reopened");
    let edited = set_property(
        &renamed,
        "left-end",
        "properties.multiplicity",
        json!("0..*"),
    )?;
    let association = object(&edited, "relations", "association")?;
    assert_eq!(
        association["properties"].get("sourceMultiplicity"),
        Some(&json!("0..*"))
    );
    for (name, format) in [("edited.json", "json"), ("edited.blx", "protobuf")] {
        let reopened = saved.reopen(&edited, name, format)?;
        assert_exported_multiplicities(&reopened, Some("0..*"))?;
    }
    println!("Then restoring the default preserves original absence rather than inventing a field");
    let restored = set_property(&edited, "left-end", "properties.multiplicity", json!(""))?;
    assert_exported_multiplicities(&restored, None)?;
    Ok(())
}

fn choose_navigation(
    project: &Value,
    id: &str,
    navigation: bellatrix_mde_contracts::protocol::NavigationKnowledge,
) -> FlowResult<Value> {
    edit(
        project,
        Command::SetAssociationEndNavigation {
            association_id: "association".into(),
            property_id: id.into(),
            navigation,
        },
    )
}

fn navigation_source() -> FlowResult<Vec<u8>> {
    let mut document: Value = serde_json::from_slice(include_bytes!(
        "../fixtures/bdd/interchange/mdj-association-defaults.mdj"
    ))?;
    let association = document
        .pointer_mut("/ownedElements/0/ownedElements/2")
        .ok_or("missing source association")?;
    association["end1"]
        .as_object_mut()
        .ok_or("missing source end")?
        .remove("navigable");
    association["end2"]["navigable"] = json!("unspecified");
    Ok(serde_json::to_vec_pretty(&document)?)
}

fn assert_navigation_export(project: &Value, left: Option<&str>, right: &str) -> FlowResult<()> {
    let bytes = export_mdj(project)?;
    let document: Value = serde_json::from_slice(&bytes)?;
    let first = mdj_object(&document, "left-end").ok_or("missing first end")?;
    let second = mdj_object(&document, "right-end").ok_or("missing second end")?;
    assert_eq!(first.get("navigable").and_then(Value::as_str), left);
    assert_eq!(second.get("navigable").and_then(Value::as_str), Some(right));
    let reimported = import_mdj(&bytes)?;
    assert_eq!(
        reimported.get("authoring_knowledge"),
        project.get("authoring_knowledge")
    );
    Ok(())
}

fn assert_navigation_consumers(project: &Value, known: bool) -> FlowResult<()> {
    use bellatrix_mde_contracts::protocol::presentation::EndpointMarkerGlyph;
    let mut document = project.clone();
    document["diagrams"] = json!([{
        "id":"navigation-preview", "name":"Navigation preview", "uml_di_type":"UMLClassDiagram",
        "views":[{
            "id":"association-view", "subject":"association",
            "style":{"uml":{"association_end_convention":"star_uml"}},
            "presentation":{"relationship_endpoints":{
                "source":{"target":{"target":"model_end", "id":"left-end"},"marker":{"source":"model"}},
                "target":{"target":{"target":"model_end", "id":"right-end"},"marker":{"source":"model"}}
            }}
        }]
    }]);
    let document = document_parts(&document)?;
    let section = project_model_section(&document.id, &document.name, &document.section_json)
        .map_err(std::io::Error::other)?;
    let resolved = section
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| view.id == "association-view")
        .and_then(|view| view.relationship.as_ref())
        .ok_or("missing relationship presentation")?;
    assert!(resolved.diagnostics.is_empty());
    let (left, right) = resolved
        .markers
        .as_ref()
        .ok_or("missing endpoint markers")?;
    if known {
        assert_eq!(
            left.glyphs
                .iter()
                .map(|part| part.glyph)
                .collect::<Vec<_>>(),
            vec![EndpointMarkerGlyph::OpenArrow]
        );
        assert_eq!(
            right
                .glyphs
                .iter()
                .map(|part| part.glyph)
                .collect::<Vec<_>>(),
            vec![EndpointMarkerGlyph::Cross]
        );
    } else {
        assert!(left.glyphs.is_empty() && right.glyphs.is_empty());
    }
    let inspector = section
        .inspect_element("left-end")
        .ok_or("missing end inspector")?;
    let knowledge = inspector
        .properties
        .iter()
        .find(|property| property.path == "authoring.navigation")
        .ok_or("missing Inspector knowledge")?;
    assert_eq!(
        knowledge.value,
        if known {
            json!("navigable")
        } else {
            json!("unverified")
        }
    );
    Ok(())
}

#[test]
fn unknown_navigation_survives_reopening_decisions_and_atomic_repair() -> FlowResult<()> {
    use bellatrix_mde_contracts::protocol::NavigationKnowledge;
    assert!(include_str!("features/mdj_association_lifecycle.feature").contains(
        "Scenario: Author navigation decisions without inventing defaults or losing source spelling"
    ));
    println!("Given absent and explicit unspecified navigation in an MDJ association");
    let source = navigation_source()?;
    let project = import_mdj(&source)?;
    assert_eq!(export_mdj(&project)?, source);
    assert_eq!(
        project["authoring_knowledge"]["associations"].get("association"),
        Some(&json!({"navigation_unverified":["left-end", "right-end"]}))
    );
    assert_navigation_consumers(&project, false)?;
    let saved = SavedFlow::new("navigation")?;
    println!("When I save, close, reopen and make an unrelated name edit");
    let renamed = set_property(&project, "left", "name", json!("RenamedLeft"))?;
    for (name, format) in [("unknown.json", "json"), ("unknown.blx", "protobuf")] {
        let reopened = saved.reopen(&renamed, name, format)?;
        assert_navigation_export(&reopened, None, "unspecified")?;
    }
    println!("When explicit navigation decisions resolve both unknown ends atomically");
    let first_decision = choose_navigation(&renamed, "left-end", NavigationKnowledge::Known(true))?;
    let decided = choose_navigation(
        &first_decision,
        "right-end",
        NavigationKnowledge::Known(false),
    )?;
    assert!(decided.get("authoring_knowledge").is_none());
    assert_navigation_consumers(&decided, true)?;
    for (name, format) in [("decided.json", "json"), ("decided.blx", "protobuf")] {
        let reopened = saved.reopen(&decided, name, format)?;
        assert_navigation_export(&reopened, Some("navigable"), "notNavigable")?;
    }
    println!("Then restoring unknown decisions in reverse order retains original source encoding");
    let restored_right = choose_navigation(&decided, "right-end", NavigationKnowledge::Unverified)?;
    let restored = choose_navigation(&restored_right, "left-end", NavigationKnowledge::Unverified)?;
    assert_eq!(
        restored.get("authoring_knowledge"),
        renamed.get("authoring_knowledge")
    );
    assert_navigation_export(&restored, None, "unspecified")?;
    for references in [
        vec!["left-end", "left-end"],
        vec!["missing-end"],
        vec!["left"],
    ] {
        let mut invalid = project.clone();
        invalid["authoring_knowledge"]["associations"]["association"] =
            json!({"navigation_unverified": references});
        let reopened = saved.reopen(&invalid, "invalid.json", "json")?;
        assert!(export_mdj(&reopened).is_err());
    }
    let mut conflicting = first_decision.clone();
    conflicting["authoring_knowledge"] = project["authoring_knowledge"].clone();
    assert!(export_mdj(&conflicting).is_err());
    println!("And ambiguous authoring identities cannot disappear in a document or API envelope");
    let mut document = project.clone();
    document["authoring_knowledge"] = Value::Null;
    let ambiguous = document.to_string().replace(
        "\"authoring_knowledge\":null",
        r#""authoring_knowledge":{"associations":{"association":{"navigation_unverified":["left-end"]},"association":{"navigation_unverified":[]}}}"#,
    );
    let path = saved.0.join("ambiguous.json");
    fs::write(&path, &ambiguous)?;
    assert!(decode_model(&fs::read(path)?, "json").is_err());
    let envelope = format!(r#"{{"api_version":1,"model":{ambiguous}}}"#);
    assert!(bellatrix_mde::validate_graph_model(&envelope).is_err());
    assert!(
        bellatrix_mde::reparent_uml_element(&format!(
            r#"{{"api_version":1,"model":{ambiguous},"element_id":"left-end","owner_id":"left"}}"#
        ))
        .is_err()
    );
    assert!(
        bellatrix_mde::reachable_graph_nodes(&format!(
            r#"{{"api_version":1,"model":{ambiguous},"roots":["left"],"relation_kinds":[]}}"#
        ))
        .is_err()
    );
    assert!(
        bellatrix_mde::analyze_generation_dependencies(&format!(
            r#"{{"api_version":1,"model":{ambiguous},"selected_ids":[]}}"#
        ))
        .is_err()
    );
    let mut duplicate_wire = encode_model(&project, "protobuf")?;
    let association_id = b"association";
    duplicate_wire.extend_from_slice(&[
        0x6a,
        u8::try_from(association_id.len() + 2)?,
        0x0a,
        u8::try_from(association_id.len())?,
    ]);
    duplicate_wire.extend_from_slice(association_id);
    fs::write(saved.0.join("ambiguous.blx"), &duplicate_wire)?;
    assert!(decode_model(&fs::read(saved.0.join("ambiguous.blx"))?, "protobuf").is_err());
    let mut native = project.clone();
    let native_fields = native.as_object_mut().ok_or("missing document")?;
    native_fields.remove("composition");
    native_fields.remove("mdj_provenance");
    let before = encode_model(&native, "json")?;
    assert!(
        edit(
            &native,
            Command::SetOwner {
                element_id: "left-end".into(),
                owner_id: Some("left".into()),
            },
        )
        .is_err()
    );
    assert_eq!(encode_model(&native, "json")?, before);
    Ok(())
}
