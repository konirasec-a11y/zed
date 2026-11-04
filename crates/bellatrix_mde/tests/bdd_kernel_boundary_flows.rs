use bellatrix_mde::{decode_model_document, encode_model_document, validate_uml_model};
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
    assert_eq!(
        expected["coverage"],
        "implemented_uml_scalars_and_integrity_only"
    );
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
    let canonical: Value = serde_json::from_str(&decode_model_document(
        &serde_json::to_vec(&mixed["model"])?, "json").map_err(std::io::Error::other)?)?;
    let saved = SavedFlow::new("mixed-domain")?;
    for name in ["json", "protobuf"] {
        println!("And I persist, close and reopen the mixed-domain project as {name}");
        let path = saved.0.join(name);
        fs::write(&path, encode_model_document(&canonical.to_string(), name).map_err(std::io::Error::other)?)?;
        let reopened: Value = serde_json::from_str(
            &decode_model_document(&fs::read(path)?, name).map_err(std::io::Error::other)?,
        )?;
        assert_eq!(
            reopened["model"],
            canonical["model"],
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
    assert!(
        blocked["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(|diagnostic| diagnostic["code"] == "UML_PROPERTY_VALUE"
                && diagnostic["subject_id"] == "root"
                && diagnostic["severity"] == "error")
    );
    println!("When I restore the original pure UML graph, its earlier findings return unchanged");
    assert_eq!(validate(&original)?, valid);
    assert_eq!(validate(&mixed)?, expected);
    Ok(())
}

fn reopen_mixed_flow(name: &str, envelope: &Value, expected: &Value) -> FlowResult<()> {
    let canonical: Value = serde_json::from_str(&decode_model_document(
        &serde_json::to_vec(&envelope["model"])?, "json").map_err(std::io::Error::other)?)?;
    let saved = SavedFlow::new(name)?;
    for name in ["json", "protobuf"] {
        println!("And I save, close and reopen this authored state using {name}");
        let path = saved.0.join(name);
        fs::write(&path, encode_model_document(&canonical.to_string(), name).map_err(std::io::Error::other)?)?;
        let reopened: Value = serde_json::from_str(
            &decode_model_document(&fs::read(path)?, name).map_err(std::io::Error::other)?,
        )?;
        assert_eq!(reopened["model"], canonical["model"]);
        assert_eq!(&validate(&reopened)?, expected);
    }
    Ok(())
}

fn model_element_mut<'a>(envelope: &'a mut Value, identifier: &str) -> FlowResult<&'a mut Value> {
    envelope["model"]["elements"]
        .as_array_mut()
        .ok_or("missing elements")?
        .iter_mut()
        .find(|element| element["id"] == identifier)
        .ok_or_else(|| format!("missing element {identifier}").into())
}

#[test]
fn closed_uml_subset_reports_namespace_errors_through_mixed_project_repair() -> FlowResult<()> {
    assert!(
        include_str!("features/kernel_boundary.feature")
            .contains("Scenario: Validate the closed UML graph alongside unmapped domains")
    );
    println!("Given a valid imported UML class project with an unrelated Rust object");
    let original = imported_project()?;
    let mut mixed = original.clone();
    mixed["model"]["elements"]
        .as_array_mut()
        .ok_or("missing elements")?
        .push(
            json!({"id":"external", "kind":"rust.struct", "name":"Customer",
            "properties":{"isAbstract":"not a UML Boolean"}}),
        );
    let accepted_subset = validate(&mixed)?;
    assert_eq!(
        accepted_subset["coverage"],
        "implemented_uml_subset_rules_only"
    );
    assert_eq!(accepted_subset["validity"], "unverified");
    assert_eq!(accepted_subset["has_errors"], true);
    let errors: Vec<_> = accepted_subset["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|diagnostic| diagnostic["severity"] == "error")
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "External Rust properties must not be evaluated as UML: {accepted_subset}"
    );
    assert_eq!(errors[0]["code"], "UML_MODEL_SEMANTICS_UNVERIFIED");
    reopen_mixed_flow("closed-subset", &mixed, &accepted_subset)?;

    println!("When I introduce two indistinguishable UML members in the same namespace");
    let mut invalid_pure = original.clone();
    model_element_mut(&mut invalid_pure, "order")?["name"] = json!("Customer");
    let pure_errors = validate(&invalid_pure)?;
    assert!(
        pure_errors["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(
                |diagnostic| diagnostic["code"] == "UML_NAMESPACE_MEMBERS_DISTINGUISHABLE"
                    && diagnostic["severity"] == "error"
            ),
        "{pure_errors}"
    );
    let mut invalid_mixed = mixed.clone();
    model_element_mut(&mut invalid_mixed, "order")?["name"] = json!("Customer");
    let partial = validate(&invalid_mixed)?;
    assert_eq!(partial["coverage"], "implemented_uml_subset_rules_only");
    assert_eq!(partial["validity"], "unverified");
    let semantic_diagnostics: Vec<_> = partial["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|diagnostic| {
            !matches!(
                diagnostic["code"].as_str(),
                Some("UML_MODEL_SEMANTICS_UNVERIFIED" | "UML_EXTERNAL_DOMAIN_UNVERIFIED")
            )
        })
        .cloned()
        .collect();
    assert_eq!(
        json!(semantic_diagnostics),
        pure_errors["diagnostics"],
        "The complete UML graph must retain the same global namespace findings"
    );
    reopen_mixed_flow("closed-invalid-subset", &invalid_mixed, &partial)?;
    println!("Then repair restores the earlier UML findings without claiming external validity");
    assert_eq!(validate(&mixed)?, accepted_subset);
    assert_eq!(validate(&invalid_mixed)?, partial);
    Ok(())
}

