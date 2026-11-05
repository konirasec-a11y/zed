use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, prepare_model_document,
    project_model_section,
};
use bellatrix_mde_contracts::{
    ModelingSection, View,
    protocol::{
        Command, ModelDocumentParts, ModelingElementKind, ModelingMemberKind, ModelingRelationKind,
    },
    ruml::{Contract, class::Class},
};
use std::{collections::BTreeSet, error::Error};

type FlowResult<T> = Result<T, Box<dyn Error>>;

struct NativeAuthoring {
    document: ModelDocumentParts,
    revision: u64,
}

impl NativeAuthoring {
    fn section(&self) -> FlowResult<ModelingSection> {
        Ok(serde_json::from_str(&self.document.section_json)?)
    }

    fn apply(&mut self, command: Command) -> FlowResult<ModelingSection> {
        let revision = self.revision.checked_add(1).ok_or("revision overflow")?;
        let candidate = apply_model_command(
            &self.document.id,
            &self.document.name,
            &self.document.section_json,
            revision,
            command.clone(),
        )
        .map_err(|error| format!("public authoring command {command:?}: {error}"))?;
        let section_json = serde_json::to_string(&candidate)?;
        let section = serde_json::from_str(&section_json)?;
        self.document.section_json = section_json;
        self.revision = revision;
        Ok(section)
    }

    fn create(&mut self, command: Command, metaclass: &str) -> FlowResult<String> {
        let before = self.section()?;
        let after = self.apply(command)?;
        let created: Vec<_> = after
            .registry
            .entries()
            .values()
            .filter(|entry| entry.type_name() == metaclass && !before.registry.exists(&entry.id))
            .map(|entry| entry.id.clone())
            .collect();
        match created.as_slice() {
            [identity] => Ok(identity.clone()),
            _ => Err(format!("expected one new {metaclass} contract, got {created:?}").into()),
        }
    }
}

fn contract<'a>(section: &'a ModelingSection, identity: &str) -> FlowResult<&'a Contract> {
    section
        .registry
        .get_entry(identity)
        .map(|entry| &entry.contract)
        .ok_or_else(|| format!("missing UML contract {identity}").into())
}

fn class<'a>(section: &'a ModelingSection, identity: &str) -> FlowResult<&'a Class> {
    match contract(section, identity)? {
        Contract::Class(class) => Ok(class),
        other => Err(format!("expected Class {identity}, got {}", other.type_name()).into()),
    }
}

fn semantic_view<'a>(section: &'a ModelingSection, identity: &str) -> FlowResult<&'a View> {
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == "classes")
        .ok_or("missing native class diagram")?;
    let views: Vec<_> = diagram
        .all_views()
        .filter(|view| view.subject.as_deref() == Some(identity))
        .collect();
    match views.as_slice() {
        [view] if view.draft.is_none() => {
            contract(section, identity)?;
            Ok(*view)
        }
        _ => Err(format!("{identity} must have one semantic View without a visual draft").into()),
    }
}

fn reopen_both_formats(
    document: &ModelDocumentParts,
    expected: &ModelingSection,
) -> FlowResult<()> {
    // Compare the typed canonical section, not a legacy projection that can lose UML fields.
    for format in ["json", "protobuf"] {
        let bytes =
            encode_model_section(&document.id, &document.name, &document.section_json, format)?;
        let reopened = prepare_model_document(&decode_model_document(&bytes, format)?)?;
        let restored: ModelingSection = serde_json::from_str(&reopened.section_json)?;
        assert_eq!(reopened.id, document.id);
        assert_eq!(reopened.name, document.name);
        assert_eq!(
            restored, *expected,
            "{format} changed authored UML or presentation"
        );
    }
    Ok(())
}

