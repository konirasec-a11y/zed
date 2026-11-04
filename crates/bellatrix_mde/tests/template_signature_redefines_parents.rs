use bellatrix_mde::{decode_model_document, encode_model_document, validate_uml_model};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn envelope(model: &Value) -> String {
    json!({"api_version": 1, "model": model}).to_string()
}

fn decode(document: &[u8], format: &str) -> Result<Value> {
    let response = decode_model_document(document, format)?;
    let mut response: Value = serde_json::from_str(&response)?;
    Ok(response["model"].take())
}

fn encode(model: &Value, format: &str) -> Result<Vec<u8>> {
    Ok(encode_model_document(&envelope(model), format)?)
}

fn import_document(model: &Value) -> Result<Value> {
    decode(&serde_json::to_vec(model)?, "json")
}

fn element_mut<'a>(model: &'a mut Value, id: &str) -> Result<&'a mut Value> {
    model["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .iter_mut()
        .find(|element| element["id"] == id)
        .ok_or_else(|| format!("missing element {id}").into())
}

const ERROR: &str = "UML_TEMPLATE_SIGNATURE_REDEFINES_PARENTS";
const INVALID: &str = "MDE_TEMPLATE_SIGNATURE_REDEFINES_PARENTS_INVALID";
const UNKNOWN: &str = "MDE_TEMPLATE_SIGNATURE_REDEFINES_PARENTS_UNVERIFIED";
fn document(rooted: bool) -> Result<Value> {
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
    import_document(&value)
}
fn author(p: &mut Value, id: &str, key: &str, value: Value) -> Result {
    element_mut(p, id)?["properties"][key] = value;
    Ok(())
}
fn issues(p: &Value) -> Result<Vec<Value>> {
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
fn expect(p: &Value, code: &str) -> Result {
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
        for format in ["json", "protobuf"] {
            expect(&decode(&encode(&p, format)?, format)?, ERROR)?;
        }
    }
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
    for format in ["json", "protobuf"] {
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
