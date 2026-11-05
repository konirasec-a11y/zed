use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, project_model_section,
};
use bellatrix_mde_contracts::protocol::{command::*, projection::ModelingProjection};
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
    let updated = apply_model_command(&id, &name, &section.to_string(), revision, command.clone())?;
    decode(
        &encode_model_section(&id, &name, &updated.to_string(), "json")?,
        "json",
    )
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
) -> Option<bellatrix_mde_contracts::protocol::inspector::ElementInspector> {
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
            view_id: None,
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
fn legacy_direct_parameters_remain_visible_without_migration() {
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
fn constrained_parameter_edits_reject_nonconforming_constraints_atomically() {
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

    author(&mut model, |raw| {
        raw["relations"].as_array_mut().expect("relations").push(serde_json::from_value(serde_json::json!({"id":"unrelated-constraint","kind":"uml.constraining_classifier","from":parameter,"to":"unrelated"})).unwrap());
    });
    add_substitution(
        &mut model,
        "unresolved-substitution",
        &exposed,
        serde_json::json!({"href":"external.uml#Contract"}),
    );

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
fn substitution_flag_edits_reject_disallowing_the_only_matching_contract() {
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

    add_substitution(
        &mut model,
        "parent-substitution",
        "parent",
        serde_json::json!({"$ref":"contract"}),
    );
    author(&mut model, |raw| {
        raw["relations"].as_array_mut().expect("relations").push(serde_json::from_value(serde_json::json!({"id":"parent-edge","kind":"uml.generalization","from":exposed,"to":"parent"})).unwrap());
    });

    add_substitution(
        &mut model,
        "local-substitution",
        &exposed,
        serde_json::json!({"$ref":"contract"}),
    );
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
fn substitution_owner_projection_agrees_across_commands_and_reopen() {
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
}
