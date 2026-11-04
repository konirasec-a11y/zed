use bellatrix_mde::{Graph, GraphIndex, Project, Relation, Scope, select_elements};

fn fixture() -> Project {
    serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap()
}

#[test]
fn index_keeps_typed_and_directed_relations() {
    let project = fixture();
    let graph = GraphIndex::new(&project);
    assert_eq!(graph.node_count(), project.elements.len() + 1);
    assert!(
        project
            .relations
            .iter()
            .filter(|relation| relation.kind == "depends_on")
            .all(|relation| graph.contains(&relation.id))
    );
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
        path.iter()
            .map(|relation| relation.id.as_str())
            .collect::<Vec<_>>(),
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
            .shortest_path("not-in-model", "type-task", None)
            .is_err()
    );
    assert!(!graph.contains("not-in-model"));
}

#[test]
fn uml_root_namespace_edit_and_reopen_flow_preserves_canonical_identity() {
    use bellatrix_mde::commands::{self, Command, ModelingElementKind, ModelingMemberKind};
    use bellatrix_mde::uml::root::{
        DirectedRelationship, Element, Model, NamedElement, Namespace, Relationship, VisibilityKind,
    };
    use bellatrix_mde::uml::{element_graph_queries as queries, references::ReferenceIndex};
    use bellatrix_mde::{Format, ModelingRelationKind, ModelingSection, UmlQuickCreate};
    use serde_json::json;

    let apply =
        |project: &Project, command| commands::apply(project, &command, 0).expect("public command");
    let create = |kind, name: &str| Command::CreateElement {
        diagram_id: "diagram".into(),
        owner_id: None,
        kind,
        name: Some(name.into()),
        x: 0.0,
        y: 0.0,
    };
    let original: Project = serde_json::from_value(json!({"id":"foundation", "name":"Authored project",
        "elements":[{"id":"old-note", "kind":"uml.note", "name":"Authored note", "description":"Separate documentation"},
            {"id":"empty-note", "kind":"uml.comment", "name":"Metadata", "properties":{"body":""}},
            {"id":"frame", "kind":"uml.frame", "name":"Editorial frame", "properties":{"custom":"preserve"}}],
        "diagrams":[{"id":"diagram", "name":"D", "uml_di_type":"UMLClassDiagram", "views":[{"id":"old-view", "subject":"old-note", "geometry":{"kind":"rect", "x":12.0,"y":13.0,"width":130.0,"height":80.0}}]}]
    })).expect("authored draft");
    let mut project = apply(&original, create(ModelingElementKind::Class, "Order"));
    let class_id = project.elements.last().expect("created class").id.clone();
    let root_id = project.root_model.clone().expect("actual Model");
    assert_eq!(project.diagrams[0].views[0], original.diagrams[0].views[0]);
    assert_eq!(project.element("frame"), original.element("frame"));
    assert_eq!(
        project.element("old-note").expect("legacy note").name,
        "Authored note"
    );
    assert_eq!(
        Model::from_project(&project)
            .element("old-note")
            .expect("note identity")
            .comment()
            .expect("Comment")
            .body(),
        Some("Authored note")
    );
    assert_eq!(
        Model::from_project(&project)
            .element("empty-note")
            .expect("note identity")
            .comment()
            .expect("Comment")
            .body(),
        Some("")
    );
    assert!(
        Model::from_project(&project)
            .element("frame")
            .expect("extension identity")
            .named()
            .is_none()
    );
    project = apply(
        &project,
        Command::AddMember {
            owner_id: class_id.clone(),
            kind: ModelingMemberKind::Attribute,
        },
    );
    let attribute_id = project.elements.last().expect("attribute").id.clone();
    project = apply(&project, create(ModelingElementKind::Class, "Item"));
    let other_id = project.elements.last().expect("second class").id.clone();
    project = apply(
        &project,
        Command::CreateRelation { owner_id: None,
            diagram_id: "diagram".into(),
            kind: ModelingRelationKind::Association,
            from: class_id.clone(),
            to: other_id.clone(),
        },
    );
    let association_id = project
        .relations
        .iter()
        .find(|relation| relation.kind == "uml.association")
        .expect("Association")
        .id
        .clone();
    project = apply(
        &project,
        Command::CreateRelation { owner_id: None,
            diagram_id: "diagram".into(),
            kind: ModelingRelationKind::Dependency,
            from: class_id.clone(),
            to: other_id.clone(),
        },
    );
    let dependency_id = project
        .relations
        .iter()
        .find(|relation| relation.kind == "uml.dependency")
        .expect("Dependency")
        .id
        .clone();
    project = apply(
        &project,
        Command::QuickEdit {
            diagram_id: "diagram".into(),
            element_id: class_id.clone(),
            action: UmlQuickCreate::Note,
        },
    );
    let note_id = project.elements.last().expect("created Comment").id.clone();
    project = apply(
        &project,
        Command::SetProperty {
            element_id: note_id.clone(),
            path: "properties.body".into(),
            value: json!("Explains Order"),
        },
    );
    let model = Model::from_project(&project);
    let root = model.root().expect("valid root").expect("root Model");
    assert_eq!(root.id(), root_id);
    let class = model
        .element(&class_id)
        .expect("Class")
        .named()
        .expect("NamedElement");
    assert_eq!(class.visibility(), Some(VisibilityKind::Public));
    assert_eq!(
        class.qualified_name(model).expect("qualified name"),
        Some("Model::Order".into())
    );
    assert_eq!(
        class.owner(model).expect("owner").expect("owned").id(),
        root_id
    );
    assert!(
        root.owned_members(model)
            .expect("members")
            .iter()
            .any(|member| member.id() == association_id)
    );
    assert!(
        class
            .owned_elements(model)
            .expect("ownedElements")
            .iter()
            .any(|element| element.id() == attribute_id)
    );
    let references = ReferenceIndex::new(model);
    let descendants = queries::all_owned_elements(&references, &root_id).expect("owned closure");
    assert!(
        descendants
            .iter()
            .any(|element| element.id() == attribute_id)
    );
    assert!(descendants.iter().any(|element| element.id() == note_id));
    assert!(!descendants.iter().any(|element| element.id() == root_id));
    assert_eq!(
        queries::all_namespaces(&references, &class_id)
            .expect("namespaces")
            .iter()
            .map(|element| element.id())
            .collect::<Vec<_>>(),
        vec![root_id.as_str()]
    );
    assert_eq!(
        queries::all_owning_packages(&references, &class_id)
            .expect("owning packages")
            .iter()
            .map(|element| element.id())
            .collect::<Vec<_>>(),
        vec![root_id.as_str()]
    );
    assert_eq!(
        queries::client_dependencies(&references, &class_id)
            .expect("client dependencies")
            .iter()
            .map(|element| element.id())
            .collect::<Vec<_>>(),
        vec![dependency_id.as_str()]
    );
    assert!(
        queries::client_dependencies(&references, &other_id)
            .expect("supplier dependencies")
            .is_empty()
    );
    let comment = class.owned_comments(model).expect("ownedComment")[0];
    assert_eq!(comment.body(), Some("Explains Order"));
    assert_eq!(
        comment.annotated_elements(model).expect("annotatedElement")[0].id(),
        class_id
    );
    assert!(comment.as_element().named().is_none());
    let association = model
        .element(&association_id)
        .expect("relationship is Element");
    assert!(association.conforms_to("Classifier"));
    assert!(association.directed_relationship().is_none());
    let mut directed_notation = project
        .relations
        .iter()
        .find(|relation| relation.id == association_id)
        .expect("Association record")
        .clone();
    directed_notation.kind = "uml.directed_association".into();
    assert_eq!(
        bellatrix_mde::relationship_root_kind(&directed_notation),
        Some(bellatrix_mde::RootMetaclass::Relationship)
    );
    assert_eq!(bellatrix_mde::element_root_kind("uml.frame"), None);
    assert_eq!(
        bellatrix_mde::element_root_kind("uml.comment"),
        Some(bellatrix_mde::RootMetaclass::Comment)
    );
    assert_eq!(
        association
            .relationship()
            .expect("Relationship")
            .related_elements(model)
            .expect("relatedElement")
            .len(),
        2
    );
    let dependency = model
        .element(&dependency_id)
        .expect("Dependency")
        .directed_relationship()
        .expect("DirectedRelationship");
    assert_eq!(dependency.sources(model).expect("source")[0].id(), class_id);
    assert_eq!(dependency.targets(model).expect("target")[0].id(), other_id);
    // Legacy extension payloads remain unverified, rather than silently valid.
    let diagnostics = model.validate();
    assert_eq!(diagnostics.iter().map(|issue| (issue.severity, issue.code, issue.subject_id.as_str(), issue.message.as_str())).collect::<Vec<_>>(), vec![
        (bellatrix_mde::StructuralSeverity::Warning, "MDE_PROPERTY_CONTRACT_UNVERIFIED", "frame", "uml.frame: unknown UML metaclass"),
        (bellatrix_mde::StructuralSeverity::Warning, "MDE_PROPERTY_CONTRACT_UNVERIFIED", attribute_id.as_str(), "declaration: no normative property or registered extension contract"),
        (bellatrix_mde::StructuralSeverity::Warning, "MDE_PROPERTY_CONTRACT_UNVERIFIED", association_id.as_str(), "sourceRole: no normative property or registered extension contract"),
        (bellatrix_mde::StructuralSeverity::Warning, "MDE_PROPERTY_CONTRACT_UNVERIFIED", association_id.as_str(), "targetRole: no normative property or registered extension contract"),
    ]);
    assert!(
        commands::apply(
            &project,
            &Command::SetOwner {
                element_id: attribute_id.clone(),
                owner_id: Some(note_id.clone())
            },
            0
        )
        .is_err()
    );
    assert!(
        commands::apply(
            &project,
            &Command::Rename {
                element_id: other_id.clone(),
                name: "Order".into()
            },
            0
        )
        .expect_err("duplicate names must remain invalid")
        .starts_with("UML_NAMESPACE_MEMBERS_DISTINGUISHABLE:")
    );
    assert!(
        commands::apply(
            &project,
            &Command::SetProperty {
                element_id: class_id.clone(),
                path: "properties.visibility".into(),
                value: json!("protected")
            },
            0
        )
        .is_err()
    );
    assert!(
        commands::apply(
            &project,
            &Command::SetProperty {
                element_id: class_id.clone(),
                path: "uml.qualifiedName".into(),
                value: json!("forged")
            },
            0
        )
        .is_err()
    );
    project = apply(&project, create(ModelingElementKind::Package, "Sales"));
    let package_id = project.elements.last().expect("Package").id.clone();
    project = apply(
        &project,
        Command::SetOwner {
            element_id: class_id.clone(),
            owner_id: Some(package_id.clone()),
        },
    );
    project = apply(
        &project,
        Command::Rename {
            element_id: class_id.clone(),
            name: "Invoice".into(),
        },
    );
    assert_eq!(
        Model::from_project(&project)
            .element(&class_id)
            .expect("same identity")
            .named()
            .expect("NamedElement")
            .qualified_name(Model::from_project(&project))
            .expect("derived"),
        Some("Model::Sales::Invoice".into())
    );
    let references = ReferenceIndex::new(Model::from_project(&project));
    assert_eq!(
        queries::all_namespaces(&references, &class_id)
            .expect("nested namespaces")
            .iter()
            .map(|element| element.id())
            .collect::<Vec<_>>(),
        vec![package_id.as_str(), root_id.as_str()]
    );
    assert_eq!(
        queries::all_owning_packages(&references, &class_id)
            .expect("nested packages")
            .iter()
            .map(|element| element.id())
            .collect::<Vec<_>>(),
        vec![package_id.as_str(), root_id.as_str()]
    );
    assert!(
        queries::all_owning_packages(&references, &attribute_id)
            .expect("class is not a package")
            .is_empty()
    );
    assert!(
        commands::apply(
            &project,
            &Command::SetOwner {
                element_id: package_id.clone(),
                owner_id: Some(package_id.clone())
            },
            0
        )
        .is_err()
    );
    project = apply(
        &project,
        Command::SetNameExpression {
            element_id: class_id.clone(),
            expression: "".into(),
        },
    );
    assert_eq!(
        Model::from_project(&project)
            .element(&class_id)
            .expect("Class")
            .named()
            .expect("NamedElement")
            .qualified_name(Model::from_project(&project))
            .expect("unnamed"),
        None
    );
    let section = ModelingSection::from_model(&project);
    let inspector = bellatrix_mde::inspect_element(&section, &note_id).expect("Comment inspector");
    assert_eq!(inspector.display_label, "Explains Order");
    assert_eq!(
        bellatrix_mde::project_hierarchy(&section)
            .iter()
            .find(|row| row.element_id == root_id)
            .expect("real root row")
            .depth,
        0
    );
    for format in [Format::Json, Format::Protobuf] {
        let reopened = bellatrix_mde::decode(
            &bellatrix_mde::encode(&project, format).expect("save"),
            format,
        )
        .expect("reopen");
        assert_eq!(reopened, project);
        assert_eq!(Model::from_project(&reopened).validate(), diagnostics);
        let reopened_index = ReferenceIndex::new(Model::from_project(&reopened));
        assert_eq!(
            queries::all_namespaces(&reopened_index, &class_id)
                .expect("reopened namespaces")
                .iter()
                .map(|element| element.id())
                .collect::<Vec<_>>(),
            vec![package_id.as_str(), root_id.as_str()]
        );
        assert_eq!(
            queries::client_dependencies(&reopened_index, &class_id)
                .expect("reopened dependencies")
                .iter()
                .map(|element| element.id())
                .collect::<Vec<_>>(),
            vec![dependency_id.as_str()]
        );
    }
    let note_link = project
        .relations
        .iter()
        .find(|relation| relation.kind == "uml.note_link" && relation.to == note_id)
        .expect("annotation link")
        .id
        .clone();
    assert_eq!(
        bellatrix_mde::relationship_root_kind(
            project
                .relations
                .iter()
                .find(|relation| relation.id == note_link)
                .expect("editor link")
        ),
        None
    );
    project = apply(
        &project,
        Command::DeleteElement {
            element_id: note_link.clone(),
        },
    );
    assert!(
        !project
            .relations
            .iter()
            .any(|relation| relation.id == note_link)
    );
    assert!(
        project.element(&note_id).is_some(),
        "deleting a displayed property link preserves its Comment"
    );
    project = apply(
        &project,
        Command::DeleteElement {
            element_id: association_id.clone(),
        },
    );
    assert!(
        Model::from_project(&project)
            .element(&association_id)
            .is_none()
    );
    assert_eq!(
        project.element(&class_id).expect("endpoint survives").id,
        class_id
    );
    assert!(original.root_model.is_none());
    assert!(
        !original
            .element("old-note")
            .expect("input unmodified")
            .properties
            .contains_key("body")
    );
}
