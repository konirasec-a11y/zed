use bellatrix_mde::{
    Format, ModelingSection, Project, StructuralSeverity,
    commands::{Command, apply}, decode, encode, inspect_element,
    uml::{classifiers, element_graph_queries, operations, operation_signatures,
        references::ReferenceIndex, root::Model},
};
use serde_json::{Value, json};
use std::collections::HashSet;

fn document() -> Project {
    decode(&serde_json::to_vec(&json!({
        "id":"project", "name":"Ownership", "root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"class","kind":"uml.class","name":"Account",
             "properties":{"collaborationUse":[{"$ref":"use"}]}},
            {"id":"amount","kind":"uml.attribute","name":"amount",
             "properties":{"lowerValue":{"$ref":"lower"},"upperValue":{"$ref":"upper"}}},
            {"id":"lower","kind":"uml.literal_integer","name":"Lower",
             "properties":{"value":0,"visibility":null}},
            {"id":"upper","kind":"uml.literal_unlimited_natural","name":"Upper",
             "properties":{"value":"*","visibility":null}},
            {"id":"condition","kind":"uml.constraint","name":"Allowed",
             "properties":{"specification":{"$ref":"specification"}}},
            {"id":"specification","kind":"uml.literal_boolean","name":"Expression",
             "properties":{"value":true,"visibility":null}},
            {"id":"use","kind":"uml.collaboration_use","name":"Usage",
             "properties":{"visibility":null}}
        ],
        "relations":[
            {"id":"owns-class","kind":"owns","from":"root","to":"class"},
            {"id":"owns-amount","kind":"owns","from":"class","to":"amount"},
            {"id":"owns-lower","kind":"owns","from":"amount","to":"lower"},
            {"id":"owns-upper","kind":"owns","from":"amount","to":"upper"},
            {"id":"owns-condition","kind":"owns","from":"class","to":"condition"},
            {"id":"owns-specification","kind":"owns","from":"condition","to":"specification"},
            {"id":"owns-use","kind":"owns","from":"class","to":"use"}
        ]
    })).expect("document bytes"), Format::Json).expect("import document")
}

fn external_property(project: &Project, id: &str, name: &str, value: Value) -> Project {
    let mut content = serde_json::to_value(project).expect("authored document");
    let element = content["elements"].as_array_mut().expect("elements").iter_mut()
        .find(|element| element["id"] == id).expect("element");
    element["properties"][name] = value;
    decode(&serde_json::to_vec(&content).expect("edited document"), Format::Json)
        .expect("reopen authored draft")
}

fn visibility_error(project: &Project, id: &str) -> bool {
    Model::from_project(project).validate().iter().any(|diagnostic|
        diagnostic.subject_id == id
            && diagnostic.severity == StructuralSeverity::Error
            && diagnostic.code == "UML_VISIBILITY_NEEDS_OWNERSHIP")
}

