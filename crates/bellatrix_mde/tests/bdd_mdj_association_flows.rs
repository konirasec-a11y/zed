use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, export_mdj_section,
    import_mdj_document, prepare_model_document, project_model_section,
};
use bellatrix_mde_contracts::protocol::{
    ModelDocumentParts, command::Command, projection::ModelingProjection,
};
use prost::Message;
use serde_json::{Value, json};
use std::{error::Error, fs, path::PathBuf};

type FlowResult<T> = Result<T, Box<dyn Error>>;

#[derive(serde::Deserialize)]
struct ModelResponse {
    model: Box<serde_json::value::RawValue>,
}

#[derive(Clone, PartialEq, Message)]
struct Blx2Document {
    #[prost(bytes, tag = "1")]
    document_json: Vec<u8>,
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

fn contract_data<'a>(project: &'a Value, id: &str, metaclass: &str) -> FlowResult<&'a Value> {
    let entry = project
        .pointer("/model/registry/entries")
        .and_then(|entries| entries.get(id))
        .ok_or_else(|| format!("missing concrete {metaclass} contract {id}"))?;
    assert_eq!(entry.get("type"), Some(&json!(metaclass)));
    let data = entry.get("data").ok_or("missing contract data")?;
    assert_eq!(data.get("id"), Some(&json!(id)));
    Ok(data)
}

fn projection(project: &Value) -> FlowResult<ModelingProjection> {
    let document = document_parts(project)?;
    Ok(
        project_model_section(&document.id, &document.name, &document.section_json)
            .map_err(std::io::Error::other)?,
    )
}

fn assert_association_contracts(project: &Value) -> FlowResult<()> {
    contract_data(project, "model", "Model")?;
    for id in ["left", "right"] {
        contract_data(project, id, "Class")?;
    }
    let association = contract_data(project, "association", "Association")?;
    assert_eq!(
        association.get("memberEnd"),
        Some(&json!(["left-end", "right-end"]))
    );
    assert_eq!(association.get("ownedEnd"), association.get("memberEnd"));
    for (id, classifier) in [("left-end", "left"), ("right-end", "right")] {
        let end = contract_data(project, id, "Property")?;
        assert_eq!(end.get("type"), Some(&json!(classifier)));
        assert_eq!(end.get("association"), Some(&json!("association")));
        assert_eq!(end.get("owningAssociation"), Some(&json!("association")));
    }
    Ok(())
}

fn assert_property_bounds(
    project: &Value,
    id: &str,
    bounds: Option<(&str, &str)>,
) -> FlowResult<()> {
    let end = contract_data(project, id, "Property")?;
    assert!(end.get("multiplicity").is_none());
    assert!(end.get("properties").is_none());
    match bounds {
        None => {
            assert_eq!(end.get("lowerValue"), Some(&Value::Null));
            assert_eq!(end.get("upperValue"), Some(&Value::Null));
        }
        Some((lower, upper)) => {
            for (field, metaclass, expected) in [
                ("lowerValue", "LiteralInteger", lower),
                ("upperValue", "LiteralUnlimitedNatural", upper),
            ] {
                let literal_id = end
                    .get(field)
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("{id}.{field} must reference a concrete {metaclass}"))?;
                let literal = contract_data(project, literal_id, metaclass)?;
                assert_eq!(literal.get("value"), Some(&json!(expected)));
            }
        }
    }
    let snapshot = projection(project)?;
    let inspector = snapshot
        .inspect_element(id)
        .ok_or("missing Property inspector")?;
    let multiplicity = inspector
        .properties
        .iter()
        .find(|property| property.path == "properties.multiplicity")
        .ok_or("missing multiplicity authoring adapter")?;
    assert!(multiplicity.editable);
    assert_eq!(
        multiplicity.value,
        match bounds {
            None => json!("1"),
            Some((lower, upper)) if lower == upper => json!(lower),
            Some((lower, upper)) => json!(format!("{lower}..{upper}")),
        }
    );
    Ok(())
}

