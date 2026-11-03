use bellatrix_mde::{
    Format, Project, StructuralSeverity, UmlQuickCreate,
    commands::{Command, ModelingMemberKind, apply}, decode, encode,
    uml::root::Model,
};
use serde_json::{Value, json};

fn document() -> Project {
    decode(&serde_json::to_vec(&json!({
        "id":"reception-drafts", "name":"Reception drafts", "root_model":"root",
        "elements":[
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"class", "kind":"uml.class", "name":"Receiver", "properties":{"isActive":true}},
            {"id":"signal", "kind":"uml.signal", "name":"Known"}
        ],
        "relations":[
            {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
            {"id":"owns-signal", "kind":"owns", "from":"root", "to":"signal"}
        ],
        "diagrams":[{
            "id":"diagram", "name":"Classes", "uml_di_type":"UMLClassDiagram",
            "width":800, "height":600,
            "views":[{"id":"class-view", "subject":"class", "style":{}, "z_index":1,
                "geometry":{"kind":"rect", "x":20, "y":20, "width":200, "height":120}}]
        }]
    })).expect("draft document"), Format::Json).expect("load draft document")
}

fn assert_incomplete(project: &Project, reception_id: &str) {
    let diagnostics = Model::from_project(project).validate();
    assert!(diagnostics.iter().any(|issue|
        issue.subject_id == reception_id && issue.code == "MDE_RECEPTION_SIGNAL_INCOMPLETE"
            && issue.severity == StructuralSeverity::Warning
            && issue.message.contains("UML conformance is not established")), "{diagnostics:?}");
    assert!(!diagnostics.iter().any(|issue|
        issue.subject_id == reception_id && issue.severity == StructuralSeverity::Error), "{diagnostics:?}");
}

#[test]
fn member_and_quick_edit_create_explicitly_incomplete_receptions_without_inventing_signals() {
    for command in [
        Command::AddMember { owner_id:"class".into(), kind:ModelingMemberKind::Reception },
        Command::QuickEdit { diagram_id:"diagram".into(), element_id:"class".into(),
            action:UmlQuickCreate::Reception },
    ] {
        let original = document();
        let created = apply(&original, &command, 1).expect("create an incomplete Reception");
        assert_eq!(created.elements.len(), original.elements.len() + 1);
        let reception = created.elements.iter().find(|element| element.kind == "uml.reception")
            .expect("created Reception");
        let reception_id = reception.id.clone();
        assert!(!reception.properties.contains_key("signal"));
        assert_eq!(created.element("signal"), original.element("signal"));
        assert_incomplete(&created, &reception_id);
        let renamed = apply(&created, &Command::Rename {
            element_id:reception_id.clone(), name:"Known".into(),
        }, 2).expect("continue authoring an incomplete Reception");
        assert_incomplete(&renamed, &reception_id);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&renamed, format).expect("save incomplete draft"), format)
                .expect("reopen incomplete draft");
            assert_eq!(reopened, renamed);
            assert_incomplete(&reopened, &reception_id);
        }
        // The current Inspector has no Signal picker. Import an explicitly
        // selected reference to verify that completion invokes the invariants.
        let mut completed = renamed.clone();
        completed.elements.iter_mut().find(|element| element.id == reception_id)
            .expect("Reception").properties.insert("signal".into(), json!({"$ref":"signal"}));
        assert!(!Model::from_project(&completed).validate().iter().any(|issue|
            issue.subject_id == reception_id && issue.code.contains("RECEPTION")));
        assert!(apply(&completed, &Command::Rename {
            element_id:reception_id.clone(), name:"Wrong".into(),
        }, 3).is_err());
        for invalid in [Value::Null, json!({"$ref":"missing"}), json!({"$ref":"class"})] {
            let mut invalid_draft = renamed.clone();
            invalid_draft.elements.iter_mut().find(|element| element.id == reception_id)
                .expect("Reception").properties.insert("signal".into(), invalid);
            let diagnostics = Model::from_project(&invalid_draft).validate();
            assert!(diagnostics.iter().any(|issue|
                issue.subject_id == reception_id && issue.code == "UML_RECEPTION_CONSTRAINT"
                    && issue.severity == StructuralSeverity::Error));
            assert!(!diagnostics.iter().any(|issue| issue.code == "MDE_RECEPTION_SIGNAL_INCOMPLETE"));
        }
    }
}
