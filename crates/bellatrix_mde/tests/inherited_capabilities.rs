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
        let mut original = project(kind);
        if receptions {
            original.elements.iter_mut().find(|element| element.id == "type-task")
                .expect("classifier").properties.insert("isActive".into(), true.into());
        }
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
    use bellatrix_mde::commands::ModelingMemberKind::{Attribute, Operation, Parameter};
    use bellatrix_mde::uml::{
        classifiers::{self, NamespaceQuery},
        references::ReferenceIndex,
        root::{ElementRef, Model},
    };
    use bellatrix_mde::{Format, decode, encode, validate_structural_model};
    use serde_json::json;

    fn apply(project: &mut Project, command: Command) -> Result<(), String> {
        *project = commands::apply(project, &command, 0)
            .map_err(|error| format!("{command:?}: {error}"))?;
        Ok(())
    }
    fn set_property(
        project: &mut Project,
        id: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), String> {
        apply(
            project,
            Command::SetProperty {
                element_id: id.into(),
                path: format!("properties.{key}"),
                value,
            },
        )
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

    // Detach before changing both lower bounds: an intermediate mismatch is
    // inconsistent under the published Property::isConsistentWith OCL.
    redefine(&mut project, &own_attribute, &[])?;
    for (id, key, value) in [
        (&own_attribute, "type", json!({"$ref": left})),
        (&public, "type", json!({"$ref": ancestor})),
        (&public, "multiplicity", json!("0..3")),
        (&own_attribute, "multiplicity", json!("0..2")),
        (&own_attribute, "aggregation", json!("composite")),
        (&public, "aggregation", json!("composite")),
    ] {
        apply(
            &mut project,
            Command::SetProperty {
                element_id: id.clone(),
                path: format!("properties.{key}"),
                value,
            },
        )?;
    }
    redefine(&mut project, &own_attribute, &[&public])?;
    assert!(
        !validate_structural_model(&project)
            .iter()
            .any(|issue| issue.subject_id == own_attribute
                && issue.code == "UML_REDEFINITION_CONSISTENCY_UNSUPPORTED")
    );
    for (id, key, value) in [
        (&own_attribute, "multiplicity", json!("1..2")),
        (&own_attribute, "aggregation", json!("none")),
        (&public, "type", json!({"$ref": leaf})),
    ] {
        let mut incompatible = project.clone();
        incompatible
            .elements
            .iter_mut()
            .find(|element| &element.id == id)
            .ok_or("Property")?
            .properties
            .insert(key.into(), value);
        let before = encode(&incompatible, Format::Json)?;
        let error = redefine(&mut incompatible, &own_attribute, &[&public])
            .err()
            .ok_or("incompatible Property redefinition accepted")?;
        assert!(error.contains("inconsistent"), "{error}");
        assert_eq!(encode(&incompatible, Format::Json)?, before);
        assert!(
            validate_structural_model(&incompatible)
                .iter()
                .any(|issue| issue.subject_id == own_attribute
                    && issue.severity == bellatrix_mde::StructuralSeverity::Error)
        );
    }
    let mut unevaluated = project.clone();
    unevaluated
        .elements
        .iter_mut()
        .find(|element| element.id == own_attribute)
        .ok_or("Property")?
        .properties
        .insert("multiplicity".into(), json!("limit..*"));
    redefine(&mut unevaluated, &own_attribute, &[&public])?;
    assert!(
        validate_structural_model(&unevaluated)
            .iter()
            .any(|issue| issue.subject_id == own_attribute
                && issue.code == "UML_REDEFINITION_CONSISTENCY_UNSUPPORTED"
                && issue.severity == bellatrix_mde::StructuralSeverity::Warning)
    );

    for (member_id, targets, expected) in [
        (
            &own_attribute,
            vec![own_attribute.clone()],
            "redefine itself",
        ),
        (
            &own_attribute,
            vec![original_operation.clone()],
            "writable redefinition role",
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

    fn configure_import(
        project: &mut Project,
        endpoints: [&str; 3],
        package: bool,
        alias: Option<&str>,
        visibility: &str,
    ) -> Result<(), String> {
        apply(
            project,
            Command::SetNamespaceImport {
                import_id: endpoints[0].into(),
                namespace_id: endpoints[1].into(),
                target_id: endpoints[2].into(),
                is_package: package,
                alias: alias.map(str::to_owned),
                visibility: visibility.into(),
            },
        )
    }
    let mut packages = Vec::new();
    for name in ["Facade", "Consumer", "Observer"] {
        apply(
            &mut project,
            Command::CreateElement {
                diagram_id: "diagram".into(),
                owner_id: Some("model".into()),
                kind: commands::ModelingElementKind::Package,
                name: Some(name.into()),
                x: 0.0,
                y: 0.0,
            },
        )?;
        packages.push(
            project
                .elements
                .iter()
                .find(|element| element.name == name)
                .ok_or("new package")?
                .id
                .clone(),
        );
    }
    let [facade, consumer, observer]: [String; 3] =
        packages.try_into().map_err(|_| "package fixture")?;
    configure_import(
        &mut project,
        ["export-record", &facade, &ancestor],
        false,
        Some("Record"),
        "public",
    )?;
    configure_import(
        &mut project,
        ["private-leaf", &facade, &leaf],
        false,
        Some("HiddenLeaf"),
        "private",
    )?;
    configure_import(
        &mut project,
        ["consume-facade", &consumer, &facade],
        true,
        None,
        "public",
    )?;
    assert_eq!(
        ids(&query(&project, &consumer)?.imported_members),
        [ancestor.as_str()]
    );
    assert_eq!(
        query(&project, &consumer)?.imported_names.get(&ancestor),
        Some(&vec![Some("Record".into())])
    );

    configure_import(
        &mut project,
        ["export-left", &facade, &left],
        false,
        Some("Record"),
        "public",
    )?;
    assert!(query(&project, &consumer)?.imported_members.is_empty());
    assert_eq!(query(&project, &facade)?.excluded_imports.len(), 2);
    configure_import(
        &mut project,
        ["export-left", &facade, &left],
        false,
        Some("LeftRecord"),
        "public",
    )?;
    apply(
        &mut project,
        Command::CreateElement {
            diagram_id: "diagram".into(),
            owner_id: Some(consumer.clone()),
            kind: commands::ModelingElementKind::Class,
            name: Some("Record".into()),
            x: 0.0,
            y: 0.0,
        },
    )?;
    let local = project
        .elements
        .iter()
        .find(|element| element.name == "Record")
        .ok_or("local Class")?
        .id
        .clone();
    assert_eq!(
        ids(&query(&project, &consumer)?.excluded_imports),
        [ancestor.as_str()]
    );
    apply(
        &mut project,
        Command::Rename {
            element_id: local,
            name: "LocalRecord".into(),
        },
    )?;
    configure_import(
        &mut project,
        ["cycle-back", &facade, &consumer],
        true,
        None,
        "public",
    )?;
    assert_eq!(query(&project, &facade)?.imported_members.len(), 4);
    configure_import(
        &mut project,
        ["private-leaf", &facade, &leaf],
        false,
        Some("HiddenLeaf"),
        "public",
    )?;
    configure_import(
        &mut project,
        ["left-alias", &facade, &left],
        false,
        Some("AlternateLeft"),
        "public",
    )?;
    configure_import(
        &mut project,
        ["local-ancestor", &consumer, &ancestor],
        false,
        Some("LocalAncestor"),
        "private",
    )?;
    configure_import(
        &mut project,
        ["observe-consumer", &observer, &consumer],
        true,
        None,
        "public",
    )?;
    configure_import(
        &mut project,
        ["self-name", &observer, &observer],
        false,
        Some("CurrentNamespace"),
        "private",
    )?;
    assert!(!ids(&query(&project, &observer)?.members).contains(&ancestor.as_str()));
    assert!(ids(&query(&project, &observer)?.members).contains(&leaf.as_str()));
    assert_eq!(
        query(&project, &observer)?.imported_names.get(&left),
        Some(&vec![
            Some("AlternateLeft".into()),
            Some("LeftRecord".into())
        ])
    );
    let before = encode(&project, Format::Json)?;
    configure_import(
        &mut project,
        ["export-left", &facade, &left],
        false,
        Some("Record"),
        "public",
    )?;
    assert!(matches!(
        query(&project, &facade),
        Err(classifiers::QueryError::Unsupported { .. })
    ));
    assert!(
        validate_structural_model(&project)
            .iter()
            .any(|issue| issue.code == "UML_NAMESPACE_QUERY_UNSUPPORTED"
                && issue.severity == bellatrix_mde::StructuralSeverity::Warning)
    );
    configure_import(
        &mut project,
        ["export-left", &facade, &left],
        false,
        Some("LeftRecord"),
        "public",
    )?;
    assert_eq!(encode(&project, Format::Json)?, before);
    assert!(
        configure_import(
            &mut project,
            ["import-private", &observer, &right],
            false,
            None,
            "public"
        )
        .is_err()
    );
    assert_eq!(encode(&project, Format::Json)?, before);

    // Continue the same inherited-feature workflow with ordered Operation signatures.
    // UML allows conformance in either Type direction, unlike a fixed language policy.
    redefine(&mut project, &own_operation, &[])?;
    redefine(&mut project, &left_operation, &[])?;
    let mut parameter_groups = Vec::new();
    for (operation, types, ranges) in [
        (
            &original_operation,
            [&ancestor, &left, &ancestor, &left],
            ["1..2", "0..2", "0..*", "0..5"],
        ),
        (
            &left_operation,
            [&left, &ancestor, &left, &ancestor],
            ["0..3", "0..2", "0..3", "1..4"],
        ),
        (
            &own_operation,
            [&ancestor, &ancestor, &ancestor, &ancestor],
            ["0..*", "0..2", "1..2", "2..3"],
        ),
    ] {
        let mut parameters = Vec::new();
        for ((direction, type_id), range) in ["in", "inout", "out", "return"]
            .into_iter()
            .zip(types)
            .zip(ranges)
        {
            apply(
                &mut project,
                Command::AddMember {
                    owner_id: operation.clone(),
                    kind: Parameter,
                },
            )?;
            let parameter = project.elements.last().ok_or("new Parameter")?.id.clone();
            for (key, value) in [
                ("direction", json!(direction)),
                ("type", json!({"$ref": type_id})),
                ("multiplicity", json!(range)),
            ] {
                set_property(&mut project, &parameter, key, value)?;
            }
            parameters.push(parameter);
        }
        parameter_groups.push(parameters);
    }
    redefine(&mut project, &left_operation, &[&original_operation])?;
    redefine(
        &mut project,
        &own_operation,
        &[&original_operation, &left_operation],
    )?;
    let left_input = &parameter_groups[1][0];
    let own_parameters = &parameter_groups[2];
    apply(
        &mut project,
        Command::Rename {
            element_id: own_parameters[0].clone(),
            name: "independentName".into(),
        },
    )?;

    // Reordering a signature changes correspondence, then restoring order repairs it.
    let before = encode(&project, Format::Json)?;
    assert!(
        apply(
            &mut project,
            Command::MoveParameter {
                parameter_id: left_input.clone(),
                up: false
            }
        )
        .is_err()
    );
    assert_eq!(encode(&project, Format::Json)?, before);
    redefine(&mut project, &own_operation, &[])?;
    redefine(&mut project, &left_operation, &[])?;
    apply(
        &mut project,
        Command::MoveParameter {
            parameter_id: left_input.clone(),
            up: false,
        },
    )?;
    let before = encode(&project, Format::Json)?;
    assert!(redefine(&mut project, &left_operation, &[&original_operation]).is_err());
    assert_eq!(encode(&project, Format::Json)?, before);
    apply(
        &mut project,
        Command::MoveParameter {
            parameter_id: left_input.clone(),
            up: true,
        },
    )?;
    redefine(&mut project, &left_operation, &[&original_operation])?;

    // An added untyped input is still a proven count mismatch, not an unknown signature.
    apply(
        &mut project,
        Command::AddMember {
            owner_id: own_operation.clone(),
            kind: Parameter,
        },
    )?;
    let extra = project.elements.last().ok_or("extra Parameter")?.id.clone();
    assert!(
        redefine(
            &mut project,
            &own_operation,
            &[&original_operation, &left_operation]
        )
        .is_err()
    );
    apply(&mut project, Command::DeleteElement { element_id: extra })?;
    redefine(
        &mut project,
        &own_operation,
        &[&original_operation, &left_operation],
    )?;
    for (parameter, key, incompatible, repaired) in [
        (
            &own_parameters[0],
            "multiplicity",
            json!("1..1"),
            json!("0..*"),
        ),
        (
            &own_parameters[1],
            "multiplicity",
            json!("0..3"),
            json!("0..2"),
        ),
        (
            &own_parameters[2],
            "multiplicity",
            json!("0..4"),
            json!("1..2"),
        ),
        (
            &own_parameters[3],
            "multiplicity",
            json!("0..6"),
            json!("2..3"),
        ),
        (&own_parameters[3], "isOrdered", json!(true), json!(false)),
        (&own_parameters[3], "isUnique", json!(false), json!(true)),
        (
            &own_parameters[1],
            "type",
            json!({"$ref": right}),
            json!({"$ref": ancestor}),
        ),
    ] {
        let before = encode(&project, Format::Json)?;
        assert!(set_property(&mut project, parameter, key, incompatible.clone()).is_err());
        assert_eq!(encode(&project, Format::Json)?, before);
        // A loaded incompatible model reports the same problem and can be repaired.
        project
            .elements
            .iter_mut()
            .find(|element| &element.id == parameter)
            .ok_or("Parameter")?
            .properties
            .insert(key.into(), incompatible);
        let before = encode(&project, Format::Json)?;
        let error = redefine(
            &mut project,
            &own_operation,
            &[&original_operation, &left_operation],
        )
        .err()
        .ok_or("incompatible Operation accepted")?;
        assert!(error.contains("inconsistent"), "{error}");
        assert_eq!(encode(&project, Format::Json)?, before);
        assert!(
            validate_structural_model(&project)
                .iter()
                .any(|issue| issue.subject_id == own_operation
                    && issue.severity == bellatrix_mde::StructuralSeverity::Error)
        );
        set_property(&mut project, parameter, key, repaired)?;
        redefine(
            &mut project,
            &own_operation,
            &[&original_operation, &left_operation],
        )?;
    }
    for operation in [&original_operation, &own_operation] {
        let mut contracts = project.clone();
        apply(
            &mut contracts,
            Command::QuickEdit {
                diagram_id: "diagram".into(),
                element_id: operation.clone(),
                action: UmlQuickCreate::TemplateParameter,
            },
        )?;
        set_property(&mut contracts, operation, "isQuery", json!(true))?;
        for element in [
            json!({"id":"operation-condition", "kind":"uml.constraint", "name":"Allowed"}),
            json!({"id":"operation-condition-value", "kind":"uml.literal_boolean", "name":"",
                "properties":{"value":true, "visibility":null}}),
            json!({"id":"operation-method", "kind":"uml.opaque_behavior", "name":"Implementation",
                "properties":{"specification":{"$ref":operation}}}),
        ] {
            contracts.elements.push(serde_json::from_value(element)?);
        }
        for (owner, child) in [
            (operation.as_str(), "operation-condition"),
            ("operation-condition", "operation-condition-value"),
            ("model", "operation-method"),
        ] {
            contracts.relations.push(serde_json::from_value(json!({
                "id":format!("owns-{child}"), "kind":"owns", "from":owner, "to":child,
            }))?);
        }
        // Reference roles have no editable Inspector adapter yet. Load the
        // authored document through public I/O before using semantic commands.
        let authored_operation = contracts
            .elements
            .iter_mut()
            .find(|element| &element.id == operation)
            .ok_or("authored Operation")?;
        for (key, value) in [
            ("precondition", json!([{"$ref":"operation-condition"}])),
            ("postcondition", json!([{"$ref":"operation-condition"}])),
            ("bodyCondition", json!({"$ref":"operation-condition"})),
            ("method", json!([{"$ref":"operation-method"}])),
            ("raisedException", json!([{"$ref":right}])),
        ] {
            authored_operation.properties.insert(key.into(), value);
        }
        contracts = decode(&encode(&contracts, Format::Json)?, Format::Json)?;
        redefine(
            &mut contracts,
            &own_operation,
            &[&original_operation, &left_operation],
        )?;
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&contracts, format)?, format)?;
            assert_eq!(contracts, reopened);
            let references = ReferenceIndex::new(Model::from_project(&reopened));
            assert!(
                bellatrix_mde::uml::operation_signatures::compare_operation_consistency(
                    &references,
                    references.element(&own_operation)?,
                    references.element(&original_operation)?,
                )?
            );
            let diagnostics = validate_structural_model(&reopened);
            assert!(
                diagnostics
                    .iter()
                    .all(|issue| issue.code != "UML_REDEFINITION_CONSISTENCY_UNSUPPORTED"),
                "{diagnostics:?}"
            );
            assert!(
                diagnostics.iter().any(|issue| issue.subject_id == *operation
                    && issue.code == "MDE_OPERATION_CONDITION_UNSUPPORTED")
            );
            let section = ModelingSection::from_model(&reopened);
            let inspector = inspect_element(&section, &own_operation)
                .ok_or("compatible Operation Inspector")?;
            assert!(
                inspector
                    .properties
                    .iter()
                    .all(|field| field.path != "uml.redefinition.status")
            );
            assert!(
                !inspect_element(&section, operation)
                    .ok_or("template Operation Inspector")?
                    .template_parameters
                    .is_empty()
            );
        }

        let before = encode(&contracts, Format::Json)?;
        let error = set_property(&mut contracts, operation, "isQuery", json!(false))
            .err()
            .ok_or("bodyCondition accepted a non-query Operation")?;
        assert!(error.contains("UML_OPERATION_BODY_CONDITION"), "{error}");
        let error = set_property(&mut contracts, &own_parameters[3], "isUnique", json!(false))
            .err()
            .ok_or("incompatible signature accepted with contracts")?;
        assert!(error.contains("inconsistent"), "{error}");
        assert!(redefine(&mut contracts, &original_operation, &[&own_operation]).is_err());
        assert_eq!(encode(&contracts, Format::Json)?, before);

        let references = ReferenceIndex::new(Model::from_project(&contracts));
        let ordered = bellatrix_mde::uml::operations::owned_parameters(&references, operation)?
            .into_iter()
            .map(|parameter| json!({"$ref":parameter.id}))
            .collect::<Vec<_>>();
        let mut projected = contracts.clone();
        projected
            .elements
            .iter_mut()
            .find(|element| &element.id == operation)
            .ok_or("Operation with explicit parameter projection")?
            .properties
            .insert("ownedParameter".into(), json!(ordered));
        projected = decode(&encode(&projected, Format::Json)?, Format::Json)?;
        redefine(
            &mut projected,
            &own_operation,
            &[&original_operation, &left_operation],
        )?;
        let reopened = decode(&encode(&projected, Format::Protobuf)?, Format::Protobuf)?;
        let references = ReferenceIndex::new(Model::from_project(&reopened));
        assert!(
            bellatrix_mde::uml::operation_signatures::compare_operation_consistency(
                &references,
                references.element(&own_operation)?,
                references.element(&original_operation)?,
            )?
        );
        projected
            .elements
            .iter_mut()
            .find(|element| &element.id == operation)
            .ok_or("Operation with explicit parameter projection")?
            .properties
            .insert("ownedParameter".into(), json!(ordered.into_iter().rev().collect::<Vec<_>>()));
        for format in [Format::Json, Format::Protobuf] {
            let invalid = decode(&encode(&projected, format)?, format)?;
            assert!(
                Model::from_project(&invalid).validate().iter().any(|issue|
                    issue.subject_id == *operation
                        && issue.code == "UML_PARAMETER_OWNERSHIP_STORAGE"
                        && issue.message.contains("canonical owns order and membership")
                        && issue.severity == bellatrix_mde::StructuralSeverity::Error)
            );
            let references = ReferenceIndex::new(Model::from_project(&invalid));
            assert!(matches!(
                bellatrix_mde::uml::operation_signatures::compare_operation_consistency(
                    &references,
                    references.element(&own_operation)?,
                    references.element(&original_operation)?,
                ),
                Err(classifiers::QueryError::Invalid { .. })
            ));
        }

        // A read-only Inspector field is not evidence of semantic rejection.
        // Imported drafts must reach each independent validator and keep their
        // formal signature consistency result separate from that violation.
        for (key, value, code, rule) in [
            (
                "bodyCondition",
                json!({"$ref":"operation-method"}),
                "UML_OPERATION_BODY_CONDITION",
                "must conform to UML Constraint",
            ),
            (
                "raisedException",
                json!([{"$ref":"operation-condition"}]),
                "UML_PROPERTY_CONTRACT",
                "Operation-raisedException",
            ),
        ] {
            let mut draft = contracts.clone();
            draft
                .elements
                .iter_mut()
                .find(|element| &element.id == operation)
                .ok_or("draft Operation")?
                .properties
                .insert(key.into(), value);
            for format in [Format::Json, Format::Protobuf] {
                let invalid = decode(&encode(&draft, format)?, format)?;
                let diagnostics = Model::from_project(&invalid).validate();
                assert!(
                    diagnostics.iter().any(|issue| issue.subject_id == *operation
                        && issue.code == code
                        && issue.message.contains(rule)
                        && issue.severity == bellatrix_mde::StructuralSeverity::Error),
                    "{diagnostics:?}"
                );
                assert!(
                    diagnostics
                        .iter()
                        .all(|issue| issue.code != "UML_REDEFINITION_CONSISTENCY_UNSUPPORTED")
                );
                let references = ReferenceIndex::new(Model::from_project(&invalid));
                assert!(
                    bellatrix_mde::uml::operation_signatures::compare_operation_consistency(
                        &references,
                        references.element(&own_operation)?,
                        references.element(&original_operation)?,
                    )?
                );
            }
        }
        assert_eq!(encode(&contracts, Format::Json)?, before);

        let condition = contracts
            .elements
            .iter_mut()
            .find(|element| element.id == "operation-condition-value")
            .ok_or("condition expression")?;
        condition.kind = "uml.literal_integer".into();
        condition.properties.insert("value".into(), json!(1));
        let invalid = decode(&encode(&contracts, Format::Json)?, Format::Json)?;
        let diagnostics = validate_structural_model(&invalid);
        assert!(
            diagnostics.iter().any(|issue| issue.subject_id == "operation-condition"
                && issue.code == "UML_CONSTRAINT_BOOLEAN"
                && issue.severity == bellatrix_mde::StructuralSeverity::Error)
        );
        assert!(
            diagnostics
                .iter()
                .all(|issue| issue.code != "UML_REDEFINITION_CONSISTENCY_UNSUPPORTED")
        );
        let condition = contracts
            .elements
            .iter_mut()
            .find(|element| element.id == "operation-condition-value")
            .ok_or("condition expression")?;
        condition.kind = "uml.literal_boolean".into();
        condition.properties.insert("value".into(), json!(false));
        contracts = decode(&encode(&contracts, Format::Json)?, Format::Json)?;
        apply(
            &mut contracts,
            Command::Rename {
                element_id: "operation-condition".into(),
                name: "MayExecute".into(),
            },
        )?;
        let repaired = decode(&encode(&contracts, Format::Protobuf)?, Format::Protobuf)?;
        assert!(
            validate_structural_model(&repaired)
                .iter()
                .all(|issue| issue.code != "UML_CONSTRAINT_BOOLEAN"
                    && issue.code != "UML_REDEFINITION_CONSISTENCY_UNSUPPORTED")
        );
    }
    // Authored expressions survive storage without being interpreted as valid/invalid.
    let mut unevaluated_operation = project.clone();
    set_property(
        &mut unevaluated_operation,
        &own_parameters[3],
        "multiplicity",
        json!("limit..*"),
    )?;
    redefine(
        &mut unevaluated_operation,
        &own_operation,
        &[&original_operation, &left_operation],
    )?;
    let unevaluated_operation =
        decode(&encode(&unevaluated_operation, Format::Json)?, Format::Json)?;
    assert!(
        validate_structural_model(&unevaluated_operation)
            .iter()
            .any(|issue| issue.subject_id == own_operation
                && issue.code == "UML_REDEFINITION_CONSISTENCY_UNSUPPORTED"
                && issue.severity == bellatrix_mde::StructuralSeverity::Warning)
    );
    let unknown_inspector = inspect_element(
        &ModelingSection::from_model(&unevaluated_operation),
        &own_operation,
    )
    .ok_or("Operation Inspector")?;
    assert!(
        unknown_inspector
            .properties
            .iter()
            .any(|field| field.path == "uml.redefinition.status")
    );

    redefine(&mut project, &own_attribute, &[&public])?;
    let expected = ids(&query(&project, &leaf)?.members)
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&project, format)?, format)?;
        assert_eq!(ids(&query(&reopened, &leaf)?.members), expected);
        let reopened_imports = query(&reopened, &observer)?;
        assert!(!ids(&reopened_imports.members).contains(&ancestor.as_str()));
        assert_eq!(
            reopened_imports.imported_names.get(&left),
            Some(&vec![
                Some("AlternateLeft".into()),
                Some("LeftRecord".into())
            ])
        );
        let import_model = Model::from_project(&reopened);
        assert_eq!(
            import_model
                .owner(
                    import_model
                        .element("consume-facade")
                        .ok_or("PackageImport")?
                )?
                .map(|owner| owner.id()),
            Some(consumer.as_str())
        );
        let import_inspector = inspect_element(&ModelingSection::from_model(&reopened), &consumer)
            .ok_or("import Inspector")?;
        assert!(
            import_inspector
                .properties
                .iter()
                .any(|field| field.path == "uml.importedNames" && !field.editable)
        );
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
                .all(|issue| issue.code != "UML_REDEFINITION_CONSISTENCY_UNSUPPORTED")
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

