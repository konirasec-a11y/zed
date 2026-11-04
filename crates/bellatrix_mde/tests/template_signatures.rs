use bellatrix_mde::commands::{self, Command};
use bellatrix_mde::{
    ModelingSection, Project, UmlQuickCreate, classifier_contents, inspect_element,
    validate_structural_model,
};

fn project(kind: &str) -> Project {
    serde_json::from_value(serde_json::json!({"id":"templates", "name":"Templates", "elements":[{"id":"owner", "name":"Owner", "kind":kind}], "diagrams":[{"id":"diagram", "name":"D", "uml_di_type":"UMLClassDiagram", "views":[{"id":"view", "subject":"owner", "geometry":{"kind":"rect", "x":0.0,"y":0.0,"width":180.0,"height":120.0}}]}]})).expect("fixture")
}
fn add(project: &Project, revision: u64) -> Result<Project, String> {
    commands::apply(
        project,
        &Command::QuickEdit {
            diagram_id: "diagram".into(),
            element_id: "owner".into(),
            action: UmlQuickCreate::TemplateParameter,
        },
        revision,
    )
}
fn owns(project: &Project, owner: &str, child: &str) -> bool {
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
        let references = bellatrix_mde::uml::references::ReferenceIndex::new(
            bellatrix_mde::uml::root::Model::from_project(&first),
        );
        let namespaces =
            bellatrix_mde::uml::element_graph_queries::all_namespaces(&references, &classifier.id)
                .expect("exposed template parameter namespaces");
        assert_eq!(
            namespaces
                .iter()
                .map(|element| element.id())
                .collect::<Vec<_>>(),
            vec!["owner"]
        );
        use bellatrix_mde::uml::root::NamedElement;
        let model = bellatrix_mde::uml::root::Model::from_project(&first);
        let named = model
            .element(&classifier.id)
            .expect("classifier")
            .named()
            .expect("named");
        assert_eq!(
            named.qualified_name(model).expect("qualified name"),
            named.name().map(|name| format!("Owner::{name}"))
        );
        assert!(
            bellatrix_mde::uml::element_graph_queries::all_owning_packages(
                &references,
                &classifier.id
            )
            .expect("template parameter is not a namespace")
            .is_empty()
        );
        assert!(
            !first
                .relations
                .iter()
                .any(|relation| relation.kind == "owns" && relation.from == classifier.id)
        );
        assert_eq!(
            parameter.properties.get("allowSubstitutable"),
            Some(&serde_json::json!(false))
        );
        assert!(
            validate_structural_model(&first)
                .iter()
                .all(|issue| !issue.code.starts_with("UML_TEMPLATE")),
            "{:?}",
            validate_structural_model(&first)
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
        let section = ModelingSection::from_model(&second);
        assert_eq!(
            inspect_element(&section, "owner")
                .expect("inspector")
                .template_parameters
                .len(),
            2
        );
        assert_eq!(
            classifier_contents(&section, "owner").template_parameters,
            2
        );
        assert_eq!(
            serde_json::from_str::<Project>(&serde_json::to_string(&second).expect("serialize"))
                .expect("deserialize"),
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
    let after_one = commands::apply(
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
    let renamed_owner = commands::apply(
        &after_one,
        &Command::Rename {
            element_id: "owner".into(),
            name: "Renamed".into(),
        },
        3,
    )
    .expect("rename owner");
    let after_last = commands::apply(
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
        commands::apply(
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
    original.elements.push(
        serde_json::from_value(
            serde_json::json!({"id":"legacy", "name":"T", "kind":"uml.template_parameter"}),
        )
        .expect("legacy"),
    );
    original.relations.push(
        serde_json::from_value(
            serde_json::json!({"id":"legacy-owner", "kind":"owns", "from":"owner", "to":"legacy"}),
        )
        .expect("ownership"),
    );
    let diagnostics = validate_structural_model(&original);
    assert!(
        diagnostics
            .iter()
            .any(|issue| issue.code == "UML_TEMPLATE_PARAMETER_SIGNATURE")
    );
    let updated = add(&original, 0).expect("add without rewriting legacy data");
    assert_eq!(
        updated
            .elements
            .iter()
            .find(|element| element.id == "legacy"),
        original
            .elements
            .iter()
            .find(|element| element.id == "legacy")
    );
    assert!(owns(&updated, "owner", "legacy"));
    assert_eq!(
        inspect_element(&ModelingSection::from_model(&updated), "owner")
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
    let feature = commands::apply(
        &original,
        &Command::AddMember {
            owner_id: reference.to.clone(),
            kind: commands::ModelingMemberKind::Attribute,
        },
        1,
    );
    assert!(feature.is_err_and(|error| error.contains("UML_TEMPLATE_PARAMETERED_FEATURES")));
    let substitutable = commands::apply(
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
        match mutation {
            "reference" => malformed
                .relations
                .retain(|relation| relation.kind != "uml.parametered_element"),
            "type" => {
                malformed
                    .elements
                    .iter_mut()
                    .find(|element| element.id == reference.to)
                    .expect("target")
                    .kind = "uml.package".into()
            }
            "owner" => malformed
                .relations
                .retain(|relation| !(relation.kind == "owns" && relation.to == reference.to)),
            "signature" => {
                malformed
                    .elements
                    .iter_mut()
                    .find(|element| element.kind == "uml.redefinable_template_signature")
                    .expect("signature")
                    .kind = "uml.template_signature".into()
            }
            _ => unreachable!(),
        }
        assert!(
            validate_structural_model(&malformed)
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
    let renamed = commands::apply(
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
    let inspector =
        inspect_element(&ModelingSection::from_model(&renamed), "owner").expect("inspector");
    assert!(
        inspector
            .template_parameters
            .iter()
            .any(|parameter| parameter.display_label == "Key")
    );
    let duplicate = commands::apply(
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
    referenced.elements.push(
        serde_json::from_value(
            serde_json::json!({"id":"client","name":"Client","kind":"uml.class"}),
        )
        .expect("client"),
    );
    referenced.relations.push(serde_json::from_value(serde_json::json!({"id":"binding","kind":"uml.template_binding","from":"client","to":signature.id})).expect("reference"));
    let before = referenced.clone();
    assert!(
        commands::apply(
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
    annotated
        .elements
        .iter_mut()
        .find(|element| element.id == signature.id)
        .expect("signature")
        .description = Some("User-authored documentation".into());
    assert!(
        commands::apply(
            &annotated,
            &Command::DeleteElement {
                element_id: parameter.id.clone()
            },
            1
        )
        .is_err()
    );
    let mut manual = generated.clone();
    manual
        .elements
        .iter_mut()
        .find(|element| element.id == signature.id)
        .expect("signature")
        .properties
        .clear();
    assert!(
        commands::apply(
            &manual,
            &Command::DeleteElement {
                element_id: parameter.id.clone()
            },
            1
        )
        .is_err_and(|error| error.contains("UML_TEMPLATE_SIGNATURE_PARAMETERS"))
    );
    let removed = commands::apply(
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
fn semantic_templates_do_not_require_views_or_add_body_compartments() {
    let mut original = project("uml.operation");
    original.diagrams[0].views.clear();
    let updated = add(&original, 0).expect("semantic operation template");
    assert!(updated.diagrams[0].views.is_empty());
    assert!(
        bellatrix_mde::quick_edit_for("uml.operation")
            .iter()
            .any(|action| action.id == "uml.add-template-parameter")
    );
    let section = ModelingSection::from_model(&updated);
    let contents = classifier_contents(&section, "owner");
    assert_eq!(contents.template_parameters, 1);
    assert_eq!(contents.compartments_height(), 0.0);
    for format in [bellatrix_mde::Format::Json, bellatrix_mde::Format::Protobuf] {
        let reopened = bellatrix_mde::decode(
            &bellatrix_mde::encode(&updated, format).expect("save semantic template"),
            format,
        )
        .expect("reopen semantic template");
        assert_eq!(reopened, updated);
        assert_eq!(
            classifier_contents(&ModelingSection::from_model(&reopened), "owner")
                .template_parameters,
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
    nested.elements.push(
        serde_json::from_value(
            serde_json::json!({"id":"nested","kind":"uml.class","name":"Nested"}),
        )
        .expect("nested"),
    );
    nested
        .relations
        .iter_mut()
        .find(|relation| relation.kind == "owns" && relation.to == reference.to)
        .expect("ownership")
        .from = "nested".into();
    nested.relations.push(serde_json::from_value(serde_json::json!({"id":"nested-owner","kind":"owns","from":reference.from,"to":"nested"})).expect("nested owner"));
    assert!(
        validate_structural_model(&nested)
            .iter()
            .any(|issue| issue.code == "UML_TEMPLATE_PARAMETERED_OWNERSHIP")
    );
    let mut external = original.clone();
    external
        .relations
        .retain(|relation| !(relation.kind == "owns" && relation.to == reference.to));
    let before = external.clone();
    let result = commands::apply(
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
        commands::apply(
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
    let renamed = commands::apply(
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
    let section = ModelingSection::from_model(&renamed);
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
        bellatrix_mde::project_hierarchy(&section)
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
    malformed
        .elements
        .iter_mut()
        .find(|element| element.id == "owner")
        .expect("owner")
        .kind = "uml.note".into();
    malformed
        .relations
        .iter_mut()
        .find(|relation| relation.kind == "owns" && relation.to == reference.to)
        .expect("exposed ownership")
        .from = "owner".into();
    let before = malformed.clone();
    assert!(
        commands::apply(
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
    use bellatrix_mde::{Format, decode, encode};
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
        model.elements.push(
            serde_json::from_value(serde_json::json!({"id":id,"kind":kind,"name":id})).unwrap(),
        );
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
        model.relations.push(
            serde_json::from_value(serde_json::json!({"id":id,"kind":kind,"from":from,"to":to}))
                .unwrap(),
        );
    }
    let rule_issues = |model: &Project| {
        validate_structural_model(model)
            .into_iter()
            .filter(|d| {
                d.code == "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED"
                    || d.code == "UML_TEMPLATE_PARAMETERED_CONSTRAINT"
            })
            .collect::<Vec<_>>()
    };
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format).unwrap(), format).unwrap();
        assert!(
            rule_issues(&reopened).is_empty(),
            "{:?}",
            rule_issues(&reopened)
        );
    }
    // True branches do not require the unimplemented Substitution alternative,
    // including the normative omitted/default true value.
    model
        .elements
        .iter_mut()
        .find(|e| e.id == parameter)
        .unwrap()
        .properties
        .remove("allowSubstitutable");
    assert!(rule_issues(&model).is_empty());
    model.relations.push(serde_json::from_value(serde_json::json!({"id":"unrelated-constraint","kind":"uml.constraining_classifier","from":parameter,"to":"unrelated"})).unwrap());
    add_substitution(
        &mut model,
        "unresolved-substitution",
        &exposed,
        serde_json::json!({"href":"external.uml#Contract"}),
    );
    let unknown = rule_issues(&model);
    assert_eq!(unknown.len(), 1, "{unknown:?}");
    assert_eq!(unknown[0].code, "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED");
    model
        .elements
        .iter_mut()
        .find(|e| e.id == parameter)
        .unwrap()
        .properties
        .insert("allowSubstitutable".into(), serde_json::json!(true));
    let before = serde_json::to_value(&model).unwrap();
    let rejected = commands::apply(
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
    model
        .elements
        .iter_mut()
        .find(|e| e.id == parameter)
        .unwrap()
        .properties
        .insert("allowSubstitutable".into(), serde_json::json!(false));
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&model, format).unwrap(), format).unwrap();
        assert!(
            rule_issues(&reopened)
                .iter()
                .any(|d| d.code == "UML_TEMPLATE_PARAMETERED_CONSTRAINT")
        );
    }
    model.relations.retain(|r| r.id != "unrelated-constraint");
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
    model
        .elements
        .iter_mut()
        .find(|e| e.id == parameter)
        .unwrap()
        .properties
        .remove("allowSubstitutable");
    for (id, kind) in [("unrelated", "uml.class"), ("bad", "uml.package")] {
        model.elements.push(
            serde_json::from_value(serde_json::json!({"id":id,"kind":kind,"name":id})).unwrap(),
        );
        model.relations.push(serde_json::from_value(serde_json::json!({"id":format!("constraint-{id}"),"kind":"uml.constraining_classifier","from":parameter,"to":id})).unwrap());
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
    for format in [bellatrix_mde::Format::Json, bellatrix_mde::Format::Protobuf] {
        let reopened =
            bellatrix_mde::decode(&bellatrix_mde::encode(&model, format).unwrap(), format).unwrap();
        let issues = validate_structural_model(&reopened);
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
    model.elements.push(
        serde_json::from_value(
            serde_json::json!({"id":"constraint","kind":"uml.class","name":"Constraint"}),
        )
        .unwrap(),
    );
    model.relations.push(serde_json::from_value(serde_json::json!({"id":"cc","kind":"uml.constraining_classifier","from":parameter,"to":"constraint"})).unwrap());
    // A stored, unresolved ancestry declaration cannot prove nonconformance.
    model
        .elements
        .iter_mut()
        .find(|e| e.id == exposed)
        .unwrap()
        .properties
        .insert(
            "general".into(),
            serde_json::json!([{"href":"external.uml#Base"}]),
        );
    let issues = validate_structural_model(&model);
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
    model
        .relations
        .iter_mut()
        .find(|r| r.id == "cc")
        .unwrap()
        .to = exposed.clone();
    assert!(
        !validate_structural_model(&model)
            .iter()
            .any(|d| d.code == "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED")
    );
    model
        .elements
        .iter_mut()
        .find(|e| e.id == exposed)
        .unwrap()
        .properties
        .remove("general");
    model
        .relations
        .iter_mut()
        .find(|r| r.id == "cc")
        .unwrap()
        .to = "constraint".into();
    model
        .elements
        .iter_mut()
        .find(|e| e.id == parameter)
        .unwrap()
        .properties
        .insert("allowSubstitutable".into(), serde_json::json!("false"));
    let issues = validate_structural_model(&model);
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
    let issues = validate_structural_model(&model);
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

fn add_substitution(model: &mut Project, id: &str, owner: &str, contract: serde_json::Value) {
    model.elements.push(serde_json::from_value(serde_json::json!({
        "id":id,"kind":"uml.substitution","name":id,
        "properties":{"contract":contract,"substitutingClassifier":{"$ref":owner},"visibility":null}
    })).unwrap());
    model.relations.push(
        serde_json::from_value(serde_json::json!({
            "id":format!("owns-{id}"),"kind":"owns","from":owner,"to":id
        }))
        .unwrap(),
    );
}

#[test]
fn substitution_flow_uses_owned_contracts_not_inheritance_and_reopens() {
    use bellatrix_mde::uml::{classifier_hierarchy, references::ReferenceIndex, root::Model};
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
    model
        .elements
        .iter_mut()
        .find(|e| e.id == parameter)
        .unwrap()
        .properties
        .remove("allowSubstitutable");
    for id in ["contract", "parent", "other"] {
        model.elements.push(
            serde_json::from_value(serde_json::json!({"id":id,"kind":"uml.class","name":id}))
                .unwrap(),
        );
    }
    model.relations.push(serde_json::from_value(serde_json::json!({"id":"constraint","kind":"uml.constraining_classifier","from":parameter,"to":"contract"})).unwrap());
    let decided = |model: &Project| {
        validate_structural_model(model)
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
    model.relations.push(serde_json::from_value(serde_json::json!({"id":"parent-edge","kind":"uml.generalization","from":exposed,"to":"parent"})).unwrap());
    assert!(
        !classifier_hierarchy::is_substitutable_for(
            &ReferenceIndex::new(Model::from_project(&model)),
            &exposed,
            "contract"
        )
        .unwrap()
    );
    add_substitution(
        &mut model,
        "local-substitution",
        &exposed,
        serde_json::json!({"$ref":"contract"}),
    );
    for format in [bellatrix_mde::Format::Json, bellatrix_mde::Format::Protobuf] {
        let reopened =
            bellatrix_mde::decode(&bellatrix_mde::encode(&model, format).unwrap(), format).unwrap();
        let index = ReferenceIndex::new(Model::from_project(&reopened));
        assert!(classifier_hierarchy::is_substitutable_for(&index, &exposed, "contract").unwrap());
        assert!(!classifier_hierarchy::is_substitutable_for(&index, &exposed, "other").unwrap());
        assert!(decided(&reopened).is_empty(), "{:?}", decided(&reopened));
    }
    model
        .elements
        .iter_mut()
        .find(|e| e.id == parameter)
        .unwrap()
        .properties
        .insert("allowSubstitutable".into(), serde_json::json!(true));
    let before = model.clone();
    let error = commands::apply(
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
    use bellatrix_mde::uml::{classifier_hierarchy, references::ReferenceIndex, root::Model};
    let mut model = project("uml.class");
    for (id, kind) in [
        ("contract", "uml.class"),
        ("other", "uml.class"),
        ("package", "uml.package"),
    ] {
        model.elements.push(
            serde_json::from_value(serde_json::json!({"id":id,"kind":kind,"name":id})).unwrap(),
        );
    }
    add_substitution(
        &mut model,
        "unknown",
        "owner",
        serde_json::json!({"href":"external.uml#Contract"}),
    );
    let query = |model: &Project| {
        classifier_hierarchy::is_substitutable_for(
            &ReferenceIndex::new(Model::from_project(model)),
            "owner",
            "contract",
        )
    };
    assert!(query(&model).is_err_and(|error| matches!(
        error,
        bellatrix_mde::uml::classifiers::QueryError::Unsupported { .. }
    )));
    add_substitution(
        &mut model,
        "known",
        "owner",
        serde_json::json!({"$ref":"contract"}),
    );
    assert!(
        query(&model).unwrap(),
        "known membership decides includes beside unknown"
    );
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
        draft
            .elements
            .iter_mut()
            .find(|e| e.id == "known")
            .unwrap()
            .properties
            .insert(field.into(), value);
        for format in [bellatrix_mde::Format::Json, bellatrix_mde::Format::Protobuf] {
            let reopened =
                bellatrix_mde::decode(&bellatrix_mde::encode(&draft, format).unwrap(), format)
                    .unwrap();
            assert!(query(&reopened).is_err());
            assert!(
                validate_structural_model(&reopened)
                    .iter()
                    .any(|d| d.code == "UML_SUBSTITUTION_CONTRACT")
            );
        }
    }
    let mut draft = model.clone();
    draft
        .elements
        .iter_mut()
        .find(|e| e.id == "owner")
        .unwrap()
        .properties
        .insert("substitution".into(), serde_json::json!([]));
    assert!(
        query(&draft).is_err(),
        "explicit empty collection cannot conceal owned substitutions"
    );
    assert!(
        query(&model).unwrap(),
        "draft checks must not mutate source"
    );
    let mut wrong_owner = model.clone();
    wrong_owner
        .relations
        .iter_mut()
        .find(|r| r.id == "owns-known")
        .unwrap()
        .from = "package".into();
    assert!(
        validate_structural_model(&wrong_owner)
            .iter()
            .any(|d| d.code == "UML_SUBSTITUTION_CONTRACT")
    );
    let mut orphan = model.clone();
    orphan.relations.retain(|r| r.id != "owns-known");
    assert!(
        validate_structural_model(&orphan)
            .iter()
            .any(|d| d.code == "UML_SUBSTITUTION_CONTRACT")
    );
    let mut binary = project("uml.class");
    binary.elements.push(
        serde_json::from_value(
            serde_json::json!({"id":"contract","kind":"uml.class","name":"Contract"}),
        )
        .unwrap(),
    );
    binary.relations.push(serde_json::from_value(serde_json::json!({"id":"binary","kind":"uml.substitution","from":"owner","to":"contract"})).unwrap());
    assert!(
        query(&binary).is_err_and(|error| matches!(
            error,
            bellatrix_mde::uml::classifiers::QueryError::Unsupported { .. }
        )),
        "unadapted binary record cannot prove absent Substitution"
    );
}

#[test]
fn substitution_owner_projection_agrees_across_queries_validation_commands_and_reopen() {
    use bellatrix_mde::uml::{
        classifier_hierarchy, references::ReferenceIndex, relationship_endpoints, root::Model,
    };
    let mut model = project("uml.class");
    model.root_model = Some("root".into());
    model.elements.push(
        serde_json::from_value(serde_json::json!({"id":"root","kind":"uml.model","name":"Root"}))
            .unwrap(),
    );
    for child in ["owner", "contract"] {
        model.relations.push(serde_json::from_value(serde_json::json!({"id":format!("root-{child}"),"kind":"owns","from":"root","to":child})).unwrap());
    }
    model.elements.push(
        serde_json::from_value(
            serde_json::json!({"id":"contract","kind":"uml.class","name":"Contract"}),
        )
        .unwrap(),
    );
    add_substitution(
        &mut model,
        "substitution",
        "owner",
        serde_json::json!({"$ref":"contract"}),
    );
    model
        .elements
        .iter_mut()
        .find(|e| e.id == "substitution")
        .unwrap()
        .properties
        .remove("substitutingClassifier");
    for format in [bellatrix_mde::Format::Json, bellatrix_mde::Format::Protobuf] {
        let reopened =
            bellatrix_mde::decode(&bellatrix_mde::encode(&model, format).unwrap(), format).unwrap();
        let index = ReferenceIndex::new(Model::from_project(&reopened));
        assert!(classifier_hierarchy::is_substitutable_for(&index, "owner", "contract").unwrap());
        let clients = relationship_endpoints::clients(&index, "substitution").unwrap();
        assert_eq!(
            clients.iter().map(|e| e.id()).collect::<Vec<_>>(),
            vec!["owner"]
        );
        let errors = Model::from_project(&reopened)
            .validate()
            .into_iter()
            .filter(|d| d.severity == bellatrix_mde::StructuralSeverity::Error)
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{errors:?}");
        let renamed = commands::apply(
            &reopened,
            &Command::Rename {
                element_id: "owner".into(),
                name: "Renamed".into(),
            },
            8,
        )
        .unwrap();
        assert!(
            classifier_hierarchy::is_substitutable_for(
                &ReferenceIndex::new(Model::from_project(&renamed)),
                "owner",
                "contract"
            )
            .unwrap()
        );
    }
    model
        .elements
        .iter_mut()
        .find(|e| e.id == "substitution")
        .unwrap()
        .properties
        .insert("client".into(), serde_json::json!([{"$ref":"contract"}]));
    let index = ReferenceIndex::new(Model::from_project(&model));
    assert!(
        relationship_endpoints::clients(&index, "substitution").is_err(),
        "stored client must include the canonical substituting Classifier"
    );
    assert!(classifier_hierarchy::is_substitutable_for(&index, "owner", "contract").is_err());
    assert!(
        Model::from_project(&model)
            .validate()
            .iter()
            .any(|d| d.severity == bellatrix_mde::StructuralSeverity::Error)
    );
}

#[test]
fn substitution_explicit_collection_keeps_decisive_membership_and_invalid_precedence() {
    use bellatrix_mde::uml::{
        classifier_hierarchy, classifiers::QueryError, references::ReferenceIndex, root::Model,
    };
    let mut model = project("uml.class");
    model.elements.push(
        serde_json::from_value(
            serde_json::json!({"id":"contract","kind":"uml.class","name":"Contract"}),
        )
        .unwrap(),
    );
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
        model
            .elements
            .iter_mut()
            .find(|e| e.id == "owner")
            .unwrap()
            .properties
            .insert("substitution".into(), entries);
        for format in [bellatrix_mde::Format::Json, bellatrix_mde::Format::Protobuf] {
            let reopened =
                bellatrix_mde::decode(&bellatrix_mde::encode(&model, format).unwrap(), format)
                    .unwrap();
            assert!(
                classifier_hierarchy::is_substitutable_for(
                    &ReferenceIndex::new(Model::from_project(&reopened)),
                    "owner",
                    "contract"
                )
                .unwrap()
            );
            assert!(
                Model::from_project(&reopened)
                    .validate()
                    .iter()
                    .any(|d| d.severity == bellatrix_mde::StructuralSeverity::Warning),
                "unresolved collection remains diagnosed"
            );
        }
    }
    for entries in [
        serde_json::json!([{"$ref":"known"},{"href":"external.uml#Substitution"},false]),
        serde_json::json!([false,{"href":"external.uml#Substitution"},{"$ref":"known"}]),
        serde_json::json!([{"$ref":"known"},{"$ref":"known"}]),
    ] {
        model
            .elements
            .iter_mut()
            .find(|e| e.id == "owner")
            .unwrap()
            .properties
            .insert("substitution".into(), entries);
        assert!(
            classifier_hierarchy::is_substitutable_for(
                &ReferenceIndex::new(Model::from_project(&model)),
                "owner",
                "contract"
            )
            .is_err_and(|e| matches!(e, QueryError::Invalid { .. }))
        );
    }
}

#[test]
fn substitution_endpoint_subsets_keep_known_members_beside_unknowns_without_partial_lists() {
    use bellatrix_mde::uml::{
        classifier_hierarchy, classifiers::QueryError, references::ReferenceIndex,
        relationship_endpoints, root::Model,
    };
    let mut model = project("uml.class");
    model.elements.push(
        serde_json::from_value(
            serde_json::json!({"id":"contract","kind":"uml.class","name":"Contract"}),
        )
        .unwrap(),
    );
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
            draft
                .elements
                .iter_mut()
                .find(|e| e.id == "known")
                .unwrap()
                .properties
                .insert(role.into(), values);
            for format in [bellatrix_mde::Format::Json, bellatrix_mde::Format::Protobuf] {
                let reopened =
                    bellatrix_mde::decode(&bellatrix_mde::encode(&draft, format).unwrap(), format)
                        .unwrap();
                let index = ReferenceIndex::new(Model::from_project(&reopened));
                assert!(
                    classifier_hierarchy::is_substitutable_for(&index, "owner", "contract")
                        .unwrap()
                );
                let complete = if role == "client" {
                    relationship_endpoints::clients(&index, "known")
                } else {
                    relationship_endpoints::suppliers(&index, "known")
                };
                assert!(
                    complete
                        .is_err_and(|error| matches!(error,
                    bellatrix_mde::uml::element_graph_queries::ElementGraphError::Unsupported { .. }
                )),
                    "full collection must remain explicitly unsupported, not invalid or silently truncated"
                );
                assert!(
                    Model::from_project(&reopened)
                        .validate()
                        .iter()
                        .any(|d| d.severity == bellatrix_mde::StructuralSeverity::Warning)
                );
            }
        }
        for values in [
            serde_json::json!([{"$ref":required},{"href":"external.uml#Endpoint"},false]),
            serde_json::json!([false,{"href":"external.uml#Endpoint"},{"$ref":required}]),
            serde_json::json!([{"$ref":required},{"$ref":required},{"href":"external.uml#Endpoint"}]),
        ] {
            let mut draft = model.clone();
            draft
                .elements
                .iter_mut()
                .find(|e| e.id == "known")
                .unwrap()
                .properties
                .insert(role.into(), values);
            assert!(
                classifier_hierarchy::is_substitutable_for(
                    &ReferenceIndex::new(Model::from_project(&draft)),
                    "owner",
                    "contract"
                )
                .is_err_and(|error| matches!(error, QueryError::Invalid { .. }))
            );
        }
    }
    // Unknown client inclusion cannot conceal an invalid supplier collection.
    let properties = &mut model
        .elements
        .iter_mut()
        .find(|e| e.id == "known")
        .unwrap()
        .properties;
    properties.insert(
        "client".into(),
        serde_json::json!([{"href":"external.uml#Endpoint"}]),
    );
    properties.insert("supplier".into(), serde_json::json!([false]));
    assert!(
        classifier_hierarchy::is_substitutable_for(
            &ReferenceIndex::new(Model::from_project(&model)),
            "owner",
            "contract"
        )
        .is_err_and(|error| matches!(error, QueryError::Invalid { .. }))
    );
}
