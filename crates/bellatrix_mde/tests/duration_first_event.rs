use bellatrix_mde::{
    Format, Project,
    commands::{Command, apply},
    encode,
};
use serde_json::{Value, json};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn document(constrained: Option<Value>, first_event: Option<Value>) -> Result<Project> {
    let mut model: Project = serde_json::from_value(
        json!({"id":"model","name":"Duration first events","root_model":"root","elements":[
            {"id":"root","kind":"uml.model","name":"Root"},
            {"id":"constraint","kind":"uml.duration_constraint","name":"Duration"},
            {"id":"first","kind":"uml.class","name":"First"},
            {"id":"second","kind":"uml.class","name":"Second"},
            {"id":"third","kind":"uml.class","name":"Third"}
        ],"relations":[
            {"id":"owns-constraint","kind":"owns","from":"root","to":"constraint"},
            {"id":"owns-first","kind":"owns","from":"root","to":"first"},
            {"id":"owns-second","kind":"owns","from":"root","to":"second"},
            {"id":"owns-third","kind":"owns","from":"root","to":"third"}
        ]}),
    )?;
    if let Some(value) = constrained {
        author(&mut model, "constrainedElement", value)?;
    }
    if let Some(value) = first_event {
        author(&mut model, "firstEvent", value)?;
    }
    Ok(model)
}
fn author(model: &mut Project, key: &str, value: Value) -> Result {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == "constraint")
        .ok_or("constraint")?
        .properties
        .insert(key.into(), value);
    Ok(())
}

#[test]
fn given_direct_first_event_edit_then_read_only_adapter_preserves_source() -> Result {
    let model = document(
        Some(json!([{"$ref":"first"},{"$ref":"second"}])),
        Some(json!([true, false])),
    )?;
    let before = encode(&model, Format::Json)?;
    let error = apply(
        &model,
        &Command::SetProperty {
            element_id: "constraint".into(),
            path: "properties.firstEvent".into(),
            value: json!([true]),
        },
        1,
    )
    .expect_err("one firstEvent cannot describe two constrained elements");
    // This proves the authoring boundary, not rollback by FIRST_EVENT: the
    // array remains read-only and the command never reaches that validator.
    assert_eq!(error.to_string(), "property is read-only");
    assert_eq!(before, encode(&model, Format::Json)?);
    Ok(())
}