#[test]
fn canonical_composite_roles_flow_queries_edits_rejects_conflicts_and_reopens() {
    let project = document();
    let references = ReferenceIndex::new(Model::from_project(&project));
    let owned = element_graph_queries::all_owned_elements(&references, "root")
        .expect("explicit composite references agree with canonical ownership");
    assert_eq!(owned.iter().map(|element| element.id()).collect::<HashSet<_>>(),
        HashSet::from(["class", "amount", "lower", "upper", "condition", "specification", "use"]));
    let members = classifiers::owned_members(&references, "class").expect("resolve ownership roles");
    assert!(members.iter().any(|member| member.id() == "amount"));
    assert!(members.iter().any(|member| member.id() == "condition"));
    assert!(!members.iter().any(|member| member.id() == "use"));
    assert!(element_graph_queries::all_namespaces(&references, "lower")
        .expect("literal namespace").is_empty());
    let qualified = Model::qualified_name_with_index(
        references.element("amount").expect("Property").named().expect("NamedElement"),
        &references,
    ).expect("qualified name");
    assert_eq!(qualified.as_deref(), Some("Root::Account::amount"));

    let edited = apply(&project, &Command::Rename {
        element_id:"class".into(), name:"Customer".into(),
    }, 1).expect("edit through the public command");
    let inspector = inspect_element(&ModelingSection::from_model(&edited), "amount")
        .expect("inspect the same canonical Property");
    assert!(inspector.properties.iter().any(|property|
        property.path == "uml.qualifiedName" && property.value == json!("Root::Customer::amount")));
    for command in [
        Command::SetOwner { element_id:"class".into(), owner_id:Some("class".into()) },
        Command::SetOwner { element_id:"lower".into(), owner_id:Some("root".into()) },
    ] {
        let before = encode(&edited, Format::Json).expect("before rejected edit");
        assert!(apply(&edited, &command, 2).is_err());
        assert_eq!(encode(&edited, Format::Json).expect("after rejected edit"), before);
    }
    let inconsistent = external_property(&edited, "condition", "specification", json!({"$ref":"lower"}));
    let references = ReferenceIndex::new(Model::from_project(&inconsistent));
    assert!(element_graph_queries::all_owned_elements(&references, "root").is_err());
    assert!(Model::from_project(&inconsistent).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "condition" && diagnostic.severity == StructuralSeverity::Error));

    let mut wrong_role = serde_json::to_value(&edited).expect("authored ownership");
    wrong_role["relations"].as_array_mut().expect("relationships").iter_mut()
        .find(|relation| relation["id"] == "owns-use").expect("ownership")["from"] = json!("amount");
    let wrong_role = decode(&serde_json::to_vec(&wrong_role).expect("draft bytes"), Format::Json)
        .expect("reopen an unresolved ownership role");
    assert!(Model::from_project(&wrong_role).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "use"
            && diagnostic.code == "UML_ELEMENT_OWNERSHIP_ROLE_UNVERIFIED"
            && diagnostic.severity == StructuralSeverity::Warning));
    let repaired = apply(&wrong_role, &Command::SetOwner {
        element_id:"use".into(), owner_id:Some("class".into()),
    }, 3).expect("repair the explicit role through the public ownership command");
    assert_eq!(repaired, edited);

    let mut two_attributes = serde_json::to_value(&edited).expect("two authored attributes");
    two_attributes["elements"].as_array_mut().expect("elements").push(json!({
        "id":"other","kind":"uml.attribute","name":"other"
    }));
    two_attributes["relations"].as_array_mut().expect("relationships").push(json!({
        "id":"owns-other","kind":"owns","from":"class","to":"other"
    }));
    let two_attributes = decode(&serde_json::to_vec(&two_attributes).expect("document bytes"),
        Format::Json).expect("reopen two attributes");
    for incomplete in [json!([{"$ref":"amount"}]), json!([])] {
        let draft = external_property(&two_attributes, "class", "ownedAttribute", incomplete);
        let references = ReferenceIndex::new(Model::from_project(&draft));
        assert!(matches!(classifiers::owned_members(&references, "class"),
            Err(classifiers::QueryError::Invalid { .. })));
        assert!(element_graph_queries::all_owned_elements(&references, "root").is_err());
        assert!(Model::from_project(&draft).validate().iter().any(|diagnostic|
            diagnostic.code == "UML_ELEMENT_OWNERSHIP_ROLE"
                && diagnostic.severity == StructuralSeverity::Error));
        assert_eq!(decode(&encode(&draft, Format::Protobuf).expect("save partial roles"),
            Format::Protobuf).expect("reopen partial roles"), draft);
    }
    let complete = external_property(&two_attributes, "class", "ownedAttribute",
        json!([{"$ref":"amount"},{"$ref":"other"}]));
    let references = ReferenceIndex::new(Model::from_project(&complete));
    assert!(classifiers::owned_members(&references, "class").expect("complete role membership")
        .iter().any(|member| member.id() == "other"));

    let legacy = external_property(&edited, "amount", "defaultValue", json!("[]"));
    let references = ReferenceIndex::new(Model::from_project(&legacy));
    assert!(matches!(element_graph_queries::all_owned_elements(&references, "root"),
        Err(element_graph_queries::ElementGraphError::Unsupported { .. })));
    assert!(Model::from_project(&legacy).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "amount" && diagnostic.code == "MDE_PROPERTY_CONTRACT_UNVERIFIED"));
    assert_eq!(decode(&encode(&legacy, Format::Json).expect("save unresolved declaration"),
        Format::Json).expect("reopen unresolved declaration"), legacy);

    let mut unknown_type = serde_json::to_value(&edited).expect("authored extension type");
    unknown_type["elements"].as_array_mut().expect("elements").iter_mut()
        .find(|element| element["id"] == "specification").expect("specification")["kind"] =
        json!("custom.expression");
    let unknown_type = decode(&serde_json::to_vec(&unknown_type).expect("extension bytes"),
        Format::Json).expect("reopen a preserved extension");
    let references = ReferenceIndex::new(Model::from_project(&unknown_type));
    assert!(matches!(element_graph_queries::all_owned_elements(&references, "root"),
        Err(element_graph_queries::ElementGraphError::Unsupported { .. })));
    assert!(Model::from_project(&unknown_type).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "condition"
            && diagnostic.code == "MDE_PROPERTY_CONTRACT_UNVERIFIED"));
    assert_eq!(decode(&encode(&unknown_type, Format::Protobuf).expect("save extension"),
        Format::Protobuf).expect("reopen extension"), unknown_type);

    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&edited, format).expect("save"), format).expect("reopen");
        assert_eq!(reopened, edited);
        let references = ReferenceIndex::new(Model::from_project(&reopened));
        assert_eq!(element_graph_queries::all_owned_elements(&references, "root")
            .expect("same composite roles after reopening").len(), 7);
        assert_eq!(reopened.element("amount").expect("same Property").properties["lowerValue"],
            json!({"$ref":"lower"}));
    }
}

