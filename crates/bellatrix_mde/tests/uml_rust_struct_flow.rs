use bellatrix_mde::generate_rust_struct;
use serde_json::{Value, json};
use std::{
    error::Error,
    io::Write,
    process::{Command, Output, Stdio},
};

type FlowResult<T> = Result<T, Box<dyn Error>>;

fn generate(request: &Value) -> FlowResult<Value> {
    Ok(serde_json::from_str(
        &generate_rust_struct(&request.to_string()).map_err(std::io::Error::other)?,
    )?)
}

fn compile(response: &Value) -> FlowResult<Output> {
    let contents = response["artifacts"][0]["contents"]
        .as_str()
        .ok_or("missing Rust source")?;
    let crate_type = response["target"]["crate_type"]
        .as_str()
        .ok_or("missing crate type")?;
    let edition = response["target"]["edition"]
        .as_str()
        .ok_or("missing edition")?;
    let mut compiler = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args([
            "--crate-type",
            crate_type,
            "--edition",
            edition,
            "--crate-name",
            "uml_struct_flow",
            "--emit",
            "metadata",
            "-o",
            "-",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    // Both the source and compiler result stay in memory. The kernel neither
    // persists an artifact nor starts the compiler on the caller's behalf.
    compiler
        .stdin
        .take()
        .ok_or("missing compiler stdin")?
        .write_all(contents.as_bytes())?;
    Ok(compiler.wait_with_output()?)
}

#[test]
fn rootless_uml_class_generates_a_traceable_rust_library_and_survives_edits() -> FlowResult<()> {
    let mut request = json!({
        "api_version": 1, "element_id": "profile",
        "model": {
            "schema_version": 1, "id": "draft", "name": "Standalone struct",
            "elements": [
                {"id":"profile", "kind":"uml.class", "name":"UserProfile",
                 "properties":{"visibility":"public"}},
                {"id":"username", "kind":"uml.attribute", "name":"username",
                 "properties":{"visibility":"public", "multiplicity":"1",
                    "type":{"href":"http://www.omg.org/spec/UML/20161101/PrimitiveTypes.xmi#String"}}},
                {"id":"enabled", "kind":"uml.attribute", "name":"enabled",
                 "properties":{"visibility":"public", "multiplicity":"1",
                    "type":{"href":"http://www.omg.org/spec/UML/20161101/PrimitiveTypes.xmi#Boolean"}}}
            ],
            "relations": [
                {"id":"owns-username", "kind":"owns", "from":"profile", "to":"username"},
                {"id":"owns-enabled", "kind":"owns", "from":"profile", "to":"enabled"}
            ]
        }
    });
    let original = request.to_string();
    let generated = generate(&request)?;
    assert_eq!(request.to_string(), original);
    assert_eq!(generated["target"]["crate_type"], "lib");
    assert_eq!(
        generated["artifacts"]
            .as_array()
            .ok_or("missing artifacts")?
            .len(),
        1
    );
    assert_eq!(generated["artifacts"][0]["path"], "userprofile.rs");
    assert_eq!(generated["artifacts"][0]["origin"], "profile");
    assert_eq!(
        generated["trace"],
        json!([
            {"source_id":"profile", "target_kind":"rust.struct", "target_name":"UserProfile"},
            {"source_id":"username", "target_kind":"rust.field", "target_name":"username"},
            {"source_id":"enabled", "target_kind":"rust.field", "target_name":"enabled"}
        ])
    );
    let contents = generated["artifacts"][0]["contents"]
        .as_str()
        .ok_or("missing source")?;
    assert!(contents.contains("pub struct UserProfile {"));
    assert!(contents.contains("pub username: ::std::string::String,"));
    assert!(contents.contains("pub enabled: bool,"));
    assert!(!contents.contains("fn main"));
    let compiled = compile(&generated)?;
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    request["model"]["elements"][0]["name"] = json!("RenamedProfile");
    request["model"]["elements"][1]["name"] = json!("display_name");
    let renamed = generate(&request)?;
    assert_eq!(renamed["artifacts"][0]["path"], "renamedprofile.rs");
    assert_eq!(renamed["artifacts"][0]["origin"], "profile");
    assert!(
        renamed["artifacts"][0]["contents"]
            .as_str()
            .ok_or("missing source")?
            .contains("pub display_name: ::std::string::String,")
    );
    let compiled = compile(&renamed)?;
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    request["model"]["elements"][1]["properties"]["multiplicity"] = json!("0..*");
    let unsupported = generate_rust_struct(&request.to_string())
        .expect_err("collection mapping must not be invented");
    assert_eq!(
        serde_json::from_str::<Value>(&unsupported)?["error"]["code"],
        "RUST_STRUCT_UNSUPPORTED"
    );
    request["model"]["elements"][1]["properties"]["multiplicity"] = json!("1");
    assert_eq!(generate(&request)?, renamed);

    // UML names are not Rust identifiers. Target syntax is the compiler's
    // responsibility.
    request["model"]["elements"][0]["name"] = json!("type");
    let rust_error = generate(&request)?;
    assert!(!compile(&rust_error)?.status.success());
    let reopened: Value = serde_json::from_str(&original)?;
    assert_eq!(generate(&reopened)?, generated);
    Ok(())
}