fn assert_projected_multiplicity(project: &Value, source: Option<&str>) -> FlowResult<()> {
    let snapshot = projection(project)?;
    let association = snapshot
        .relations
        .iter()
        .find(|relation| relation.id == "association")
        .ok_or("missing concrete Association projection")?;
    assert_eq!(
        association
            .presentation
            .source_labels
            .multiplicity
            .as_deref(),
        source
    );
    assert!(
        association
            .presentation
            .target_labels
            .multiplicity
            .is_none()
    );
    if let Some(expected) = source {
        let inspector = snapshot
            .inspect_element("association")
            .ok_or("missing Association inspector")?;
        let multiplicity = inspector
            .properties
            .iter()
            .find(|property| property.path == "properties.sourceMultiplicity")
            .ok_or("missing derived source multiplicity")?;
        assert_eq!(multiplicity.value, json!(expected));
    }
    let association = contract_data(project, "association", "Association")?;
    assert!(association.get("sourceMultiplicity").is_none());
    assert!(association.get("targetMultiplicity").is_none());
    Ok(())
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
    assert_association_contracts(&project)?;
    for id in ["left-end", "right-end"] {
        assert_property_bounds(&project, id, None)?;
    }
    assert_projected_multiplicity(&project, None)?;
    let association = contract_data(&project, "association", "Association")?;
    assert_eq!(
        association.get("navigableOwnedEnd"),
        Some(&json!(["right-end"]))
    );
    assert_eq!(export_mdj(&project)?, original.as_slice());
    let saved = SavedFlow::new("multiplicity")?;
    println!("When a class is renamed and the document is saved, closed and reopened");
    let renamed = set_property(&project, "left", "name", json!("RenamedLeft"))?;
    for (name, format) in [("renamed.json", "json"), ("renamed.blx", "protobuf")] {
        let reopened = saved.reopen(&renamed, name, format)?;
        assert_property_bounds(&reopened, "left-end", None)?;
        assert_property_bounds(&reopened, "right-end", None)?;
        assert_exported_multiplicities(&reopened, None)?;
    }
    println!("When a formerly omitted multiplicity is authored and reopened");
    let edited = set_property(
        &renamed,
        "left-end",
        "properties.multiplicity",
        json!("0..*"),
    )?;
    assert_property_bounds(&edited, "left-end", Some(("0", "*")))?;
    assert_projected_multiplicity(&edited, Some("0..*"))?;
    for (name, format) in [("edited.json", "json"), ("edited.blx", "protobuf")] {
        let reopened = saved.reopen(&edited, name, format)?;
        assert_property_bounds(&reopened, "left-end", Some(("0", "*")))?;
        assert_property_bounds(&reopened, "right-end", None)?;
        assert_projected_multiplicity(&reopened, Some("0..*"))?;
        assert_exported_multiplicities(&reopened, Some("0..*"))?;
    }
    let before_invalid = encode_model(&edited, "json")?;
    assert!(
        set_property(
            &edited,
            "left-end",
            "properties.multiplicity",
            json!("2..1")
        )
        .is_err()
    );
    assert_eq!(encode_model(&edited, "json")?, before_invalid);
    println!("Then restoring the default preserves original absence rather than inventing a field");
    let restored = set_property(&edited, "left-end", "properties.multiplicity", json!(""))?;
    assert_property_bounds(&restored, "left-end", None)?;
    assert_projected_multiplicity(&restored, None)?;
    let edited_end = contract_data(&edited, "left-end", "Property")?;
    let restored_entries = restored
        .pointer("/model/registry/entries")
        .ok_or("missing restored registry")?;
    for field in ["lowerValue", "upperValue"] {
        let literal_id = edited_end
            .get(field)
            .and_then(Value::as_str)
            .ok_or("missing authored bound identity")?;
        assert!(restored_entries.get(literal_id).is_none());
    }
    assert_exported_multiplicities(&restored, None)?;
    for (name, format) in [("restored.json", "json"), ("restored.blx", "protobuf")] {
        let reopened = saved.reopen(&restored, name, format)?;
        assert_property_bounds(&reopened, "left-end", None)?;
        assert_exported_multiplicities(&reopened, None)?;
    }
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
    assert_association_contracts(&reimported)?;
    assert_eq!(
        reimported.pointer("/model/authoringKnowledge"),
        project.pointer("/model/authoringKnowledge")
    );
    Ok(())
}

