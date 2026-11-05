use bellatrix_mde::{Project, Relation};

#[test]
fn slot_owner_transition_uses_one_canonical_edge_through_commands_and_roundtrips()
-> Result<(), Box<dyn std::error::Error>> {
    use bellatrix_mde::commands::{self, Command};
    use bellatrix_mde::uml::{
        classifier_hierarchy, classifiers,
        references::ReferenceIndex,
        root::{Element, Model},
        schema,
    };
    use bellatrix_mde::{Format, decode, encode};
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
        "not a slottable feature",
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

    let references = ReferenceIndex::new(Model::from_project(&moved));
    assert!(references.element("class")?.conforms_to("Class"));
    assert!(references.element("other-class")?.conforms_to("Class"));
    assert!(!classifier_hierarchy::conforms_to(
        &references,
        "class",
        "other-class"
    )?);
    let mut inherited = moved.clone();
    inherited.relations.push(Relation {
        id: "inherits".into(),
        kind: "uml.generalization".into(),
        from: "class".into(),
        to: "other-class".into(),
        properties: Default::default(),
    });
    inherited = commands::apply(
        &inherited,
        &Command::SetProperty {
            element_id: "other-feature".into(),
            path: "properties.visibility".into(),
            value: json!("private"),
        },
        5,
    )?;
    inherited
        .elements
        .iter_mut()
        .find(|element| element.id == "slot")
        .ok_or("slot")?
        .properties
        .insert("definingFeature".into(), json!({"$ref": "other-feature"}));
    inherited = commands::apply(&inherited, &assign(Some("first")), 6)?;
    assert_eq!(owner_of(&inherited)?, "first");
    let references = ReferenceIndex::new(Model::from_project(&inherited));
    assert!(classifier_hierarchy::conforms_to(
        &references,
        "class",
        "other-class"
    )?);
    assert!(classifier_hierarchy::conforms_to(
        &references,
        "class",
        "class"
    )?);
    let query = classifiers::query(&references, "class")?;
    assert!(
        !query
            .members
            .iter()
            .any(|member| member.id() == "other-feature")
    );
    let classifier = query.classifier.ok_or("Class query")?;
    assert_eq!(
        classifier
            .all_parents
            .iter()
            .map(|parent| parent.id())
            .collect::<Vec<_>>(),
        ["other-class"]
    );
    assert!(
        classifier
            .all_slottable_features?
            .iter()
            .any(|feature| feature.id() == "other-feature")
    );
    // A final leaf still inherits and supplies Slots, but an existing parent
    // cannot become final while this Generalization remains.
    reject(
        &inherited,
        Command::SetProperty {
            element_id: "other-class".into(),
            path: "properties.isFinalSpecialization".into(),
            value: json!(true),
        },
        "non_final_parents",
    )?;
    let mut final_component = commands::apply(
        &inherited,
        &Command::SetProperty {
            element_id: "class".into(),
            path: "properties.isFinalSpecialization".into(),
            value: json!(true),
        },
        7,
    )?;
    // Reopen a Component specialization of a Class: compatible metaclass kinds
    // are broader than identity equality, while instance ancestry is unchanged.
    final_component
        .elements
        .iter_mut()
        .find(|element| element.id == "class")
        .ok_or("specific")?
        .kind = "uml.component".into();
    final_component = commands::apply(&final_component, &assign(Some("second")), 7)?;
    assert_eq!(owner_of(&final_component)?, "second");
    let mut incompatible_kind = final_component.clone();
    incompatible_kind
        .elements
        .iter_mut()
        .find(|element| element.id == "other-class")
        .ok_or("parent")?
        .kind = "uml.interface".into();
    reject(&incompatible_kind, assign(Some("first")), "specialize_type")?;

    incompatible_kind
        .elements
        .iter_mut()
        .find(|element| element.id == "other-class")
        .ok_or("parent")?
        .kind = "uml.class".into();
    let repaired_kind = commands::apply(&incompatible_kind, &assign(Some("first")), 7)?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&repaired_kind, format)?, format)?;
        let moved = commands::apply(&reopened, &assign(Some("second")), 7)?;
        assert_eq!(owner_of(&moved)?, "second");
    }

    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&inherited, format)?, format)?;
        let transferred = commands::apply(&reopened, &assign(Some("second")), 7)?;
        assert_eq!(owner_of(&transferred)?, "second");
    }

    inherited
        .elements
        .iter_mut()
        .find(|element| element.id == "slot")
        .ok_or("slot")?
        .properties
        .insert("definingFeature".into(), json!({"$ref": "feature"}));
    inherited = commands::apply(
        &inherited,
        &Command::SetRedefinedElements {
            element_id: "feature".into(),
            redefined_ids: vec!["other-feature".into()],
        },
        9,
    )?;
    inherited = commands::apply(&inherited, &assign(Some("second")), 10)?;
    let mut obsolete = inherited.clone();
    obsolete
        .elements
        .iter_mut()
        .find(|element| element.id == "slot")
        .ok_or("slot")?
        .properties
        .insert("definingFeature".into(), json!({"$ref": "other-feature"}));
    reject(&obsolete, assign(Some("first")), "not a slottable feature")?;

    let mut descendant = inherited.clone();
    descendant.elements.push(serde_json::from_value(
        json!({"id": "descendant", "kind": "uml.class", "name": "Descendant"}),
    )?);
    for (id, kind, from, to) in [
        ("owns-descendant", "owns", "model", "descendant"),
        (
            "descendant-inherits",
            "uml.generalization",
            "descendant",
            "class",
        ),
    ] {
        descendant.relations.push(Relation {
            id: id.into(),
            kind: kind.into(),
            from: from.into(),
            to: to.into(),
            properties: Default::default(),
        });
    }
    descendant
        .elements
        .iter_mut()
        .find(|element| element.id == "first")
        .ok_or("first")?
        .properties
        .insert("classifier".into(), json!([{"$ref": "descendant"}]));
    // The normative OCL applies descendant.inherit to all ancestors' direct
    // attributes; it does not transitively suppress other-feature here.
    let descendant = commands::apply(&descendant, &assign(Some("first")), 11)?;
    assert_eq!(owner_of(&descendant)?, "first");
    let references = ReferenceIndex::new(Model::from_project(&descendant));
    let slottable = classifiers::query(&references, "descendant")?
        .classifier
        .ok_or("descendant Classifier")?
        .all_slottable_features?;
    for feature in ["feature", "other-feature"] {
        assert!(slottable.iter().any(|candidate| candidate.id() == feature));
    }

    inherited.relations.push(Relation {
        id: "cycle-back".into(),
        kind: "uml.generalization".into(),
        from: "other-class".into(),
        to: "class".into(),
        properties: Default::default(),
    });
    reject(
        &inherited,
        assign(Some("first")),
        "cycle in UML generalization",
    )?;
    let references = ReferenceIndex::new(Model::from_project(&inherited));
    assert!(matches!(
        classifier_hierarchy::conforms_to(&references, "class", "other-class"),
        Err(classifiers::QueryError::Invalid { .. })
    ));
    inherited
        .relations
        .retain(|relation| relation.id != "cycle-back");
    let repaired = commands::apply(&inherited, &assign(Some("first")), 11)?;
    assert_eq!(owner_of(&repaired)?, "first");
    let restored = decode(&encode(&repaired, Format::Protobuf)?, Format::Protobuf)?;

    // Continue the same classifier/Slot lifecycle with subsetting instead of replacement.
    // A subset restricts the upper bound but may have a lower lower-bound.
    let subset = |property: &str, targets: &[&str]| Command::SetSubsettedProperties {
        property_id: property.into(),
        subsetted_ids: targets.iter().map(|id| (*id).into()).collect(),
    };
    let set = |id: &str, key: &str, value| Command::SetProperty {
        element_id: id.into(),
        path: format!("properties.{key}"),
        value,
    };
    let mut subset_model = commands::apply(
        &restored,
        &Command::SetRedefinedElements {
            element_id: "feature".into(),
            redefined_ids: vec![],
        },
        12,
    )?;
    for (id, key, value) in [
        ("feature", "type", json!({"$ref": "class"})),
        ("other-feature", "type", json!({"$ref": "other-class"})),
        ("feature", "multiplicity", json!("0..3")),
        ("other-feature", "multiplicity", json!("2..5")),
    ] {
        subset_model = commands::apply(&subset_model, &set(id, key, value), 13)?;
    }
    subset_model = commands::apply(&subset_model, &subset("feature", &["other-feature"]), 14)?;
    subset_model
        .elements
        .iter_mut()
        .find(|element| element.id == "slot")
        .ok_or("Slot")?
        .properties
        .insert("definingFeature".into(), json!({"$ref": "other-feature"}));
    subset_model = commands::apply(&subset_model, &assign(Some("second")), 15)?;
    assert_eq!(owner_of(&subset_model)?, "second"); // subsetting did not remove the original feature
    for owner in ["other-class", "class"] {
        subset_model = commands::apply(
            &subset_model,
            &Command::SetOwner {
                element_id: "feature".into(),
                owner_id: Some(owner.into()),
            },
            16,
        )?; // equality of contexts is valid, unlike strict redefinition ancestry
    }
    for (command, reason) in [
        (subset("feature", &["missing"]), "unknown Element"),
        (subset("feature", &["class"]), "Property"),
        (
            subset("feature", &["other-feature", "other-feature"]),
            "unique",
        ),
        (set("feature", "multiplicity", json!("0..6")), "subsetting"),
        (set("feature", "multiplicity", json!("0..*")), "subsetting"),
        (
            Command::Rename {
                element_id: "feature".into(),
                name: "other".into(),
            },
            "subsetting",
        ),
    ] {
        reject(&subset_model, command, reason)?;
    }
    subset_model = commands::apply(
        &subset_model,
        &set("other-feature", "multiplicity", json!("2..*")),
        17,
    )?;
    subset_model = commands::apply(
        &subset_model,
        &set(
            "feature",
            "multiplicity",
            json!("0..10000000000000000000000000000000000000000"),
        ),
        18,
    )?;
    let mut unknown = commands::apply(
        &subset_model,
        &set("feature", "multiplicity", json!("0..limit")),
        19,
    )?;
    unknown = commands::apply(&unknown, &subset("feature", &["other-feature"]), 20)?;
    let unknown = decode(&encode(&unknown, Format::Protobuf)?, Format::Protobuf)?;

    let section = bellatrix_mde::ModelingSection::from_model(&unknown);
    assert!(
        bellatrix_mde::inspect_element(&section, "feature")
            .ok_or("Inspector")?
            .properties
            .iter()
            .any(|property| property.path == "uml.subsetting.status" && !property.editable)
    );

    // Import an unknown lower expression with a known upper violation, then repair
    // only the upper literal. Overall multiplicity remains unknown, independently.
    let mut bounds = subset_model.clone();
    bounds
        .elements
        .iter_mut()
        .find(|element| element.id == "other-feature")
        .ok_or("target")?
        .properties
        .insert("multiplicity".into(), json!("2..5"));
    let feature = bounds
        .elements
        .iter_mut()
        .find(|element| element.id == "feature")
        .ok_or("feature")?;
    feature
        .properties
        .insert("lowerValue".into(), json!({"$ref":"subset-lower"}));
    feature
        .properties
        .insert("upperValue".into(), json!({"$ref":"subset-upper"}));
    for element in [
        json!({"id":"subset-lower","kind":"uml.opaque_expression","name":"","properties":{"visibility":null,"body":["limit"],"language":["OCL"]}}),
        json!({"id":"subset-upper","kind":"uml.literal_unlimited_natural","name":"","properties":{"visibility":null,"value":"6"}}),
    ] {
        bounds.elements.push(serde_json::from_value(element)?);
    }
    for to in ["subset-lower", "subset-upper"] {
        bounds.relations.push(Relation {
            id: format!("owns-{to}"),
            kind: "owns".into(),
            from: "feature".into(),
            to: to.into(),
            properties: Default::default(),
        });
    }
    reject(&bounds, subset("feature", &["other-feature"]), "subsetting")?;
    commands::apply(&bounds, &set("subset-upper", "value", json!("3")), 20)?;

    // The same subset constraint applies to a n-ary Association end. Its contexts
    // are the other end Types, not the owning Association or its generalizations.
    for element in [
        json!({"id":"selection", "kind":"uml.association", "name":"Selection", "properties":{"memberEnd":[{"$ref":"selected"},{"$ref":"selection-context"},{"$ref":"selection-context2"}]}}),
        json!({"id":"selected", "kind":"uml.attribute", "name":"selected", "properties":{"type":{"$ref":"class"},"multiplicity":"0..2"}}),
        json!({"id":"selection-context", "kind":"uml.attribute", "name":"context", "properties":{"type":{"$ref":"class"}}}),
        json!({"id":"selection-context2", "kind":"uml.attribute", "name":"context2", "properties":{"type":{"$ref":"other-class"}}}),
        json!({"id":"unrelated", "kind":"uml.class", "name":"Unrelated"}),
    ] {
        subset_model.elements.push(serde_json::from_value(element)?);
    }
    for (from, to) in [
        ("model", "selection"),
        ("model", "unrelated"),
        ("selection", "selected"),
        ("selection", "selection-context"),
        ("selection", "selection-context2"),
    ] {
        subset_model.relations.push(Relation {
            id: format!("owns-{to}"),
            kind: "owns".into(),
            from: from.into(),
            to: to.into(),
            properties: Default::default(),
        });
    }
    subset_model = commands::apply(&subset_model, &subset("selected", &["other-feature"]), 21)?;
    for (id, key, value) in [
        ("selected", "type", json!({"$ref":"unrelated"})),
        ("selection-context2", "type", json!({"$ref":"unrelated"})),
    ] {
        reject(&subset_model, set(id, key, value), "subsetting")?;
    }
    let mut cycle = subset_model.clone();
    cycle.relations.push(Relation {
        id: "subset-context-cycle".into(),
        kind: "uml.generalization".into(),
        from: "other-class".into(),
        to: "class".into(),
        properties: Default::default(),
    });
    reject(
        &cycle,
        subset("selected", &["other-feature"]),
        "cycle in UML generalization",
    )?;
    for format in [Format::Json, Format::Protobuf] {
        let reopened = decode(&encode(&subset_model, format)?, format)?;
        let transferred = commands::apply(&reopened, &assign(Some("first")), 22)?;

        let section = bellatrix_mde::ModelingSection::from_model(&transferred);
        let inspector =
            bellatrix_mde::inspect_element(&section, "selected").ok_or("end Inspector")?;
        assert_eq!(
            inspector
                .properties
                .iter()
                .find(|property| property.path == "uml.subsettingContext")
                .ok_or("contexts")?
                .value,
            json!([{"$ref":"class"},{"$ref":"other-class"}])
        );
        assert!(
            transferred
                .elements
                .iter()
                .all(|element| !element.properties.contains_key("subsettingContext"))
        );
    }
    Ok(())
}
