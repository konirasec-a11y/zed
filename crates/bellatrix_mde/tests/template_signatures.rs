use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, project_model_section,
    validate_uml_model,
};
use bellatrix_mde_protocol::{
    command::*,
    projection::ModelingProjection,
    trace::{DiagnosticSeverity, ModelingDiagnostic},
};
use serde_json::{Value, json};

type FlowResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Debug)]
struct Document {
    model: Value,
    read: ModelingProjection,
}
impl std::ops::Deref for Document {
    type Target = ModelingProjection;
    fn deref(&self) -> &Self::Target {
        &self.read
    }
}
impl serde::Serialize for Document {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&self.model, serializer)
    }
}
impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.model == other.model
    }
}
fn parts(model: &Value) -> FlowResult<(String, String, Value)> {
    let id = model["id"]
        .as_str()
        .ok_or("project identity missing")?
        .to_owned();
    let name = model["name"]
        .as_str()
        .ok_or("project name missing")?
        .to_owned();
    let mut section = model.as_object().ok_or("project missing")?.clone();
    for key in ["id", "name", "schema_version"] {
        section.remove(key);
    }
    Ok((id, name, Value::Object(section)))
}
fn decode(bytes: &[u8], format: &str) -> FlowResult<Document> {
    let response: Value = serde_json::from_str(&decode_model_document(bytes, format)?)?;
    let model = response["model"].clone();
    let (id, name, section) = parts(&model)?;
    let read = project_model_section(&id, &name, &section.to_string())?;
    Ok(Document { model, read })
}
fn encode(project: &Document, format: &str) -> FlowResult<Vec<u8>> {
    let (id, name, section) = parts(&project.model)?;
    Ok(encode_model_section(
        &id,
        &name,
        &section.to_string(),
        format,
    )?)
}
fn apply(project: &Document, command: &Command, revision: u64) -> Result<Document, String> {
    apply_candidate(project, command, revision).map_err(|error| error.to_string())
}
fn apply_candidate(project: &Document, command: &Command, revision: u64) -> FlowResult<Document> {
    let (id, name, section) = parts(&project.model)?;
    let updated = apply_model_command(
        &id,
        &name,
        &section.to_string(),
        revision,
        command.clone(),
    )?;
    decode(
        &encode_model_section(&id, &name, &updated.to_string(), "json")?,
        "json",
    )
}
fn model_diagnostics(project: &Document) -> Vec<ModelingDiagnostic> {
    let response = validate_uml_model(&json!({"api_version":1,"model":project.model}).to_string())
        .expect("public UML validation");
    let response: Value = serde_json::from_str(&response).expect("validation response");
    serde_json::from_value(response["diagnostics"].clone()).expect("public diagnostics")
}
fn author(project: &mut Document, edit: impl FnOnce(&mut Value)) {
    let mut model = project.model.clone();
    edit(&mut model);
    *project = decode(&serde_json::to_vec(&model).expect("authored bytes"), "json")
        .expect("authored document");
}
fn inspected(project: &Document, id: &str, path: &str) -> Value {
    project
        .inspections
        .get(id)
        .expect("public inspector")
        .properties
        .iter()
        .find(|field| field.path == path)
        .expect("public property")
        .value
        .clone()
}
fn inspect_element(
    project: &Document,
    id: &str,
) -> Option<bellatrix_mde_protocol::inspector::ElementInspector> {
    project.inspections.get(id).cloned()
}
fn raw_element<'a>(project: &'a Document, id: &str) -> &'a Value {
    project.model["elements"]
        .as_array()
        .expect("elements")
        .iter()
        .find(|element| element["id"] == id)
        .expect("element")
}
fn project(kind: &str) -> Document {
    decode(&serde_json::to_vec(&serde_json::json!({"id":"templates", "name":"Templates", "elements":[{"id":"owner", "name":"Owner", "kind":kind}], "diagrams":[{"id":"diagram", "name":"D", "uml_di_type":"UMLClassDiagram", "views":[{"id":"view", "subject":"owner", "geometry":{"kind":"rect", "x":0.0,"y":0.0,"width":180.0,"height":120.0}}]}]})).expect("fixture bytes"), "json").expect("fixture")
}
fn add(project: &Document, revision: u64) -> Result<Document, String> {
    apply(
        project,
        &Command::QuickEdit {
            diagram_id: "diagram".into(),
            element_id: "owner".into(),
            action: UmlQuickCreate::TemplateParameter,
        },
        revision,
    )
}
fn owns(project: &Document, owner: &str, child: &str) -> bool {
    project
        .relations
        .iter()
        .any(|relation| relation.kind == "owns" && relation.from == owner && relation.to == child)
}

