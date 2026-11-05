use bellatrix_mde::{Format, Project, decode, encode};
use serde_json::json;

fn document() -> Project {
    decode(
        &serde_json::to_vec(&json!({
            "id":"collection-contracts", "name":"Collection contracts", "root_model":"root",
            "elements":[
                {"id":"root", "kind":"uml.model", "name":"Root"},
                {"id":"class", "kind":"uml.class", "name":"Owner"},
                {"id":"operation", "kind":"uml.operation", "name":"accept"},
                {"id":"other", "kind":"uml.operation", "name":"other"},
                {"id":"parameter", "kind":"uml.parameter", "name":"input"},
                {"id":"comment", "kind":"uml.comment", "name":"Comment"},
                {"id":"custom", "kind":"custom.type", "name":"Custom"}
            ],
            "relations":[
                {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
                {"id":"owns-operation", "kind":"owns", "from":"class", "to":"operation"},
                {"id":"owns-other", "kind":"owns", "from":"class", "to":"other"},
                {"id":"owns-parameter", "kind":"owns", "from":"other", "to":"parameter"},
                {"id":"owns-comment", "kind":"owns", "from":"root", "to":"comment"},
                {"id":"owns-custom", "kind":"owns", "from":"root", "to":"custom"}
            ]
        }))
        .expect("collection document"),
        Format::Json,
    )
    .expect("load collection document")
}

#[test]
fn mixed_collection_survives_round_trip() {
    for values in [
        json!([{"href":"other.uml#Type"}, {"$ref":"missing"}]),
        json!([{"$ref":"missing"}, {"href":"other.uml#Type"}]),
    ] {
        let mut project = document();
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "operation")
            .expect("Operation")
            .properties
            .insert("raisedException".into(), values);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(
                &encode(&project, format).expect("save mixed collection"),
                format,
            )
            .expect("reopen mixed collection");
            assert_eq!(reopened, project);
        }
    }
}

#[test]
fn external_reference_shape_survives_round_trip() {
    for values in [
        json!([{"href":"other.uml#Type"}]),
        json!([{"href":false}]),
        json!([{"href":null}]),
        json!([{"href":12}]),
        json!([{"href":"other.uml#Type", "$ref":"class"}]),
        json!([{"href":"other.uml#Type"}, {"$ref":"missing"}]),
        json!([{"$ref":"missing"}, {"href":"other.uml#Type"}]),
    ] {
        let mut project = document();
        project
            .elements
            .iter_mut()
            .find(|element| element.id == "operation")
            .expect("Operation")
            .properties
            .insert("raisedException".into(), values);
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&project, format).expect("save draft"), format)
                .expect("reload draft");
            assert_eq!(reopened, project);
        }
    }
}
