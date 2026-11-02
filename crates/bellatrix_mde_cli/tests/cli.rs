use smol::process::Command;
use std::{fs, path::PathBuf};
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_bellatrix-mde")
}
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bellatrix_mde/fixtures/todo.json")
}
fn unique(name: &str) -> PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("bellatrix-{name}-{}-{now}", std::process::id()))
}
#[tokio::test]
async fn cli_generates_real_project_and_refuses_overwrite() {
    let output = unique("rust");
    let result = Command::new(binary())
        .args([
            "generate",
            fixture().to_str().unwrap(),
            "--target",
            "rust",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .await
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.join("Cargo.toml").exists());
    assert!(output.join("src/task.rs").exists());
    assert_eq!(
        fs::read(output.join("assets/logo.bin")).unwrap(),
        vec![0, 1, 2, 3, 255, 0, 127]
    );
    let retry = Command::new(binary())
        .args([
            "generate",
            fixture().to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .await
        .unwrap();
    assert!(!retry.status.success());
    fs::remove_dir_all(output).unwrap();
}
#[tokio::test]
async fn cli_exports_offline_html_and_svg() {
    let output = unique("html");
    let result = Command::new(binary())
        .args([
            "export",
            fixture().to_str().unwrap(),
            "--format",
            "html",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .await
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        fs::read_to_string(output.join("index.html"))
            .unwrap()
            .contains("Task List")
    );
    assert!(fs::read_dir(output.join("diagrams")).unwrap().count() >= 2);
    fs::remove_dir_all(output).unwrap();
}
#[tokio::test]
async fn cli_analyzes_graph_without_generating_anything() {
    let found = Command::new(binary())
        .args([
            "analyze",
            fixture().to_str().unwrap(),
            "--from",
            "type-list",
            "--to",
            "fn-add",
            "--relation",
            "owns",
        ])
        .output()
        .await
        .unwrap();
    assert!(
        found.status.success(),
        "{}",
        String::from_utf8_lossy(&found.stderr)
    );
    let stdout = String::from_utf8_lossy(&found.stdout);
    assert!(stdout.contains("Shortest directed path"));
    assert!(stdout.contains("type-list --[owns]--> fn-add"));
    let invalid = Command::new(binary())
        .args(["analyze", fixture().to_str().unwrap(), "--to", "fn-add"])
        .output()
        .await
        .unwrap();
    assert!(!invalid.status.success());
}
