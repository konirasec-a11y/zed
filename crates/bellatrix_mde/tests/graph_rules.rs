use bellatrix_mde::{Graph, GraphError, Project, Relation};

#[test]
fn rust_ownership_and_containment_are_validated() {
    let mut project: Project = serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap();
    project.relations.push(Relation {
        id: "second-owner".into(),
        kind: "owns".into(),
        from: "type-task".into(),
        to: "fn-add".into(),
        properties: Default::default(),
    });
    assert!(matches!(
        Graph::new(&project).validate(),
        Err(GraphError::MultipleOwners(_))
    ));
    project.relations.pop();
    project.relations.push(Relation {
        id: "invalid-nesting".into(),
        kind: "owns".into(),
        from: "type-list".into(),
        to: "dir-src".into(),
        properties: Default::default(),
    });
    assert!(matches!(
        Graph::new(&project).validate(),
        Err(GraphError::InvalidRelation(_))
    ));
}

#[test]
fn slot_owner_transition_uses_one_canonical_edge_through_commands_and_roundtrips()
-> Result<(), Box<dyn std::error::Error>> {
    use bellatrix_mde::commands::{self, Command};
    use bellatrix_mde::uml::{
        root::{Element, Model},
        schema,
    };
    use bellatrix_mde::{Format, StructuralSeverity, decode, encode, validate_structural_model};
    use serde_json::json;

    let mut project: Project = serde_json::from_value(json!({
        "id": "instances", "name": "Instances", "root_model": "model",
        "elements": [
            {"id": "model", "kind": "uml.model", "name": "Model"},
            {"id": "class", "kind": "uml.class", "name": "Record"},
            {"id": "other-class", "kind": "uml.class", "name": "Other"},
            {"id": "feature", "kind": "uml.attribute", "name": "value"},
            {"id": "other-feature", "kind": "uml.attribute", "name": "other"},
            {"id": "first", "kind": "uml.instance_specification", "name": "first", "properties": {"classifier": [{"$ref": "class"}]}},
            {"id": "second", "kind": "uml.instance_specification", "name": "second", "properties": {"classifier": [{"$ref": "class"}]}},
            {"id": "slot", "kind": "uml.slot", "name": "", "properties": {"definingFeature": {"$ref": "feature"}}}
        ],
        "relations": [
            {"id": "owns-class", "kind": "owns", "from": "model", "to": "class"},
            {"id": "owns-other-class", "kind": "owns", "from": "model", "to": "other-class"},
            {"id": "owns-first", "kind": "owns", "from": "model", "to": "first"},
            {"id": "owns-second", "kind": "owns", "from": "model", "to": "second"},
            {"id": "owns-feature", "kind": "owns", "from": "class", "to": "feature"},
            {"id": "owns-other-feature", "kind": "owns", "from": "other-class", "to": "other-feature"}
        ]
    }))?;
    let slot_contract =
        schema::uml_property("InstanceSpecification", "slot")?.ok_or("slot contract")?;
    assert_eq!((slot_contract.lower, slot_contract.upper), ("0", "*"));
    assert_eq!(slot_contract.aggregation, schema::Aggregation::Composite);
    let owner_contract = schema::uml_property("Slot", "owningInstance")?.ok_or("owner contract")?;
    assert_eq!((owner_contract.lower, owner_contract.upper), ("1", "1"));
    assert_eq!(
        owner_contract
            .opposites
            .first()
            .and_then(|reference| reference.target),
        Some(slot_contract.id)
    );

    let assign = |owner: Option<&str>| Command::SetOwner {
        element_id: "slot".into(),
        owner_id: owner.map(str::to_owned),
    };
    let owner_of = |project: &Project| -> Result<String, String> {
        let model = Model::from_project(project);
        Ok(model
            .element("slot")
            .and_then(|element| element.slot())
            .ok_or("slot")?
            .owning_instance(model)?
            .id()
            .into())
    };
    let reject = |project: &Project,
                  command: Command,
                  expected: &str|
     -> Result<(), Box<dyn std::error::Error>> {
        let before = encode(project, Format::Json)?;
        let owner = owner_of(project)?;
        let error = commands::apply(project, &command, 99)
            .err()
            .ok_or("command unexpectedly succeeded")?;
        assert!(error.contains(expected), "{error}");
        assert_eq!(encode(project, Format::Json)?, before);
        assert_eq!(owner_of(project)?, owner);
        Ok(())
    };
    project = commands::apply(&project, &assign(Some("first")), 1)?;
    assert_eq!(owner_of(&project)?, "first");
    let ownership_id = project
        .relations
        .iter()
        .find(|relation| relation.kind == "owns" && relation.to == "slot")
        .ok_or("ownership edge")?
        .id
        .clone();
    for owner in [None, Some("missing"), Some("class")] {
        reject(&project, assign(owner), "existing InstanceSpecification")?;
    }
    let moved = commands::apply(&project, &assign(Some("second")), 2)?;
    assert_eq!(owner_of(&project)?, "first");
    assert_eq!(owner_of(&moved)?, "second");
    let model = Model::from_project(&moved);
    let first = model
        .element("first")
        .and_then(|element| element.instance_specification())
        .ok_or("first instance")?;
    let second = model
        .element("second")
        .and_then(|element| element.instance_specification())
        .ok_or("second instance")?;
    assert!(first.slots(model)?.is_empty());
    assert_eq!(
        second
            .slots(model)?
            .iter()
            .map(|slot| slot.id())
            .collect::<Vec<_>>(),
        ["slot"]
    );
    assert_eq!(
        second
            .classifiers(model)?
            .first()
            .map(|element| element.id()),
        Some("class")
    );
    let edges = moved
        .relations
        .iter()
        .filter(|relation| relation.kind == "owns" && relation.to == "slot")
        .collect::<Vec<_>>();
    assert_eq!(edges.len(), 1);
    assert_eq!(
        edges.first().map(|relation| &relation.id),
        Some(&ownership_id)
    );
    assert_eq!(commands::apply(&moved, &assign(Some("second")), 3)?, moved);
    assert!(
        model
            .validate()
            .iter()
            .all(|issue| issue.severity != StructuralSeverity::Error)
    );
    let aggregate_issues = validate_structural_model(&moved);
    assert!(
        aggregate_issues
            .iter()
            .all(|issue| issue.severity != StructuralSeverity::Error),
        "{aggregate_issues:?}"
    );
    assert!(
        aggregate_issues
            .iter()
            .all(|issue| issue.code != "MDE_UML_METACLASS_UNSUPPORTED")
    );
    for format in [Format::Json, Format::Protobuf] {
        let restored = decode(&encode(&moved, format)?, format)?;
        assert_eq!(restored, moved);
        assert_eq!(owner_of(&restored)?, "second");
        assert!(
            !restored
                .element("slot")
                .ok_or("slot")?
                .properties
                .contains_key("owningInstance")
        );
        assert!(
            !restored
                .element("second")
                .ok_or("second")?
                .properties
                .contains_key("slot")
        );
    }

    for (value, expected) in [
        (json!({"$ref": "missing"}), "unknown Element reference"),
        (json!({"$ref": "class"}), "StructuralFeature"),
    ] {
        let mut invalid = moved.clone();
        invalid
            .elements
            .iter_mut()
            .find(|element| element.id == "slot")
            .ok_or("slot")?
            .properties
            .insert("definingFeature".into(), value);
        reject(&invalid, assign(Some("first")), expected)?;
    }
    for value in [
        json!([]),
        json!([{"$ref": "missing"}]),
        json!([{"$ref": "feature"}]),
        json!([{"$ref": "class"}, {"$ref": "class"}]),
    ] {
        let mut invalid = moved.clone();
        invalid
            .elements
            .iter_mut()
            .find(|element| element.id == "first")
            .ok_or("first instance")?
            .properties
            .insert("classifier".into(), value);
        reject(&invalid, assign(Some("first")), "classifier")?;
    }
    for value in [None, Some(json!(null))] {
        let mut invalid = moved.clone();
        let properties = &mut invalid
            .elements
            .iter_mut()
            .find(|element| element.id == "slot")
            .ok_or("slot")?
            .properties;
        match value {
            Some(value) => {
                properties.insert("definingFeature".into(), value);
            }
            None => {
                properties.remove("definingFeature");
            }
        }
        reject(&invalid, assign(Some("second")), "definingFeature")?;
    }
    let mut unrelated_feature = moved.clone();
    unrelated_feature
        .elements
        .iter_mut()
        .find(|element| element.id == "slot")
        .ok_or("slot")?
        .properties
        .insert("definingFeature".into(), json!({"$ref": "other-feature"}));
    reject(
        &unrelated_feature,
        assign(Some("first")),
        "Unsupported Slot capability",
    )?;
    let mut parallel_storage = moved.clone();
    parallel_storage
        .elements
        .iter_mut()
        .find(|element| element.id == "slot")
        .ok_or("slot")?
        .properties
        .insert("owningInstance".into(), json!("first"));
    reject(
        &parallel_storage,
        assign(Some("first")),
        "canonical ownership edge",
    )?;
    let mut stored_inverse = moved.clone();
    stored_inverse
        .elements
        .iter_mut()
        .find(|element| element.id == "second")
        .ok_or("second instance")?
        .properties
        .insert("slot".into(), json!(["slot"]));
    reject(
        &stored_inverse,
        assign(Some("first")),
        "canonical ownership edge",
    )?;
    let mut malformed_instance = moved.clone();
    malformed_instance
        .elements
        .iter_mut()
        .find(|element| element.id == "first")
        .ok_or("first instance")?
        .properties
        .insert("classifier".into(), json!([{"$ref": "missing"}]));
    assert!(
        validate_structural_model(&malformed_instance)
            .iter()
            .any(|issue| issue.subject_id == "first"
                && issue.code == "UML_INSTANCE_CLASSIFIER"
                && issue.severity == StructuralSeverity::Error)
    );
    malformed_instance.root_model = None;
    malformed_instance
        .elements
        .iter_mut()
        .find(|element| element.id == "slot")
        .ok_or("slot")?
        .properties
        .insert("definingFeature".into(), json!({"$ref": "missing"}));
    let rootless_issues = validate_structural_model(&malformed_instance);
    for (identifier, code) in [
        ("first", "UML_INSTANCE_CLASSIFIER"),
        ("slot", "UML_SLOT_CONTRACT"),
    ] {
        assert!(
            rootless_issues
                .iter()
                .any(|issue| issue.subject_id == identifier
                    && issue.code == code
                    && issue.severity == StructuralSeverity::Error)
        );
    }
    for (identifier, property, value) in [
        ("slot", "value", json!(["unevaluated-value"])),
        ("first", "specification", json!("unevaluated-specification")),
    ] {
        let mut unevaluated = moved.clone();
        unevaluated
            .elements
            .iter_mut()
            .find(|element| element.id == identifier)
            .ok_or("value owner")?
            .properties
            .insert(property.into(), value);
        reject(
            &unevaluated,
            assign(Some("first")),
            "Unsupported Slot capability",
        )?;
        assert!(
            validate_structural_model(&unevaluated)
                .iter()
                .any(|issue| issue.subject_id == identifier
                    && issue.code.ends_with("CAPABILITY_UNSUPPORTED")
                    && issue.severity == StructuralSeverity::Warning)
        );
    }

    let mut duplicate = moved.clone();
    let mut other_slot = duplicate.element("slot").ok_or("slot")?.clone();
    other_slot.id = "other-slot".into();
    duplicate.elements.push(other_slot);
    duplicate.relations.push(Relation {
        id: "owns-other-slot".into(),
        kind: "owns".into(),
        from: "first".into(),
        to: "other-slot".into(),
        properties: Default::default(),
    });
    reject(&duplicate, assign(Some("first")), "same definingFeature")?;

    let mut unsupported = moved.clone();
    unsupported.relations.push(Relation {
        id: "inherits".into(),
        kind: "uml.generalization".into(),
        from: "class".into(),
        to: "other-class".into(),
        properties: Default::default(),
    });
    reject(
        &unsupported,
        assign(Some("first")),
        "Unsupported Slot capability",
    )?;
    assert!(
        validate_structural_model(&unsupported)
            .iter()
            .any(|issue| issue.code == "MDE_SLOT_CAPABILITY_UNSUPPORTED"
                && issue.severity == StructuralSeverity::Warning)
    );
    assert!(
        Model::from_project(&unsupported)
            .validate()
            .iter()
            .all(|issue| issue.code != "UML_SLOT_CONTRACT")
    );
    Ok(())
}