fn assert_navigation_consumers(project: &Value, known: bool) -> FlowResult<()> {
    use bellatrix_mde_contracts::protocol::presentation::EndpointMarkerGlyph;
    let association = contract_data(project, "association", "Association")?;
    assert_eq!(
        association.get("navigableOwnedEnd"),
        Some(&if known {
            json!(["left-end"])
        } else {
            json!([])
        })
    );
    let mut document = project.clone();
    document["model"]["diagrams"] = json!([{
        "id":"navigation-preview", "name":"Navigation preview", "uml_di_type":"UMLClassDiagram",
        "model_elements":[],
        "views":[{
            "id":"left-view", "subject":"left",
            "geometry":{"kind":"rect", "x":20.0, "y":20.0, "width":100.0, "height":60.0}
        }, {
            "id":"right-view", "subject":"right",
            "geometry":{"kind":"rect", "x":300.0, "y":20.0, "width":100.0, "height":60.0}
        }, {
            "id":"association-view", "subject":"association",
            "geometry":{"kind":"path", "start":{"x":120.0,"y":50.0},
                "segments":[{"kind":"line","to":{"x":300.0,"y":50.0}}]},
            "endpoints":{"source_view_id":"left-view", "target_view_id":"right-view"},
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
    assert!(
        resolved.diagnostics.is_empty(),
        "{:?}",
        resolved.diagnostics
    );
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
    let navigability = inspector
        .properties
        .iter()
        .find(|property| property.path == "uml.isNavigable")
        .ok_or("missing derived UML navigability")?;
    assert!(!navigability.editable);
    if known {
        assert_eq!(navigability.value, json!(true));
    } else {
        assert!(
            navigability
                .value
                .as_str()
                .is_some_and(|value| value.contains("unverified"))
        );
    }
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
    assert_association_contracts(&project)?;
    assert_eq!(export_mdj(&project)?, source);
    assert_eq!(
        project.pointer("/model/authoringKnowledge/associations/association"),
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
    assert!(decided.pointer("/model/authoringKnowledge").is_none());
    assert_navigation_consumers(&decided, true)?;
    for (name, format) in [("decided.json", "json"), ("decided.blx", "protobuf")] {
        let reopened = saved.reopen(&decided, name, format)?;
        assert_navigation_export(&reopened, Some("navigable"), "notNavigable")?;
    }
    println!("Then restoring unknown decisions in reverse order retains original source encoding");
    let restored_right = choose_navigation(&decided, "right-end", NavigationKnowledge::Unverified)?;
    let restored = choose_navigation(&restored_right, "left-end", NavigationKnowledge::Unverified)?;
    assert_eq!(
        restored.pointer("/model/authoringKnowledge"),
        renamed.pointer("/model/authoringKnowledge")
    );
    assert_navigation_export(&restored, None, "unspecified")?;
    for references in [
        vec!["left-end", "left-end"],
        vec!["missing-end"],
        vec!["left"],
    ] {
        let mut invalid = project.clone();
        invalid["model"]["authoringKnowledge"]["associations"]["association"] =
            json!({"navigation_unverified": references});
        let reopened = saved.reopen(&invalid, "invalid.json", "json")?;
        assert!(export_mdj(&reopened).is_err());
    }
    let mut conflicting = first_decision.clone();
    conflicting["model"]["authoringKnowledge"] = project["model"]["authoringKnowledge"].clone();
    assert!(export_mdj(&conflicting).is_err());
    println!("And ambiguous authoring identities cannot disappear in a document or API envelope");
    let mut document = project.clone();
    document["model"]["authoringKnowledge"] = Value::Null;
    let ambiguous = document.to_string().replace(
        "\"authoringKnowledge\":null",
        r#""authoringKnowledge":{"associations":{"association":{"navigation_unverified":["left-end"]},"association":{"navigation_unverified":[]}}}"#,
    );
    let path = saved.0.join("ambiguous.json");
    fs::write(&path, &ambiguous)?;
    let Err(error) = decode_model(&fs::read(path)?, "json") else {
        return Err("duplicate Association knowledge identity was accepted in JSON".into());
    };
    assert!(
        error
            .to_string()
            .contains("duplicate Association knowledge identity")
    );
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
    let original_wire = encode_model(&project, "protobuf")?;
    let payload = original_wire
        .strip_prefix(b"BLX2")
        .ok_or("missing BLX2 header")?;
    let mut wire = Blx2Document::decode(payload)?;
    assert_eq!(
        serde_json::from_slice::<Value>(&wire.document_json)?,
        project
    );
    // BLX2 carries canonical JSON in field 1; an unknown protobuf field cannot test duplicate identities.
    wire.document_json = ambiguous.as_bytes().to_vec();
    let mut duplicate_wire = b"BLX2".to_vec();
    duplicate_wire.extend(wire.encode_to_vec());
    fs::write(saved.0.join("ambiguous.blx"), &duplicate_wire)?;
    let Err(error) = decode_model(&fs::read(saved.0.join("ambiguous.blx"))?, "protobuf") else {
        return Err("duplicate Association knowledge identity was accepted in BLX2".into());
    };
    assert!(
        error
            .to_string()
            .contains("duplicate Association knowledge identity")
    );
    let mut native = project.clone();
    let native_fields = native
        .get_mut("model")
        .and_then(Value::as_object_mut)
        .ok_or("missing canonical model")?;
    native_fields.remove("composition");
    native_fields.remove("mdjProvenance");
    assert_navigation_consumers(&native, false)?;
    for (name, format) in [("native.json", "json"), ("native.blx", "protobuf")] {
        let reopened = saved.reopen(&native, name, format)?;
        assert_navigation_consumers(&reopened, false)?;
    }
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
