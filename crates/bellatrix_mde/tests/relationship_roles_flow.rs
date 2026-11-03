use bellatrix_mde::{
    Format, Project, StructuralSeverity,
    commands::{Command, apply}, decode, encode,
    uml::{
        element_graph_queries::{self, ElementGraphError},
        references::ReferenceIndex,
        relationship_endpoints,
        root::{DirectedRelationship, ElementRef, Model, Relationship},
    },
};
use serde_json::{Value, json};
use std::collections::HashSet;

fn document() -> Project {
    decode(&serde_json::to_vec(&json!({
        "id":"project", "name":"Relationships", "root_model":"root",
        "elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"a","kind":"uml.class","name":"Customer"},
            {"id":"b","kind":"uml.class","name":"Service"},
            {"id":"c","kind":"uml.class","name":"Repository"},
            {"id":"d","kind":"uml.interface","name":"Contract"},
            {"id":"comment","kind":"uml.comment","name":"Documentation"},
            {"id":"object","kind":"uml.usage","name":"ObjectDependency",
             "properties":{"client":[{"$ref":"a"},{"$ref":"d"}],
                           "supplier":[{"$ref":"a"},{"$ref":"c"}]}},
            {"id":"flow","kind":"uml.information_flow","name":"Information",
             "properties":{"informationSource":[{"$ref":"a"},{"$ref":"b"}],
                           "informationTarget":[{"$ref":"c"},{"$ref":"d"}],
                           "conveyed":[{"$ref":"c"}]}}
        ],
        "relations":[
            {"id":"owns-a","kind":"owns","from":"root","to":"a"},
            {"id":"owns-b","kind":"owns","from":"root","to":"b"},
            {"id":"owns-c","kind":"owns","from":"root","to":"c"},
            {"id":"owns-d","kind":"owns","from":"root","to":"d"},
            {"id":"owns-comment","kind":"owns","from":"root","to":"comment"},
            {"id":"owns-object","kind":"owns","from":"root","to":"object"},
            {"id":"owns-flow","kind":"owns","from":"root","to":"flow"},
            {"id":"owns-multi","kind":"owns","from":"root","to":"multi"},
            {"id":"owns-binary","kind":"owns","from":"root","to":"binary"},
            {"id":"multi","kind":"uml.dependency","from":"b","to":"d",
             "properties":{"name":"Multiple",
                           "client":[{"$ref":"a"},{"$ref":"b"}],
                           "supplier":[{"$ref":"c"},{"$ref":"d"}]}},
            {"id":"binary","kind":"uml.dependency","from":"b","to":"c"}
        ]
    })).expect("document bytes"), Format::Json).expect("import authored relationships")
}

fn ids(values: Vec<ElementRef<'_>>) -> HashSet<String> {
    values.into_iter().map(|element| element.id().to_owned()).collect()
}

