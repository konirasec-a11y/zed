use bellatrix_mde::{
    Operation, OperationDefinition, OperationGraph, OperationGraphError, OperationValue,
    OperationValues, Plan, PortType, python_construction_script,
};
use std::path::PathBuf;

fn record_order(
    id: &str,
    _: &OperationValues,
    _: &OperationValues,
    plan: &mut Plan,
) -> Result<OperationValue, OperationGraphError> {
    plan.warn("TEST_EXECUTION_ORDER", id, Some(id));
    Ok(OperationValue::Text(id.into()))
}

fn operations(bytes: &[u8], reverse: bool) -> OperationGraph {
    let mut graph = OperationGraph::new();
    graph
        .register(OperationDefinition {
            kind: "record_order".into(),
            version: 1,
            inputs: vec![],
            output: PortType::Text,
            execute: record_order,
        })
        .expect("register pure observer");
    let mut nodes = vec![
        ("root", Operation::RootDirectory),
        ("directory", Operation::Directory { name: "src".into() }),
        (
            "content",
            Operation::RenderedContent {
                bytes: bytes.to_vec(),
            },
        ),
        (
            "file",
            Operation::File {
                name: "main.txt".into(),
            },
        ),
    ];
    if reverse {
        nodes.reverse();
    }
    for (id, node) in nodes {
        graph.add(id, node).expect("add operation");
    }
    let observer_ids = if reverse {
        ["observer-z", "observer-a"]
    } else {
        ["observer-a", "observer-z"]
    };
    for id in observer_ids {
        graph
            .add_registered(id, "record_order", 1, OperationValues::new())
            .expect("add observer");
    }
    let mut links = vec![
        ("root", "directory", "parent"),
        ("directory", "file", "directory"),
        ("content", "file", "content"),
    ];
    if reverse {
        links.reverse();
    }
    for (source, target, port) in links {
        graph.connect(source, target, port).expect("connect input");
    }
    graph
}

#[derive(Debug, PartialEq, Eq)]
struct Output {
    artifacts: Vec<(PathBuf, Option<Vec<u8>>, String)>,
    diagnostics: Vec<(String, String, Option<String>)>,
    script: String,
}

fn snapshot(plan: &Plan) -> Output {
    Output {
        artifacts: plan
            .artifacts()
            .map(|artifact| {
                (
                    artifact.path.clone(),
                    artifact.contents.clone(),
                    artifact.origin.clone(),
                )
            })
            .collect(),
        diagnostics: plan
            .diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code.clone(),
                    diagnostic.message.clone(),
                    diagnostic.element.clone(),
                )
            })
            .collect(),
        script: python_construction_script(plan).expect("serialize script without executing it"),
    }
}

fn execute(graph: &OperationGraph) -> Output {
    snapshot(&graph.execute().expect("execute built-in pure operations"))
}

#[test]
fn operation_plans_and_scripts_are_repeatable_aba_parallel_and_insertion_independent() {
    let first = operations(b"first\0\xff\n", false);
    let second = operations(b"second", true);
    let expected = execute(&first);
    assert_eq!(
        expected
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.1.as_str())
            .collect::<Vec<_>>(),
        vec!["observer-a", "observer-z"]
    );
    assert_eq!(execute(&operations(b"first\0\xff\n", true)), expected);
    for _ in 0..16 {
        assert_eq!(execute(&first), expected);
        assert_ne!(execute(&second), expected);
        assert_eq!(execute(&first), expected);
    }
    std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    let before = execute(&first);
                    let other = execute(&second);
                    let after = execute(&first);
                    (before, other, after)
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            let (before, other, after) = handle.join().expect("pure execution worker");
            assert_eq!(before, expected);
            assert_eq!(after, expected);
            assert_ne!(other, expected);
        }
    });
}

#[test]
fn plan_artifact_order_and_script_bytes_do_not_follow_insertion_order() {
    let mut first = Plan::new();
    let mut second = Plan::new();
    let entries = [
        ("z.txt", Some(vec![255, 0]), "last"),
        ("A.txt", Some(vec![65]), "first"),
        ("empty", None, "directory"),
    ];
    for (path, bytes, origin) in entries.iter() {
        first
            .insert((*path).into(), bytes.clone(), *origin)
            .expect("first insertion");
    }
    for (path, bytes, origin) in entries.iter().rev() {
        second
            .insert((*path).into(), bytes.clone(), *origin)
            .expect("reverse insertion");
    }
    assert_eq!(snapshot(&first), snapshot(&second));
    assert_eq!(
        first
            .artifacts()
            .map(|artifact| artifact.path.clone())
            .collect::<Vec<_>>(),
        vec![
            PathBuf::from("A.txt"),
            PathBuf::from("empty"),
            PathBuf::from("z.txt")
        ]
    );
}