#[test]
fn template_creation_builds_signature_parameter_and_parametered_classifier() {
    for (kind, signature_kind) in [
        ("uml.class", "uml.redefinable_template_signature"),
        ("uml.component", "uml.redefinable_template_signature"),
        ("uml.artifact", "uml.redefinable_template_signature"),
        ("uml.interface", "uml.redefinable_template_signature"),
        ("uml.package", "uml.template_signature"),
        ("uml.model", "uml.template_signature"),
        ("uml.operation", "uml.template_signature"),
    ] {
        let original = project(kind);
        let first = add(&original, 0).expect(kind);
        let signature = first
            .elements
            .iter()
            .find(|element| element.kind == signature_kind)
            .expect("signature");
        let parameter = first
            .elements
            .iter()
            .find(|element| element.kind == "uml.classifier_template_parameter")
            .expect("parameter");
        assert!(owns(&first, "owner", &signature.id));
        assert!(owns(&first, &signature.id, &parameter.id));
        assert!(!owns(&first, "owner", &parameter.id));
        let reference = first
            .relations
            .iter()
            .find(|relation| {
                relation.kind == "uml.parametered_element" && relation.from == parameter.id
            })
            .expect("parameteredElement");
        let classifier = first
            .elements
            .iter()
            .find(|element| element.id == reference.to)
            .expect("parametered classifier");
        assert_eq!(classifier.kind, "uml.class");
        assert!(owns(&first, &parameter.id, &classifier.id));
        assert_eq!(
            inspected(&first, &classifier.id, "uml.namespace"),
            json!("owner")
        );
        assert_eq!(
            inspected(&first, &classifier.id, "uml.qualifiedName"),
            json!(format!("Owner::{}", classifier.name))
        );
        assert!(
            !first
                .relations
                .iter()
                .any(|relation| relation.kind == "owns" && relation.from == classifier.id)
        );
        assert_eq!(
            raw_element(&first, &parameter.id)["properties"].get("allowSubstitutable"),
            Some(&serde_json::json!(false))
        );
        assert!(
            model_diagnostics(&first)
                .iter()
                .all(|issue| !issue.code.starts_with("UML_TEMPLATE")),
            "{:?}",
            model_diagnostics(&first)
        );
        let second = add(&first, 1).expect("second parameter");
        assert_eq!(
            second
                .elements
                .iter()
                .filter(|element| element.kind == signature_kind)
                .count(),
            1
        );
        let section = &second;
        assert_eq!(
            inspect_element(&section, "owner")
                .expect("inspector")
                .template_parameters
                .len(),
            2
        );
        assert_eq!(
            section
                .elements
                .iter()
                .find(|element| element.id == "owner")
                .expect("owner")
                .template_parameters
                .len(),
            2
        );
        assert_eq!(
            decode(&encode(&second, "json").expect("serialize"), "json").expect("deserialize"),
            second
        );
        assert_eq!(original, project(kind));
    }
    for kind in ["uml.attribute", "uml.port", "uml.note"] {
        assert!(add(&project(kind), 0).is_err(), "{kind}");
    }
}

#[test]
fn removing_parameters_cleans_only_the_owned_generated_template_graph() {
    let first = add(&project("uml.component"), 0).expect("template");
    let second = add(&first, 1).expect("second parameter");
    let parameters = second
        .elements
        .iter()
        .filter(|element| element.kind == "uml.classifier_template_parameter")
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();
    let after_one = apply(
        &second,
        &Command::DeleteElement {
            element_id: parameters[0].clone(),
        },
        2,
    )
    .expect("delete first");
    assert_eq!(
        after_one
            .elements
            .iter()
            .filter(|element| element.kind == "uml.redefinable_template_signature")
            .count(),
        1
    );
    let renamed_owner = apply(
        &after_one,
        &Command::Rename {
            element_id: "owner".into(),
            name: "Renamed".into(),
        },
        3,
    )
    .expect("rename owner");
    let after_last = apply(
        &renamed_owner,
        &Command::DeleteElement {
            element_id: parameters[1].clone(),
        },
        3,
    )
    .expect("delete last");
    assert_eq!(after_last.elements.len(), 1);
    assert!(after_last.relations.is_empty());
    assert_eq!(after_last.elements[0].id, "owner");
    let exposed = first
        .relations
        .iter()
        .find(|relation| relation.kind == "uml.parametered_element")
        .expect("reference");
    assert!(
        apply(
            &first,
            &Command::DeleteElement {
                element_id: exposed.to.clone()
            },
            4
        )
        .is_err()
    );
}

#[test]
fn legacy_direct_parameters_remain_visible_and_diagnosed_without_migration() {
    let mut original = project("uml.class");
    author(&mut original, |raw| {
        raw["elements"].as_array_mut().expect("elements").push(
            serde_json::from_value(
                serde_json::json!({"id":"legacy", "name":"T", "kind":"uml.template_parameter"}),
            )
            .expect("legacy"),
        );
    });
    author(&mut original, |raw| {
        raw["relations"].as_array_mut().expect("relations").push(
        serde_json::from_value(
            serde_json::json!({"id":"legacy-owner", "kind":"owns", "from":"owner", "to":"legacy"}),
        )
        .expect("ownership"),
    );
    });
    let diagnostics = model_diagnostics(&original);
    assert!(
        diagnostics
            .iter()
            .any(|issue| issue.code == "UML_TEMPLATE_PARAMETER_SIGNATURE")
    );
    let updated = add(&original, 0).expect("add without rewriting legacy data");
    assert_eq!(
        raw_element(&updated, "legacy"),
        raw_element(&original, "legacy")
    );
    assert!(owns(&updated, "owner", "legacy"));
    assert_eq!(
        inspect_element(&updated, "owner")
            .expect("inspector")
            .template_parameters
            .len(),
        2
    );
}

#[test]
fn template_reference_and_feature_constraints_reject_edits_atomically() {
    let original = add(&project("uml.component"), 0).expect("template");
    let reference = original
        .relations
        .iter()
        .find(|relation| relation.kind == "uml.parametered_element")
        .expect("reference");
    let before = original.clone();
    let feature = apply(
        &original,
        &Command::AddMember {
            owner_id: reference.to.clone(),
            kind: ModelingMemberKind::Attribute,
        },
        1,
    );
    assert!(feature.is_err_and(|error| error.contains("UML_TEMPLATE_PARAMETERED_FEATURES")));
    let substitutable = apply(
        &original,
        &Command::SetProperty {
            element_id: reference.from.clone(),
            path: "properties.allowSubstitutable".into(),
            value: serde_json::json!(true),
        },
        1,
    );
    assert!(substitutable.is_err_and(|error| error.contains("UML_TEMPLATE_PARAMETER_CONSTRAINT")));
    assert_eq!(original, before);
    for (mutation, code) in [
        ("reference", "UML_TEMPLATE_PARAMETERED_ELEMENT"),
        ("type", "UML_TEMPLATE_PARAMETERED_TYPE"),
        ("owner", "UML_TEMPLATE_PARAMETERED_OWNERSHIP"),
        ("signature", "UML_TEMPLATE_CLASSIFIER_SIGNATURE"),
    ] {
        let mut malformed = original.clone();
        author(&mut malformed, |raw| match mutation {
            "reference" => raw["relations"]
                .as_array_mut()
                .expect("relations")
                .retain(|relation| relation["kind"] != "uml.parametered_element"),
            "type" => {
                raw["elements"]
                    .as_array_mut()
                    .expect("elements")
                    .iter_mut()
                    .find(|element| element["id"] == reference.to)
                    .expect("target")["kind"] = json!("uml.package")
            }
            "owner" => raw["relations"]
                .as_array_mut()
                .expect("relations")
                .retain(|relation| !(relation["kind"] == "owns" && relation["to"] == reference.to)),
            "signature" => {
                raw["elements"]
                    .as_array_mut()
                    .expect("elements")
                    .iter_mut()
                    .find(|element| element["kind"] == "uml.redefinable_template_signature")
                    .expect("signature")["kind"] = json!("uml.template_signature")
            }
            _ => unreachable!(),
        });
        assert!(
            model_diagnostics(&malformed)
                .iter()
                .any(|issue| issue.code == code),
            "{mutation}"
        );
    }
}

