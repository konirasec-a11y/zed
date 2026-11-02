use bellatrix_mde::{Graph, GraphError, Project, Relation};

#[test]
fn rust_ownership_and_containment_are_validated() {
    let mut project: Project = serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap();
    project.relations.push(Relation { id: "second-owner".into(), kind: "owns".into(), from: "type-task".into(), to: "fn-add".into(), properties: Default::default() });
    assert!(matches!(Graph::new(&project).validate(), Err(GraphError::MultipleOwners(_))));
    project.relations.pop();
    project.relations.push(Relation { id: "invalid-nesting".into(), kind: "owns".into(), from: "type-list".into(), to: "dir-src".into(), properties: Default::default() });
    assert!(matches!(Graph::new(&project).validate(), Err(GraphError::InvalidRelation(_))));
}
