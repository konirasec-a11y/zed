use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_PROPERTY_SUBSETTING_CONTEXT_CONFORMS";
const INVALID: &str = "MDE_PROPERTY_SUBSETTING_CONTEXT_CONFORMS_INVALID";
const UNKNOWN: &str = "MDE_PROPERTY_SUBSETTING_CONTEXT_CONFORMS_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut model = json!({"id":"project","name":"Subsetting rules","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"owner","kind":"uml.class","name":"Owner"},
        {"id":"super","kind":"uml.class","name":"Super"},
        {"id":"middle","kind":"uml.class","name":"Middle"},
        {"id":"sub","kind":"uml.class","name":"Sub"},
        {"id":"general","kind":"uml.attribute","name":"General","properties":{"type":{"$ref":"super"}}},
        {"id":"specific","kind":"uml.attribute","name":"Specific","properties":{"type":{"$ref":"super"},"subsettedProperty":[{"$ref":"general"}]}}
    ],"relations":[
        {"id":"sub-middle","kind":"uml.generalization","from":"sub","to":"middle"},
        {"id":"middle-super","kind":"uml.generalization","from":"middle","to":"super"}
    ]});
    for (owner, child) in [
        ("root", "owner"),
        ("root", "super"),
        ("root", "middle"),
        ("root", "sub"),
        ("owner", "general"),
        ("owner", "specific"),
    ] {
        model["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    if !rooted {
        model.as_object_mut().ok_or("model")?.remove("root_model");
    }
    Ok(serde_json::from_value(model)?)
}
fn author(model: &mut Project, id: &str, key: &str, value: Value) -> Result {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .properties
        .insert(key.into(), value);
    Ok(())
}
fn issues(model: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":model}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "specific"
                && [ERROR, INVALID, UNKNOWN]
                    .iter()
                    .any(|code| issue["code"] == *code)
        })
        .cloned()
        .collect())
}
fn expect(model: &Project, code: &str) -> Result {
    let found = issues(model)?;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["code"], code, "{found:#?}");
    assert_eq!(
        found[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}
fn reparent(model: &mut Project, child: &str, owner: Option<&str>) -> Result {
    if let Some(owner) = owner {
        model
            .relations
            .iter_mut()
            .find(|relation| relation.id == format!("owns-{child}"))
            .ok_or("owner")?
            .from = owner.into();
    } else {
        model
            .relations
            .retain(|relation| relation.id != format!("owns-{child}"));
    }
    Ok(())
}
fn context_association(model: &mut Project, property: &str, types: Vec<Value>) -> Result {
    let id = format!("association-{property}");
    let mut members = vec![json!({"$ref":property})];
    for (index, value) in types.into_iter().enumerate() {
        let end = format!("{property}-context-{index}");
        members.push(json!({"$ref":end}));
        model.elements.push(serde_json::from_value(
            json!({"id":end,"kind":"uml.attribute","name":end,"properties":{"type":value}}),
        )?);
        model.relations.push(serde_json::from_value(
            json!({"id":format!("owns-{end}"),"kind":"owns","from":id,"to":end}),
        )?);
    }
    model.elements.push(serde_json::from_value(
        json!({"id":id,"kind":"uml.association","name":id,"properties":{"memberEnd":members}}),
    )?);
    Ok(())
}
#[test]
fn direct_and_transitive_owner_conformance_are_not_strict_specialization() -> Result {
    for rooted in [false, true] {
        let mut model = document(rooted)?;
        assert!(issues(&model)?.is_empty());
        reparent(&mut model, "general", Some("super"))?;
        for owner in ["super", "middle", "sub"] {
            reparent(&mut model, "specific", Some(owner))?;
            assert!(issues(&model)?.is_empty());
        }
        reparent(&mut model, "general", Some("sub"))?;
        reparent(&mut model, "specific", Some("super"))?;
        expect(&model, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&model, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn independent_context_rule_does_not_inspect_property_name_type_or_bounds() -> Result {
    let mut model = document(true)?;
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "specific")
        .ok_or("specific")?
        .name = "General".into();
    for key in ["type", "upperValue"] {
        author(
            &mut model,
            "specific",
            key,
            json!({"href":"external.uml#Value"}),
        )?;
    }
    assert!(issues(&model)?.is_empty());
    reparent(&mut model, "specific", None)?;
    expect(&model, ERROR)?;
    Ok(())
}
#[test]
fn no_subsets_is_vacuous_but_empty_source_context_defeats_external_subset() -> Result {
    let mut model = document(true)?;
    reparent(&mut model, "specific", None)?;
    author(&mut model, "specific", "subsettedProperty", json!([]))?;
    assert!(issues(&model)?.is_empty());
    author(
        &mut model,
        "specific",
        "subsettedProperty",
        json!([{"href":"external.uml#Property"}]),
    )?;
    expect(&model, ERROR)?;
    let aggregate = bellatrix_mde::uml::root::Model::from_project(&model).validate();
    assert!(
        aggregate
            .iter()
            .any(|issue| issue.subject_id == "specific" && issue.code == "UML_PROPERTY_SUBSETTING")
    );
    assert!(!aggregate.iter().any(|issue| issue.subject_id == "specific"
        && issue.code == "UML_PROPERTY_SUBSETTING_UNSUPPORTED"));

    author(
        &mut model,
        "specific",
        "subsettedProperty",
        json!([{"href":"external.uml#Property"},{"$ref":"missing"}]),
    )?;
    expect(&model, INVALID)?;
    Ok(())
}
#[test]
fn empty_general_context_defeats_unknown_source_context() -> Result {
    let mut model = document(true)?;
    model.elements.push(serde_json::from_value(
        json!({"id":"unknown-owner","kind":"uml.unknown_classifier","name":"Unknown"}),
    )?);
    reparent(&mut model, "specific", Some("unknown-owner"))?;
    expect(&model, UNKNOWN)?;
    reparent(&mut model, "general", None)?;
    expect(&model, ERROR)?;
    Ok(())
}
#[test]
fn nary_contexts_use_other_end_types_not_attribute_owner_or_own_type() -> Result {
    let mut model = document(true)?;
    context_association(
        &mut model,
        "specific",
        vec![json!({"$ref":"sub"}), json!({"$ref":"middle"})],
    )?;
    context_association(&mut model, "general", vec![json!({"$ref":"super"})])?;
    assert!(issues(&model)?.is_empty());
    author(
        &mut model,
        "specific-context-1",
        "type",
        json!({"$ref":"owner"}),
    )?;
    expect(&model, ERROR)?;
    Ok(())
}
#[test]
fn known_general_witness_decides_exists_without_exposing_partial_public_context() -> Result {
    use bellatrix_mde::uml::{
        classifiers::QueryError, property_subsetting, references::ReferenceIndex, root::Model,
    };
    let mut model = document(true)?;
    context_association(&mut model, "specific", vec![json!({"$ref":"sub"})])?;
    context_association(
        &mut model,
        "general",
        vec![json!({"href":"external.uml#Type"}), json!({"$ref":"super"})],
    )?;
    assert!(issues(&model)?.is_empty());
    let references = ReferenceIndex::new(Model::from_project(&model));
    assert!(matches!(
        property_subsetting::subsetting_context(&references, references.element("general")?),
        Err(QueryError::Unsupported { .. })
    ));
    Ok(())
}
#[test]
fn known_bad_source_context_dominates_unknown_neighbor_in_both_orders() -> Result {
    for reverse in [false, true] {
        let mut model = document(true)?;
        let mut types = vec![json!({"href":"external.uml#Type"}), json!({"$ref":"owner"})];
        if reverse {
            types.reverse();
        }
        context_association(&mut model, "specific", types)?;
        context_association(&mut model, "general", vec![json!({"$ref":"super"})])?;
        expect(&model, ERROR)?;
    }
    Ok(())
}
#[test]
fn incomplete_source_forall_remains_unknown_without_a_known_counterexample() -> Result {
    let mut model = document(true)?;
    context_association(
        &mut model,
        "specific",
        vec![json!({"href":"external.uml#Type"}), json!({"$ref":"sub"})],
    )?;
    context_association(&mut model, "general", vec![json!({"$ref":"super"})])?;
    expect(&model, UNKNOWN)?;
    Ok(())
}
#[test]
fn invalid_context_end_types_dominate_known_witness_and_unknown() -> Result {
    for reverse in [false, true] {
        for invalid in [json!({"$ref":"missing"}), json!(false)] {
            let mut model = document(true)?;
            context_association(&mut model, "specific", vec![json!({"$ref":"sub"})])?;
            let mut types = vec![
                json!({"$ref":"super"}),
                json!({"href":"external.uml#Type"}),
                invalid,
            ];
            if reverse {
                types.reverse();
            }
            context_association(&mut model, "general", types)?;
            expect(&model, INVALID)?;
        }
    }
    Ok(())
}
#[test]
fn unknown_subset_neighbor_does_not_mask_local_context_mismatch() -> Result {
    for reverse in [false, true] {
        let mut model = document(true)?;
        reparent(&mut model, "specific", Some("owner"))?;
        reparent(&mut model, "general", Some("super"))?;
        let mut values = vec![
            json!({"href":"external.uml#Property"}),
            json!({"$ref":"general"}),
        ];
        if reverse {
            values.reverse();
        }
        author(&mut model, "specific", "subsettedProperty", json!(values))?;
        expect(&model, ERROR)?;
    }
    Ok(())
}
#[test]
fn malformed_subset_collection_is_invalid_even_with_empty_context() -> Result {
    let mut model = document(true)?;
    reparent(&mut model, "specific", None)?;
    for value in [
        Value::Null,
        json!(42),
        json!([{"$ref":"general"},{"$ref":"general"}]),
        json!([{"$ref":"owner"}]),
        json!([{"href":"external.uml#P"},false]),
    ] {
        author(&mut model, "specific", "subsettedProperty", value)?;
        expect(&model, INVALID)?;
    }
    Ok(())
}