#[test]
fn parameter_renames_target_the_exposed_classifier_and_reject_duplicate_names() {
    let original = add(&add(&project("uml.class"), 0).expect("first"), 1).expect("second");
    let parameters = original
        .elements
        .iter()
        .filter(|element| element.kind == "uml.classifier_template_parameter")
        .collect::<Vec<_>>();
    let renamed = apply(
        &original,
        &Command::Rename {
            element_id: parameters[0].id.clone(),
            name: "Key".into(),
        },
        2,
    )
    .expect("rename");
    let reference = renamed
        .relations
        .iter()
        .find(|relation| {
            relation.kind == "uml.parametered_element" && relation.from == parameters[0].id
        })
        .expect("reference");
    assert_eq!(
        renamed
            .elements
            .iter()
            .find(|element| element.id == reference.to)
            .expect("target")
            .name,
        "Key"
    );
    let inspector = inspect_element(&renamed, "owner").expect("inspector");
    assert!(
        inspector
            .template_parameters
            .iter()
            .any(|parameter| parameter.display_label == "Key")
    );
    let duplicate = apply(
        &renamed,
        &Command::SetProperty {
            element_id: parameters[1].id.clone(),
            path: "name".into(),
            value: serde_json::json!("Key"),
        },
        3,
    );
    assert!(duplicate.is_err_and(|error| error.contains("UML_TEMPLATE_PARAMETER_NAMES")));
}

#[test]
fn signature_cleanup_does_not_delete_external_references_or_user_signatures() {
    let generated = add(&project("uml.class"), 0).expect("template");
    let parameter = generated
        .elements
        .iter()
        .find(|element| element.kind == "uml.classifier_template_parameter")
        .expect("parameter");
    let signature = generated
        .elements
        .iter()
        .find(|element| element.kind == "uml.redefinable_template_signature")
        .expect("signature");
    let mut referenced = generated.clone();
    author(&mut referenced, |raw| {
        raw["elements"].as_array_mut().expect("elements").push(
            serde_json::from_value(
                serde_json::json!({"id":"client","name":"Client","kind":"uml.class"}),
            )
            .expect("client"),
        );
    });
    author(&mut referenced, |raw| {
        raw["relations"].as_array_mut().expect("relations").push(serde_json::from_value(serde_json::json!({"id":"binding","kind":"uml.template_binding","from":"client","to":signature.id})).expect("reference"));
    });
    let before = referenced.clone();
    assert!(
        apply(
            &referenced,
            &Command::DeleteElement {
                element_id: parameter.id.clone()
            },
            1
        )
        .is_err_and(|error| error.contains("UML_TEMPLATE_IN_USE"))
    );
    assert_eq!(referenced, before);
    let mut annotated = generated.clone();
    author(&mut annotated, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|element| element["id"] == signature.id)
            .expect("signature")["description"] = json!("User-authored documentation");
    });
    assert!(
        apply(
            &annotated,
            &Command::DeleteElement {
                element_id: parameter.id.clone()
            },
            1
        )
        .is_err()
    );
    let mut manual = generated.clone();
    author(&mut manual, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|element| element["id"] == signature.id)
            .expect("signature")["properties"]
            .as_object_mut()
            .expect("properties")
            .clear();
    });
    assert!(
        apply(
            &manual,
            &Command::DeleteElement {
                element_id: parameter.id.clone()
            },
            1
        )
        .is_err_and(|error| error.contains("UML_TEMPLATE_SIGNATURE_PARAMETERS"))
    );
    let removed = apply(
        &manual,
        &Command::DeleteElement {
            element_id: signature.id.clone(),
        },
        1,
    )
    .expect("explicit signature delete");
    assert_eq!(removed.elements.len(), 1);
}

#[test]
fn semantic_templates_remain_view_free_through_reopening() {
    let mut original = project("uml.operation");
    author(&mut original, |raw| {
        raw["diagrams"][0]["views"]
            .as_array_mut()
            .expect("views")
            .clear()
    });
    let updated = add(&original, 0).expect("semantic operation template");
    assert!(updated.diagrams[0].views.is_empty());
    assert!(
        updated
            .elements
            .iter()
            .find(|element| element.id == "owner")
            .expect("owner")
            .quick_actions
            .iter()
            .any(|action| action.id == "uml.add-template-parameter")
    );
    let section = &updated;
    assert_eq!(
        section
            .elements
            .iter()
            .find(|element| element.id == "owner")
            .expect("owner")
            .template_parameters
            .len(),
        1
    );
    for format in ["json", "protobuf"] {
        let reopened = decode(
            &encode(&updated, format).expect("save semantic template"),
            format,
        )
        .expect("reopen semantic template");
        assert_eq!(reopened, updated);
        assert_eq!(
            reopened
                .elements
                .iter()
                .find(|element| element.id == "owner")
                .expect("owner")
                .template_parameters
                .len(),
            1
        );
    }
}