#[test]
fn parameter_relations_follow_canonical_roles_stream_edits_and_reopened_drafts()
-> Result<(), Box<dyn std::error::Error>> {
    use bellatrix_mde::{Format, StructuralSeverity, decode, encode, uml::root::Model};
    use serde_json::{Value, json};

    fn edit_document(
        project: &Project,
        subject: &str,
        property: &str,
        value: Value,
    ) -> Result<Project, Box<dyn std::error::Error>> {
        let mut document: Value = serde_json::from_slice(&encode(project, Format::Json)?)?;
        let element = document["elements"].as_array_mut().ok_or("elements")?
            .iter_mut().find(|element| element["id"] == subject).ok_or("subject")?;
        element["properties"][property] = value;
        Ok(decode(&serde_json::to_vec(&document)?, Format::Json)?)
    }

    fn parameter_error(project: &Project, rule: &str) -> bool {
        Model::from_project(project).validate().iter().any(|issue|
            issue.subject_id == "parameter"
                && issue.severity == StructuralSeverity::Error
                && issue.message.contains(rule))
    }

    let mut project = decode(&serde_json::to_vec(&json!({
        "id":"project", "name":"Parameter collaboration", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"class", "kind":"uml.class", "name":"Service"},
            {"id":"operation", "kind":"uml.operation", "name":"call"},
            {"id":"behavior", "kind":"uml.opaque_behavior", "name":"Execute", "properties":{}},
            {"id":"parameter", "kind":"uml.parameter", "name":"input",
             "properties":{"type":{"$ref":"class"}}},
            {"id":"property", "kind":"uml.attribute", "name":"peer"},
            {"id":"collaboration", "kind":"uml.collaboration", "name":"Session", "properties":{}},
            {"id":"connector", "kind":"uml.connector", "name":"channel"},
            {"id":"end", "kind":"uml.connector_end", "name":"",
             "properties":{"role":{"$ref":"property"}}},
            {"id":"other-end", "kind":"uml.connector_end", "name":"",
             "properties":{"role":{"$ref":"property"}}}
        ],
        "relations":[
            {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
            {"id":"owns-operation", "kind":"owns", "from":"class", "to":"operation"},
            {"id":"owns-behavior", "kind":"owns", "from":"root", "to":"behavior"},
            {"id":"owns-parameter", "kind":"owns", "from":"behavior", "to":"parameter"},
            {"id":"owns-property", "kind":"owns", "from":"class", "to":"property"},
            {"id":"owns-collaboration", "kind":"owns", "from":"root", "to":"collaboration"},
            {"id":"owns-connector", "kind":"owns", "from":"class", "to":"connector"},
            {"id":"owns-end", "kind":"owns", "from":"connector", "to":"end"},
            {"id":"owns-other-end", "kind":"owns", "from":"connector", "to":"other-end"}
        ]
    }))?, Format::Json)?;
    assert!(!parameter_error(&project, "Parameter-reentrant_behaviors"));
    assert!(!parameter_error(&project, "Parameter-connector_end"));

    // The inspector can edit authored flags; omitted Behavior::isReentrant is
    // still effectively true when this Parameter becomes a stream.
    project = edit_document(&project, "parameter", "isStream", json!(false))?;
    let stream = Command::SetProperty {
        element_id: "parameter".into(), path: "properties.isStream".into(), value: json!(true),
    };
    let before = encode(&project, Format::Json)?;
    let error = commands::apply(&project, &stream, 0).err().ok_or("accepted reentrant stream")?;
    assert!(error.contains("Parameter-reentrant_behaviors"), "{error}");
    assert_eq!(encode(&project, Format::Json)?, before);
    project = edit_document(&project, "behavior", "isReentrant", json!(false))?;
    project = commands::apply(&project, &stream, 1)?;
    assert!(!parameter_error(&project, "Parameter-reentrant_behaviors"));

    let reentrant = Command::SetProperty {
        element_id: "behavior".into(), path: "properties.isReentrant".into(), value: json!(true),
    };
    let before = encode(&project, Format::Json)?;
    let error = commands::apply(&project, &reentrant, 2).err().ok_or("accepted reentrant Behavior")?;
    assert!(error.contains("Parameter-reentrant_behaviors"), "{error}");
    assert_eq!(encode(&project, Format::Json)?, before);
    let draft = edit_document(&project, "behavior", "isReentrant", json!(true))?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&draft, format)?, format)?;
        assert_eq!(draft, reopened);
        assert!(parameter_error(&reopened, "Parameter-reentrant_behaviors"));
        project = commands::apply(&reopened, &Command::SetProperty {
            element_id: "behavior".into(), path: "properties.isReentrant".into(), value: json!(false),
        }, 3)?;
        assert!(!parameter_error(&project, "Parameter-reentrant_behaviors"));
    }

    // Changing the canonical owner to an Operation removes the Behavior role.
    project = commands::apply(&project, &Command::SetOwner {
        element_id: "parameter".into(), owner_id: Some("operation".into()),
    }, 4)?;
    project = commands::apply(&project, &reentrant, 5)?;
    assert!(!parameter_error(&project, "Parameter-reentrant_behaviors"));
    let before = encode(&project, Format::Json)?;
    let return_to_behavior = Command::SetOwner {
        element_id: "parameter".into(), owner_id: Some("behavior".into()),
    };
    let error = commands::apply(&project, &return_to_behavior, 6)
        .err().ok_or("accepted a stream in a reentrant owner")?;
    assert!(error.contains("Parameter-reentrant_behaviors"), "{error}");
    assert_eq!(encode(&project, Format::Json)?, before);
    project = commands::apply(&project, &Command::SetProperty {
        element_id: "behavior".into(), path: "properties.isReentrant".into(), value: json!(false),
    }, 7)?;
    project = commands::apply(&project, &return_to_behavior, 8)?;

    // A ConnectorEnd points to the Parameter even when no stored `end` exists.
    // Collaboration participation is a typed reference, not its lexical owner.
    let draft = edit_document(&project, "end", "role", json!({"$ref":"parameter"}))?;
    assert!(parameter_error(&draft, "Parameter-connector_end"));
    let unrelated = edit_document(&draft, "other-end", "role", json!({"$ref":"missing"}))?;
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&unrelated, format)?, format)?;
        assert!(parameter_error(&reopened, "Parameter-connector_end"));
        reopened.elements.reverse();
        assert!(parameter_error(&reopened, "Parameter-connector_end"));
        assert!(Model::from_project(&reopened).validate().iter().any(|issue|
            issue.subject_id == "other-end" && issue.code == "UML_PROPERTY_CONTRACT"));
    }
    let unresolved = edit_document(&project, "end", "role", json!({"$ref":"missing"}))?;
    assert!(!parameter_error(&unresolved, "Parameter-connector_end"));
    assert!(Model::from_project(&unresolved).validate().iter().any(|issue|
        issue.subject_id == "parameter" && issue.code == "MDE_PARAMETER_CONSTRAINT_UNVERIFIED"
            && issue.message.contains("Parameter-connector_end")));
    let forged = edit_document(&draft, "parameter", "collaboration", json!([{"$ref":"collaboration"}]))?;
    assert!(parameter_error(&forged, "Parameter-connector_end"));
    assert!(Model::from_project(&forged).validate().iter().any(|issue|
        issue.subject_id == "parameter" && issue.code == "MDE_PARAMETER_CONSTRAINT_UNVERIFIED"));
    let reopened = decode(&encode(&draft, Format::Protobuf)?, Format::Protobuf)?;
    assert!(parameter_error(&reopened, "Parameter-connector_end"));
    project = edit_document(&reopened, "collaboration", "collaborationRole", json!([{"$ref":"parameter"}]))?;
    assert!(!parameter_error(&project, "Parameter-connector_end"));
    let unrelated = edit_document(&project, "other-end", "role", json!({"$ref":"missing"}))?;
    let before = encode(&unrelated, Format::Json)?;
    let error = commands::apply(&unrelated, &Command::DeleteElement {
        element_id: "collaboration".into(),
    }, 9).err().ok_or("accepted removal of the Parameter's only Collaboration")?;
    assert!(error.contains("Parameter-connector_end"), "{error}");
    assert_eq!(encode(&unrelated, Format::Json)?, before);
    assert_eq!(Model::from_project(&project).owner(
        Model::from_project(&project).element("parameter").ok_or("parameter")?
    )?.map(|owner| owner.id()), Some("behavior"));

    for (subject, property, value, unverified) in [
        ("collaboration", "collaborationRole", json!([{"$ref":"missing"}]), true),
        ("collaboration", "collaborationRole", json!([{"$ref":"class"}]), true),
        ("collaboration", "collaborationRole", json!([{"$ref":"parameter"},{"$ref":"parameter"}]), false),
        ("collaboration", "collaborationRole", json!([{"$ref":"missing"},{"$ref":"parameter"}]), false),
        ("end", "role", json!({"$ref":"class"}), false),
        ("end", "role", json!({"$ref":"missing"}), false),
        ("end", "role", json!({"href":"external.uml#parameter"}), false),
    ] {
        let invalid = edit_document(&project, subject, property, value)?;
        let reopened = decode(&encode(&invalid, Format::Json)?, Format::Json)?;
        let diagnostics = Model::from_project(&reopened).validate();
        assert_eq!(diagnostics.iter().any(|issue|
            issue.subject_id == "parameter"
                && issue.code == "MDE_PARAMETER_CONSTRAINT_UNVERIFIED"
                && issue.message.contains("Parameter-connector_end")), unverified, "{diagnostics:?}");
        assert!(diagnostics.iter().any(|issue|
            issue.subject_id == subject
                && matches!(issue.code, "UML_PROPERTY_CONTRACT" | "MDE_PROPERTY_CONTRACT_UNVERIFIED")),
            "{diagnostics:?}");
        assert!(!parameter_error(&reopened, "Parameter-connector_end"));
    }
    let wrong_boolean = edit_document(&project, "behavior", "isReentrant", json!("false"))?;
    assert!(Model::from_project(&wrong_boolean).validate().iter().any(|issue|
        issue.subject_id == "parameter" && issue.code == "MDE_PARAMETER_CONSTRAINT_UNVERIFIED"
            && issue.message.contains("Parameter-reentrant_behaviors")));
    let forged = edit_document(&project, "parameter", "behavior", json!({"$ref":"class"}))?;
    assert!(Model::from_project(&forged).validate().iter().any(|issue|
        issue.subject_id == "parameter" && issue.code == "UML_PARAMETER_OWNERSHIP_STORAGE"));
    let exception = edit_document(&project, "parameter", "direction", json!("return"))?;
    let exception = edit_document(&exception, "parameter", "isException", json!(true))?;
    assert!(parameter_error(&exception, "Parameter::stream_and_exception"));

    let mut ordered = project.clone();
    ordered.relations.iter_mut().find(|relation| relation.id == "owns-parameter")
        .ok_or("Parameter owner")?.from = "operation".into();
    let mut second_parameter = ordered.element("parameter").ok_or("Parameter")?.clone();
    second_parameter.id = "second-parameter".into();
    second_parameter.name = "otherInput".into();
    ordered.elements.push(second_parameter);
    ordered.relations.push(serde_json::from_value(json!({
        "id":"owns-second-parameter", "kind":"owns", "from":"operation", "to":"second-parameter"
    }))?);
    ordered = edit_document(&ordered, "operation", "ownedParameter",
        json!([{"$ref":"parameter"},{"$ref":"second-parameter"}]))?;
    assert!(!Model::from_project(&ordered).validate().iter().any(|issue|
        issue.subject_id == "operation" && issue.code == "UML_PARAMETER_OWNERSHIP_STORAGE"));
    let before = encode(&ordered, Format::Json)?;
    let error = commands::apply(&ordered, &Command::MoveParameter {
        parameter_id: "parameter".into(), up: false,
    }, 10).err().ok_or("accepted divergent explicit Parameter order")?;
    assert!(error.contains("canonical owns order"), "{error}");
    assert_eq!(encode(&ordered, Format::Json)?, before);
    let divergent = edit_document(&ordered, "operation", "ownedParameter",
        json!([{"$ref":"second-parameter"},{"$ref":"parameter"}]))?;
    assert!(Model::from_project(&divergent).validate().iter().any(|issue|
        issue.subject_id == "operation" && issue.code == "UML_PARAMETER_OWNERSHIP_STORAGE"
            && issue.message.contains("canonical owns order")));

    project = edit_document(&project, "behavior", "ownedParameter", json!([{"$ref":"parameter"}]))?;
    assert!(!Model::from_project(&project).validate().iter().any(|issue|
        issue.subject_id == "behavior" && issue.code == "UML_PARAMETER_OWNERSHIP_STORAGE"));
    let error = commands::apply(&project, &reentrant, 11)
        .err().ok_or("accepted reentrant Behavior with explicit Parameter role")?;
    assert!(error.contains("Parameter-reentrant_behaviors"), "{error}");

    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&project, format)?, format)?;
        assert_eq!(project, reopened);
        assert!(!parameter_error(&reopened, "Parameter-connector_end"));
        assert!(!parameter_error(&reopened, "Parameter-reentrant_behaviors"));
        let parameter = reopened.element("parameter").ok_or("same Parameter identity")?;
        assert!(parameter.properties.get("isStream").and_then(Value::as_bool) == Some(true));
        assert!(["end", "collaboration", "behavior"].iter().all(|key| !parameter.properties.contains_key(*key)));
    }
    Ok(())
}
