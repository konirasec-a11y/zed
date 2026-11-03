use bellatrix_mde::{Plan, python_construction_script};
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn construction_script_builds_the_rendered_project() -> Result<(), Box<dyn std::error::Error>> {
    let mut plan = Plan::new();
    plan.file("src/main.rs", vec![0, 255, b'\n'], "rendered source")?;
    plan.directory("empty", "empty directory")?;
    plan.file("README.md", b"project\n".to_vec(), "rendered readme")?;
    let script = python_construction_script(&plan)?;

    let mut reordered = Plan::new();
    reordered.file("README.md", b"project\n".to_vec(), "rendered readme")?;
    reordered.directory("empty", "empty directory")?;
    reordered.file("src/main.rs", vec![0, 255, b'\n'], "rendered source")?;
    assert_eq!(script, python_construction_script(&reordered)?);

    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary =
        std::env::temp_dir().join(format!("bellatrix-flow-{}-{unique}", std::process::id()));
    fs::create_dir(&temporary)?;
    let script_path = temporary.join("construct.py");
    let output = temporary.join("project");
    fs::write(&script_path, script)?;
    let python = if cfg!(windows) { "python" } else { "python3" };
    #[expect(
        clippy::disallowed_methods,
        reason = "the integration flow waits for the standalone script to finish"
    )]
    let run = || Command::new(python).arg(&script_path).arg(&output).status();
    assert!(run()?.success());
    assert_eq!(fs::read(output.join("src/main.rs"))?, vec![0, 255, b'\n']);
    assert_eq!(fs::read(output.join("README.md"))?, b"project\n");
    assert!(output.join("empty").is_dir());
    fs::write(output.join("README.md"), b"changed")?;
    assert!(run()?.success());
    assert_eq!(fs::read(output.join("README.md"))?, b"project\n");
    fs::remove_dir_all(temporary)?;
    Ok(())
}
