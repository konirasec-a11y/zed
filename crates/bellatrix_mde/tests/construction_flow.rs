use bellatrix_mde::{
    InputPort, Operation, OperationDefinition, OperationGraph, OperationGraphError, OperationValue,
    OperationValues, Plan, PortType, python_construction_script,
};
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn construction_script_builds_the_rendered_project() -> Result<(), Box<dyn std::error::Error>> {
    fn combine(
        _: &str,
        inputs: &OperationValues,
        _: &OperationValues,
        _: &mut Plan,
    ) -> Result<OperationValue, OperationGraphError> {
        match (inputs.get("left"), inputs.get("right")) {
            (Some(OperationValue::Content(left)), Some(OperationValue::Content(right)))
                if left == right =>
            {
                Ok(OperationValue::Content(left.clone()))
            }
            _ => Err(OperationGraphError::InvalidArguments(
                "expected matching content".into(),
            )),
        }
    }
    fn graph(reverse: bool) -> Result<OperationGraph, Box<dyn std::error::Error>> {
        let mut graph = OperationGraph::new();
        graph.register(OperationDefinition {
            kind: "combine".into(),
            version: 1,
            inputs: vec![
                InputPort {
                    name: "left".into(),
                    port_type: PortType::Content,
                },
                InputPort {
                    name: "right".into(),
                    port_type: PortType::Content,
                },
            ],
            output: PortType::Content,
            execute: combine,
        })?;
        graph.add_registered("combined", "combine", 1, OperationValues::new())?;
        let mut nodes = vec![
            ("root", Operation::RootDirectory),
            ("src", Operation::Directory { name: "src".into() }),
            (
                "empty",
                Operation::Directory {
                    name: "empty".into(),
                },
            ),
            (
                "source",
                Operation::RenderedContent {
                    bytes: vec![0, 255, b'\n'],
                },
            ),
            (
                "readme",
                Operation::RenderedContent {
                    bytes: b"project\n".to_vec(),
                },
            ),
            (
                "main",
                Operation::File {
                    name: "main.rs".into(),
                },
            ),
            (
                "readme_file",
                Operation::File {
                    name: "README.md".into(),
                },
            ),
        ];
        if reverse {
            nodes.reverse();
        }
        for (id, node) in nodes {
            graph.add(id, node)?;
        }
        for (source, target, port) in [
            ("root", "src", "parent"),
            ("root", "empty", "parent"),
            ("src", "main", "directory"),
            ("source", "main", "content"),
            ("root", "readme_file", "directory"),
            ("readme", "combined", "left"),
            ("readme", "combined", "right"),
            ("combined", "readme_file", "content"),
        ] {
            graph.connect(source, target, port)?;
        }
        Ok(graph)
    }
    let plan = graph(false)?.execute()?;
    let script = python_construction_script(&plan)?;
    assert_eq!(
        script,
        python_construction_script(&graph(true)?.execute()?)?
    );
    let mut incompatible = graph(false)?;
    incompatible.add("extra", Operation::File { name: "bad".into() })?;
    assert!(matches!(
        incompatible.connect("root", "extra", "content"),
        Err(OperationGraphError::TypeMismatch { .. })
    ));
    assert!(matches!(
        incompatible.execute(),
        Err(OperationGraphError::MissingInput { .. })
    ));
    let mut cyclic = OperationGraph::new();
    cyclic.add("a", Operation::Directory { name: "a".into() })?;
    cyclic.add("b", Operation::Directory { name: "b".into() })?;
    cyclic.connect("a", "b", "parent")?;
    cyclic.connect("b", "a", "parent")?;
    assert!(matches!(
        cyclic.execute(),
        Err(OperationGraphError::Cycle(_))
    ));

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