#[test]
fn qa_rejects_transitive_only_parameter_ownership_and_external_rename() {
    let original = add(&project("uml.class"), 0).expect("template");
    let reference = original
        .relations
        .iter()
        .find(|relation| relation.kind == "uml.parametered_element")
        .expect("reference");
    let mut nested = original.clone();
    author(&mut nested, |raw| {
        raw["elements"].as_array_mut().expect("elements").push(
            serde_json::from_value(
                serde_json::json!({"id":"nested","kind":"uml.class","name":"Nested"}),
            )
            .expect("nested"),
        );
    });
    author(&mut nested, |raw| {
        raw["relations"]
            .as_array_mut()
            .expect("relations")
            .iter_mut()
            .find(|relation| relation["kind"] == "owns" && relation["to"] == reference.to)
            .expect("ownership")["from"] = "nested".into();
    });
    author(&mut nested, |raw| {
        raw["relations"].as_array_mut().expect("relations").push(serde_json::from_value(serde_json::json!({"id":"nested-owner","kind":"owns","from":reference.from,"to":"nested"})).expect("nested owner"));
    });
    assert!(
        model_diagnostics(&nested)
            .iter()
            .any(|issue| issue.code == "UML_TEMPLATE_PARAMETERED_OWNERSHIP")
    );
    let mut external = original.clone();
    author(&mut external, |raw| {
        raw["relations"]
            .as_array_mut()
            .expect("relations")
            .retain(|relation| !(relation["kind"] == "owns" && relation["to"] == reference.to));
    });
    let before = external.clone();
    let result = apply(
        &external,
        &Command::Rename {
            element_id: reference.from.clone(),
            name: "Hijacked".into(),
        },
        1,
    );
    assert!(
        result.is_err(),
        "invalid parameter must not rename external user element"
    );
    assert_eq!(external, before);
    assert!(
        apply(
            &external,
            &Command::SetProperty {
                element_id: reference.from.clone(),
                path: "name".into(),
                value: serde_json::json!("Hijacked")
            },
            1
        )
        .is_err()
    );
}

#[test]
fn qa_parameter_identity_uses_exposed_name_in_all_domain_projections() {
    let original = add(&project("uml.class"), 0).expect("template");
    let reference = original
        .relations
        .iter()
        .find(|relation| relation.kind == "uml.parametered_element")
        .expect("reference");
    let renamed = apply(
        &original,
        &Command::Rename {
            element_id: reference.to.clone(),
            name: "Value".into(),
        },
        1,
    )
    .expect("rename exposed classifier");
    assert_eq!(
        renamed
            .elements
            .iter()
            .find(|element| element.id == reference.from)
            .expect("stored parameter")
            .name,
        original
            .elements
            .iter()
            .find(|element| element.id == reference.from)
            .expect("original parameter")
            .name
    );
    let section = &renamed;
    let selected = inspect_element(&section, &reference.from).expect("parameter inspector");
    assert_eq!(selected.name, "Value");
    assert_eq!(selected.display_label, "Value");
    assert_eq!(
        selected
            .properties
            .iter()
            .find(|property| property.path == "name")
            .expect("name property")
            .value,
        serde_json::json!("Value")
    );
    let owner = inspect_element(&section, "owner").expect("owner inspector");
    assert_eq!(owner.template_parameters[0].name, "Value");
    assert_eq!(
        section
            .hierarchy_rows
            .iter()
            .find(|row| row.element_id == reference.from)
            .expect("parameter row")
            .label,
        "Value"
    );
}

#[test]
fn parameter_rename_rejects_invalid_signature_owner_before_following_reference() {
    let mut malformed = add(&project("uml.class"), 0).expect("template");
    let reference = malformed
        .relations
        .iter()
        .find(|relation| relation.kind == "uml.parametered_element")
        .expect("reference")
        .clone();
    author(&mut malformed, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|element| element["id"] == "owner")
            .expect("owner")["kind"] = "uml.note".into();
    });
    author(&mut malformed, |raw| {
        raw["relations"]
            .as_array_mut()
            .expect("relations")
            .iter_mut()
            .find(|relation| relation["kind"] == "owns" && relation["to"] == reference.to)
            .expect("exposed ownership")["from"] = "owner".into();
    });
    let before = malformed.clone();
    assert!(
        apply(
            &malformed,
            &Command::Rename {
                element_id: reference.from,
                name: "Wrong".into()
            },
            1
        )
        .is_err()
    );
    assert_eq!(malformed, before);
}

