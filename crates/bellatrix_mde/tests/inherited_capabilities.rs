use bellatrix_mde::commands::{self, Command, ModelingMemberKind};
use bellatrix_mde::{
    ModelingSection, Project, UmlCapability, UmlQuickCreate, UmlType, inspect_element,
    quick_edit_for,
};

fn project(kind: &str) -> Project {
    let mut project: Project =
        serde_json::from_str(include_str!("../fixtures/todo.json")).expect("fixture");
    project
        .elements
        .iter_mut()
        .find(|element| element.id == "type-task")
        .expect("subject")
        .kind = kind.into();
    project
}

#[test]
fn inherited_members_use_the_same_contract_in_commands_and_quick_edit() {
    for (kind, attributes, operations, receptions, ports, parts) in [
        ("uml.component", true, true, true, true, true),
        ("uml.artifact", true, true, false, false, false),
        ("uml.signal", true, false, false, false, false),
        ("uml.collaboration", true, false, false, false, true),
        ("uml.node", true, true, true, true, true),
        ("uml.device", true, true, true, true, true),
        ("uml.execution_environment", true, true, true, true, true),
        ("uml.stereotype", true, true, true, true, true),
        ("uml.nary_association", false, false, false, false, false),
        ("uml.package", false, false, false, false, false),
        ("uml.model", false, false, false, false, false),
    ] {
        let original = project(kind);
        let before = original.clone();
        let semantic_type = UmlType::from_kind(kind).expect("type");
        for (member, capability, action, supported) in [
            (
                ModelingMemberKind::Attribute,
                UmlCapability::Attribute,
                "uml.add-attribute",
                attributes,
            ),
            (
                ModelingMemberKind::Operation,
                UmlCapability::Operation,
                "uml.add-operation",
                operations,
            ),
            (
                ModelingMemberKind::Reception,
                UmlCapability::Reception,
                "uml.add-reception",
                receptions,
            ),
        ] {
            assert_eq!(
                semantic_type.supports(capability),
                supported,
                "{kind} {capability:?}"
            );
            assert_eq!(
                quick_edit_for(kind).iter().any(|entry| entry.id == action),
                supported,
                "{kind} {action}"
            );
            let result = commands::apply(
                &original,
                &Command::AddMember {
                    owner_id: "type-task".into(),
                    kind: member,
                },
                0,
            );
            assert_eq!(result.is_ok(), supported, "{kind} {member:?}: {result:?}");
            if let Ok(updated) = result {
                let member = updated.elements.last().expect("new member");
                assert!(
                    updated
                        .relations
                        .iter()
                        .any(|relation| relation.kind == "owns"
                            && relation.from == "type-task"
                            && relation.to == member.id)
                );
            }
        }
        for (action, supported) in [(UmlQuickCreate::Port, ports), (UmlQuickCreate::Part, parts)] {
            let action_id = match action {
                UmlQuickCreate::Port => "uml.add-port",
                UmlQuickCreate::Part => "uml.add-part",
                _ => unreachable!(),
            };
            assert_eq!(
                quick_edit_for(kind)
                    .iter()
                    .any(|entry| entry.id == action_id),
                supported
            );
            let result = commands::apply(
                &original,
                &Command::QuickEdit {
                    diagram_id: "diagram-tasks".into(),
                    element_id: "type-task".into(),
                    action,
                },
                0,
            );
            assert_eq!(result.is_ok(), supported, "{kind} {action:?}: {result:?}");
        }
        assert_eq!(original, before);
    }
}

#[test]
fn inspector_respects_inherited_metaclass_properties() {
    for (kind, classifier, active) in [
        ("uml.component", true, true),
        ("uml.artifact", true, false),
        ("uml.interface", true, false),
        ("uml.collaboration", true, false),
        ("uml.package", false, false),
        ("uml.model", false, false),
    ] {
        let section = ModelingSection::from_model(&project(kind));
        let inspector = inspect_element(&section, "type-task").expect("inspector");
        for (field, expected) in [
            ("isAbstract", classifier),
            ("isFinalSpecialization", classifier),
            ("isActive", active),
        ] {
            assert_eq!(
                inspector
                    .properties
                    .iter()
                    .any(|property| property.path == format!("properties.{field}")),
                expected,
                "{kind}: {field}"
            );
        }
    }
}

#[test]
fn legacy_template_schema_does_not_claim_a_normative_metaclass() {
    assert_eq!(UmlType::Template.metaclass(), None);
    let section = ModelingSection::from_model(&project("uml.template"));
    let inspector = inspect_element(&section, "type-task").expect("legacy inspector");
    for field in ["stereotype", "visibility", "isAbstract", "isLeaf"] {
        assert!(
            inspector
                .properties
                .iter()
                .any(|property| property.path == format!("properties.{field}"))
        );
    }
}