fn expected(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn external_edit(project: &Project, subject: &str, property: &str, value: Value) -> Project {
    let mut content = serde_json::to_value(project).expect("authored snapshot");
    let collection = if project.elements.iter().any(|element| element.id == subject) {
        "elements"
    } else {
        "relations"
    };
    let element = content[collection].as_array_mut().expect("records")
        .iter_mut().find(|element| element["id"] == subject).expect("subject");
    element["properties"][property] = value;
    decode(&serde_json::to_vec(&content).expect("edited draft"), Format::Json)
        .expect("reload authored draft")
}

fn verify_roles(project: &Project) {
    let model = Model::from_project(project);
    let references = ReferenceIndex::new(model);
    for (id, clients, suppliers) in [
        ("multi", expected(&["a", "b"]), expected(&["c", "d"])),
        ("object", expected(&["a", "d"]), expected(&["a", "c"])),
        ("binary", expected(&["b"]), expected(&["c"])),
    ] {
        let relationship = references.element(id).expect("relationship identity")
            .directed_relationship().expect("object and edge expose the same trait");
        assert_eq!(ids(relationship.sources(model).expect("sources")), clients);
        assert_eq!(ids(relationship.targets(model).expect("targets")), suppliers);
        assert_eq!(ids(relationship_endpoints::clients(&references, id).expect("clients")), clients);
        assert_eq!(ids(relationship_endpoints::suppliers(&references, id).expect("suppliers")), suppliers);
        assert_eq!(ids(relationship.related_elements(model).expect("derived union")),
            clients.union(&suppliers).cloned().collect::<HashSet<_>>());
    }
    assert_eq!(ids(element_graph_queries::client_dependencies(&references, "a")
        .expect("inverse includes non-projected client")), expected(&["object", "multi"]));
    assert_eq!(ids(element_graph_queries::client_dependencies(&references, "b")
        .expect("inverse includes binary adapter")), expected(&["binary", "multi"]));
    assert!(element_graph_queries::client_dependencies(&references, "c")
        .expect("suppliers are not clients").is_empty());
    let flow = references.element("flow").expect("InformationFlow")
        .directed_relationship().expect("DirectedRelationship object");
    assert_eq!(ids(flow.sources(model).expect("source subsets")), expected(&["a", "b"]));
    assert_eq!(ids(flow.targets(model).expect("target subsets")), expected(&["c", "d"]));
    assert_eq!(ids(flow.related_elements(model).expect("related union")), expected(&["a", "b", "c", "d"]));
    assert!(!model.validate().iter().any(|diagnostic|
        matches!(diagnostic.subject_id.as_str(), "multi" | "object" | "binary" | "flow")
            && (diagnostic.code == "UML_RELATIONSHIP_ENDPOINTS"
                || diagnostic.code == "MDE_RELATIONSHIP_ENDPOINTS_UNVERIFIED"
                || diagnostic.message.contains("canonical binary endpoint"))));
}

#[test]
fn relationship_authoring_flow_preserves_multiple_roles_objects_inverses_and_binary_views() {
    let project = document();
    verify_roles(&project);
    let edited = apply(&project, &Command::Rename {
        element_id: "a".into(), name: "Account".into(),
    }, 1).expect("edit a client without changing identities");
    verify_roles(&edited);
    let before = encode(&edited, Format::Json).expect("before rejected deletion");
    assert!(apply(&edited, &Command::DeleteElement { element_id: "a".into() }, 2).is_err());
    assert_eq!(encode(&edited, Format::Json).expect("original snapshot"), before);
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&edited, format).expect("save"), format).expect("reopen");
        assert_eq!(reopened, edited);
        verify_roles(&reopened);
        let projection = reopened.relations.iter().find(|relation| relation.id == "multi")
            .expect("same relation identity");
        assert_eq!((projection.from.as_str(), projection.to.as_str()), ("b", "d"));
    }
    let expanded = external_edit(&edited, "multi", "client",
        json!([{"$ref":"d"},{"$ref":"b"},{"$ref":"a"}]));
    let expanded = apply(&expanded, &Command::Rename {
        element_id: "d".into(), name: "StorageContract".into(),
    }, 3).expect("edit after expanding the semantic endpoint collection");
    let references = ReferenceIndex::new(Model::from_project(&expanded));
    assert_eq!(ids(relationship_endpoints::sources(&references, "multi").expect("all three sources")),
        expected(&["a", "b", "d"]));
    assert_eq!(ids(element_graph_queries::client_dependencies(&references, "d").expect("updated inverse")),
        expected(&["object", "multi"]));
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&expanded, format).expect("save expanded roles"), format)
            .expect("reopen expanded roles");
        assert_eq!(reopened, expanded);
    }
}