#[test]
fn cross_domain_namespace_and_type_references_preserve_provable_scalar_errors() -> FlowResult<()> {
    assert!(include_str!("features/kernel_boundary.feature").contains(
        "Scenario: Suspend global rules at external references while retaining proved UML errors"
    ));
    println!("Given an imported UML graph with external Type and namespace references");
    let original = imported_project()?;
    let mut cross_domain = original.clone();
    cross_domain["model"]["elements"]
        .as_array_mut()
        .ok_or("missing elements")?
        .extend([
            json!({"id":"external-type", "kind":"rust.struct", "name":"Price"}),
            json!({"id":"external-namespace", "kind":"c4.software_system", "name":"Orders"}),
        ]);
    model_element_mut(&mut cross_domain, "price")?["properties"]["type"] =
        json!({"$ref":"external-type"});
    cross_domain["model"]["relations"].as_array_mut().ok_or("missing relations")?.push(
        json!({"id":"external-import", "kind":"uml.package_import", "from":"domain", "to":"external-namespace"}));
    let unknown = validate(&cross_domain)?;
    assert_eq!(
        unknown["coverage"],
        "implemented_uml_scalars_and_integrity_only"
    );
    assert_eq!(unknown["validity"], "unverified");
    let errors: Vec<_> = unknown["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|diagnostic| diagnostic["severity"] == "error")
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "External references must not become false missing/type/namespace errors: {unknown}"
    );
    assert_eq!(errors[0]["code"], "UML_MODEL_SEMANTICS_UNVERIFIED");
    for subject in ["price", "external-import"] {
        assert!(
            unknown["diagnostics"]
                .as_array()
                .ok_or("missing diagnostics")?
                .iter()
                .any(
                    |diagnostic| diagnostic["code"] == "UML_EXTERNAL_REFERENCE_UNVERIFIED"
                        && diagnostic["subject_id"] == subject
                ),
            "{unknown}"
        );
    }
    reopen_mixed_flow("cross-domain", &cross_domain, &unknown)?;

    println!("When I introduce an independently invalid Boolean on a UML Class");
    let mut invalid = cross_domain.clone();
    model_element_mut(&mut invalid, "customer")?["properties"]["isAbstract"] = json!("yes");
    let proved = validate(&invalid)?;
    assert_eq!(proved["validity"], "unverified");
    assert_eq!(proved["has_errors"], true);
    let errors: Vec<_> = proved["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|diagnostic| diagnostic["severity"] == "error")
        .collect();
    assert_eq!(
        errors.len(),
        2,
        "Only capacity and the proved scalar violation are errors: {proved}"
    );
    assert!(
        errors
            .iter()
            .any(|diagnostic| diagnostic["code"] == "UML_PROPERTY_VALUE"
                && diagnostic["subject_id"] == "customer")
    );
    reopen_mixed_flow("cross-domain-proved-error", &invalid, &proved)?;
    println!("Then scalar repair removes only the proved violation, not the unverified boundary");
    assert_eq!(validate(&cross_domain)?, unknown);

    println!("When only an external object's reverse reference reaches the UML namespace");
    let mut reverse = original.clone();
    reverse["model"]["elements"]
        .as_array_mut()
        .ok_or("missing elements")?
        .push(
            json!({"id":"external-reverse", "kind":"rust.struct", "name":"Unmapped",
            "properties":{"scope":{"$ref":"domain"}}}),
        );
    let reverse_response = validate(&reverse)?;
    assert_eq!(
        reverse_response["coverage"],
        "implemented_uml_scalars_and_integrity_only"
    );
    assert_eq!(reverse_response["validity"], "unverified");
    assert!(
        reverse_response["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(
                |diagnostic| diagnostic["code"] == "UML_EXTERNAL_REFERENCE_UNVERIFIED"
                    && diagnostic["subject_id"] == "external-reverse"
            )
    );
    reopen_mixed_flow("reverse-cross-domain", &reverse, &reverse_response)?;
    println!("And the reverse reference remains a boundary on external ownership metadata");
    model_element_mut(&mut reverse, "external-reverse")?["properties"] = json!({});
    reverse["model"]["elements"]
        .as_array_mut()
        .ok_or("missing elements")?
        .push(json!({"id":"external-container", "kind":"c4.software_system", "name":"Container"}));
    reverse["model"]["relations"]
        .as_array_mut()
        .ok_or("missing relations")?
        .push(
            json!({"id":"external-ownership", "kind":"owns", "from":"external-container",
            "to":"external-reverse", "properties":{"scope":{"$ref":"domain"}}}),
        );
    let ownership_boundary = validate(&reverse)?;
    assert_eq!(
        ownership_boundary["coverage"],
        "implemented_uml_scalars_and_integrity_only"
    );
    assert_eq!(ownership_boundary["validity"], "unverified");
    assert!(
        ownership_boundary["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(
                |diagnostic| diagnostic["code"] == "UML_EXTERNAL_REFERENCE_UNVERIFIED"
                    && diagnostic["subject_id"] == "external-ownership"
            )
    );
    reopen_mixed_flow("reverse-ownership-boundary", &reverse, &ownership_boundary)?;
    println!(
        "Then reverse dependencies also suspend global absence queries, and pure repair is repeatable"
    );
    assert_eq!(validate(&original)?["validity"], "no_violations_found");
    assert_eq!(validate(&invalid)?, proved);
    Ok(())
}

#[test]
fn cross_domain_ownership_retains_multiple_owner_and_cycle_findings_through_repair()
-> FlowResult<()> {
    assert!(include_str!("features/kernel_boundary.feature").contains(
        "Scenario: Preserve canonical ownership integrity across opaque domain boundaries"
    ));
    println!("Given a valid UML project alongside two opaque external objects");
    let original = imported_project()?;
    let mut mixed = original.clone();
    mixed["model"]["elements"]
        .as_array_mut()
        .ok_or("missing elements")?
        .extend([
            json!({"id":"external-a", "kind":"rust.struct", "name":"A"}),
            json!({"id":"external-b", "kind":"c4.software_system", "name":"B"}),
        ]);
    let baseline = validate(&mixed)?;
    assert_eq!(baseline["coverage"], "implemented_uml_subset_rules_only");
    assert_eq!(baseline["validity"], "unverified");
    let integrity_error = |response: &Value, message: &str| -> FlowResult<()> {
        assert_eq!(response["validity"], "unverified");
        assert_eq!(response["has_errors"], true);
        let errors: Vec<_> = response["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .filter(|diagnostic| diagnostic["severity"] == "error")
            .collect();
        assert_eq!(
            errors.len(),
            2,
            "Only capacity and the proved ownership violation are errors: {response}"
        );
        assert!(
            errors
                .iter()
                .any(|diagnostic| diagnostic["code"] == "GRAPH_INTEGRITY"
                    && diagnostic["subject_id"] == "class-project"
                    && diagnostic["message"] == message),
            "{response}"
        );
        Ok(())
    };
    println!("When a UML Class already owned by its Package also receives an external owner");
    let mut multiple = mixed.clone();
    multiple["model"]["relations"].as_array_mut().ok_or("missing relations")?.push(
        json!({"id":"external-second-owner", "kind":"owns", "from":"external-a", "to":"customer"}));
    let multiple_result = validate(&multiple)?;
    integrity_error(
        &multiple_result,
        "element customer has more than one semantic owner",
    )?;
    reopen_mixed_flow("mixed-multiple-owners", &multiple, &multiple_result)?;
    println!("Then removing only the second owner repairs the proved violation");
    multiple["model"]["relations"]
        .as_array_mut()
        .ok_or("missing relations")?
        .retain(|relation| relation["id"] != "external-second-owner");
    assert_eq!(multiple, mixed);
    assert_eq!(validate(&multiple)?, baseline);

    println!("When an ownership path returns to the UML Class through two external identities");
    let mut cyclic = mixed.clone();
    let relations = cyclic["model"]["relations"]
        .as_array_mut()
        .ok_or("missing relations")?;
    relations.retain(|relation| !(relation["kind"] == "owns" && relation["to"] == "customer"));
    relations.extend([
        json!({"id":"uml-to-external", "kind":"owns", "from":"customer", "to":"external-a"}),
        json!({"id":"external-middle", "kind":"owns", "from":"external-a", "to":"external-b"}),
        json!({"id":"external-to-uml", "kind":"owns", "from":"external-b", "to":"customer"}),
    ]);
    let cyclic_result = validate(&cyclic)?;
    integrity_error(&cyclic_result, "cycle in owns hierarchy")?;
    reopen_mixed_flow("mixed-ownership-cycle", &cyclic, &cyclic_result)?;
    println!(
        "Then restoring the authored ownership edges repairs the cycle without mapping external types"
    );
    cyclic["model"]["relations"] = mixed["model"]["relations"].clone();
    assert_eq!(cyclic, mixed);
    assert_eq!(validate(&cyclic)?, baseline);

    println!("When an ownership cycle remains entirely within a disconnected external domain");
    let mut isolated = mixed.clone();
    isolated["model"]["relations"]
        .as_array_mut()
        .ok_or("missing relations")?
        .extend([
            json!({"id":"isolated-forward", "kind":"owns", "from":"external-a", "to":"external-b"}),
            json!({"id":"isolated-reverse", "kind":"owns", "from":"external-b", "to":"external-a"}),
        ]);
    let isolated_result = validate(&isolated)?;
    assert_eq!(
        isolated_result["coverage"],
        "implemented_uml_subset_rules_only"
    );
    assert_eq!(isolated_result["validity"], "unverified");
    assert!(
        !isolated_result["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(|diagnostic| diagnostic["code"] == "GRAPH_INTEGRITY"),
        "{isolated_result}"
    );
    reopen_mixed_flow("isolated-external-cycle", &isolated, &isolated_result)?;
    println!("When an unrelated cross-domain Type reference forces partial integrity evaluation");
    let mut partial_isolated = isolated.clone();
    partial_isolated["model"]["elements"]
        .as_array_mut()
        .ok_or("missing elements")?
        .push(json!({"id":"external-type-only", "kind":"rust.struct", "name":"Opaque Type"}));
    model_element_mut(&mut partial_isolated, "price")?["properties"]["type"] =
        json!({"$ref":"external-type-only"});
    let partial_isolated_result = validate(&partial_isolated)?;
    assert_eq!(
        partial_isolated_result["coverage"],
        "implemented_uml_scalars_and_integrity_only"
    );
    assert_eq!(partial_isolated_result["validity"], "unverified");
    assert_eq!(partial_isolated_result["has_errors"], true);
    let errors: Vec<_> = partial_isolated_result["diagnostics"]
        .as_array()
        .ok_or("missing diagnostics")?
        .iter()
        .filter(|diagnostic| diagnostic["severity"] == "error")
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "Partial ownership checks must still exclude the disconnected external cycle: {partial_isolated_result}"
    );
    assert_eq!(errors[0]["code"], "UML_MODEL_SEMANTICS_UNVERIFIED");
    assert!(
        partial_isolated_result["diagnostics"]
            .as_array()
            .ok_or("missing diagnostics")?
            .iter()
            .any(
                |diagnostic| diagnostic["code"] == "UML_EXTERNAL_REFERENCE_UNVERIFIED"
                    && diagnostic["subject_id"] == "price"
            )
    );
    reopen_mixed_flow(
        "partial-isolated-external-cycle",
        &partial_isolated,
        &partial_isolated_result,
    )?;
    assert_eq!(validate(&isolated)?, isolated_result);
    println!(
        "Then the external-only cycle is not claimed as a UML violation, and prior findings repeat"
    );
    assert_eq!(validate(&mixed)?, baseline);
    assert_eq!(validate(&original)?["validity"], "no_violations_found");
    Ok(())
}