#[test]
fn constrained_parameter_flow_decides_identity_ancestry_and_reopens() {
    let mut model = add(&project("uml.class"), 0).expect("template");
    let parameter = model
        .elements
        .iter()
        .find(|e| e.kind == "uml.classifier_template_parameter")
        .unwrap()
        .id
        .clone();
    let exposed = model
        .relations
        .iter()
        .find(|r| r.kind == "uml.parametered_element")
        .unwrap()
        .to
        .clone();
    for (id, kind) in [
        ("base", "uml.class"),
        ("middle", "uml.class"),
        ("unrelated", "uml.class"),
    ] {
        author(&mut model, |raw| {
            raw["elements"].as_array_mut().expect("elements").push(
                serde_json::from_value(serde_json::json!({"id":id,"kind":kind,"name":id})).unwrap(),
            );
        });
    }
    for (id, kind, from, to) in [
        ("first", "uml.generalization", exposed.as_str(), "middle"),
        ("second", "uml.generalization", "middle", "base"),
        (
            "identity",
            "uml.constraining_classifier",
            parameter.as_str(),
            exposed.as_str(),
        ),
        (
            "direct",
            "uml.constraining_classifier",
            parameter.as_str(),
            "middle",
        ),
        (
            "transitive",
            "uml.constraining_classifier",
            parameter.as_str(),
            "base",
        ),
    ] {
        author(&mut model, |raw| {
            raw["relations"].as_array_mut().expect("relations").push(
                serde_json::from_value(
                    serde_json::json!({"id":id,"kind":kind,"from":from,"to":to}),
                )
                .unwrap(),
            );
        });
    }
    let rule_issues = |model: &Document| {
        model_diagnostics(model)
            .into_iter()
            .filter(|d| {
                d.code == "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED"
                    || d.code == "UML_TEMPLATE_PARAMETERED_CONSTRAINT"
            })
            .collect::<Vec<_>>()
    };
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&model, format).unwrap(), format).unwrap();
        assert!(
            rule_issues(&reopened).is_empty(),
            "{:?}",
            rule_issues(&reopened)
        );
    }
    // True branches do not require the unimplemented Substitution alternative,
    // including the normative omitted/default true value.
    author(&mut model, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|e| e["id"] == parameter)
            .unwrap()["properties"]
            .as_object_mut()
            .expect("properties")
            .remove("allowSubstitutable");
    });
    assert!(rule_issues(&model).is_empty());
    author(&mut model, |raw| {
        raw["relations"].as_array_mut().expect("relations").push(serde_json::from_value(serde_json::json!({"id":"unrelated-constraint","kind":"uml.constraining_classifier","from":parameter,"to":"unrelated"})).unwrap());
    });
    add_substitution(
        &mut model,
        "unresolved-substitution",
        &exposed,
        serde_json::json!({"href":"external.uml#Contract"}),
    );
    let unknown = rule_issues(&model);
    assert_eq!(unknown.len(), 1, "{unknown:?}");
    assert_eq!(unknown[0].code, "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED");
    author(&mut model, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|e| e["id"] == parameter)
            .unwrap()["properties"]
            .as_object_mut()
            .expect("properties")
            .insert("allowSubstitutable".into(), serde_json::json!(true));
    });
    let before = serde_json::to_value(&model).unwrap();
    let rejected = apply(
        &model,
        &Command::SetProperty {
            element_id: parameter.clone(),
            path: "properties.allowSubstitutable".into(),
            value: serde_json::json!(false),
        },
        2,
    );
    let error = rejected.expect_err("nonconforming constraint became valid");
    assert!(
        error.contains("UML_TEMPLATE_PARAMETERED_CONSTRAINT"),
        "{error}"
    );
    assert_eq!(before, serde_json::to_value(&model).unwrap());
    // Imported invalid drafts remain diagnosable after reopening; repairing the
    // offending relationship restores the decided rule without changing IDs.
    author(&mut model, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|e| e["id"] == parameter)
            .unwrap()["properties"]
            .as_object_mut()
            .expect("properties")
            .insert("allowSubstitutable".into(), serde_json::json!(false));
    });
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&model, format).unwrap(), format).unwrap();
        assert!(
            rule_issues(&reopened)
                .iter()
                .any(|d| d.code == "UML_TEMPLATE_PARAMETERED_CONSTRAINT")
        );
    }
    author(&mut model, |raw| {
        raw["relations"]
            .as_array_mut()
            .expect("relations")
            .retain(|r| r["id"] != "unrelated-constraint");
    });
    assert!(rule_issues(&model).is_empty());
}

#[test]
fn constrained_parameter_draft_keeps_unknown_and_invalid_obligations_visible() {
    let mut model = add(&project("uml.class"), 0).unwrap();
    let parameter = model
        .elements
        .iter()
        .find(|e| e.kind == "uml.classifier_template_parameter")
        .unwrap()
        .id
        .clone();
    author(&mut model, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|e| e["id"] == parameter)
            .unwrap()["properties"]
            .as_object_mut()
            .expect("properties")
            .remove("allowSubstitutable");
    });
    for (id, kind) in [("unrelated", "uml.class"), ("bad", "uml.package")] {
        author(&mut model, |raw| {
            raw["elements"].as_array_mut().expect("elements").push(
                serde_json::from_value(serde_json::json!({"id":id,"kind":kind,"name":id})).unwrap(),
            );
        });
        author(&mut model, |raw| {
            raw["relations"].as_array_mut().expect("relations").push(serde_json::from_value(serde_json::json!({"id":format!("constraint-{id}"),"kind":"uml.constraining_classifier","from":parameter,"to":id})).unwrap());
        });
    }
    let exposed = model
        .relations
        .iter()
        .find(|r| r.kind == "uml.parametered_element")
        .unwrap()
        .to
        .clone();
    add_substitution(
        &mut model,
        "unresolved-substitution",
        &exposed,
        serde_json::json!({"href":"external.uml#Contract"}),
    );
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&model, format).unwrap(), format).unwrap();
        let issues = model_diagnostics(&reopened);
        assert!(
            issues
                .iter()
                .any(|d| d.code == "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED")
        );
        assert!(
            issues
                .iter()
                .any(|d| d.code == "UML_TEMPLATE_CONSTRAINING_TYPE")
        );
    }
}

