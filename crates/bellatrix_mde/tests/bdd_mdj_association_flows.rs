use bellatrix_mde::{
    Project,
    commands::{Command, apply},
    decode_model_document, encode_model_document, export_mdj_document, import_mdj_document,
    validate_uml_model,
};
use serde_json::{Value, json};
use std::{error::Error, fs, path::PathBuf};

type FlowResult<T> = Result<T, Box<dyn Error>>;

#[derive(serde::Deserialize)]
struct ModelResponse {
    model: Box<serde_json::value::RawValue>,
}

fn model_response(response: &str) -> FlowResult<Project> {
    let response: ModelResponse = serde_json::from_str(response)?;
    Ok(serde_json::from_str(response.model.get())?)
}

fn model_request(project: &Project) -> FlowResult<String> {
    Ok(format!(
        r#"{{"api_version":1,"model":{}}}"#,
        serde_json::to_string(project)?
    ))
}

fn import_mdj(bytes: &[u8]) -> FlowResult<Project> {
    model_response(&import_mdj_document(bytes).map_err(std::io::Error::other)?)
}

fn export_mdj(project: &Project) -> FlowResult<Vec<u8>> {
    Ok(export_mdj_document(&model_request(project)?).map_err(std::io::Error::other)?)
}

fn encode_model(project: &Project, format: &str) -> FlowResult<Vec<u8>> {
    Ok(encode_model_document(&model_request(project)?, format).map_err(std::io::Error::other)?)
}

