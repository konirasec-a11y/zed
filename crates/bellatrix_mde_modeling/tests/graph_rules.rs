use bellatrix_mde_model::{Project, Relation};
use bellatrix_mde_modeling::{Graph, GraphError};

#[test]
fn rust_ownership_and_containment_are_validated() {
    let mut p: Project = serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
    p.relations.push(Relation {
        id: "second-owner".into(),
        kind: "owns".into(),
        from: "type-task".into(),
        to: "fn-add".into(),
        properties: Default::default(),
    });
    assert!(matches!(
        Graph::new(&p).validate(),
        Err(GraphError::MultipleOwners(_))
    ));
    p.relations.pop();
    p.relations.push(Relation {
        id: "invalid-nesting".into(),
        kind: "owns".into(),
        from: "type-list".into(),
        to: "dir-src".into(),
        properties: Default::default(),
    });
    assert!(matches!(
        Graph::new(&p).validate(),
        Err(GraphError::InvalidRelation(_))
    ));
}