#[test]
fn constrained_parameter_flow_preserves_unresolved_ancestry_and_invalid_flags() {
    let mut model = add(&project("uml.class"), 0).unwrap();
    let parameter = model
        .elements
        .iter()
        .find(|e| e.kind == "uml.classifier_template_parameter")
        .unwrap()
        .id
        .clone();
    let exposed = model
        .relations
        .iter()
        .find(|r| r.kind == "uml.parametered_element")
        .unwrap()
        .to
        .clone();
    author(&mut model, |raw| {
        raw["elements"].as_array_mut().expect("elements").push(
            serde_json::from_value(
                serde_json::json!({"id":"constraint","kind":"uml.class","name":"Constraint"}),
            )
            .unwrap(),
        );
    });
    author(&mut model, |raw| {
        raw["relations"].as_array_mut().expect("relations").push(serde_json::from_value(serde_json::json!({"id":"cc","kind":"uml.constraining_classifier","from":parameter,"to":"constraint"})).unwrap());
    });
    // A stored, unresolved ancestry declaration cannot prove nonconformance.
    author(&mut model, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|e| e["id"] == exposed)
            .unwrap()["properties"]
            .as_object_mut()
            .expect("properties")
            .insert(
                "general".into(),
                serde_json::json!([{"href":"external.uml#Base"}]),
            );
    });
    let issues = model_diagnostics(&model);
    assert!(
        issues
            .iter()
            .any(|d| d.code == "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED")
    );
    assert!(
        !issues
            .iter()
            .any(|d| d.code == "UML_TEMPLATE_PARAMETERED_CONSTRAINT")
    );
    // Identity is true even when the alternative ancestry query is unresolved.
    author(&mut model, |raw| {
        raw["relations"]
            .as_array_mut()
            .expect("relations")
            .iter_mut()
            .find(|r| r["id"] == "cc")
            .unwrap()["to"] = json!(exposed);
    });
    assert!(
        !model_diagnostics(&model)
            .iter()
            .any(|d| d.code == "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED")
    );
    author(&mut model, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|e| e["id"] == exposed)
            .unwrap()["properties"]
            .as_object_mut()
            .expect("properties")
            .remove("general");
    });
    author(&mut model, |raw| {
        raw["relations"]
            .as_array_mut()
            .expect("relations")
            .iter_mut()
            .find(|r| r["id"] == "cc")
            .unwrap()["to"] = "constraint".into();
    });
    author(&mut model, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|e| e["id"] == parameter)
            .unwrap()["properties"]
            .as_object_mut()
            .expect("properties")
            .insert("allowSubstitutable".into(), serde_json::json!("false"));
    });
    let issues = model_diagnostics(&model);
    assert!(
        issues
            .iter()
            .any(|d| d.code == "UML_TEMPLATE_PARAMETER_FLAG_TYPE")
    );
    assert!(
        !issues
            .iter()
            .any(|d| d.code == "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED")
    );
    assert!(
        issues
            .iter()
            .any(|d| d.code == "UML_TEMPLATE_PARAMETERED_CONSTRAINT"),
        "unknown allowSubstitutable AND absent Substitution is false"
    );
    add_substitution(
        &mut model,
        "matching-substitution",
        &exposed,
        serde_json::json!({"$ref":"constraint"}),
    );
    let issues = model_diagnostics(&model);
    assert!(
        issues
            .iter()
            .any(|d| d.code == "UML_TEMPLATE_PARAMETER_FLAG_TYPE")
    );
    assert!(
        issues
            .iter()
            .any(|d| d.code == "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED"),
        "unknown allowSubstitutable AND true Substitution remains unknown"
    );
    assert!(
        !issues
            .iter()
            .any(|d| d.code == "UML_TEMPLATE_PARAMETERED_CONSTRAINT")
    );
}

fn add_substitution(model: &mut Document, id: &str, owner: &str, contract: serde_json::Value) {
    author(model, |raw| {
        raw["elements"].as_array_mut().expect("elements").push(serde_json::from_value(serde_json::json!({
        "id":id,"kind":"uml.substitution","name":id,
        "properties":{"contract":contract,"substitutingClassifier":{"$ref":owner},"visibility":null}
    })).unwrap());
    });
    author(model, |raw| {
        raw["relations"].as_array_mut().expect("relations").push(
            serde_json::from_value(serde_json::json!({
                "id":format!("owns-{id}"),"kind":"owns","from":owner,"to":id
            }))
            .unwrap(),
        );
    });
}

#[test]
fn substitution_flow_uses_owned_contracts_not_inheritance_and_reopens() {
    let mut model = add(&project("uml.class"), 0).unwrap();
    let parameter = model
        .elements
        .iter()
        .find(|e| e.kind == "uml.classifier_template_parameter")
        .unwrap()
        .id
        .clone();
    let exposed = model
        .relations
        .iter()
        .find(|r| r.kind == "uml.parametered_element")
        .unwrap()
        .to
        .clone();
    author(&mut model, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|e| e["id"] == parameter)
            .unwrap()["properties"]
            .as_object_mut()
            .expect("properties")
            .remove("allowSubstitutable");
    });
    for id in ["contract", "parent", "other"] {
        author(&mut model, |raw| {
            raw["elements"].as_array_mut().expect("elements").push(
                serde_json::from_value(serde_json::json!({"id":id,"kind":"uml.class","name":id}))
                    .unwrap(),
            );
        });
    }
    author(&mut model, |raw| {
        raw["relations"].as_array_mut().expect("relations").push(serde_json::from_value(serde_json::json!({"id":"constraint","kind":"uml.constraining_classifier","from":parameter,"to":"contract"})).unwrap());
    });
    let decided = |model: &Document| {
        model_diagnostics(model)
            .into_iter()
            .filter(|d| {
                d.code == "UML_TEMPLATE_PARAMETERED_CONSTRAINT"
                    || d.code == "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED"
            })
            .collect::<Vec<_>>()
    };
    // A closed empty set proves false, even when allowSubstitutable defaults true.
    assert!(
        decided(&model)
            .iter()
            .any(|d| d.code == "UML_TEMPLATE_PARAMETERED_CONSTRAINT")
    );
    add_substitution(
        &mut model,
        "parent-substitution",
        "parent",
        serde_json::json!({"$ref":"contract"}),
    );
    author(&mut model, |raw| {
        raw["relations"].as_array_mut().expect("relations").push(serde_json::from_value(serde_json::json!({"id":"parent-edge","kind":"uml.generalization","from":exposed,"to":"parent"})).unwrap());
    });
    assert!(
        decided(&model)
            .iter()
            .any(|diagnostic| diagnostic.code == "UML_TEMPLATE_PARAMETERED_CONSTRAINT"),
        "parent-owned Substitution does not satisfy the exposed Classifier's constraint"
    );
    add_substitution(
        &mut model,
        "local-substitution",
        &exposed,
        serde_json::json!({"$ref":"contract"}),
    );
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&model, format).unwrap(), format).unwrap();
        assert!(decided(&reopened).is_empty(), "{:?}", decided(&reopened));
    }
    author(&mut model, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|e| e["id"] == parameter)
            .unwrap()["properties"]
            .as_object_mut()
            .expect("properties")
            .insert("allowSubstitutable".into(), serde_json::json!(true));
    });
    let before = model.clone();
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: parameter.clone(),
            path: "properties.allowSubstitutable".into(),
            value: serde_json::json!(false),
        },
        7,
    )
    .expect_err("disallowing the only satisfying alternative must reject atomically");
    assert!(
        error.contains("UML_TEMPLATE_PARAMETERED_CONSTRAINT"),
        "{error}"
    );
    assert_eq!(model, before);
}

