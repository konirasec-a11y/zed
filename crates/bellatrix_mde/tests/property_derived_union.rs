use bellatrix_mde::{
    Project, StructuralSeverity,
    uml::{root::Model, schema},
};
use serde_json::{Value, json};

const RULES: [(&str, &str); 2] = [
    ("isReadOnly", "Property-derived_union_is_read_only"),
    ("isDerived", "Property-derived_union_is_derived"),
];

fn document(kind: &str, properties: Value) -> Project {
    serde_json::from_value(json!({
        "id":"project", "name":"Derived unions", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"class", "kind":"uml.class", "name":"Owner"},
            {"id":"property", "kind":kind, "name":"items", "properties":properties}
        ],
        "relations":[
            {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
            {"id":"owns-property", "kind":"owns", "from":"class", "to":"property"}
        ]
    }))
    .expect("canonical document")
}

#[test]
fn complete_boolean_truth_tables_match_both_published_implications() {
    for kind in ["uml.attribute", "uml.port", "uml.extension_end"] {
        for union in [false, true] {
            for read_only in [false, true] {
                for derived in [false, true] {
                    let project = document(
                        kind,
                        json!({
                            "isDerivedUnion":union, "isReadOnly":read_only, "isDerived":derived
                        }),
                    );
                    let issues = Model::from_project(&project).validate();
                    for ((_, rule), consequent) in RULES.iter().zip([read_only, derived]) {
                        let matching: Vec<_> = issues
                            .iter()
                            .filter(|issue| {
                                issue.subject_id == "property" && issue.message.contains(rule)
                            })
                            .collect();
                        assert_eq!(
                            matching.len(),
                            usize::from(union && !consequent),
                            "{kind}: union={union}, read_only={read_only}, derived={derived}: {matching:?}"
                        );
                        assert!(
                            matching
                                .iter()
                                .all(|issue| issue.severity == StructuralSeverity::Error)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn absent_scalar_defaults_are_false_not_null() {
    for kind in ["uml.attribute", "uml.port", "uml.extension_end"] {
        let project = document(kind, json!({}));
        let model = Model::from_project(&project);
        let property = model.element("property").expect("Property");
        for key in ["isDerivedUnion", "isReadOnly", "isDerived"] {
            assert_eq!(
                schema::read_effective_boolean(property, key),
                Ok(false),
                "{kind}::{key}"
            );
            let project = document(kind, json!({key:null}));
            let model = Model::from_project(&project);
            assert!(
                schema::read_effective_boolean(model.element("property").expect("Property"), key)
                    .is_err()
            );
        }
    }
}

#[test]
fn invalid_scalars_remain_contract_errors_even_when_implication_is_vacuous() {
    for key in ["isDerivedUnion", "isReadOnly", "isDerived"] {
        for value in [
            Value::Null,
            json!(0),
            json!("false"),
            json!([]),
            json!({"href":"https://example.invalid/boolean"}),
        ] {
            for union in [false, true] {
                let mut properties =
                    json!({"isDerivedUnion":union, "isReadOnly":true, "isDerived":true});
                properties[key] = value.clone();
                let project = document("uml.attribute", properties);
                let issues = Model::from_project(&project).validate();
                assert!(
                    issues.iter().any(|issue| issue.subject_id == "property"
                        && issue.code == "UML_PROPERTY_VALUE"
                        && issue.severity == StructuralSeverity::Error
                        && issue.message.contains(key)),
                    "{key}={value}, union={union}: {issues:?}"
                );
            }
        }
    }
}
