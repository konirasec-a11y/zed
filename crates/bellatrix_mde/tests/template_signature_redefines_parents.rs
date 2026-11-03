use bellatrix_mde::{Format, Project, decode, encode, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const ERROR: &str = "UML_TEMPLATE_SIGNATURE_REDEFINES_PARENTS";
const INVALID: &str = "MDE_TEMPLATE_SIGNATURE_REDEFINES_PARENTS_INVALID";
const UNKNOWN: &str = "MDE_TEMPLATE_SIGNATURE_REDEFINES_PARENTS_UNVERIFIED";
fn document(rooted: bool) -> Result<Project> {
    let mut value = json!({"id":"p","name":"Signatures","root_model":"root","elements":[
        {"id":"root","name":"Root","kind":"uml.model"},
        {"id":"child","name":"Child","kind":"uml.class"},
        {"id":"parent","name":"Parent","kind":"uml.class"},
        {"id":"grand","name":"Grand","kind":"uml.class"},
        {"id":"child-signature","name":"ChildSignature","kind":"uml.redefinable_template_signature","properties":{"extendedSignature":[{"$ref":"parent-signature"}]}},
        {"id":"parent-signature","name":"ParentSignature","kind":"uml.redefinable_template_signature","properties":{"extendedSignature":[{"$ref":"grand-signature"}]}},
        {"id":"grand-signature","name":"GrandSignature","kind":"uml.redefinable_template_signature"}
    ],"relations":[
        {"id":"child-parent","kind":"uml.generalization","from":"child","to":"parent"},
        {"id":"parent-grand","kind":"uml.generalization","from":"parent","to":"grand"},
        {"id":"owns-child","kind":"owns","from":"root","to":"child"},
        {"id":"owns-parent","kind":"owns","from":"root","to":"parent"},
        {"id":"owns-grand","kind":"owns","from":"root","to":"grand"},
        {"id":"owns-child-signature","kind":"owns","from":"child","to":"child-signature"},
        {"id":"owns-parent-signature","kind":"owns","from":"parent","to":"parent-signature"},
        {"id":"owns-grand-signature","kind":"owns","from":"grand","to":"grand-signature"}
    ]});
    if !rooted {
        value.as_object_mut().ok_or("project")?.remove("root_model");
    }
    Ok(serde_json::from_value(value)?)
}
fn author(p: &mut Project, id: &str, key: &str, value: Value) -> Result {
    p.elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element")?
        .properties
        .insert(key.into(), value);
    Ok(())
}
fn issues(p: &Project) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":p}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "child-signature"
                && [ERROR, INVALID, UNKNOWN]
                    .iter()
                    .any(|code| issue["code"] == *code)
        })
        .cloned()
        .collect())
}
fn expect(p: &Project, code: &str) -> Result {
    let found = issues(p)?;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["code"], code, "{found:#?}");
    Ok(())
}
#[test]
fn transitive_signature_closure_covers_all_classifier_ancestors() -> Result {
    for rooted in [false, true] {
        let mut p = document(rooted)?;
        assert!(issues(&p)?.is_empty());
        author(&mut p, "parent-signature", "extendedSignature", json!([]))?;
        expect(&p, ERROR)?;
        for format in [Format::Json, Format::Protobuf] {
            expect(&decode(&encode(&p, format)?, format)?, ERROR)?;
        }
    }
    Ok(())
}
#[test]
fn direct_signature_only_does_not_satisfy_transitive_parent_requirement() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "child-signature",
        "extendedSignature",
        json!([{"$ref":"grand-signature"}]),
    )?;
    expect(&p, ERROR)?;
    author(
        &mut p,
        "child-signature",
        "extendedSignature",
        json!([{"$ref":"parent-signature"},{"$ref":"grand-signature"}]),
    )?;
    assert!(issues(&p)?.is_empty());
    Ok(())
}
#[test]
fn ancestors_without_template_signatures_and_empty_ancestry_are_vacuous() -> Result {
    let mut p = document(false)?;
    p.relations
        .retain(|relation| relation.kind != "uml.generalization");
    author(&mut p, "child-signature", "extendedSignature", json!([]))?;
    assert!(issues(&p)?.is_empty());
    p.relations.push(serde_json::from_value(
        json!({"id":"child-parent","kind":"uml.generalization","from":"child","to":"parent"}),
    )?);
    p.elements
        .retain(|element| element.id != "parent-signature");
    p.relations
        .retain(|relation| relation.to != "parent-signature");
    assert!(issues(&p)?.is_empty());
    Ok(())
}
#[test]
fn external_extension_is_unknown_unless_known_closure_proves_membership() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "child-signature",
        "extendedSignature",
        json!([{"href":"external.uml#Signature"}]),
    )?;
    expect(&p, UNKNOWN)?;
    author(
        &mut p,
        "child-signature",
        "extendedSignature",
        json!([{"href":"external.uml#Signature"},{"$ref":"parent-signature"}]),
    )?;
    assert!(issues(&p)?.is_empty());
    Ok(())
}
#[test]
fn malformed_extension_dominates_unknown_and_empty_ancestry() -> Result {
    for reverse in [false, true] {
        for invalid in [
            json!(42),
            json!({"$ref":"missing"}),
            json!({"$ref":"parent"}),
            json!({"href":42}),
        ] {
            let mut p = document(false)?;
            p.relations
                .retain(|relation| relation.kind != "uml.generalization");
            let mut values = vec![json!({"href":"external.uml#Signature"}), invalid];
            if reverse {
                values.reverse();
            }
            author(
                &mut p,
                "child-signature",
                "extendedSignature",
                Value::Array(values),
            )?;
            expect(&p, INVALID)?;
        }
    }
    Ok(())
}
#[test]
fn owner_projection_and_parent_owned_signature_must_agree_with_ownership() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "child-signature",
        "classifier",
        json!({"$ref":"parent"}),
    )?;
    expect(&p, INVALID)?;
    author(
        &mut p,
        "child-signature",
        "classifier",
        json!({"$ref":"child"}),
    )?;
    author(&mut p, "parent", "ownedTemplateSignature", Value::Null)?;
    expect(&p, INVALID)?;
    Ok(())
}
#[test]
fn cyclic_extended_closure_terminates_and_diamonds_do_not_duplicate_members() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "grand-signature",
        "extendedSignature",
        json!([{"$ref":"parent-signature"}]),
    )?;
    author(
        &mut p,
        "child-signature",
        "extendedSignature",
        json!([{"$ref":"parent-signature"},{"$ref":"grand-signature"}]),
    )?;
    assert!(issues(&p)?.is_empty());
    Ok(())
}
#[test]
fn hierarchy_cycles_are_invalid_even_with_external_extension() -> Result {
    let mut p = document(false)?;
    p.relations.push(serde_json::from_value(
        json!({"id":"cycle","kind":"uml.generalization","from":"grand","to":"child"}),
    )?);
    author(
        &mut p,
        "child-signature",
        "extendedSignature",
        json!([{"href":"external.uml#Signature"}]),
    )?;
    expect(&p, INVALID)?;
    Ok(())
}

