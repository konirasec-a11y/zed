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
    let layout = bellatrix_mde::template_signature_layout(180.0, 2).expect("signature overlay");
    assert_eq!(layout.left + layout.width, 196.0);
    assert_eq!(layout.top, 8.0 - layout.height);
    assert_eq!(layout.outsets(180.0, 120.0), (44.0, 16.0, 0.0));
    let tall = bellatrix_mde::template_signature_layout(180.0, 10).expect("tall signature");
    assert_eq!(tall.outsets(180.0, 20.0), (204.0, 16.0, 0.0));
    assert!(bellatrix_mde::template_signature_layout(180.0, 0).is_none());
    assert!(bellatrix_mde::template_signature_layout(f64::NAN, 2).is_none());
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
fn qa_applies_uml_substitutability_default_and_checks_constraint_targets() {
    let mut original = add(&project("uml.class"), 0).expect("template");
    let parameter_id = original
        .elements
        .iter()
        .find(|element| element.kind == "uml.classifier_template_parameter")
        .expect("parameter")
        .id
        .clone();
    original
        .elements
        .iter_mut()
        .find(|element| element.id == parameter_id)
        .expect("parameter")
        .properties
        .remove("allowSubstitutable");
    assert!(
        validate_structural_model(&original)
            .iter()
            .any(|issue| issue.code == "UML_TEMPLATE_PARAMETER_CONSTRAINT")
    );
    original.elements.push(
        serde_json::from_value(
            serde_json::json!({"id":"not-classifier","kind":"uml.package","name":"Wrong"}),
        )
        .expect("package"),
    );
    original.relations.push(serde_json::from_value(serde_json::json!({"id":"bad-constraint","kind":"uml.constraining_classifier","from":parameter_id,"to":"not-classifier"})).expect("constraint"));
    assert!(
        validate_structural_model(&original)
            .iter()
            .any(|issue| issue.code == "UML_TEMPLATE_CONSTRAINING_TYPE")
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
fn signature_does_not_change_body_geometry_or_interface_anchors() {
    let ordinary = bellatrix_mde::ClassifierContents::default();
    for count in [1, 2, 10] {
        let contents = bellatrix_mde::ClassifierContents {
            template_parameters: count,
            ..ordinary
        };
        assert_eq!(
            contents.compartments_height(),
            ordinary.compartments_height()
        );
        let first = bellatrix_mde::interface_layout(180.0, 120.0, ordinary);
        let second = bellatrix_mde::interface_layout(180.0, 120.0, contents);
        assert_eq!(first, second);
        assert_eq!(
            first.anchor_toward(bellatrix_mde::Point { x: 500.0, y: 500.0 }),
            second.anchor_toward(bellatrix_mde::Point { x: 500.0, y: 500.0 })
        );
    }
}
#[test]
fn expanded_hit_bounds_preserve_body_resize_and_connection_coordinates_at_zoom() {
    for count in [1, 2, 10] {
        let shape = bellatrix_mde::template_signature_layout(180.0, count).expect("signature");
        for zoom in [0.5, 1.0, 2.0] {
            let (top, right, bottom) = shape.outsets(180.0, 120.0);
            let body_y = 200.0 * zoom;
            let card_y = body_y - top * zoom;
            let card_width = (180.0 + right) * zoom;
            let card_height = (120.0 + top + bottom) * zoom;
            assert_eq!(card_y + top * zoom, body_y);
            assert_eq!(card_width - right * zoom, 180.0 * zoom);
            assert_eq!(card_y + card_height - bottom * zoom, body_y + 120.0 * zoom);
            assert!(shape.top * zoom + top * zoom >= 0.0);
            assert!((shape.left + shape.width) * zoom <= card_width);
            assert!((shape.top + shape.height + top) * zoom <= card_height);
            assert!(180.0 * zoom + right * zoom + 6.0 > (shape.left + shape.width) * zoom);
        }
    }
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
