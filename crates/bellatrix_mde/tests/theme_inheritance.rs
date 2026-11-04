use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document,
};
use bellatrix_mde_contracts::protocol::{Command, RelationshipRouting, Style};
use serde_json::{Value, json};

type FlowResult<T> = Result<T, Box<dyn std::error::Error>>;

fn decode(bytes: &[u8], format: &str) -> FlowResult<Value> {
    let mut envelope: Value = serde_json::from_str(&decode_model_document(bytes, format)?)?;
    Ok(envelope["model"].take())
}

fn encode(model: &Value, format: &str) -> FlowResult<Vec<u8>> {
    Ok(encode_model_document(
        &json!({"api_version": 1, "model": model}).to_string(),
        format,
    )?)
}

fn apply(model: &Value, command: &Command, revision: u64) -> FlowResult<Value> {
    let parts = prepare_model_document(&json!({"api_version": 1, "model": model}).to_string())?;
    let updated = apply_model_command(
        &parts.id,
        &parts.name,
        &parts.section_json,
        revision,
        command.clone(),
    )?;
    decode(
        &encode_model_section(&parts.id, &parts.name, &updated.to_string(), "json")?,
        "json",
    )
}

fn edge(model: &mut Value) -> FlowResult<&mut Value> {
    model["diagrams"][0]["views"]
        .as_array_mut()
        .ok_or("missing views")?
        .iter_mut()
        .find(|view| view["id"] == "view-dep")
        .ok_or_else(|| "missing edge".into())
}

#[test]
fn restoring_inheritance_preserves_saved_geometry_and_other_views()
-> Result<(), Box<dyn std::error::Error>> {
    let mut original = decode(include_bytes!("../fixtures/todo.json"), "json")?;
    let view = edge(&mut original)?;
    view["style"]["stroke"] = json!("#123456");
    view["style"]["routing"] = serde_json::to_value(RelationshipRouting::Curve)?;
    let restored_style = Style {
        routing: Some(RelationshipRouting::Curve),
        ..Style::default()
    };
    let command = Command::SetViewStyle {
        diagram_id: original["diagrams"][0]["id"]
            .as_str()
            .ok_or("missing diagram")?
            .into(),
        view_id: "view-dep".into(),
        style: restored_style.clone(),
    };
    let updated = apply(&original, &command, 1)?;
    let mut expected = original.clone();
    edge(&mut expected)?["style"] = serde_json::to_value(restored_style)?;
    assert_eq!(updated, expected);
    for format in ["json", "protobuf"] {
        let reopened = decode(&encode(&updated, format)?, format)?;
        assert_eq!(reopened, expected);
        assert_eq!(apply(&reopened, &command, 2)?, reopened);
    }
    Ok(())
}