#[test]
fn author_classes_and_relationships_from_blank_then_reopen() -> FlowResult<()> {
    let envelope = decode_model_document(include_bytes!("../fixtures/blank.json"), "json")?;
    let mut flow = NativeAuthoring {
        document: prepare_model_document(&envelope)?,
        revision: 0,
    };
    let initial = flow.section()?;
    assert_eq!(initial.root_model.as_deref(), Some("model"));
    assert_eq!(initial.registry.entries().len(), 1);
    let [initial_diagram] = initial.diagrams.as_slice() else {
        return Err("blank fixture must contain its one native class diagram".into());
    };
    assert_eq!(initial_diagram.id, "classes");
    assert_eq!(initial_diagram.uml_di_type, "UMLClassDiagram");
    assert!(initial_diagram.model_elements.is_empty());
    assert!(initial_diagram.views.is_empty());

    // The native toolbar has no selected owner; adding one here would hide the regression.
    let invoice = flow.create(
        Command::CreateElement {
            diagram_id: "classes".into(),
            owner_id: None,
            kind: ModelingElementKind::Class,
            name: Some("Invoice".into()),
            x: 100.0,
            y: 100.0,
        },
        "Class",
    )?;
    let created = flow.section()?;
    assert_eq!(class(&created, &invoice)?.name.as_deref(), Some("Invoice"));
    let invoice_view = semantic_view(&created, &invoice)?.id.clone();
    let Contract::Model(root) = contract(&created, "model")? else {
        return Err("native root must remain a Model contract".into());
    };
    assert_eq!(root.packaged_element, vec![invoice.clone()]);

    let document = flow.create(
        Command::CreateElement {
            diagram_id: "classes".into(),
            owner_id: None,
            kind: ModelingElementKind::Class,
            name: Some("Document".into()),
            x: 450.0,
            y: 100.0,
        },
        "Class",
    )?;
    let printable = flow.create(
        Command::CreateElement {
            diagram_id: "classes".into(),
            owner_id: None,
            kind: ModelingElementKind::Interface,
            name: Some("Printable".into()),
            x: 450.0,
            y: 350.0,
        },
        "Interface",
    )?;

    for name in ["CustomerInvoice", "SalesInvoice"] {
        let renamed = flow.apply(Command::Rename {
            element_id: invoice.clone(),
            name: name.into(),
        })?;
        assert_eq!(class(&renamed, &invoice)?.name.as_deref(), Some(name));
        assert_eq!(semantic_view(&renamed, &invoice)?.id, invoice_view);
        let projection = project_model_section(
            &flow.document.id,
            &flow.document.name,
            &flow.document.section_json,
        )?;
        let displayed = projection
            .elements
            .iter()
            .find(|element| element.id == invoice)
            .ok_or("renamed Class disappeared from the public UI projection")?;
        assert_eq!(displayed.name, name);
    }

    let attribute = flow.create(
        Command::AddMember {
            owner_id: invoice.clone(),
            kind: ModelingMemberKind::Attribute,
        },
        "Property",
    )?;
    let operation = flow.create(
        Command::AddMember {
            owner_id: invoice.clone(),
            kind: ModelingMemberKind::Operation,
        },
        "Operation",
    )?;
    let association = flow.create(
        Command::CreateRelation {
            diagram_id: "classes".into(),
            owner_id: Some("model".into()),
            kind: ModelingRelationKind::Association,
            from: invoice.clone(),
            to: document.clone(),
        },
        "Association",
    )?;
    let generalization = flow.create(
        Command::CreateRelation {
            diagram_id: "classes".into(),
            owner_id: Some(invoice.clone()),
            kind: ModelingRelationKind::Generalization,
            from: invoice.clone(),
            to: document.clone(),
        },
        "Generalization",
    )?;
    let dependency = flow.create(
        Command::CreateRelation {
            diagram_id: "classes".into(),
            owner_id: Some("model".into()),
            kind: ModelingRelationKind::Dependency,
            from: document.clone(),
            to: printable.clone(),
        },
        "Dependency",
    )?;
    let realization = flow.create(
        Command::CreateRelation {
            diagram_id: "classes".into(),
            owner_id: Some(invoice.clone()),
            kind: ModelingRelationKind::InterfaceRealization,
            from: invoice.clone(),
            to: printable.clone(),
        },
        "InterfaceRealization",
    )?;

    let authored = flow.section()?;
    let invoice_class = class(&authored, &invoice)?;
    assert_eq!(invoice_class.name.as_deref(), Some("SalesInvoice"));
    assert_eq!(invoice_class.owned_attribute, vec![attribute.clone()]);
    assert_eq!(invoice_class.owned_operation, vec![operation.clone()]);
    assert_eq!(invoice_class.generalization, vec![generalization.clone()]);
    assert_eq!(
        invoice_class.interface_realization,
        vec![realization.clone()]
    );
    assert!(matches!(
        contract(&authored, &attribute)?,
        Contract::Property(_)
    ));
    let Contract::Operation(owned_operation) = contract(&authored, &operation)? else {
        return Err("AddMember must persist an Operation contract".into());
    };
    assert_eq!(owned_operation.class.as_deref(), Some(invoice.as_str()));
    assert!(matches!(
        contract(&authored, &printable)?,
        Contract::Interface(_)
    ));
    assert_eq!(
        class(&authored, &document)?.name.as_deref(),
        Some("Document")
    );

    let Contract::Model(root) = contract(&authored, "model")? else {
        return Err("authored root must remain a Model contract".into());
    };
    assert_eq!(
        root.packaged_element.iter().collect::<BTreeSet<_>>(),
        [&invoice, &document, &printable, &association, &dependency]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );

    let Contract::Association(binary) = contract(&authored, &association)? else {
        return Err("relationship command must persist an Association contract".into());
    };
    let [source_end, target_end] = binary.member_end.as_slice() else {
        return Err("binary Association must retain two Property memberEnds".into());
    };
    assert_eq!(binary.owned_end, binary.member_end);
    for (identity, classifier) in [(source_end, &invoice), (target_end, &document)] {
        let Contract::Property(end) = contract(&authored, identity)? else {
            return Err(format!("Association end {identity} is not a Property").into());
        };
        assert_eq!(end.type_.as_deref(), Some(classifier.as_str()));
        assert_eq!(end.association.as_deref(), Some(association.as_str()));
        assert_eq!(
            end.owning_association.as_deref(),
            Some(association.as_str())
        );
    }

    let Contract::Generalization(inheritance) = contract(&authored, &generalization)? else {
        return Err("relationship command must persist a Generalization contract".into());
    };
    assert_eq!(
        (&inheritance.specific, &inheritance.general),
        (&invoice, &document)
    );
    let Contract::Dependency(requirement) = contract(&authored, &dependency)? else {
        return Err("relationship command must persist a Dependency contract".into());
    };
    assert_eq!(requirement.client, vec![document.clone()]);
    assert_eq!(requirement.supplier, vec![printable.clone()]);
    let Contract::InterfaceRealization(implementation) = contract(&authored, &realization)? else {
        return Err("relationship command must persist an InterfaceRealization contract".into());
    };
    assert_eq!(implementation.implementing_classifier, invoice);
    assert_eq!(implementation.contract, printable);
    assert_eq!(implementation.client, vec![invoice.clone()]);
    assert_eq!(implementation.supplier, vec![printable.clone()]);
    for identity in [
        &invoice,
        &document,
        &printable,
        &association,
        &generalization,
        &dependency,
        &realization,
    ] {
        semantic_view(&authored, identity)?;
    }

    reopen_both_formats(&flow.document, &authored)?;

    let deleted = flow.apply(Command::DeleteElement {
        element_id: "model".into(),
    })?;
    assert!(deleted.root_model.is_none());
    assert!(deleted.registry.entries().is_empty());
    let [empty_diagram] = deleted.diagrams.as_slice() else {
        return Err("deleting UML content must retain the independent class diagram".into());
    };
    assert_eq!(empty_diagram.id, initial_diagram.id);
    assert_eq!(empty_diagram.uml_di_type, "UMLClassDiagram");
    assert!(empty_diagram.model_elements.is_empty());
    assert!(empty_diagram.views.is_empty());
    reopen_both_formats(&flow.document, &deleted)?;
    Ok(())
}