#[test]
fn substitution_draft_flow_preserves_unknowns_and_rejects_bad_ownership_or_types() {
    let mut model = project("uml.class");
    for (id, kind) in [
        ("contract", "uml.class"),
        ("other", "uml.class"),
        ("package", "uml.package"),
    ] {
        author(&mut model, |raw| {
            raw["elements"].as_array_mut().expect("elements").push(
                serde_json::from_value(serde_json::json!({"id":id,"kind":kind,"name":id})).unwrap(),
            );
        });
    }
    add_substitution(
        &mut model,
        "unknown",
        "owner",
        serde_json::json!({"href":"external.uml#Contract"}),
    );
    assert!(
        model_diagnostics(&model)
            .iter()
            .any(
                |diagnostic| diagnostic.code == "MDE_SUBSTITUTION_UNVERIFIED"
                    && diagnostic.subject_id == "unknown"
                    && diagnostic.severity == DiagnosticSeverity::Warning
            )
    );
    add_substitution(
        &mut model,
        "known",
        "owner",
        serde_json::json!({"$ref":"contract"}),
    );
    let before_drafts = model.clone();
    for (field, value) in [
        ("contract", serde_json::json!({"$ref":"package"})),
        ("contract", serde_json::Value::Null),
        ("contract", serde_json::json!({"$ref":"missing"})),
        (
            "substitutingClassifier",
            serde_json::json!({"$ref":"other"}),
        ),
    ] {
        let mut draft = model.clone();
        author(&mut draft, |raw| {
            raw["elements"]
                .as_array_mut()
                .expect("elements")
                .iter_mut()
                .find(|e| e["id"] == "known")
                .unwrap()["properties"]
                .as_object_mut()
                .expect("properties")
                .insert(field.into(), value);
        });
        for format in ["json", "protobuf"] {
            let reopened = decode(&encode(&draft, format).unwrap(), format).unwrap();
            assert!(
                model_diagnostics(&reopened)
                    .iter()
                    .any(|d| d.code == "UML_SUBSTITUTION_CONTRACT")
            );
        }
    }
    let mut draft = model.clone();
    author(&mut draft, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|element| element["id"] == "owner")
            .expect("owner")["properties"]["substitution"] = json!([]);
    });
    assert!(
        model_diagnostics(&draft)
            .iter()
            .any(|diagnostic| diagnostic.code == "UML_SUBSTITUTION_CONTRACT"
                && diagnostic.severity == DiagnosticSeverity::Error),
        "explicit empty collection cannot conceal owned substitutions"
    );
    assert_eq!(model, before_drafts, "draft checks must not mutate source");
    let mut wrong_owner = model.clone();
    author(&mut wrong_owner, |raw| {
        raw["relations"]
            .as_array_mut()
            .expect("relations")
            .iter_mut()
            .find(|r| r["id"] == "owns-known")
            .unwrap()["from"] = "package".into();
    });
    assert!(
        model_diagnostics(&wrong_owner)
            .iter()
            .any(|d| d.code == "UML_SUBSTITUTION_CONTRACT")
    );
    let mut orphan = model.clone();
    author(&mut orphan, |raw| {
        raw["relations"]
            .as_array_mut()
            .expect("relations")
            .retain(|r| r["id"] != "owns-known");
    });
    assert!(
        model_diagnostics(&orphan)
            .iter()
            .any(|d| d.code == "UML_SUBSTITUTION_CONTRACT")
    );
}

#[test]
fn substitution_owner_projection_agrees_across_validation_commands_and_reopen() {
    let mut model = project("uml.class");
    author(&mut model, |raw| raw["root_model"] = json!("root"));
    author(&mut model, |raw| {
        raw["elements"].as_array_mut().expect("elements").push(
            serde_json::from_value(
                serde_json::json!({"id":"root","kind":"uml.model","name":"Root"}),
            )
            .unwrap(),
        );
    });
    for child in ["owner", "contract"] {
        author(&mut model, |raw| {
            raw["relations"].as_array_mut().expect("relations").push(serde_json::from_value(serde_json::json!({"id":format!("root-{child}"),"kind":"owns","from":"root","to":child})).unwrap());
        });
    }
    author(&mut model, |raw| {
        raw["elements"].as_array_mut().expect("elements").push(
            serde_json::from_value(
                serde_json::json!({"id":"contract","kind":"uml.class","name":"Contract"}),
            )
            .unwrap(),
        );
    });
    add_substitution(
        &mut model,
        "substitution",
        "owner",
        serde_json::json!({"$ref":"contract"}),
    );
    author(&mut model, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|e| e["id"] == "substitution")
            .unwrap()["properties"]
            .as_object_mut()
            .expect("properties")
            .remove("substitutingClassifier");
    });
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&model, format).unwrap(), format).unwrap();
        assert_eq!(
            inspected(&reopened, "substitution", "uml.owner"),
            json!("owner")
        );
        let errors = model_diagnostics(&reopened)
            .into_iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{errors:?}");
        let renamed = apply(
            &reopened,
            &Command::Rename {
                element_id: "owner".into(),
                name: "Renamed".into(),
            },
            8,
        )
        .unwrap();
        assert_eq!(
            inspected(&renamed, "substitution", "uml.owner"),
            json!("owner")
        );
    }
    author(&mut model, |raw| {
        raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|e| e["id"] == "substitution")
            .unwrap()["properties"]
            .as_object_mut()
            .expect("properties")
            .insert("client".into(), serde_json::json!([{"$ref":"contract"}]));
    });
    assert!(
        model_diagnostics(&model)
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    );
}