fn decode_model(bytes: &[u8], format: &str) -> FlowResult<Project> {
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

    fn reopen(&self, project: &Project, name: &str, format: &str) -> FlowResult<Project> {
        let path = self.0.join(name);
        fs::write(&path, encode_model(project, format)?)?;
        let reopened = decode_model(&fs::read(path)?, format)?;
        assert_eq!(&reopened, project);
        assert_eq!(validate(&reopened)?, validate(project)?);
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

fn validate(project: &Project) -> FlowResult<Value> {
    Ok(serde_json::from_str(
        &validate_uml_model(&json!({"api_version":1, "model":project}).to_string())
            .map_err(std::io::Error::other)?,
    )?)
}

fn set_property(project: &Project, id: &str, path: &str, value: Value) -> FlowResult<Project> {
    apply(
        project,
        &Command::SetProperty {
            element_id: id.into(),
            path: path.into(),
            value,
        },
        1,
    )
    .map_err(|error| std::io::Error::other(error).into())
}

fn assert_exported_multiplicities(project: &Project, left: Option<&str>) -> FlowResult<()> {
    let bytes = export_mdj(project)?;
    let document: Value = serde_json::from_slice(&bytes)?;
    let first = mdj_object(&document, "left-end").ok_or("missing first MDJ end")?;
    let second = mdj_object(&document, "right-end").ok_or("missing second MDJ end")?;
    assert_eq!(
        first.get("multiplicity").and_then(Value::as_str),
        left
    );
    assert_eq!(second.get("multiplicity"), Some(&json!("")));
    assert_eq!(
        first.get("navigable"),
        Some(&json!("notNavigable"))
    );
    assert_eq!(
        second.get("navigable"),
        Some(&json!("navigable"))
    );
    let reimported = import_mdj(&bytes)?;
    assert_eq!(validate(&reimported)?, validate(project)?);
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
        let end = project
            .elements
            .iter()
            .find(|end| end.id == id)
            .ok_or("missing end")?;
        assert_eq!(end.kind, "uml.attribute");
        assert_eq!(end.properties.get("multiplicity"), Some(&json!("")));
    }
    let association = project
        .relations
        .iter()
        .find(|relation| relation.id == "association")
        .ok_or("missing canonical association")?;
    assert_eq!(association.kind, "uml.association");
    assert_eq!(
        association.properties.get("navigableOwnedEnd"),
        Some(&json!([{"$ref":"right-end"}]))
    );
    assert_eq!(export_mdj(&project)?, original.as_slice());
    let saved = SavedFlow::new("multiplicity")?;
    println!("When a class is renamed and the document is saved, closed and reopened");
    let renamed = set_property(&project, "left", "name", json!("RenamedLeft"))?;
    for (name, format) in [
        ("renamed.json", "json"),
        ("renamed.blx", "protobuf"),
    ] {
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
    let association = edited
        .relations
        .iter()
        .find(|relation| relation.id == "association")
        .ok_or("missing edited association")?;
    assert_eq!(
        association.properties.get("sourceMultiplicity"),
        Some(&json!("0..*"))
    );
    for (name, format) in [
        ("edited.json", "json"),
        ("edited.blx", "protobuf"),
    ] {
        let reopened = saved.reopen(&edited, name, format)?;
        assert_exported_multiplicities(&reopened, Some("0..*"))?;
    }
    println!("Then restoring the default preserves original absence rather than inventing a field");
    let restored = set_property(&edited, "left-end", "properties.multiplicity", json!(""))?;
    assert_exported_multiplicities(&restored, None)?;
    Ok(())
}

fn choose_navigation(
    project: &Project,
    id: &str,
    navigation: bellatrix_mde::NavigationKnowledge,
) -> FlowResult<Project> {
    apply(
        project,
        &Command::SetAssociationEndNavigation {
            association_id: "association".into(),
            property_id: id.into(),
            navigation,
        },
        1,
    )
    .map_err(|error| std::io::Error::other(error).into())
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

fn assert_navigation_export(project: &Project, left: Option<&str>, right: &str) -> FlowResult<()> {
    let bytes = export_mdj(project)?;
    let document: Value = serde_json::from_slice(&bytes)?;
    let first = mdj_object(&document, "left-end").ok_or("missing first end")?;
    let second = mdj_object(&document, "right-end").ok_or("missing second end")?;
    assert_eq!(
        first.get("navigable").and_then(Value::as_str),
        left
    );
    assert_eq!(
        second.get("navigable").and_then(Value::as_str),
        Some(right)
    );
    let reimported = import_mdj(&bytes)?;
    assert_eq!(reimported.authoring_knowledge, project.authoring_knowledge);
    assert_eq!(validate(&reimported)?, validate(project)?);
    Ok(())
}

fn assert_navigation_consumers(project: &Project, known: bool) -> FlowResult<()> {
    use bellatrix_mde::{EndpointMarkerGlyph, ModelingSection, View};
    let section = ModelingSection::from_model(project);
    let view: View = serde_json::from_value(json!({
        "id":"association-view", "subject":"association",
        "style":{"uml":{"association_end_convention":"star_uml"}},
        "presentation":{"relationship_endpoints":{
            "source":{"target":{"target":"model_end", "id":"left-end"},"marker":{"source":"model"}},
            "target":{"target":{"target":"model_end", "id":"right-end"},"marker":{"source":"model"}}
        }}
    }))?;
    let resolved = bellatrix_mde::resolve_relationship_view(&section, &view)
        .ok_or("missing relationship presentation")?;
    assert!(resolved.diagnostics.is_empty());
    let (left, right) = resolved.markers.ok_or("missing endpoint markers")?;
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
    let inspector =
        bellatrix_mde::inspect_element(&section, "left-end").ok_or("missing end inspector")?;
    let knowledge = inspector
        .properties
        .iter()
        .find(|property| property.path == "authoring.navigation")
        .ok_or("missing Inspector knowledge")?;
    assert_eq!(
        knowledge.value,
        if known {
            json!({"status":"known", "value":true})
        } else {
            json!({"status":"unverified"})
        }
    );
    Ok(())
}

fn has_code(response: &Value, code: &str) -> bool {
    response["diagnostics"]
        .as_array()
        .is_some_and(|diagnostics| {
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic["code"] == code)
        })
}

#[test]
fn unknown_navigation_survives_reopening_decisions_and_atomic_repair() -> FlowResult<()> {
    use bellatrix_mde::{
        AssociationKnowledge, NavigationKnowledge,
        uml::{associations, references::ReferenceIndex, root::Model},
    };
    assert!(include_str!("features/mdj_association_lifecycle.feature").contains(
        "Scenario: Author navigation decisions without inventing defaults or losing source spelling"
    ));
    println!("Given absent and explicit unspecified navigation in an MDJ association");
    let source = navigation_source()?;
    let project = import_mdj(&source)?;
    assert_eq!(export_mdj(&project)?, source);
    assert_eq!(
        project.authoring_knowledge.associations.get("association"),
        Some(&AssociationKnowledge {
            navigation_unverified: vec!["left-end".into(), "right-end".into()],
        })
    );
    let references = ReferenceIndex::new(Model::from_project(&project));
    for id in ["left-end", "right-end"] {
        assert_eq!(
            associations::navigation_knowledge(&references, id)?,
            NavigationKnowledge::Unverified
        );
        assert!(associations::is_navigable(&references, id).is_err());
    }
    assert!(associations::navigable_owned_ends(&references, "association").is_err());
    assert_navigation_consumers(&project, false)?;
    let findings = validate(&project)?;
    assert_eq!(findings["validity"], "unverified");
    assert_eq!(findings["has_errors"], true);
    assert_eq!(
        findings["coverage"],
        "implemented_rules_with_incomplete_authoring"
    );
    assert!(has_code(&findings, "UML_MODEL_SEMANTICS_UNVERIFIED"));
    let mut invalid_scalar = project.clone();
    invalid_scalar
        .elements
        .iter_mut()
        .find(|element| element.id == "left")
        .ok_or("missing class")?
        .properties
        .insert("visibility".into(), json!("invalid-visibility"));
    let invalid_findings = validate(&invalid_scalar)?;
    assert_eq!(invalid_findings["validity"], "unverified");
    assert!(
        invalid_findings["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(|diagnostic| diagnostic["subject_id"] == "left"
                && diagnostic["severity"] == "error")
    );
    let saved = SavedFlow::new("navigation")?;
    println!("When I save, close, reopen and make an unrelated name edit");
    let renamed = set_property(&project, "left", "name", json!("RenamedLeft"))?;
    for (name, format) in [
        ("unknown.json", "json"),
        ("unknown.blx", "protobuf"),
    ] {
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
    assert!(decided.authoring_knowledge.is_empty());
    assert_navigation_consumers(&decided, true)?;
    let references = ReferenceIndex::new(Model::from_project(&decided));
    assert!(associations::is_navigable(&references, "left-end")?);
    assert!(!associations::is_navigable(&references, "right-end")?);
    assert!(!has_code(
        &validate(&decided)?,
        "UML_MODEL_SEMANTICS_UNVERIFIED"
    ));
    for (name, format) in [
        ("decided.json", "json"),
        ("decided.blx", "protobuf"),
    ] {
        let reopened = saved.reopen(&decided, name, format)?;
        assert_navigation_export(&reopened, Some("navigable"), "notNavigable")?;
    }
    println!("Then restoring unknown decisions in reverse order retains original source encoding");
    let restored_right = choose_navigation(&decided, "right-end", NavigationKnowledge::Unverified)?;
    let restored = choose_navigation(&restored_right, "left-end", NavigationKnowledge::Unverified)?;
    assert_eq!(restored.authoring_knowledge, renamed.authoring_knowledge);
    assert_navigation_export(&restored, None, "unspecified")?;
    for references in [
        vec!["left-end", "left-end"],
        vec!["missing-end"],
        vec!["left"],
    ] {
        let mut invalid = project.clone();
        invalid.authoring_knowledge.associations.insert(
            "association".into(),
            AssociationKnowledge {
                navigation_unverified: references.into_iter().map(str::to_owned).collect(),
            },
        );
        let reopened = saved.reopen(&invalid, "invalid.json", "json")?;
        assert!(has_code(
            &validate(&reopened)?,
            "MDE_NAVIGATION_KNOWLEDGE_INVALID"
        ));
        assert!(export_mdj(&reopened).is_err());
    }
    let mut conflicting = first_decision.clone();
    conflicting.authoring_knowledge = project.authoring_knowledge.clone();
    assert!(has_code(
        &validate(&conflicting)?,
        "MDE_NAVIGATION_KNOWLEDGE_INVALID"
    ));
    assert!(export_mdj(&conflicting).is_err());
    println!("And ambiguous authoring identities cannot disappear in a document or API envelope");
    let mut document = serde_json::to_value(&project)?;
    document["authoring_knowledge"] = Value::Null;
    let ambiguous = document.to_string().replace(
        "\"authoring_knowledge\":null",
        r#""authoring_knowledge":{"associations":{"association":{"navigation_unverified":["left-end"]},"association":{"navigation_unverified":[]}}}"#,
    );
    let path = saved.0.join("ambiguous.json");
    fs::write(&path, &ambiguous)?;
    assert!(decode_model(&fs::read(path)?, "json").is_err());
    let envelope = format!(r#"{{"api_version":1,"model":{ambiguous}}}"#);
    assert!(validate_uml_model(&envelope).is_err());
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
    native.composition.clear();
    native.mdj_provenance = None;
    let before = encode_model(&native, "json")?;
    assert!(
        apply(
            &native,
            &Command::SetOwner {
                element_id: "left-end".into(),
                owner_id: Some("left".into()),
            },
            1
        )
        .is_err()
    );
    assert_eq!(encode_model(&native, "json")?, before);
    Ok(())
}
