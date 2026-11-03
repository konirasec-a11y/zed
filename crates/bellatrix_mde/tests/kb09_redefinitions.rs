//! Normative redefinition and Slot membership through public graph queries and commands.
use bellatrix_mde::{
    Format, Project, StructuralSeverity,
    commands::{self, Command},
    decode, encode,
    uml::{classifiers, references::ReferenceIndex, root::Model},
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn document(diamond: bool) -> Project {
    let mut value = json!({
        "id":"project", "name":"Redefinitions", "root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"base","kind":"uml.class","name":"Base"},
            {"id":"middle","kind":"uml.class","name":"Middle"},
            {"id":"leaf","kind":"uml.class","name":"Leaf"},
            {"id":"other","kind":"uml.class","name":"Other"},
            {"id":"type","kind":"uml.class","name":"Type"},
            {"id":"original","kind":"uml.attribute","name":"original", "properties":{"type":{"$ref":"type"}}},
            {"id":"replacement","kind":"uml.attribute","name":"replacement", "properties":{"type":{"$ref":"type"},"redefinedProperty":[{"$ref":"original"}]}},
            {"id":"last","kind":"uml.attribute","name":"last", "properties":{"type":{"$ref":"type"}}}
        ],
        "relations":[
            {"id":"middle-base","kind":"uml.generalization","from":"middle","to":"base"},
            {"id":"leaf-middle","kind":"uml.generalization","from":"leaf","to":"middle"}
        ]
    });
    for (owner, child) in [
        ("root", "base"),
        ("root", "middle"),
        ("root", "leaf"),
        ("root", "other"),
        ("root", "type"),
        ("base", "original"),
        ("middle", "replacement"),
        ("leaf", "last"),
    ] {
        value["relations"]
            .as_array_mut()
            .expect("relations")
            .push(json!({
                "id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child
            }));
    }
    if diamond {
        value["relations"]
            .as_array_mut()
            .expect("relations")
            .extend([
                json!({"id":"other-base","kind":"uml.generalization","from":"other","to":"base"}),
                json!({"id":"leaf-other","kind":"uml.generalization","from":"leaf","to":"other"}),
            ]);
    }
    decode(&serde_json::to_vec(&value).expect("JSON"), Format::Json).expect("document")
}

fn edit(project: &Project, change: impl FnOnce(&mut Value)) -> Project {
    let mut value: Value =
        serde_json::from_slice(&encode(project, Format::Json).expect("save")).expect("JSON");
    change(&mut value);
    decode(&serde_json::to_vec(&value).expect("JSON"), Format::Json).expect("reload")
}

fn slots(project: &Project, id: &str) -> BTreeSet<String> {
    let references = ReferenceIndex::new(Model::from_project(project));
    classifiers::query(&references, id)
        .expect("namespace")
        .classifier
        .expect("classifier")
        .all_slottable_features
        .expect("normative slottable features")
        .iter()
        .map(|element| element.id().to_owned())
        .collect()
}

#[test]
fn ancestral_redefinitions_follow_literal_ocl_through_chain_diamond_and_reopen() {
    for diamond in [false, true] {
        let project = document(diamond);
        assert_eq!(
            slots(&project, "middle"),
            BTreeSet::from(["replacement".into()])
        );
        // The ancestor's attribute reappears through allParents().attribute.
        assert_eq!(
            slots(&project, "leaf"),
            BTreeSet::from(["original".into(), "replacement".into(), "last".into()])
        );
        let project = commands::apply(
            &project,
            &Command::SetRedefinedElements {
                element_id: "last".into(),
                redefined_ids: vec!["replacement".into()],
            },
            1,
        )
        .expect("redefine immediate inherited feature");
        assert_eq!(
            slots(&project, "leaf"),
            BTreeSet::from(["original".into(), "last".into()])
        );
        let project = commands::apply(
            &project,
            &Command::SetRedefinedElements {
                element_id: "last".into(),
                redefined_ids: vec!["replacement".into(), "original".into()],
            },
            2,
        )
        .expect("explicitly redefine both ancestors");
        let reopened =
            decode(&encode(&project, Format::Json).expect("save"), Format::Json).expect("reopen");
        assert_eq!(slots(&reopened, "leaf"), BTreeSet::from(["last".into()]));
        assert!(
            !Model::from_project(&reopened)
                .validate()
                .iter()
                .any(|issue| issue.severity == StructuralSeverity::Error),
            "{:?}",
            Model::from_project(&reopened).validate()
        );
    }
}

