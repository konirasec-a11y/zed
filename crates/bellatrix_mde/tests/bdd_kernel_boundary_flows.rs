use bellatrix_mde::{Format, Project, decode_model_document, encode, validate_uml_model};
use serde_json::{Value, json};
use std::{error::Error, fs, path::PathBuf};

type FlowResult<T> = Result<T, Box<dyn Error>>;

struct SavedFlow(PathBuf);
impl SavedFlow {
    fn new(name: &str) -> FlowResult<Self> {
        let directory = std::env::temp_dir().join(format!(
            "bellatrix-boundary-bdd-{}-{name}",
            std::process::id()
        ));
        fs::create_dir(&directory)?;
        Ok(Self(directory))
    }
}
impl Drop for SavedFlow {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "Could not clean BDD flow directory {}: {error}",
                self.0.display()
            );
        }
    }
}

fn imported_project() -> FlowResult<Value> {
    Ok(serde_json::from_str(
        &decode_model_document(
            include_bytes!("../fixtures/bdd/structural/class.valid.json"),
            "json",
        )
        .map_err(std::io::Error::other)?,
    )?)
}

fn validate(envelope: &Value) -> FlowResult<Value> {
    let response: Value = serde_json::from_str(
        &validate_uml_model(&envelope.to_string()).map_err(std::io::Error::other)?,
    )?;
    assert_eq!(response["api_version"], 1);
    assert_eq!(
        response["has_errors"],
        response["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(|diagnostic| diagnostic["severity"] == "error")
    );
    Ok(response)
}

#[test]
fn uml_validation_survives_malformed_presentation_save_reopen_and_repair() -> FlowResult<()> {
    assert!(
        include_str!("features/kernel_boundary.feature")
            .contains("Scenario: Preserve UML findings across malformed presentation and repair")
    );
    println!("Given an imported, valid authored UML class project");
    let original = imported_project()?;
    let valid = validate(&original)?;
    assert_eq!(valid["has_errors"], false);
    assert_eq!(valid["validity"], "no_violations_found");
    let mut semantic_error = original.clone();
    semantic_error["model"]["elements"][0]["properties"]["visibility"] = json!("not-a-visibility");
    let invalid = validate(&semantic_error)?;
    assert_eq!(invalid["has_errors"], true);
    assert_eq!(invalid["validity"], "invalid");
    let saved = SavedFlow::new("presentation")?;
    for (name, authored, expected) in [
        ("valid", &original, &valid),
        ("invalid", &semantic_error, &invalid),
    ] {
        println!("When presentation is corrupted without changing the {name} semantic graph");
        for (variant, presentation) in [
            (
                "structure",
                json!({"diagrams":{"not":"an array"}, "hierarchy":false,
                "composition":17, "mdj_provenance":["malformed"]}),
            ),
            (
                "views",
                json!({"diagrams":[{"id":"root", "width":-1, "views":[
                {"id":"root", "subject":"missing", "geometry":{"kind":"unknown"},
                 "style":{"opacity":"not numeric"}}]}],
                "hierarchy":{"root":{"parent_id":"root"}},
                "composition":{"absent":{"parent":{"source":"object","id":"missing"},"slot":"bad"}}}),
            ),
        ] {
            let mut corrupted = authored.clone();
            for (field, value) in presentation
                .as_object()
                .ok_or("presentation must be an object")?
            {
                corrupted["model"][field] = value.clone();
            }
            assert_eq!(
                corrupted["model"]["elements"],
                authored["model"]["elements"]
            );
            assert_eq!(
                corrupted["model"]["relations"],
                authored["model"]["relations"]
            );
            assert!(
                decode_model_document(&serde_json::to_vec(&corrupted["model"])?, "json").is_err(),
                "The corruption must really fail the full document contract"
            );
            assert_eq!(&validate(&corrupted)?, expected);
            println!(
                "And I save the supplied envelope, close it and reopen its exact JSON content"
            );
            // The document codec correctly rejects malformed presentation. Persist
            // the request itself so this flow tests the semantic API's own boundary.
            let path = saved.0.join(format!("{name}-{variant}.json"));
            fs::write(&path, serde_json::to_vec(&corrupted)?)?;
            let reopened: Value = serde_json::from_slice(&fs::read(path)?)?;
            assert_eq!(reopened, corrupted);
            assert_eq!(&validate(&reopened)?, expected);
            println!(
                "Then its UML findings are unchanged, and presentation repair restores the original request"
            );
            assert_eq!(&validate(authored)?, expected);
        }
    }
    assert_eq!(
        validate(&original)?,
        valid,
        "Invalid calls must not contaminate repair"
    );
    Ok(())
}

#[test]
fn mixed_domain_project_remains_unverified_after_save_reopen_and_repair() -> FlowResult<()> {
    assert!(include_str!("features/kernel_boundary.feature").contains(
        "Scenario: Keep mixed-domain validation explicitly blocked through reopening and repair"
    ));
    println!("Given an imported UML class project with known accepted semantic findings");
    let original = imported_project()?;
    let valid = validate(&original)?;
    assert_eq!(valid["has_errors"], false);
    println!("When I add Rust and C4 elements and explicit cross-domain references");
    let mut mixed = original.clone();
    mixed["model"]["elements"]
        .as_array_mut()
        .ok_or("missing elements")?
        .extend([
            json!({"id":"external-rust", "kind":"rust.struct", "name":"External Rust",
            "properties":{"isAbstract":"not a UML boolean"}}),
            json!({"id":"external-c4", "kind":"c4.software_system", "name":"External C4"}),
        ]);
    mixed["model"]["relations"]
        .as_array_mut()
        .ok_or("missing relations")?
        .push(json!({"id":"external-owner", "kind":"owns", "from":"root", "to":"external-rust"}));
    mixed["model"]["elements"][0]["properties"]["extensionReferences"] =
        json!([{"$ref":"external-rust"},{"$ref":"external-c4"},{"$ref":"external-rust"}]);
    let expected = validate(&mixed)?;
    assert_eq!(expected["validity"], "unverified");
    assert_eq!(expected["coverage"], "mixed_domain_semantics_unverified");
    assert_eq!(
        expected["has_errors"], true,
        "Error-only consumers must not see success"
    );
    let diagnostics = expected["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?;
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["severity"] == "error")
        .collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0]["code"], "UML_MODEL_SEMANTICS_UNVERIFIED");
    assert_eq!(errors[0]["subject_id"], mixed["model"]["id"]);
    for (code, subject) in [
        ("UML_EXTERNAL_DOMAIN_UNVERIFIED", "external-rust"),
        ("UML_EXTERNAL_DOMAIN_UNVERIFIED", "external-c4"),
        ("UML_EXTERNAL_REFERENCE_UNVERIFIED", "external-owner"),
        ("UML_EXTERNAL_REFERENCE_UNVERIFIED", "root"),
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic["code"] == code
                    && diagnostic["subject_id"] == subject
                    && diagnostic["severity"] == "warning"),
            "{expected}"
        );
    }
    let project: Project = serde_json::from_value(mixed["model"].clone())?;
    let saved = SavedFlow::new("mixed-domain")?;
    for (format, name) in [(Format::Json, "json"), (Format::Protobuf, "protobuf")] {
        println!("And I persist, close and reopen the mixed-domain project as {name}");
        let path = saved.0.join(name);
        fs::write(&path, encode(&project, format)?)?;
        let reopened: Value = serde_json::from_str(
            &decode_model_document(&fs::read(path)?, name).map_err(std::io::Error::other)?,
        )?;
        assert_eq!(
            reopened["model"],
            serde_json::to_value(&project)?,
            "External objects, references and repeated collection entries must survive"
        );
        assert_eq!(validate(&reopened)?, expected);
    }
    println!("Then evaluation remains explicitly unverified, never a UML validity verdict");
    let mut independently_invalid = mixed.clone();
    independently_invalid["model"]["elements"][0]["properties"]["visibility"] = json!("invalid");
    let blocked = validate(&independently_invalid)?;
    assert_eq!(blocked["validity"], "unverified");
    assert_eq!(blocked["has_errors"], true);
    println!("When I restore the original pure UML graph, its earlier findings return unchanged");
    assert_eq!(validate(&original)?, valid);
    assert_eq!(validate(&mixed)?, expected);
    Ok(())
}
