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

const RULE: &str = "InstanceSpecification-deployment_artifact:";
const ERROR: &str = "UML_INSTANCE_SPECIFICATION_DEPLOYMENT_ARTIFACT";
const UNKNOWN: &str = "MDE_INSTANCE_SPECIFICATION_DEPLOYMENT_ARTIFACT_UNVERIFIED";

fn document(rooted: bool) -> Result<Value> {
    let mut value = json!({"id":"project","name":"Instance deployment","root_model":"root",
    "elements":[
        {"id":"root","kind":"uml.model","name":"Root"},
        {"id":"node","kind":"uml.node","name":"Node"},
        {"id":"artifact","kind":"uml.artifact","name":"Artifact"},
        {"id":"class","kind":"uml.class","name":"Class"},
        {"id":"instance","kind":"uml.instance_specification","name":"Instance","properties":{"classifier":[{"$ref":"artifact"}]}}
    ],"relations":[
        {"id":"deployment","kind":"uml.deployment","from":"node","to":"instance","properties":{"visibility":null,"deployedArtifact":[{"$ref":"instance"}]}}
    ]});
    for (owner, child) in [
        ("root", "node"),
        ("root", "artifact"),
        ("root", "class"),
        ("root", "instance"),
        ("node", "deployment"),
    ] {
        value["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .push(json!({"id":format!("owns-{child}"),"kind":"owns","from":owner,"to":child}));
    }
    if !rooted {
        value.as_object_mut().ok_or("object")?.remove("root_model");
    }
    import_document(&value)
}
fn author(project: &mut Value, id: &str, key: &str, value: Value) -> Result {
    if let Some(element) = project["elements"]
        .as_array_mut()
        .ok_or("elements")?
        .iter_mut()
        .find(|element| element["id"] == id)
    {
        element["properties"][key] = value;
    } else {
        project["relations"]
            .as_array_mut()
            .ok_or("relations")?
            .iter_mut()
            .find(|relation| relation["id"] == id)
            .ok_or("relation")?["properties"][key] = value;
    }
    Ok(())
}
fn issues(project: &Value) -> Result<Vec<Value>> {
    let response: Value = serde_json::from_str(&validate_uml_model(
        &json!({"api_version":1,"model":project}).to_string(),
    )?)?;
    Ok(response["diagnostics"]
        .as_array()
        .ok_or("diagnostics")?
        .iter()
        .filter(|issue| {
            issue["subject_id"] == "instance"
                && issue["message"]
                    .as_str()
                    .is_some_and(|message| message.starts_with(RULE))
        })
        .cloned()
        .collect())
}
fn expect(project: &Value, code: &str) -> Result {
    let issues = issues(project)?;
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert_eq!(issues[0]["code"], code, "{issues:#?}");
    assert_eq!(
        issues[0]["severity"],
        if code == UNKNOWN { "warning" } else { "error" }
    );
    Ok(())
}

#[test]
fn given_deployed_instance_then_an_artifact_classifier_is_required() -> Result {
    for rooted in [true, false] {
        let mut project = document(rooted)?;
        assert!(issues(&project)?.is_empty());
        for value in [json!([]), json!([{"$ref":"class"}])] {
            author(&mut project, "instance", "classifier", value)?;
            expect(&project, ERROR)?;
            for format in ["json", "protobuf"] {
                expect(&decode(&encode(&project, format)?, format)?, ERROR)?;
            }
        }
        project["elements"]
            .as_array_mut()
            .ok_or("elements")?
            .iter_mut()
            .find(|element| element["id"] == "instance")
            .ok_or("instance")?["properties"]
            .as_object_mut()
            .ok_or("properties")?
            .remove("classifier");
        expect(&project, ERROR)?;
    }
    Ok(())
}