// Namespace/Classifier fixture patterns adapted from uml4net at
// 8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d, Apache-2.0,
// Copyright 2019-2026 Starion Group S.A. See references/uml4net/ for attribution.
#[test]
fn namespace_queries_follow_hierarchy_commands_redefinition_and_reopen()
-> Result<(), Box<dyn std::error::Error>> {
    use bellatrix_mde::commands::ModelingMemberKind::{Attribute, Operation};
    use bellatrix_mde::uml::{
        classifiers::{self, NamespaceQuery},
        references::ReferenceIndex,
        root::{ElementRef, Model},
    };
    use bellatrix_mde::{Format, decode, encode, validate_structural_model};
    use serde_json::json;

    fn apply(project: &mut Project, command: Command) -> Result<(), String> {
        *project = commands::apply(project, &command, 0)?;
        Ok(())
    }
    fn visibility(project: &mut Project, id: &str, value: &str) -> Result<(), String> {
        apply(
            project,
            Command::SetProperty {
                element_id: id.into(),
                path: "properties.visibility".into(),
                value: json!(value),
            },
        )
    }
    fn redefine(project: &mut Project, id: &str, targets: &[&str]) -> Result<(), String> {
        apply(
            project,
            Command::SetRedefinedElements {
                element_id: id.into(),
                redefined_ids: targets.iter().map(|id| (*id).into()).collect(),
            },
        )
    }
    fn query<'a>(
        project: &'a Project,
        id: &str,
    ) -> Result<NamespaceQuery<'a>, classifiers::QueryError> {
        classifiers::query(&ReferenceIndex::new(Model::from_project(project)), id)
    }
    fn ids<'a>(members: &[ElementRef<'a>]) -> Vec<&'a str> {
        members.iter().map(|member| member.id()).collect()
    }
    let mut project: Project = serde_json::from_value(json!({
        "id":"kernel", "name":"Kernel", "root_model":"model",
        "elements":[
            {"id":"model", "kind":"uml.model", "name":"Root"},
            {"id":"ancestor", "kind":"uml.class", "name":"Ancestor"},
            {"id":"left", "kind":"uml.class", "name":"Left"},
            {"id":"right", "kind":"uml.class", "name":"Right"},
            {"id":"leaf", "kind":"uml.class", "name":"Leaf"},
            {"id":"public", "kind":"uml.attribute", "name":"public", "properties":{"visibility":"public"}},
            {"id":"private", "kind":"uml.attribute", "name":"private", "properties":{"visibility":"private"}},
            {"id":"protected", "kind":"uml.attribute", "name":"protected", "properties":{"visibility":"protected"}},
            {"id":"package", "kind":"uml.attribute", "name":"package", "properties":{"visibility":"package"}},
            {"id":"left-attribute", "kind":"uml.attribute", "name":"leftValue"},
            {"id":"right-attribute", "kind":"uml.attribute", "name":"rightValue"},
            {"id":"original", "kind":"uml.operation", "name":"original", "properties":{"visibility":"public"}},
            {"id":"replacement", "kind":"uml.operation", "name":"replacement", "properties":{"visibility":"public"}}
        ],
        "relations":[],
        "diagrams":[{"id":"diagram", "name":"Classes", "uml_di_type":"UMLClassDiagram", "width":1200, "height":800}]
    }))?;
    for (owner, children) in [
        ("model", &["ancestor", "left", "right", "leaf"][..]),
        (
            "ancestor",
            &["public", "private", "protected", "package", "original"][..],
        ),
        ("left", &["left-attribute", "replacement"][..]),
        ("right", &["right-attribute"][..]),
    ] {
        for child in children {
            apply(
                &mut project,
                Command::SetOwner {
                    element_id: (*child).into(),
                    owner_id: Some(owner.into()),
                },
            )?;
        }
    }
    for (child, parent) in [
        ("left", "ancestor"),
        ("right", "ancestor"),
        ("leaf", "left"),
        ("leaf", "right"),
    ] {
        project.relations.push(bellatrix_mde::Relation {
            id: format!("{child}-{parent}"),
            kind: "uml.generalization".into(),
            from: child.into(),
            to: parent.into(),
            properties: Default::default(),
        });
    }
    let [
        ancestor,
        left,
        right,
        leaf,
        public,
        private,
        protected,
        package,
        left_attribute,
        right_attribute,
        original_operation,
        left_operation,
    ] = [
        "ancestor",
        "left",
        "right",
        "leaf",
        "public",
        "private",
        "protected",
        "package",
        "left-attribute",
        "right-attribute",
        "original",
        "replacement",
    ]
    .map(str::to_owned);
    apply(
        &mut project,
        Command::AddMember {
            owner_id: leaf.clone(),
            kind: Attribute,
        },
    )?;
    let own_attribute = project.elements.last().ok_or("new attribute")?.id.clone();
    redefine(&mut project, &left_operation, &[&original_operation])?;

    let result = query(&project, &leaf)?;
    assert_eq!(ids(&result.owned_members), vec![own_attribute.as_str()]);
    let classifier = result.classifier.ok_or("Classifier query")?;
    assert_eq!(ids(&classifier.features), vec![own_attribute.as_str()]);
    assert_eq!(ids(&classifier.attributes), vec![own_attribute.as_str()]);
    assert_eq!(
        ids(&classifier.all_attributes),
        vec![
            own_attribute.as_str(),
            left_attribute.as_str(),
            right_attribute.as_str(),
            public.as_str(),
            protected.as_str(),
            package.as_str()
        ]
    );
    assert!(!ids(&classifier.inherited_members).contains(&private.as_str()));
    assert_eq!(
        ids(&classifier.inherited_members)
            .iter()
            .filter(|id| **id == public)
            .count(),
        1
    );
    // A diamond's other branch still contributes the original: inherit() only
    // removes targets of THIS classifier's owned redefiners, as the XMI states.
    assert!(ids(&classifier.inherited_members).contains(&original_operation.as_str()));
    assert!(ids(&classifier.inherited_members).contains(&left_operation.as_str()));
    let left_query = query(&project, &left)?
        .classifier
        .ok_or("left Classifier")?;
    assert!(!ids(&left_query.inherited_members).contains(&original_operation.as_str()));
    assert!(ids(&query(&project, &ancestor)?.owned_members).contains(&private.as_str()));

    apply(
        &mut project,
        Command::AddMember {
            owner_id: leaf.clone(),
            kind: Operation,
        },
    )?;
    let own_operation = project.elements.last().ok_or("new operation")?.id.clone();
    redefine(
        &mut project,
        &own_operation,
        &[&original_operation, &left_operation],
    )?;
    redefine(&mut project, &own_attribute, &[&public])?;
    let result = query(&project, &leaf)?
        .classifier
        .ok_or("leaf Classifier")?;
    assert!(!ids(&result.inherited_members).contains(&original_operation.as_str()));
    assert!(!ids(&result.inherited_members).contains(&left_operation.as_str()));
    assert!(!ids(&result.all_attributes).contains(&public.as_str()));
    assert!(ids(&result.all_features).contains(&own_operation.as_str()));

    for (member_id, targets, expected) in [
        (
            &own_attribute,
            vec![own_attribute.clone()],
            "redefine itself",
        ),
        (
            &own_attribute,
            vec![original_operation.clone()],
            "wrong metaclass",
        ),
        (
            &own_attribute,
            vec![public.clone(), public.clone()],
            "unique",
        ),
        (&own_attribute, vec!["unknown".into()], "unknown Element"),
        (
            &left_attribute,
            vec![right_attribute.clone()],
            "ancestor Classifier",
        ),
    ] {
        let before = encode(&project, Format::Json)?;
        let error = commands::apply(
            &project,
            &Command::SetRedefinedElements {
                element_id: member_id.clone(),
                redefined_ids: targets,
            },
            0,
        )
        .err()
        .ok_or("redefinition unexpectedly accepted")?;
        assert!(error.contains(expected), "{error}");
        assert_eq!(encode(&project, Format::Json)?, before);
    }

    visibility(&mut project, &protected, "private")?;
    visibility(&mut project, &private, "public")?;
    let result = query(&project, &leaf)?
        .classifier
        .ok_or("leaf Classifier")?;
    assert!(!ids(&result.all_attributes).contains(&protected.as_str()));
    assert!(ids(&result.all_attributes).contains(&private.as_str()));
    redefine(&mut project, &own_attribute, &[])?;
    assert!(
        ids(&query(&project, &leaf)?
            .classifier
            .ok_or("leaf Classifier")?
            .all_attributes)
        .contains(&public.as_str())
    );
    visibility(&mut project, &right, "private")?;
    let package_query = query(&project, "model")?;
    assert!(ids(&package_query.members).contains(&right.as_str()));
    assert!(!ids(&package_query.visible_members.ok_or("Package query")?).contains(&right.as_str()));

    let expected = ids(&query(&project, &leaf)?.members)
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&project, format)?, format)?;
        assert_eq!(ids(&query(&reopened, &leaf)?.members), expected);
        let diagnostics = validate_structural_model(&reopened);
        assert!(
            diagnostics
                .iter()
                .all(|issue| issue.severity != bellatrix_mde::StructuralSeverity::Error),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|issue| issue.code == "UML_REDEFINITION_CONSISTENCY_UNSUPPORTED")
        );
        let section = ModelingSection::from_model(&reopened);
        let inspector = inspect_element(&section, &leaf).ok_or("Inspector")?;
        let field = inspector
            .properties
            .iter()
            .find(|field| field.path == "uml.allAttributes")
            .ok_or("derived Inspector field")?;
        assert!(!field.editable);
        assert!(
            field
                .value
                .as_array()
                .is_some_and(|values| values.contains(&json!({"$ref": private})))
        );
        assert!(
            reopened
                .elements
                .iter()
                .all(|element| !element.properties.contains_key("inheritedMember"))
        );
    }
    Ok(())
}

