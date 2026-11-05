use bellatrix_mde::{
    Format, Project, UmlQuickCreate,
    commands::{Command, ModelingMemberKind, apply},
    decode, encode,
};
use serde_json::json;

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

#[test]
fn member_and_quick_edit_create_explicitly_incomplete_receptions_without_inventing_signals() {
    for command in [
        Command::AddMember {
            owner_id: "class".into(),
            kind: ModelingMemberKind::Reception,
        },
        Command::QuickEdit {
            diagram_id: "diagram".into(),
            element_id: "class".into(),
            action: UmlQuickCreate::Reception,
        },
    ] {
        let original = document();
        let created = apply(&original, &command, 1).expect("create an incomplete Reception");
        assert_eq!(created.elements.len(), original.elements.len() + 1);
        let reception = created
            .elements
            .iter()
            .find(|element| element.kind == "uml.reception")
            .expect("created Reception");
        let reception_id = reception.id.clone();
        assert!(!reception.properties.contains_key("signal"));
        assert_eq!(created.element("signal"), original.element("signal"));
        let renamed = apply(
            &created,
            &Command::Rename {
                element_id: reception_id.clone(),
                name: "Known".into(),
            },
            2,
        )
        .expect("continue authoring an incomplete Reception");
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(
                &encode(&renamed, format).expect("save incomplete draft"),
                format,
            )
            .expect("reopen incomplete draft");
            assert_eq!(reopened, renamed);
        }
    }
}