#[test]
fn substitution_explicit_collection_reopens_unknowns_and_diagnoses_invalid_entries() {
    let mut model = project("uml.class");
    author(&mut model, |raw| {
        raw["elements"].as_array_mut().expect("elements").push(
            serde_json::from_value(
                serde_json::json!({"id":"contract","kind":"uml.class","name":"Contract"}),
            )
            .unwrap(),
        );
    });
    add_substitution(
        &mut model,
        "known",
        "owner",
        serde_json::json!({"$ref":"contract"}),
    );
    for entries in [
        serde_json::json!([{"$ref":"known"},{"href":"external.uml#Substitution"}]),
        serde_json::json!([{"href":"external.uml#Substitution"},{"$ref":"known"}]),
    ] {
        author(&mut model, |raw| {
            raw["elements"]
                .as_array_mut()
                .expect("elements")
                .iter_mut()
                .find(|e| e["id"] == "owner")
                .unwrap()["properties"]
                .as_object_mut()
                .expect("properties")
                .insert("substitution".into(), entries);
        });
        for format in ["json", "protobuf"] {
            let reopened = decode(&encode(&model, format).unwrap(), format).unwrap();

            assert!(
                model_diagnostics(&reopened)
                    .iter()
                    .any(|d| d.severity == DiagnosticSeverity::Warning),
                "unresolved collection remains diagnosed"
            );
        }
    }
    for entries in [
        serde_json::json!([{"$ref":"known"},{"href":"external.uml#Substitution"},false]),
        serde_json::json!([false,{"href":"external.uml#Substitution"},{"$ref":"known"}]),
        serde_json::json!([{"$ref":"known"},{"$ref":"known"}]),
    ] {
        author(&mut model, |raw| {
            raw["elements"]
                .as_array_mut()
                .expect("elements")
                .iter_mut()
                .find(|e| e["id"] == "owner")
                .unwrap()["properties"]
                .as_object_mut()
                .expect("properties")
                .insert("substitution".into(), entries);
        });
        assert!(
            model_diagnostics(&model)
                .iter()
                .any(|d| d.severity == DiagnosticSeverity::Error
                    && d.subject_id == "owner"
                    && d.message.starts_with("Classifier-substitution:"))
        );
    }
}

#[test]
fn substitution_endpoint_subsets_reopen_unknowns_and_diagnose_invalid_entries() {
    let mut model = project("uml.class");
    author(&mut model, |raw| {
        raw["elements"].as_array_mut().expect("elements").push(
            serde_json::from_value(
                serde_json::json!({"id":"contract","kind":"uml.class","name":"Contract"}),
            )
            .unwrap(),
        );
    });
    add_substitution(
        &mut model,
        "known",
        "owner",
        serde_json::json!({"$ref":"contract"}),
    );
    for (role, required) in [("client", "owner"), ("supplier", "contract")] {
        for values in [
            serde_json::json!([{"$ref":required},{"href":"external.uml#Endpoint"}]),
            serde_json::json!([{"href":"external.uml#Endpoint"},{"$ref":required}]),
        ] {
            let mut draft = model.clone();
            author(&mut draft, |raw| {
                raw["elements"]
                    .as_array_mut()
                    .expect("elements")
                    .iter_mut()
                    .find(|e| e["id"] == "known")
                    .unwrap()["properties"]
                    .as_object_mut()
                    .expect("properties")
                    .insert(role.into(), values);
            });
            for format in ["json", "protobuf"] {
                let reopened = decode(&encode(&draft, format).unwrap(), format).unwrap();

                assert!(
                    model_diagnostics(&reopened)
                        .iter()
                        .any(|d| d.severity == DiagnosticSeverity::Warning)
                );
            }
        }
        for values in [
            serde_json::json!([{"$ref":required},{"href":"external.uml#Endpoint"},false]),
            serde_json::json!([false,{"href":"external.uml#Endpoint"},{"$ref":required}]),
            serde_json::json!([{"$ref":required},{"$ref":required},{"href":"external.uml#Endpoint"}]),
        ] {
            let mut draft = model.clone();
            author(&mut draft, |raw| {
                raw["elements"]
                    .as_array_mut()
                    .expect("elements")
                    .iter_mut()
                    .find(|e| e["id"] == "known")
                    .unwrap()["properties"]
                    .as_object_mut()
                    .expect("properties")
                    .insert(role.into(), values);
            });
            assert!(
                model_diagnostics(&draft)
                    .iter()
                    .any(|d| d.severity == DiagnosticSeverity::Error
                        && d.code == "UML_SUBSTITUTION_CONTRACT")
            );
        }
    }
    // Unknown client inclusion cannot conceal an invalid supplier collection.
    author(&mut model, |raw| {
        let properties = raw["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|element| element["id"] == "known")
            .expect("known")["properties"]
            .as_object_mut()
            .expect("properties");
        properties.insert("client".into(), json!([{"href":"external.uml#Endpoint"}]));
        properties.insert("supplier".into(), json!([false]));
    });
    assert!(
        model_diagnostics(&model)
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error
                && d.code == "UML_SUBSTITUTION_CONTRACT")
    );
}