#[test]
fn visibility_flow_uses_derived_namespace_and_recovers_an_invalid_draft() {
    let project = document();
    let model = Model::from_project(&project);
    let use_element = model.element("use").expect("CollaborationUse");
    assert_eq!(model.owner(use_element).expect("owner").expect("owned").id(), "class");
    assert!(model.namespace(use_element.named().expect("NamedElement"))
        .expect("resolved non-member role").is_none());
    assert!(!visibility_error(&project, "use"));
    assert!(!visibility_error(&project, "class"));
    let inspector = inspect_element(&ModelingSection::from_model(&project), "use")
        .expect("inspect a non-member owned NamedElement");
    assert!(inspector.properties.iter().any(|property|
        property.path == "uml.namespace" && property.value.is_null()));

    assert!(apply(&project, &Command::SetProperty {
        element_id:"use".into(), path:"properties.visibility".into(), value:json!("public"),
    }, 1).is_err());
    let draft = external_property(&project, "use", "visibility", json!("public"));
    assert!(visibility_error(&draft, "use"));
    let draft = external_property(&draft, "amount", "isReadOnly", json!("not a Boolean"));
    let restored = decode(&encode(&draft, Format::Protobuf).expect("save invalid draft"),
        Format::Protobuf).expect("reopen invalid draft");
    assert!(visibility_error(&restored, "use"));
    let repaired = apply(&restored, &Command::SetProperty {
        element_id:"use".into(), path:"properties.visibility".into(), value:Value::Null,
    }, 2).expect("repair visibility without blocking on an unrelated draft error");
    assert!(!visibility_error(&repaired, "use"));
    assert!(Model::from_project(&repaired).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "amount" && diagnostic.severity == StructuralSeverity::Error));
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&repaired, format).expect("save repair"), format).expect("reopen repair");
        assert_eq!(reopened, repaired);
        assert!(!visibility_error(&reopened, "use"));
    }
}

