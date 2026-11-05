use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, prepare_model_document,
    project_model_section,
};
use bellatrix_mde_contracts::{
    ModelingSection, View,
    protocol::{
        Command, ModelDocumentParts, ModelingElementKind, ModelingMemberKind, ModelingRelationKind,
        inspector::PropertyEditor, projection::ModelingProjection,
    },
    ruml::{Contract, class::Class},
};
use serde_json::json;
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

    fn projection(&self) -> FlowResult<ModelingProjection> {
        Ok(project_model_section(
            &self.document.id,
            &self.document.name,
            &self.document.section_json,
        )?)
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

    for (name, command) in [
        (
            "CustomerInvoice",
            Command::Rename {
                element_id: invoice.clone(),
                name: "CustomerInvoice".into(),
            },
        ),
        (
            "SalesInvoice",
            Command::Rename {
                element_id: invoice.clone(),
                name: "SalesInvoice".into(),
            },
        ),
        (
            "CustomerInvoice",
            Command::SetProperty {
                element_id: invoice.clone(),
                path: "name".into(),
                value: json!("CustomerInvoice"),
            },
        ),
        (
            "SalesInvoice",
            Command::SetProperty {
                element_id: invoice.clone(),
                path: "name".into(),
                value: json!("SalesInvoice"),
            },
        ),
        (
            "CustomerInvoice",
            Command::SetNameExpression {
                element_id: invoice.clone(),
                expression: "+ CustomerInvoice".into(),
            },
        ),
        (
            "SalesInvoice",
            Command::SetNameExpression {
                element_id: invoice.clone(),
                expression: "+ SalesInvoice".into(),
            },
        ),
    ] {
        let renamed = flow.apply(command)?;
        assert_eq!(class(&renamed, &invoice)?.name.as_deref(), Some(name));
        assert_eq!(semantic_view(&renamed, &invoice)?.id, invoice_view);
        let projection = flow.projection()?;
        let Some(Contract::Class(projected_class)) = projection.contract(&invoice) else {
            return Err("UI snapshot must retain the concrete Class contract".into());
        };
        assert_eq!(projected_class.name.as_deref(), Some(name));
        let displayed = projection
            .elements
            .iter()
            .find(|element| element.id == invoice)
            .ok_or("renamed Class disappeared from the public UI projection")?;
        assert_eq!(displayed.name, name);
        let inspector = projection
            .inspect_element(&invoice)
            .ok_or("renamed Class disappeared from the Inspector")?;
        let name_properties: Vec<_> = inspector
            .properties
            .iter()
            .filter(|property| matches!(property.path.as_str(), "name" | "properties.name"))
            .collect();
        let [name_property] = name_properties.as_slice() else {
            return Err(
                "Class must expose one name editor without a properties.name duplicate".into(),
            );
        };
        assert_eq!(name_property.path, "name");
        assert!(name_property.editable);
        assert_eq!(name_property.value, json!(name));
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
    let nested = flow.create(
        Command::CreateElement {
            diagram_id: "classes".into(),
            owner_id: Some(invoice.clone()),
            kind: ModelingElementKind::Class,
            name: Some("LineItem".into()),
            x: 100.0,
            y: 350.0,
        },
        "Class",
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
    let association_view = semantic_view(&flow.section()?, &association)?.id.clone();
    for name in ["documentReference", "sourceDocument"] {
        let renamed = flow.apply(Command::SetProperty {
            element_id: association.clone(),
            path: "properties.name".into(),
            value: json!(name),
        })?;
        let Contract::Association(named_association) = contract(&renamed, &association)? else {
            return Err("renaming must preserve the Association contract".into());
        };
        assert_eq!(named_association.name.as_deref(), Some(name));
        assert_eq!(semantic_view(&renamed, &association)?.id, association_view);
        let projection = flow.projection()?;
        let Some(Contract::Association(projected_association)) = projection.contract(&association)
        else {
            return Err("UI snapshot must retain the concrete Association contract".into());
        };
        assert_eq!(projected_association.name.as_deref(), Some(name));
        let inspector = projection
            .inspect_element(&association)
            .ok_or("renamed Association disappeared from the Inspector")?;
        let name_property = inspector
            .properties
            .iter()
            .find(|property| property.path == "properties.name")
            .ok_or("Association Inspector must expose its canonical name property")?;
        assert_eq!(name_property.value, json!(name));
        assert!(name_property.editable);
    }
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

    let before_navigation = flow.section()?;
    let Contract::Association(before_association) = contract(&before_navigation, &association)?
    else {
        return Err("Association must retain its contract before editing its ends".into());
    };
    let member_ends = before_association.member_end.clone();
    let association_inspector = flow
        .projection()?
        .inspect_element(&association)
        .ok_or("Association must expose its member ends in the Inspector")?;
    assert_eq!(
        association_inspector
            .association_ends
            .iter()
            .map(|member| member.id.clone())
            .collect::<Vec<_>>(),
        member_ends
    );
    let selected_end = association_inspector
        .association_ends
        .first()
        .ok_or("Association must offer an end to inspect")?
        .id
        .clone();
    for navigation in ["navigable", "notNavigable", "unverified", "navigable"] {
        let changed = flow.apply(Command::SetProperty {
            element_id: selected_end.clone(),
            path: "authoring.navigation".into(),
            value: json!(navigation),
        })?;
        let Contract::Association(changed_association) = contract(&changed, &association)? else {
            return Err("editing a member end must preserve the Association contract".into());
        };
        assert_eq!(changed_association.member_end, member_ends);
        assert_eq!(
            changed_association.navigable_owned_end,
            if navigation == "navigable" {
                vec![selected_end.clone()]
            } else {
                Vec::<String>::new()
            }
        );
        assert_eq!(
            changed
                .authoring_knowledge
                .associations
                .get(&association)
                .is_some_and(|knowledge| knowledge.navigation_unverified.contains(&selected_end)),
            navigation == "unverified"
        );
        assert_eq!(semantic_view(&changed, &association)?.id, association_view);
        let projection = flow.projection()?;
        let end_inspector = projection
            .inspect_element(&selected_end)
            .ok_or("selecting an Association member end must expose its Property Inspector")?;
        assert!(matches!(
            projection.contract(&selected_end),
            Some(Contract::Property(_))
        ));
        let navigation_editor = end_inspector
            .properties
            .iter()
            .find(|property| property.path == "authoring.navigation")
            .ok_or("Association-owned Property must expose navigation knowledge")?;
        assert_eq!(
            navigation_editor.editor,
            PropertyEditor::Choice(vec![
                "navigable".into(),
                "notNavigable".into(),
                "unverified".into()
            ])
        );
        assert!(navigation_editor.editable);
        assert_eq!(navigation_editor.value, json!(navigation));
        let derived_navigation = end_inspector
            .properties
            .iter()
            .find(|property| property.path == "uml.isNavigable")
            .ok_or("Property must retain its derived isNavigable result")?;
        assert_eq!(derived_navigation.editor, PropertyEditor::ReadOnly);
        assert!(!derived_navigation.editable);
        if navigation == "unverified" {
            assert!(
                derived_navigation
                    .value
                    .as_str()
                    .is_some_and(|message| message.contains("unverified"))
            );
        } else {
            assert_eq!(derived_navigation.value, json!(navigation == "navigable"));
        }
        reopen_both_formats(&flow.document, &changed)?;
    }

    let authored = flow.section()?;
    let invoice_class = class(&authored, &invoice)?;
    assert_eq!(invoice_class.name.as_deref(), Some("SalesInvoice"));
    assert_eq!(invoice_class.owned_attribute, vec![attribute.clone()]);
    assert_eq!(invoice_class.owned_operation, vec![operation.clone()]);
    assert_eq!(invoice_class.nested_classifier, vec![nested.clone()]);
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
    assert_eq!(class(&authored, &nested)?.name.as_deref(), Some("LineItem"));

    let projection = flow.projection()?;
    let Some(Contract::Class(projected_class)) = projection.contract(&invoice) else {
        return Err("authored UI snapshot must retain the concrete Class contract".into());
    };
    assert_eq!(projected_class, invoice_class);
    assert!(projection.contracts.owns_attribute(&invoice, &attribute));
    for other_member in [&operation, &nested, &association] {
        assert!(!projection.contracts.owns_attribute(&invoice, other_member));
    }
    assert!(!projection.contracts.owns_attribute(&document, &attribute));
    let inspector = projection
        .inspect_element(&invoice)
        .ok_or("authored Class must remain inspectable")?;
    let final_specialization = inspector
        .properties
        .iter()
        .find(|property| property.path == "properties.isFinalSpecialization")
        .ok_or("Class Inspector must explain the unavailable final-specialization field")?;
    assert_eq!(final_specialization.editor, PropertyEditor::ReadOnly);
    assert!(!final_specialization.editable);
    assert_eq!(final_specialization.value, json!("Unavailable"));

    let generalization_inspector = projection
        .inspect_element(&generalization)
        .ok_or("authored Generalization must remain inspectable")?;
    for unsupported_path in ["properties.name", "properties.sourceMultiplicity"] {
        assert!(
            generalization_inspector
                .properties
                .iter()
                .all(|property| property.path != unsupported_path),
            "Generalization must not expose {unsupported_path} as its UML property"
        );
    }
    let association_inspector = projection
        .inspect_element(&association)
        .ok_or("authored Association must remain inspectable")?;
    assert!(
        association_inspector
            .properties
            .iter()
            .all(|property| property.path != "properties.navigable")
    );
    assert_eq!(
        association_inspector
            .association_ends
            .iter()
            .map(|member| member.id.clone())
            .collect::<Vec<_>>(),
        member_ends
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
    assert_eq!(binary.name.as_deref(), Some("sourceDocument"));
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
        &nested,
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