#[test]
fn known_missing_parent_signature_beats_unknown_hierarchy_branch() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "child",
        "general",
        json!([{"href":"external.uml#Parent"}]),
    )?;
    expect(&p, UNKNOWN)?;
    author(&mut p, "child-signature", "extendedSignature", json!([]))?;
    expect(&p, ERROR)?;
    author(&mut p, "parent", "ownedTemplateSignature", json!(42))?;
    expect(&p, INVALID)?;
    Ok(())
}

#[test]
fn malformed_extension_is_checked_before_unresolved_classifier_owner() -> Result {
    let mut p = document(false)?;
    p.elements
        .iter_mut()
        .find(|element| element.id == "child")
        .ok_or("child")?
        .kind = "uml.unresolved_classifier".into();
    author(
        &mut p,
        "child-signature",
        "extendedSignature",
        json!([{"href":"external.uml#Signature"}]),
    )?;
    expect(&p, UNKNOWN)?;
    author(
        &mut p,
        "child-signature",
        "extendedSignature",
        json!([{"href":"external.uml#Signature"},42]),
    )?;
    expect(&p, INVALID)?;
    Ok(())
}

#[test]
fn malformed_transitive_extension_is_not_hidden_by_known_required_membership() -> Result {
    let mut p = document(false)?;
    author(
        &mut p,
        "child-signature",
        "extendedSignature",
        json!([{"$ref":"parent-signature"},{"$ref":"grand-signature"}]),
    )?;
    author(
        &mut p,
        "grand-signature",
        "extendedSignature",
        json!([{"href":"external.uml#Signature"},42]),
    )?;
    expect(&p, INVALID)?;
    Ok(())
}

#[test]
fn invalid_snapshot_can_be_repaired_and_revalidation_does_not_cache_results() -> Result {
    let valid = document(true)?;
    let mut invalid = valid.clone();
    author(
        &mut invalid,
        "child-signature",
        "extendedSignature",
        json!([]),
    )?;
    assert!(issues(&valid)?.is_empty());
    expect(&invalid, ERROR)?;
    assert!(issues(&valid)?.is_empty());
    for format in [Format::Json, Format::Protobuf] {
        let mut reopened = decode(&encode(&invalid, format)?, format)?;
        expect(&reopened, ERROR)?;
        author(
            &mut reopened,
            "child-signature",
            "extendedSignature",
            json!([{"$ref":"parent-signature"}]),
        )?;
        assert!(issues(&reopened)?.is_empty());
    }
    Ok(())
}

#[test]
fn unknown_owned_signature_metaclass_is_not_a_false_inverse_contradiction() -> Result {
    let mut p = document(false)?;
    p.elements
        .iter_mut()
        .find(|element| element.id == "parent-signature")
        .ok_or("parent-signature")?
        .kind = "uml.future_signature".into();
    author(
        &mut p,
        "parent",
        "ownedTemplateSignature",
        json!({"$ref":"parent-signature"}),
    )?;
    expect(&p, UNKNOWN)?;
    author(&mut p, "child-signature", "extendedSignature", json!([]))?;
    expect(&p, ERROR)?;
    Ok(())
}

#[test]
fn malformed_owner_projections_dominate_unresolved_owner_metaclass() -> Result {
    for property in ["classifier", "template"] {
        for value in [
            json!(42),
            Value::Null,
            json!({"href":42}),
            json!({"$ref":"parent"}),
        ] {
            let mut p = document(false)?;
            p.elements
                .iter_mut()
                .find(|element| element.id == "child")
                .ok_or("child")?
                .kind = "uml.future_classifier".into();
            author(&mut p, "child-signature", property, value)?;
            expect(&p, INVALID)?;
        }
    }
    Ok(())
}
