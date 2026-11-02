use bellatrix_mde_model::{Project, Relation};
use bellatrix_mde_modeling::{Graph, GraphIndex, Scope, select_elements};

fn fixture() -> Project {
    serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap()
}

#[test]
fn index_keeps_typed_and_directed_relations() {
    let project = fixture();
    let graph = GraphIndex::new(&project);
    assert_eq!(graph.node_count(), project.elements.len());
    assert_eq!(graph.edge_count(), project.relations.len());
    assert_eq!(graph.kinds().get("depends_on"), Some(&1));
    assert_eq!(
        graph.outgoing("type-list", Some("depends_on")).unwrap()[0].to,
        "type-task"
    );
    assert!(
        graph
            .outgoing("type-task", Some("depends_on"))
            .unwrap()
            .is_empty()
    );
    let path = graph
        .shortest_path("type-list", "fn-add", Some("owns"))
        .unwrap()
        .unwrap();
    assert_eq!(
        path.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        vec!["owns-fn-add"]
    );
    assert!(
        graph
            .shortest_path("fn-add", "type-list", Some("owns"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn subgraph_follows_hierarchy_only_not_dependencies() {
    let project = fixture();
    let scope = select_elements(&project, &Scope::Elements(vec!["type-list".into()])).unwrap();
    assert!(scope.contains("fn-add"));
    assert!(!scope.contains("type-task"));
    assert!(!scope.contains("file-task"));
}

#[test]
fn dependency_cycles_are_visible_but_not_invalid_global_graphs() {
    let mut project = fixture();
    project.relations.push(Relation {
        id: "back-reference".into(),
        kind: "depends_on".into(),
        from: "type-task".into(),
        to: "type-list".into(),
        properties: Default::default(),
    });
    Graph::new(&project).validate().unwrap();
    assert_eq!(
        GraphIndex::new(&project).cyclic_components("depends_on"),
        vec![vec!["type-list".to_owned(), "type-task".to_owned()]]
    );
    assert!(
        GraphIndex::new(&project)
            .cyclic_components("owns")
            .is_empty()
    );
}

#[test]
fn semantic_ownership_cycle_is_rejected() {
    let mut project = fixture();
    project.relations.push(Relation {
        id: "cyclic-owner".into(),
        kind: "owns".into(),
        from: "fn-add".into(),
        to: "type-list".into(),
        properties: Default::default(),
    });
    assert!(Graph::new(&project).validate().is_err());
}

#[test]
fn unknown_nodes_cannot_be_queried_as_existing() {
    let project = fixture();
    let graph = GraphIndex::new(&project);
    assert!(
        graph
            .shortest_path("not-in-bellatrix-mde-model", "type-task", None)
            .is_err()
    );
    assert!(!graph.contains("not-in-bellatrix-mde-model"));
}