#[test]
fn invalid_redefinition_targets_and_cycles_are_rejected_without_mutating_source() {
    let project = document(false);
    let before = encode(&project, Format::Json).expect("snapshot");
    for targets in [
        vec!["last"],
        vec!["type"],
        vec!["replacement", "replacement"],
        vec!["missing"],
    ] {
        assert!(
            commands::apply(
                &project,
                &Command::SetRedefinedElements {
                    element_id: "last".into(),
                    redefined_ids: targets.into_iter().map(str::to_owned).collect(),
                },
                1
            )
            .is_err()
        );
        assert_eq!(encode(&project, Format::Json).expect("unchanged"), before);
    }
    let cycle = edit(&project, |value| {
        value["relations"]
            .as_array_mut()
            .expect("relations")
            .push(json!({
                "id":"cycle","kind":"uml.generalization","from":"base","to":"leaf"
            }));
    });
    let references = ReferenceIndex::new(Model::from_project(&cycle));
    assert!(classifiers::query(&references, "leaf").is_err());
    let redefinition_cycle = edit(&project, |value| {
        value["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|element| element["id"] == "original")
            .expect("original")["properties"]["redefinedProperty"] =
            json!([{"$ref":"replacement"}]);
    });
    assert!(
        Model::from_project(&redefinition_cycle)
            .validate()
            .iter()
            .any(|issue| issue.severity == StructuralSeverity::Error
                && issue.subject_id == "original")
    );
}

#[test]
fn classifier_redefinition_uses_normative_false_default_not_operation_comparator() {
    let project = edit(&document(false), |value| {
        value["elements"].as_array_mut().expect("elements").extend([
            json!({"id":"nested-base","kind":"uml.class","name":"NestedBase"}),
            json!({"id":"nested-specific","kind":"uml.class","name":"NestedSpecific",
                "properties":{"redefinedClassifier":[{"$ref":"nested-base"}]}}),
        ]);
        value["relations"].as_array_mut().expect("relations").extend([
            json!({"id":"owns-nested-base","kind":"owns","from":"base","to":"nested-base"}),
            json!({"id":"owns-nested-specific","kind":"owns","from":"middle","to":"nested-specific"}),
        ]);
    });
    let diagnostics = Model::from_project(&project).validate();
    assert!(
        diagnostics
            .iter()
            .any(|issue| issue.subject_id == "nested-specific"
                && issue.severity == StructuralSeverity::Error
                && issue.message.contains("inconsistent")),
        "{diagnostics:?}"
    );
    assert!(
        !diagnostics
            .iter()
            .any(|issue| issue.message.contains("filtering is not implemented"))
    );
}

#[test]
fn slots_use_ancestral_private_attributes_but_reject_locally_redefined_targets() {
    let project = edit(&document(false), |value| {
        value["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|element| element["id"] == "replacement")
            .expect("replacement")["properties"]["visibility"] = json!("private");
        value["elements"].as_array_mut().expect("elements").extend([
            json!({"id":"instance","kind":"uml.instance_specification","name":"Instance",
                "properties":{"classifier":[{"$ref":"leaf"}]}}),
            json!({"id":"middle-instance","kind":"uml.instance_specification","name":"MiddleInstance", "properties":{"classifier":[{"$ref":"middle"}]}}),
            json!({"id":"slot","kind":"uml.slot","name":"",
                "properties":{"definingFeature":{"$ref":"original"}}}),
        ]);
        value["relations"]
            .as_array_mut()
            .expect("relations")
            .extend([
                json!({"id":"owns-instance","kind":"owns","from":"root","to":"instance"}),
                json!({"id":"owns-slot","kind":"owns","from":"instance","to":"slot"}),
                json!({"id":"owns-middle-instance","kind":"owns","from":"root","to":"middle-instance"}),
            ]);
    });
    assert!(
        !Model::from_project(&project)
            .validate()
            .iter()
            .any(|issue| issue.severity == StructuralSeverity::Error),
        "{:?}",
        Model::from_project(&project).validate()
    );
    assert!(slots(&project, "leaf").contains("replacement"));
    let before = encode(&project, Format::Json).expect("snapshot");
    let error = commands::apply(
        &project,
        &Command::SetOwner {
            element_id: "slot".into(),
            owner_id: Some("middle-instance".into()),
        },
        1,
    )
    .expect_err("middle's own replacement excludes original");
    assert!(error.contains("not a slottable feature"), "{error}");
    assert_eq!(before, encode(&project, Format::Json).expect("unchanged"));
    let updated = edit(&project, |value| {
        value["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|element| element["id"] == "slot")
            .expect("slot")["properties"]["definingFeature"] = json!({"$ref":"replacement"});
    });
    assert!(
        !Model::from_project(&updated)
            .validate()
            .iter()
            .any(|issue| issue.severity == StructuralSeverity::Error)
    );
    let updated = commands::apply(
        &updated,
        &Command::SetOwner {
            element_id: "slot".into(),
            owner_id: Some("middle-instance".into()),
        },
        2,
    )
    .expect("replacement remains slottable in its defining classifier");
    let reopened = decode(
        &encode(&updated, Format::Protobuf).expect("save"),
        Format::Protobuf,
    )
    .expect("reopen");
    assert!(
        !Model::from_project(&reopened)
            .validate()
            .iter()
            .any(|issue| issue.severity == StructuralSeverity::Error)
    );
}

#[test]
fn independent_diamond_redefinitions_are_not_a_language_specific_override_ambiguity() {
    let project = edit(&document(true), |value| {
        value["elements"]
            .as_array_mut()
            .expect("elements")
            .push(json!({
                "id":"parallel","kind":"uml.attribute","name":"parallel",
                "properties":{"type":{"$ref":"type"},"redefinedProperty":[{"$ref":"original"}]}
            }));
        value["relations"]
            .as_array_mut()
            .expect("relations")
            .push(json!({
                "id":"owns-parallel","kind":"owns","from":"other","to":"parallel"
            }));
    });
    assert_eq!(
        slots(&project, "leaf"),
        BTreeSet::from([
            "original".into(),
            "replacement".into(),
            "parallel".into(),
            "last".into(),
        ])
    );
    let resolved = commands::apply(
        &project,
        &Command::SetRedefinedElements {
            element_id: "last".into(),
            redefined_ids: vec!["replacement".into(), "parallel".into()],
        },
        1,
    )
    .expect("explicitly replace both independent branch members");
    assert_eq!(
        slots(&resolved, "leaf"),
        BTreeSet::from(["original".into(), "last".into()])
    );
    let final_target = edit(&project, |value| {
        value["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|element| element["id"] == "parallel")
            .expect("parallel")["properties"]["isLeaf"] = json!(true);
    });
    let error = commands::apply(
        &final_target,
        &Command::SetRedefinedElements {
            element_id: "last".into(),
            redefined_ids: vec!["parallel".into()],
        },
        1,
    )
    .expect_err("leaf target cannot be redefined");
    assert!(error.contains("isLeaf"), "{error}");
}

#[test]
fn malformed_redefinition_collections_are_not_treated_as_absent_after_reopen() {
    for (kind, role) in [
        ("uml.attribute", "redefinedProperty"),
        ("uml.operation", "redefinedOperation"),
    ] {
        for (value, message) in [
            (Value::Null, "must be an array"),
            (json!({"$ref":"original"}), "must be an array"),
            (json!([null]), "cannot contain null"),
        ] {
            let project = edit(&document(false), |document| {
                let element = document["elements"]
                    .as_array_mut()
                    .expect("elements")
                    .iter_mut()
                    .find(|element| element["id"] == "replacement")
                    .expect("replacement");
                element["kind"] = json!(kind);
                element["properties"] = if kind == "uml.attribute" {
                    json!({"type":{"$ref":"type"}})
                } else {
                    json!({})
                };
                element["properties"][role] = value;
            });
            let references = ReferenceIndex::new(Model::from_project(&project));
            let error =
                classifiers::query(&references, "middle").expect_err("reject malformed collection");
            assert!(
                matches!(error, classifiers::QueryError::Invalid { .. }),
                "{error}"
            );
            assert!(error.to_string().contains(message), "{error}");
            assert!(
                Model::from_project(&project)
                    .validate()
                    .iter()
                    .any(|issue| issue.subject_id == "replacement"
                        && issue.severity == StructuralSeverity::Error
                        && issue.message.contains(message))
            );
            let repaired = commands::apply(
                &project,
                &Command::SetRedefinedElements {
                    element_id: "replacement".into(),
                    redefined_ids: vec![],
                },
                1,
            )
            .expect("repair malformed collection with explicit empty collection");
            let references = ReferenceIndex::new(Model::from_project(&repaired));
            assert!(classifiers::query(&references, "middle").is_ok());
        }
    }
}

#[test]
fn port_redefinition_selects_a_role_that_accepts_all_targets_and_replaces_old_roles() {
    let project = edit(&document(false), |value| {
        value["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|element| element["id"] == "last")
            .expect("last")["kind"] = json!("uml.port");
        value["elements"]
            .as_array_mut()
            .expect("elements")
            .push(json!({
                "id":"base-port","kind":"uml.port","name":"basePort",
                "properties":{"type":{"$ref":"type"},"aggregation":"composite"}
            }));
        value["elements"]
            .as_array_mut()
            .expect("elements")
            .iter_mut()
            .find(|element| element["id"] == "last")
            .expect("last")["properties"]["aggregation"] = json!("composite");
        value["relations"]
            .as_array_mut()
            .expect("relations")
            .push(json!({
                "id":"owns-base-port","kind":"owns","from":"base","to":"base-port"
            }));
    });
    let mut project = project;
    for (targets, role) in [
        (vec!["original"], "redefinedProperty"),
        (vec!["base-port"], "redefinedPort"),
        (vec!["original", "base-port"], "redefinedProperty"),
        (vec!["base-port", "original"], "redefinedProperty"),
        (vec![], "redefinedPort"),
    ] {
        project = commands::apply(
            &project,
            &Command::SetRedefinedElements {
                element_id: "last".into(),
                redefined_ids: targets.iter().map(|id| (*id).to_owned()).collect(),
            },
            1,
        )
        .expect("typed subset role must not restrict the base Property role");
        let value: Value =
            serde_json::from_slice(&encode(&project, Format::Json).expect("save")).expect("JSON");
        let properties = &value["elements"]
            .as_array()
            .expect("elements")
            .iter()
            .find(|element| element["id"] == "last")
            .expect("last")["properties"];
        assert_eq!(
            properties[role],
            Value::Array(targets.iter().map(|id| json!({"$ref":id})).collect())
        );
        let other_role = if role == "redefinedPort" {
            "redefinedProperty"
        } else {
            "redefinedPort"
        };
        assert!(
            properties.get(other_role).is_none(),
            "old role targets must be cleared"
        );
        let features = slots(&project, "leaf");
        for target in ["original", "base-port"] {
            assert_eq!(features.contains(target), !targets.contains(&target));
        }
        project = decode(
            &encode(&project, Format::Protobuf).expect("save"),
            Format::Protobuf,
        )
        .expect("reopen");
    }
}

#[test]
fn property_consistency_preserves_published_lower_bound_clause_and_roundtrips() {
    use bellatrix_mde::uml::feature_compatibility::{
        compare_multiplicity_containment, compare_property_consistency,
    };
    // The first four cases distinguish the published OCL from UMLR-424's
    // proposed correction. The issue is open, so containment is not substituted.
    for (general, specific, literal, containment) in [
        (Some("1..4"), Some("2..3"), false, true),
        (Some("1..4"), Some("1..5"), true, false),
        (Some("1..4"), Some("1..*"), true, false),
        (Some("1..*"), Some("2..3"), false, true),
        (Some("1..4"), Some("1..3"), true, true),
        (Some("1..4"), Some("0..3"), false, false),
        (None, None, true, true),
        (None, Some("1..*"), true, false),
    ] {
        let project = edit(&document(false), |value| {
            for element in value["elements"].as_array_mut().expect("elements") {
                let bound = match element["id"].as_str() {
                    Some("original") => general,
                    Some("replacement") => specific,
                    _ => continue,
                };
                if let Some(bound) = bound {
                    element["properties"]["multiplicity"] = json!(bound);
                }
            }
        });
        for format in [Format::Json, Format::Protobuf] {
            let reopened =
                decode(&encode(&project, format).expect("save"), format).expect("reopen");
            let references = ReferenceIndex::new(Model::from_project(&reopened));
            let original = references.element("original").expect("original");
            let replacement = references.element("replacement").expect("replacement");
            assert_eq!(
                compare_property_consistency(&references, replacement, original)
                    .expect("literal consistency"),
                literal,
                "{general:?} <- {specific:?}"
            );
            assert_eq!(
                compare_multiplicity_containment(&references, replacement, original)
                    .expect("containment remains independent"),
                containment
            );
            let diagnostics = Model::from_project(&reopened).validate();
            assert_eq!(
                diagnostics
                    .iter()
                    .any(|issue| issue.subject_id == "replacement"
                        && issue.severity == StructuralSeverity::Error
                        && issue.message.contains("inconsistent")),
                !literal,
                "{diagnostics:?}"
            );
            let before = encode(&reopened, Format::Json).expect("snapshot");
            let applied = commands::apply(
                &reopened,
                &Command::SetRedefinedElements {
                    element_id: "replacement".into(),
                    redefined_ids: vec!["original".into()],
                },
                1,
            );
            assert_eq!(
                applied.is_ok(),
                literal,
                "{general:?} <- {specific:?}: {applied:?}"
            );
            if let Ok(updated) = applied {
                assert_eq!(
                    slots(&updated, "middle"),
                    BTreeSet::from(["replacement".into()])
                );
            }
            assert_eq!(encode(&reopened, Format::Json).expect("unchanged"), before);
        }
    }
}

#[test]
fn property_consistency_evaluates_owned_bounds_independently() {
    use bellatrix_mde::uml::feature_compatibility::compare_property_consistency;
    for (lower_kind, lower_value, expected) in [
        ("uml.literal_integer", json!(0), "false"),
        ("uml.literal_integer", json!(1), "true"),
        ("uml.literal_integer", json!(2), "unsupported"),
        ("uml.opaque_expression", Value::Null, "unsupported"),
        ("uml.literal_null", Value::Null, "invalid"),
        ("uml.literal_integer", Value::Null, "invalid"),
    ] {
        let project = edit(&document(false), |value| {
            let elements = value["elements"].as_array_mut().expect("elements");
            let replacement = elements
                .iter_mut()
                .find(|element| element["id"] == "replacement")
                .expect("replacement");
            replacement["properties"]["lowerValue"] = json!({"$ref":"specific-lower"});
            replacement["properties"]["upperValue"] = json!({"$ref":"specific-upper"});
            let mut lower = json!({"id":"specific-lower", "kind":lower_kind, "name":"",
                "properties":{"visibility":null}});
            if lower_kind == "uml.literal_integer" {
                lower["properties"]["value"] = lower_value.clone();
            }
            elements.extend([
                lower,
                json!({"id":"specific-upper", "kind":"uml.opaque_expression", "name":"",
                    "properties":{"visibility":null}}),
            ]);
            value["relations"].as_array_mut().expect("relations").extend([
                json!({"id":"owns-specific-lower","kind":"owns","from":"replacement","to":"specific-lower"}),
                json!({"id":"owns-specific-upper","kind":"owns","from":"replacement","to":"specific-upper"}),
            ]);
        });
        for format in [Format::Json, Format::Protobuf] {
            let reopened =
                decode(&encode(&project, format).expect("save"), format).expect("reopen");
            let references = ReferenceIndex::new(Model::from_project(&reopened));
            let result = compare_property_consistency(
                &references,
                references.element("replacement").expect("replacement"),
                references.element("original").expect("original"),
            );
            let actual = match result {
                Ok(false) => "false",
                Ok(true) => "true",
                Err(classifiers::QueryError::Unsupported { .. }) => "unsupported",
                Err(classifiers::QueryError::Invalid { .. }) => "invalid",
            };
            assert_eq!(actual, expected, "{lower_kind}: {lower_value}");
            if expected == "false" {
                assert!(
                    Model::from_project(&reopened)
                        .validate()
                        .iter()
                        .any(|issue| issue.subject_id == "replacement"
                            && issue.severity == StructuralSeverity::Error
                            && issue.message.contains("inconsistent"))
                );
                let before = encode(&reopened, Format::Json).expect("snapshot");
                assert!(
                    commands::apply(
                        &reopened,
                        &Command::SetRedefinedElements {
                            element_id: "replacement".into(),
                            redefined_ids: vec!["original".into()],
                        },
                        1
                    )
                    .is_err()
                );
                assert_eq!(encode(&reopened, Format::Json).expect("unchanged"), before);
            }
        }
    }
}

#[test]
fn property_consistency_prioritizes_invalid_lower_in_either_argument() {
    use bellatrix_mde::uml::feature_compatibility::compare_property_consistency;
    for invalid_owner in ["original", "replacement"] {
        let project = edit(&document(false), |value| {
            for owner in ["original", "replacement"] {
                let lower_id = format!("{owner}-lower");
                let elements = value["elements"].as_array_mut().expect("elements");
                elements
                    .iter_mut()
                    .find(|element| element["id"] == owner)
                    .expect("property")["properties"]["lowerValue"] = json!({"$ref":lower_id});
                elements.push(json!({"id":lower_id,
                    "kind":if owner == invalid_owner { "uml.literal_null" } else { "uml.opaque_expression" },
                    "name":"", "properties":{"visibility":null}}));
                value["relations"]
                    .as_array_mut()
                    .expect("relations")
                    .push(json!({
                        "id":format!("owns-{lower_id}"), "kind":"owns", "from":owner,"to":lower_id
                    }));
            }
        });
        for format in [Format::Json, Format::Protobuf] {
            let reopened =
                decode(&encode(&project, format).expect("save"), format).expect("reopen");
            let references = ReferenceIndex::new(Model::from_project(&reopened));
            let result = compare_property_consistency(
                &references,
                references.element("replacement").expect("replacement"),
                references.element("original").expect("original"),
            );
            assert!(
                matches!(result, Err(classifiers::QueryError::Invalid { .. })),
                "invalid lower owned by {invalid_owner}: {result:?}"
            );
        }
    }
}