#[test]
fn relationship_draft_flow_distinguishes_invalid_unresolved_and_derived_roles_then_recovers() {
    let project = document();
    for value in [
        json!([]), json!([{"$ref":"a"},{"$ref":"a"}]),
        json!([{"$ref":"missing"}]), json!([{"$ref":"comment"}]), json!("a"),
    ] {
        let draft = external_edit(&project, "object", "client", value);
        let references = ReferenceIndex::new(Model::from_project(&draft));
        assert!(matches!(relationship_endpoints::clients(&references, "object"),
            Err(ElementGraphError::Invalid { .. })));
        assert!(Model::from_project(&draft).validate().iter().any(|diagnostic|
            diagnostic.subject_id == "object" && diagnostic.severity == StructuralSeverity::Error));
        let reopened = decode(&encode(&draft, Format::Protobuf).expect("save draft"),
            Format::Protobuf).expect("reopen invalid draft without destroying it");
        assert_eq!(reopened, draft);
        let repaired = external_edit(&reopened, "object", "client", json!([{"$ref":"a"},{"$ref":"d"}]));
        verify_roles(&repaired);
    }
    let external = external_edit(&project, "object", "client", json!([{"href":"other.xmi#Client"}]));
    assert!(matches!(relationship_endpoints::clients(
        &ReferenceIndex::new(Model::from_project(&external)), "object"),
        Err(ElementGraphError::Unsupported { .. })));
    let derived = external_edit(&project, "object", "source", json!([{"$ref":"b"}]));
    assert!(matches!(relationship_endpoints::sources(
        &ReferenceIndex::new(Model::from_project(&derived)), "object"),
        Err(ElementGraphError::Unsupported { .. })));
    let inverse = external_edit(&project, "a", "clientDependency", json!([{"$ref":"binary"}]));
    assert!(element_graph_queries::client_dependencies(
        &ReferenceIndex::new(Model::from_project(&inverse)), "a").is_err());
    let inconsistent = external_edit(&project, "multi", "client", json!([{"$ref":"a"}]));
    let references = ReferenceIndex::new(Model::from_project(&inconsistent));
    assert_eq!(ids(relationship_endpoints::clients(&references, "multi").expect("full semantic role")),
        expected(&["a"]));
    assert!(Model::from_project(&inconsistent).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "multi" && diagnostic.code == "UML_RELATIONSHIP_ENDPOINTS"
            && diagnostic.message.contains("binary projection")));
}

#[test]
fn dependency_specialization_flow_reads_subset_roles_without_duplicate_authoritative_lists() {
    let mut content = serde_json::to_value(document()).expect("document");
    content["elements"].as_array_mut().expect("elements").push(json!({
        "id":"realization", "kind":"uml.interface_realization", "name":"Implementation",
        "properties":{"implementingClassifier":{"$ref":"b"}, "contract":{"$ref":"d"},
                      "visibility":null}
    }));
    content["relations"].as_array_mut().expect("relations").push(json!({
        "id":"owns-realization", "kind":"owns", "from":"b", "to":"realization"
    }));
    let project = decode(&serde_json::to_vec(&content).expect("specialized document"), Format::Json)
        .expect("import specialized relationship");
    let references = ReferenceIndex::new(Model::from_project(&project));
    assert_eq!(ids(relationship_endpoints::clients(&references, "realization")
        .expect("implementingClassifier subsets client")), expected(&["b"]));
    assert_eq!(ids(relationship_endpoints::suppliers(&references, "realization")
        .expect("contract subsets supplier")), expected(&["d"]));
    assert!(element_graph_queries::client_dependencies(&references, "b").expect("inherited inverse")
        .iter().any(|dependency| dependency.id() == "realization"));
    let edited = apply(&project, &Command::Rename {
        element_id: "b".into(), name: "ImplementationService".into(),
    }, 1).expect("edit using derived Dependency roles");
    let expanded = external_edit(&edited, "realization", "client", json!([{"$ref":"a"},{"$ref":"b"}]));
    let references = ReferenceIndex::new(Model::from_project(&expanded));
    assert_eq!(ids(relationship_endpoints::sources(&references, "realization")
        .expect("specialized role remains a subset")), expected(&["a", "b"]));
    let invalid = external_edit(&expanded, "realization", "client", json!([{"$ref":"a"}]));
    assert!(Model::from_project(&invalid).validate().iter().any(|diagnostic|
        diagnostic.subject_id == "realization" && diagnostic.severity == StructuralSeverity::Error
            && diagnostic.message.contains("specialized roles must subset")));
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&expanded, format).expect("save subsets"), format).expect("reopen subsets");
        assert_eq!(reopened, expanded);
        let references = ReferenceIndex::new(Model::from_project(&reopened));
        assert_eq!(ids(relationship_endpoints::sources(&references, "realization").expect("same subsets")),
            expected(&["a", "b"]));
    }
}
