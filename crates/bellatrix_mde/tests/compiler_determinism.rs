use bellatrix_mde::{
    GenerationModel, Operation, OperationDefinition, OperationGraph, OperationGraphError,
    OperationValue, OperationValues, Plan, PortType, Project, Scope, analyze_generation_graph,
    python_construction_script, select_elements,
};
use serde_json::json;
use std::{collections::HashSet, path::PathBuf};

fn source(name: &str) -> Project {
    serde_json::from_value(json!({
        "id": "project", "name": name,
        "elements": [
            {"id": "z-client", "kind": "uml.class", "name": name},
            {"id": "b-independent", "kind": "uml.class", "name": "Independent"},
            {"id": "a-base", "kind": "uml.class", "name": "Base"},
            {"id": "external", "kind": "uml.class", "name": "External"}
        ],
        "relations": [
            {"id": "external-reference", "kind": "uml.usage", "from": "z-client", "to": "external"},
            {"id": "dependency", "kind": "uml.dependency", "from": "z-client", "to": "a-base"},
            {"id": "duplicate-dependency", "kind": "uml.dependency", "from": "z-client", "to": "a-base"}
        ]
    })).expect("determinism fixture")
}

fn selection(reverse: bool) -> HashSet<String> {
    let mut ids = vec!["z-client", "a-base", "b-independent"];
    if reverse {
        ids.reverse();
    }
    ids.into_iter().map(str::to_owned).collect()
}

fn analyze(project: &Project, reverse: bool) -> GenerationModel {
    analyze_generation_graph(project, &selection(reverse)).expect("acyclic generation")
}

#[test]
fn generation_is_repeatable_aba_and_parallel_with_stable_ties_and_references() {
    let first = source("First");
    let second = source("Second");
    let original = first.clone();
    let expected = analyze(&first, false);
    assert_eq!(
        expected
            .units
            .iter()
            .map(|unit| unit.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a-base", "b-independent", "z-client"]
    );
    let client = expected.units.last().expect("client unit");
    assert_eq!(
        client
            .requires
            .iter()
            .map(|reference| (reference.id.as_str(), reference.relation_kind.as_str()))
            .collect::<Vec<_>>(),
        vec![("a-base", "uml.dependency"), ("external", "uml.usage")]
    );
    for iteration in 0..32 {
        assert_eq!(analyze(&first, iteration % 2 == 0), expected);
        assert_ne!(analyze(&second, false), expected);
        assert_eq!(analyze(&first, false), expected);
    }
    std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|iteration| {
                let first = &first;
                let second = &second;
                scope.spawn(move || {
                    let before = analyze(first, iteration % 2 == 0);
                    let other = analyze(second, false);
                    let after = analyze(first, false);
                    (before, other, after)
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            let (before, other, after) = handle.join().expect("pure analysis worker");
            assert_eq!(before, expected);
            assert_eq!(after, expected);
            assert_ne!(other, expected);
        }
    });
    let mut reordered = first.clone();
    reordered.elements.reverse();
    reordered.relations.reverse();
    assert_eq!(analyze(&reordered, true), expected);
    assert_eq!(first, original);
    assert_eq!(
        select_elements(&first, &Scope::Project).expect("all elements"),
        select_elements(&reordered, &Scope::Project).expect("same semantic selection")
    );
}

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

#[test]
fn invalid_operation_graph_reports_the_same_first_error_across_repeated_and_parallel_calls() {
    let mut graph = OperationGraph::new();
    graph
        .add(
            "z-file",
            Operation::File {
                name: "z.txt".into(),
            },
        )
        .expect("file");
    graph
        .add(
            "a-file",
            Operation::File {
                name: "a.txt".into(),
            },
        )
        .expect("file");
    let error = || graph.execute().expect_err("unconnected file").to_string();
    let expected = error();
    assert_eq!(expected, "input a-file.directory is missing");
    for _ in 0..16 {
        assert_eq!(error(), expected);
    }
    std::thread::scope(|scope| {
        let handles = (0..8).map(|_| scope.spawn(error)).collect::<Vec<_>>();
        for handle in handles {
            assert_eq!(handle.join().expect("error worker"), expected);
        }
    });
}

#[test]
fn generation_cycle_errors_are_repeatable_aba_and_parallel() {
    let mut cyclic = source("Cyclic");
    cyclic.relations.push(
        serde_json::from_value(json!({
            "id": "back", "kind": "uml.dependency", "from": "a-base", "to": "z-client"
        }))
        .expect("cycle edge"),
    );
    let valid = source("Valid");
    let selected = selection(false);
    let error = || {
        analyze_generation_graph(&cyclic, &selected)
            .expect_err("cycle")
            .to_string()
    };
    let expected = error();
    for _ in 0..32 {
        assert_eq!(error(), expected);
        assert!(analyze_generation_graph(&valid, &selection(true)).is_ok());
        assert_eq!(error(), expected);
    }
    std::thread::scope(|scope| {
        let handles = (0..8).map(|_| scope.spawn(error)).collect::<Vec<_>>();
        for handle in handles {
            assert_eq!(handle.join().expect("cycle worker"), expected);
        }
    });
}