#[test]
fn parameter_role_flow_isolated_from_unrelated_malformed_composites() {
    let mut content = serde_json::to_value(document()).expect("authored namespace");
    content["elements"].as_array_mut().expect("elements").extend([
        json!({"id":"operation","kind":"uml.operation","name":"accept",
            "properties":{"bodyCondition":"unfinished condition"}}),
        json!({"id":"parameter","kind":"uml.parameter","name":"input",
            "properties":{"type":{"$ref":"class"}}}),
    ]);
    content["relations"].as_array_mut().expect("relationships").extend([
        json!({"id":"owns-operation","kind":"owns","from":"class","to":"operation"}),
        json!({"id":"owns-parameter","kind":"owns","from":"operation","to":"parameter"}),
    ]);
    let inferred = decode(&serde_json::to_vec(&content).expect("draft bytes"), Format::Json)
        .expect("reopen the incomplete condition");
    let explicit = external_property(&inferred, "operation", "ownedParameter",
        json!([{"$ref":"parameter"}]));
    for draft in [inferred, explicit] {
        let references = ReferenceIndex::new(Model::from_project(&draft));
        assert_eq!(operations::owned_parameters(&references, "operation")
            .expect("the unrelated Constraint does not change parameter ownership")
            .iter().map(|parameter| parameter.id.as_str()).collect::<Vec<_>>(),
            vec!["parameter"]);
        let operation = references.element("operation").expect("Operation");
        assert!(operation_signatures::compare_operation_signature(&references, operation, operation)
            .expect("formal signature does not depend on the unfinished condition"));
        assert!(Model::from_project(&draft).validate().iter().any(|diagnostic|
            diagnostic.subject_id == "operation" && diagnostic.code == "UML_PROPERTY_CONTRACT"
                && diagnostic.severity == StructuralSeverity::Error));

        let edited = apply(&draft, &Command::Rename {
            element_id:"parameter".into(), name:"renamed".into(),
        }, 1).expect("edit the parameter while preserving the independent condition error");
        for value in [json!({"$ref":"parameter"}), json!([false,{"$ref":"parameter"}])] {
            let contradiction = external_property(&edited, "operation", "bodyCondition", value);
            let references = ReferenceIndex::new(Model::from_project(&contradiction));
            assert!(operations::owned_parameters(&references, "operation").is_err());
            assert!(Model::from_project(&contradiction).validate().iter().any(|diagnostic|
                diagnostic.subject_id == "parameter" && diagnostic.code == "UML_ELEMENT_OWNERSHIP_ROLE"
                    && diagnostic.severity == StructuralSeverity::Error));
        }
        let repaired = apply(&edited, &Command::SetProperty {
            element_id:"operation".into(), path:"properties.bodyCondition".into(), value:Value::Null,
        }, 2).expect("repair only the unrelated composite condition");
        for saved in [&edited, &repaired] {
            for format in [Format::Json, Format::Protobuf] {
                let reopened = decode(&encode(saved, format).expect("save draft"), format)
                    .expect("reopen draft");
                assert_eq!(&reopened, saved);
                let references = ReferenceIndex::new(Model::from_project(&reopened));
                assert_eq!(operations::owned_parameters(&references, "operation")
                    .expect("preserved canonical parameter role")[0].name, "renamed");
            }
        }
    }
}

#[test]
fn external_parameter_ownership_stays_unverified_while_malformed_roles_are_invalid() {
    let project = decode(&serde_json::to_vec(&json!({
        "id":"external-parameters", "name":"External parameters", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"class", "kind":"uml.class", "name":"Owner"},
            {"id":"operation", "kind":"uml.operation", "name":"accept",
             "properties":{"ownedParameter":[{"href":"other.uml#parameter"}]}},
            {"id":"behavior", "kind":"uml.opaque_behavior", "name":"Execute",
             "properties":{"ownedParameter":[{"href":"other.uml#parameter"}]}}
        ],
        "relations":[
            {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
            {"id":"owns-operation", "kind":"owns", "from":"class", "to":"operation"},
            {"id":"owns-behavior", "kind":"owns", "from":"root", "to":"behavior"}
        ]
    })).expect("external parameter document"), Format::Json).expect("preserve external roles");
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&project, format).expect("save unresolved roles"), format)
            .expect("reopen unresolved roles");
        assert_eq!(reopened, project);
        let model = Model::from_project(&reopened);
        let diagnostics = model.validate();
        for owner in ["operation", "behavior"] {
            assert!(diagnostics.iter().any(|diagnostic|
                diagnostic.subject_id == owner
                    && diagnostic.code == "MDE_PARAMETER_OWNERSHIP_UNVERIFIED"
                    && diagnostic.severity == StructuralSeverity::Warning), "{diagnostics:?}");
            assert!(!diagnostics.iter().any(|diagnostic|
                diagnostic.subject_id == owner
                    && diagnostic.severity == StructuralSeverity::Error), "{diagnostics:?}");
            let malformed = external_property(&reopened, owner, "ownedParameter", json!([false]));
            assert!(Model::from_project(&malformed).validate().iter().any(|diagnostic|
                diagnostic.subject_id == owner
                    && diagnostic.code == "UML_PARAMETER_OWNERSHIP_STORAGE"
                    && diagnostic.severity == StructuralSeverity::Error));
        }
        let references = ReferenceIndex::new(model);
        let operation = references.element("operation").expect("Operation identity");
        assert!(matches!(operation_signatures::compare_operation_signature(
            &references, operation, operation,
        ), Err(classifiers::QueryError::Unsupported { .. })));
        assert!(diagnostics.iter().any(|diagnostic|
            diagnostic.subject_id == "operation"
                && diagnostic.code == "MDE_OPERATION_PARAMETERS_UNVERIFIED"
                && diagnostic.severity == StructuralSeverity::Warning));
    }
}
