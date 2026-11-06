use bellatrix_mde::{
    apply_model_command, decode_model_document, encode_model_section, prepare_model_document,
    project_model_section,
};
use bellatrix_mde_contracts::{
    Authored, Geometry, InheritedStyleProperty, ModelingSection, PresentationReference,
    RelationshipRouting, StyleInheritance, View, ViewEndpoints,
    protocol::{
        Command, ModelDocumentParts, ModelingElementKind, ModelingMemberKind, ModelingRelationKind,
        UmlQuickCreate, inspector::PropertyEditor, projection::ModelingProjection,
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

fn assert_retargeted_view(
    flow: &NativeAuthoring,
    relationship: &str,
    original: &View,
    source: &str,
    target: &str,
    source_view: Option<&str>,
    target_view: Option<&str>,
) -> FlowResult<()> {
    let section = flow.section()?;
    let mut expected = original.clone();
    expected.endpoints = Some(ViewEndpoints {
        source_view_id: source_view.map(str::to_owned),
        target_view_id: target_view.map(str::to_owned),
    });
    assert_eq!(semantic_view(&section, relationship)?, &expected);
    let projection = flow.projection()?;
    assert_eq!(
        projection.contract(relationship),
        Some(contract(&section, relationship)?)
    );
    let relation = projection
        .relations
        .iter()
        .find(|relation| relation.id == relationship)
        .ok_or("retargeted relationship disappeared from the public projection")?;
    assert_eq!(
        (relation.from.as_str(), relation.to.as_str()),
        (source, target)
    );
    let projected_view = projection
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| view.id == original.id)
        .ok_or("retargeted relationship lost its existing View")?;
    assert_eq!(projected_view.endpoints, expected.endpoints);
    let projected_relationship = projected_view
        .relationship
        .as_ref()
        .ok_or("retargeted View lost its resolved relationship")?;
    let has_unattached_endpoint = source_view.is_none() || target_view.is_none();
    assert_eq!(
        projected_relationship
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("unattached")),
        has_unattached_endpoint
    );
    if has_unattached_endpoint {
        assert!(projected_view.routed_geometry.is_none());
    } else {
        assert!(projected_view.routed_geometry.is_some());
    }
    Ok(())
}

fn assert_realization_endpoints(
    section: &ModelingSection,
    realization: &str,
    source: &str,
    target: &str,
    owners: [&str; 3],
) -> FlowResult<()> {
    let Contract::InterfaceRealization(implementation) = contract(section, realization)? else {
        return Err("retargeting must preserve the InterfaceRealization contract".into());
    };
    assert_eq!(implementation.name.as_deref(), Some("PrintableContract"));
    assert_eq!(implementation.implementing_classifier, source);
    assert_eq!(implementation.contract, target);
    assert_eq!(implementation.client, vec![source.to_owned()]);
    assert_eq!(implementation.supplier, vec![target.to_owned()]);
    for owner in owners {
        assert_eq!(
            class(section, owner)?.interface_realization,
            if owner == source {
                vec![realization.to_owned()]
            } else {
                Vec::new()
            }
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
    assert_eq!(initial_diagram.model_elements, vec!["model"]);
    assert_eq!(
        initial.hierarchy["classes"].parent_id.as_deref(),
        Some("model")
    );
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

    let explorer = flow.projection()?.explorer.added;
    let invoice_rows = explorer
        .iter()
        .filter(|row| row.element_id == invoice)
        .collect::<Vec<_>>();
    assert_eq!(invoice_rows.len(), 1);
    assert_eq!(invoice_rows[0].depth, 2);
    assert_eq!(explorer[0].element_id, "model");
    assert_eq!(explorer[1].element_id, "classes");
    let before_delete = flow.document.section_json.clone();
    let error = flow
        .apply(Command::DeleteElement {
            element_id: "model".into(),
        })
        .expect_err("a diagram's model context cannot be deleted while still referenced");
    assert!(error.to_string().contains("UMLDI_CONTEXT_IN_USE"));
    assert_eq!(flow.document.section_json, before_delete);

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
    let input_parameter = flow.create(
        Command::AddMember {
            owner_id: operation.clone(),
            kind: ModelingMemberKind::Parameter,
        },
        "Parameter",
    )?;
    let return_parameter = flow.create(
        Command::AddMember {
            owner_id: operation.clone(),
            kind: ModelingMemberKind::Parameter,
        },
        "Parameter",
    )?;
    flow.apply(Command::SetProperty {
        element_id: return_parameter.clone(),
        path: "properties.direction".into(),
        value: json!("return"),
    })?;
    for (element_id, multiplicity) in [
        (&attribute, "0..*"),
        (&input_parameter, "2..4"),
        (&return_parameter, "0..1"),
    ] {
        flow.apply(Command::SetProperty {
            element_id: element_id.clone(),
            path: "properties.type".into(),
            value: json!({"$ref": document}),
        })?;
        flow.apply(Command::SetProperty {
            element_id: element_id.clone(),
            path: "properties.multiplicity".into(),
            value: json!(multiplicity),
        })?;
    }
    let with_multiplicity = flow.section()?;
    let projection = flow.projection()?;
    let projected_class_view = projection
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| view.id == invoice_view)
        .ok_or("missing classifier View")?;
    assert!(
        projected_class_view
            .member_labels
            .get(&attribute)
            .is_some_and(|label| label.contains("[0..*]"))
    );
    let operation_label = projected_class_view
        .member_labels
        .get(&operation)
        .ok_or("missing operation label")?;
    assert!(operation_label.contains("[2..4]"));
    assert!(operation_label.ends_with(": Document [0..1]"));
    let original_style = semantic_view(&with_multiplicity, &invoice)?.style.clone();
    let mut hidden_style = original_style.clone();
    hidden_style.uml.show_multiplicity = Some(false);
    let hidden = flow.apply(Command::SetViewStyle {
        diagram_id: "classes".into(),
        view_id: invoice_view.clone(),
        style: hidden_style,
    })?;
    assert_eq!(hidden.registry, with_multiplicity.registry);
    let hidden_projection = flow.projection()?;
    let hidden_class_view = hidden_projection
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| view.id == invoice_view)
        .ok_or("missing styled classifier View")?;
    for member in [&attribute, &operation] {
        assert!(
            !hidden_class_view
                .member_labels
                .get(member)
                .ok_or("missing styled member label")?
                .contains('[')
        );
    }
    flow.apply(Command::SetViewStyle {
        diagram_id: "classes".into(),
        view_id: invoice_view.clone(),
        style: original_style,
    })?;
    reopen_both_formats(&flow.document, &flow.section()?)?;

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

    let exportable = flow.create(
        Command::CreateModelElement {
            owner_id: Some("model".into()),
            kind: ModelingElementKind::Interface,
            name: Some("Exportable".into()),
        },
        "Interface",
    )?;
    flow.apply(Command::SetProperty {
        element_id: realization.clone(),
        path: "properties.name".into(),
        value: json!("PrintableContract"),
    })?;
    for relationship in [&generalization, &realization] {
        let view = semantic_view(&flow.section()?, relationship)?.clone();
        let mut style = view.style.clone();
        style.stroke = Some("#4169e1".into());
        style.stroke_width = Some(2.5);
        flow.apply(Command::SetViewStyle {
            diagram_id: "classes".into(),
            view_id: view.id.clone(),
            style,
        })?;
        flow.apply(Command::SetViewLabel {
            view_id: view.id,
            label: Some("authored relationship label".into()),
        })?;
    }
    let before_retarget = flow.section()?;
    let generalization_view = semantic_view(&before_retarget, &generalization)?.clone();
    let realization_view = semantic_view(&before_retarget, &realization)?.clone();
    let printable_view = semantic_view(&before_retarget, &printable)?.id.clone();
    let association_original = semantic_view(&before_retarget, &association)?.clone();
    let Contract::Association(binary) = contract(&before_retarget, &association)? else {
        return Err("retargeting requires the concrete Association".into());
    };
    let [association_source_end, association_target_end] = binary.member_end.as_slice() else {
        return Err("retargeting requires two memberEnds".into());
    };
    let association_source_end = association_source_end.clone();
    let association_target_end = association_target_end.clone();
    for source in [&document, &invoice] {
        let changed = flow.apply(Command::SetProperty {
            element_id: association_source_end.clone(),
            path: "properties.type".into(),
            value: json!({"$ref": source}),
        })?;
        assert_retargeted_view(
            &flow,
            &association,
            &association_original,
            source,
            &document,
            Some(&semantic_view(&changed, source)?.id),
            Some(&semantic_view(&changed, &document)?.id),
        )?;
    }
    for (path, target, specific, general) in [
        ("general", &nested, &invoice, &nested),
        ("general", &document, &invoice, &document),
        ("specific", &nested, &nested, &document),
        ("specific", &invoice, &invoice, &document),
    ] {
        let changed = flow.apply(Command::SetProperty {
            element_id: generalization.clone(),
            path: format!("properties.{path}"),
            value: json!({"$ref": target}),
        })?;
        let Contract::Generalization(inheritance) = contract(&changed, &generalization)? else {
            return Err("retargeting must preserve the Generalization contract".into());
        };
        assert_eq!(
            (&inheritance.specific, &inheritance.general),
            (specific, general)
        );
        for owner in [&invoice, &document, &nested] {
            assert_eq!(
                class(&changed, owner)?.generalization,
                if owner == specific {
                    vec![generalization.clone()]
                } else {
                    Vec::new()
                }
            );
        }
        assert_retargeted_view(
            &flow,
            &generalization,
            &generalization_view,
            specific,
            general,
            Some(&semantic_view(&changed, specific)?.id),
            Some(&semantic_view(&changed, general)?.id),
        )?;
    }
    // A semantic target remains valid without a View, or with ambiguous occurrences.
    for occurrence_count in 0..=2 {
        if occurrence_count > 0 {
            flow.apply(Command::PlaceElementView {
                diagram_id: "classes".into(),
                element_id: exportable.clone(),
                x: 750.0,
                y: 150.0 * f64::from(occurrence_count),
            })?;
        }
        let exportable_view = if occurrence_count == 1 {
            Some(semantic_view(&flow.section()?, &exportable)?.id.clone())
        } else {
            None
        };
        for target in [&exportable, &printable] {
            let changed = flow.apply(Command::SetProperty {
                element_id: realization.clone(),
                path: "properties.contract".into(),
                value: json!({"$ref": target}),
            })?;
            assert_realization_endpoints(
                &changed,
                &realization,
                &invoice,
                target,
                [&invoice, &document, &nested],
            )?;
            let target_view = if target == &printable {
                Some(printable_view.as_str())
            } else {
                exportable_view.as_deref()
            };
            assert_retargeted_view(
                &flow,
                &realization,
                &realization_view,
                &invoice,
                target,
                Some(&invoice_view),
                target_view,
            )?;
            reopen_both_formats(&flow.document, &changed)?;
        }
        for target in [&exportable, &document] {
            let changed = flow.apply(Command::SetProperty {
                element_id: association_target_end.clone(),
                path: "properties.type".into(),
                value: json!({"$ref": target}),
            })?;
            let Contract::Association(binary) = contract(&changed, &association)? else {
                return Err("retargeting must preserve the Association contract".into());
            };
            assert_eq!(
                binary.member_end,
                [
                    association_source_end.clone(),
                    association_target_end.clone()
                ]
            );
            let target_view = if target == &document {
                Some(semantic_view(&changed, &document)?.id.as_str())
            } else {
                exportable_view.as_deref()
            };
            assert_retargeted_view(
                &flow,
                &association,
                &association_original,
                &invoice,
                target,
                Some(&invoice_view),
                target_view,
            )?;
            reopen_both_formats(&flow.document, &changed)?;
        }
    }
    for implementer in [&document, &invoice] {
        let changed = flow.apply(Command::SetProperty {
            element_id: realization.clone(),
            path: "properties.implementingClassifier".into(),
            value: json!({"$ref": implementer}),
        })?;
        assert_realization_endpoints(
            &changed,
            &realization,
            implementer,
            &printable,
            [&invoice, &document, &nested],
        )?;
        assert_retargeted_view(
            &flow,
            &realization,
            &realization_view,
            implementer,
            &printable,
            Some(&semantic_view(&changed, implementer)?.id),
            Some(&printable_view),
        )?;
    }
    for (relationship, path, wrong_type) in [
        (&generalization, "general", &attribute),
        (&generalization, "specific", &attribute),
        (&realization, "contract", &invoice),
        (&realization, "implementingClassifier", &printable),
    ] {
        let unchanged = flow.document.section_json.clone();
        let revision = flow.revision;
        assert!(
            flow.apply(Command::SetProperty {
                element_id: relationship.clone(),
                path: format!("properties.{path}"),
                value: json!({"$ref": wrong_type}),
            })
            .is_err(),
            "{path} must reject an endpoint of the wrong UML type"
        );
        assert_eq!(flow.document.section_json, unchanged);
        assert_eq!(flow.revision, revision);
    }

    for (relationship, edited_element, path, original_view) in [
        (
            &realization,
            &realization,
            "properties.contract",
            &realization_view,
        ),
        (
            &association,
            &association_target_end,
            "properties.type",
            &association_original,
        ),
    ] {
        let mut manual_view = original_view.clone();
        manual_view.style.routing = Some(RelationshipRouting::Manual);
        manual_view.geometry = flow
            .projection()?
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .find(|view| view.id == original_view.id)
            .and_then(|view| view.routed_geometry.clone())
            .ok_or("manual path authoring requires the rendered relationship geometry")?;
        let manual = flow.apply(Command::SetViewPath {
            diagram_id: "classes".into(),
            view_id: original_view.id.clone(),
            geometry: manual_view.geometry.clone(),
        })?;
        assert_eq!(semantic_view(&manual, relationship)?, &manual_view);
        let manual_section = flow.document.section_json.clone();
        let manual_revision = flow.revision;
        let Err(error) = flow.apply(Command::SetProperty {
            element_id: edited_element.clone(),
            path: path.into(),
            value: json!({"$ref": exportable}),
        }) else {
            return Err("retargeting a manually routed relationship must be rejected".into());
        };
        assert!(error.to_string().to_ascii_lowercase().contains("manual"));
        assert_eq!(flow.document.section_json, manual_section);
        assert_eq!(flow.revision, manual_revision);
        assert_eq!(semantic_view(&flow.section()?, relationship)?, &manual_view);
    }

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
    let explorer = flow.projection()?.explorer.added;
    let invoice_rows = explorer
        .iter()
        .filter(|row| row.element_id == invoice)
        .collect::<Vec<_>>();
    assert_eq!(invoice_rows.len(), 1);
    let nested_row = explorer
        .iter()
        .find(|row| row.element_id == nested)
        .ok_or("missing owned classifier in Explorer")?;
    assert_eq!(nested_row.depth, invoice_rows[0].depth + 1);
    assert_eq!(
        explorer
            .iter()
            .filter(|row| row.element_id == exportable)
            .count(),
        1,
        "one semantic ID has one Explorer row even with multiple authored views"
    );

    let mut multi_diagram = NativeAuthoring {
        document: flow.document.clone(),
        revision: flow.revision,
    };
    let with_diagram = multi_diagram.apply(Command::CreateDiagram {
        kind: bellatrix_mde_contracts::protocol::StructuralDiagramKind::Class,
        name: "Other diagram".into(),
        model_elements: vec!["model".into()],
    })?;
    let extra_diagram = with_diagram
        .diagrams
        .iter()
        .find(|diagram| diagram.id != "classes")
        .ok_or("missing second diagram")?
        .id
        .clone();
    multi_diagram.apply(Command::PlaceElementView {
        diagram_id: extra_diagram.clone(),
        element_id: invoice.clone(),
        x: 20.0,
        y: 20.0,
    })?;
    let same_name = multi_diagram.create(
        Command::CreateElement {
            diagram_id: extra_diagram,
            owner_id: None,
            kind: ModelingElementKind::Class,
            name: Some("SalesInvoice".into()),
            x: 350.0,
            y: 20.0,
        },
        "Class",
    )?;
    assert_ne!(same_name, invoice);
    let before_projection = multi_diagram.document.section_json.clone();
    let projection = multi_diagram.projection()?;
    for rows in [&projection.explorer.added, &projection.explorer.name] {
        let mut identities = BTreeSet::new();
        for row in rows.iter().filter(|row| {
            !row.diagram
                && row
                    .view
                    .as_ref()
                    .is_none_or(|view| view.subject_id.is_some())
        }) {
            assert!(
                identities.insert(row.element_id.clone()),
                "duplicated Explorer ID {}",
                row.element_id
            );
        }
        assert_eq!(
            rows.iter().filter(|row| row.element_id == invoice).count(),
            1
        );
        assert_eq!(
            rows.iter()
                .filter(|row| row.element_id == same_name)
                .count(),
            1
        );
    }
    for row in &projection.explorer.added {
        if !row.diagram
            && row
                .view
                .as_ref()
                .is_none_or(|view| view.subject_id.is_some())
        {
            assert_eq!(
                projection
                    .explorer
                    .name
                    .iter()
                    .find(|candidate| candidate.element_id == row.element_id)
                    .map(|candidate| &candidate.key),
                Some(&row.key)
            );
        }
    }
    assert_eq!(multi_diagram.document.section_json, before_projection);
    assert_eq!(
        projection
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .filter(|view| view.subject.as_deref() == Some(invoice.as_str()))
            .count(),
        2
    );
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
        [
            &invoice,
            &document,
            &printable,
            &exportable,
            &association,
            &dependency
        ]
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

    let mut with_layout_leaf = authored.clone();
    // These deletion scenarios deliberately exercise an independent diagram;
    // the native blank's contextual diagram was verified to reject root deletion above.
    with_layout_leaf
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == "classes")
        .ok_or("missing independent diagram for deletion scenarios")?
        .model_elements
        .clear();
    with_layout_leaf.hierarchy.remove("classes");

    let layout_view = with_layout_leaf
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == "classes")
        .ok_or("missing class diagram before loading a layout leaf")?
        .views
        .iter_mut()
        .find(|view| view.subject.as_deref() == Some(exportable.as_str()))
        .ok_or("missing top-level Exportable View")?;
    assert!(layout_view.children.is_empty());
    assert!(layout_view.draft.is_none());
    layout_view.geometry = Geometry::Layout {
        bounds: Default::default(),
    };
    assert_eq!(
        serde_json::to_value(&layout_view.geometry)?,
        json!({"kind": "layout", "bounds": {}})
    );
    let layout_leaf = layout_view.clone();
    let layout_bytes = encode_model_section(
        &flow.document.id,
        &flow.document.name,
        &serde_json::to_string(&with_layout_leaf)?,
        "json",
    )?;
    flow.document = prepare_model_document(&decode_model_document(&layout_bytes, "json")?)?;
    assert_eq!(flow.section()?, with_layout_leaf);
    reopen_both_formats(&flow.document, &with_layout_leaf)?;

    let mut with_style_dependent = with_layout_leaf.clone();
    let mut dependent_view = layout_leaf.clone();
    let layout_view_id = dependent_view.id.clone();
    dependent_view.id = "surviving-style-dependent".into();
    dependent_view.subject = None;
    dependent_view.presentation.style_inheritance = Some(StyleInheritance {
        source: Authored::Value(layout_view_id),
        properties: BTreeSet::from([InheritedStyleProperty::Fill]),
    });
    with_style_dependent
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == "classes")
        .ok_or("missing class diagram for the surviving style dependency")?
        .views
        .push(dependent_view);
    let dependent_bytes = encode_model_section(
        &flow.document.id,
        &flow.document.name,
        &serde_json::to_string(&with_style_dependent)?,
        "json",
    )?;
    let mut guarded_flow = NativeAuthoring {
        document: prepare_model_document(&decode_model_document(&dependent_bytes, "json")?)?,
        revision: flow.revision,
    };
    let rejection = guarded_flow
        .apply(Command::DeleteElement {
            element_id: "model".into(),
        })
        .err()
        .ok_or("deletion must not strand a surviving View's inherited style")?;
    assert!(rejection.to_string().contains("surviving-style-dependent"));
    assert_eq!(guarded_flow.revision, flow.revision);
    assert_eq!(guarded_flow.section()?, with_style_dependent);
    reopen_both_formats(&guarded_flow.document, &with_style_dependent)?;

    let mut with_nested_leaves = with_layout_leaf.clone();
    let nested_diagram = with_nested_leaves
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == "classes")
        .ok_or("missing class diagram for nested leaf deletion")?;
    let mut rectangular_occurrences = nested_diagram.views.iter().filter(|view| {
        view.subject.as_deref() == Some(exportable.as_str()) && view.id != layout_leaf.id
    });
    let rectangular_leaf = rectangular_occurrences
        .next()
        .ok_or("missing second Exportable occurrence")?
        .clone();
    assert!(rectangular_occurrences.next().is_none());
    assert!(matches!(rectangular_leaf.geometry, Geometry::Rect { .. }));
    assert!(rectangular_leaf.children.is_empty());
    assert!(rectangular_leaf.draft.is_none());
    let rectangular_leaf_id = rectangular_leaf.id.clone();
    nested_diagram
        .views
        .retain(|view| view.id != layout_leaf.id && view.id != rectangular_leaf.id);
    let before_sibling: View = serde_json::from_value(json!({
        "id": "surviving-sibling-before", "subject": null,
        "geometry": {"kind": "rect", "x": 15.0, "y": 25.0, "width": 80.0, "height": 45.0},
        "style": {"fill": "#dceeff", "stroke": "#112233", "stroke_width": 1.5}
    }))?;
    let after_sibling: View = serde_json::from_value(json!({
        "id": "surviving-sibling-after", "subject": null,
        "geometry": {"kind": "rect", "x": 350.0, "y": 210.0, "width": 110.0, "height": 75.0},
        "style": {"fill": "#ffeedd", "stroke": "#654321", "stroke_width": 3.0}
    }))?;
    let mut visual_container: View = serde_json::from_value(json!({
        "id": "surviving-visual-container", "subject": null,
        "geometry": {"kind": "rect", "x": 10.0, "y": 20.0, "width": 900.0, "height": 600.0},
        "style": {"fill": "#f8f8f8", "stroke": "#334455", "stroke_width": 2.0}
    }))?;
    visual_container.children = vec![
        before_sibling.clone(),
        layout_leaf,
        rectangular_leaf,
        after_sibling.clone(),
    ];
    let mut expected_container = visual_container.clone();
    expected_container.children = vec![before_sibling, after_sibling];
    nested_diagram.views.push(visual_container);
    let mut expected_diagram = nested_diagram.clone();
    expected_diagram.views = vec![expected_container];
    let nested_bytes = encode_model_section(
        &flow.document.id,
        &flow.document.name,
        &serde_json::to_string(&with_nested_leaves)?,
        "json",
    )?;
    let mut nested_flow = NativeAuthoring {
        document: prepare_model_document(&decode_model_document(&nested_bytes, "json")?)?,
        revision: flow.revision,
    };
    assert_eq!(nested_flow.section()?, with_nested_leaves);
    reopen_both_formats(&nested_flow.document, &with_nested_leaves)?;

    let mut occurrence_flow = NativeAuthoring {
        document: prepare_model_document(&decode_model_document(&nested_bytes, "json")?)?,
        revision: flow.revision,
    };
    let mut expected_occurrence_removed = with_nested_leaves.clone();
    expected_occurrence_removed
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == "classes")
        .ok_or("missing class diagram for nested occurrence removal")?
        .views
        .iter_mut()
        .find(|view| view.id == "surviving-visual-container")
        .ok_or("missing visual container for nested occurrence removal")?
        .children
        .retain(|view| view.id != rectangular_leaf_id);
    let occurrence_removed = occurrence_flow.apply(Command::RemoveView {
        diagram_id: "classes".into(),
        view_id: rectangular_leaf_id,
    })?;
    assert_eq!(occurrence_removed, expected_occurrence_removed);
    reopen_both_formats(&occurrence_flow.document, &expected_occurrence_removed)?;

    assert!(with_nested_leaves.diagrams.iter().all(|diagram| {
        diagram
            .all_views()
            .all(|view| view.subject.as_deref() != Some(attribute.as_str()))
    }));
    let mut with_property_reference = with_nested_leaves.clone();
    with_property_reference
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == "classes")
        .ok_or("missing class diagram for the surviving model reference")?
        .views
        .iter_mut()
        .find(|view| view.id == "surviving-visual-container")
        .ok_or("missing surviving visual container")?
        .presentation
        .references
        .insert(
            "owned-property-reference".into(),
            Authored::Value(PresentationReference::Model(attribute.clone())),
        );
    let reference_bytes = encode_model_section(
        &flow.document.id,
        &flow.document.name,
        &serde_json::to_string(&with_property_reference)?,
        "json",
    )?;
    let mut reference_flow = NativeAuthoring {
        document: prepare_model_document(&decode_model_document(&reference_bytes, "json")?)?,
        revision: flow.revision,
    };
    let rejection = reference_flow
        .apply(Command::DeleteElement {
            element_id: "model".into(),
        })
        .err()
        .ok_or(
            "deletion must not strand a surviving container's reference to an unshown Property",
        )?;
    assert!(rejection.to_string().contains("surviving-visual-container"));
    assert!(rejection.to_string().contains(&attribute));
    assert_eq!(reference_flow.revision, flow.revision);
    assert_eq!(reference_flow.section()?, with_property_reference);
    reopen_both_formats(&reference_flow.document, &with_property_reference)?;

    let nested_deleted = nested_flow.apply(Command::DeleteElement {
        element_id: "model".into(),
    })?;
    assert!(nested_deleted.root_model.is_none());
    assert!(nested_deleted.registry.entries().is_empty());
    assert_eq!(nested_deleted.diagrams, vec![expected_diagram]);
    reopen_both_formats(&nested_flow.document, &nested_deleted)?;

    let annotation_bytes = encode_model_section(
        &flow.document.id,
        &flow.document.name,
        &serde_json::to_string(&authored)?,
        "json",
    )?;
    let mut annotation_flow = NativeAuthoring {
        document: prepare_model_document(&decode_model_document(&annotation_bytes, "json")?)?,
        revision: flow.revision,
    };
    let before_occurrence = annotation_flow.section()?;
    let duplicate = annotation_flow.apply(Command::PlaceElementView {
        diagram_id: "classes".into(),
        element_id: invoice.clone(),
        x: 1200.0,
        y: 850.0,
    })?;
    let selected_view = duplicate
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| {
            view.subject.as_deref() == Some(invoice.as_str())
                && !before_occurrence
                    .diagrams
                    .iter()
                    .flat_map(|diagram| diagram.all_views())
                    .any(|before| before.id == view.id)
        })
        .ok_or("missing second Invoice occurrence")?
        .clone();
    let occurrence_revision = annotation_flow.revision;
    for view_id in [None, Some("missing-view".into())] {
        assert!(
            annotation_flow
                .apply(Command::QuickEdit {
                    diagram_id: "classes".into(),
                    element_id: invoice.clone(),
                    view_id,
                    action: UmlQuickCreate::Note,
                })
                .is_err()
        );
        assert_eq!(annotation_flow.revision, occurrence_revision);
        assert_eq!(annotation_flow.section()?, duplicate);
    }
    let comment = annotation_flow.create(
        Command::QuickEdit {
            view_id: Some(selected_view.id.clone()),
            diagram_id: "classes".into(),
            element_id: invoice.clone(),
            action: UmlQuickCreate::Note,
        },
        "Comment",
    )?;
    let annotated = annotation_flow.section()?;
    let Contract::Comment(annotation) = contract(&annotated, &comment)? else {
        return Err("annotation authoring must retain a concrete Comment".into());
    };
    assert_eq!(annotation.annotated_element, vec![invoice.clone()]);
    let annotation_projection = annotation_flow.projection()?;
    let mut note_links = annotation_projection.relations.iter().filter(|relation| {
        relation.kind == "uml.note_link" && relation.from == invoice && relation.to == comment
    });
    let note_link = note_links
        .next()
        .ok_or("missing projected Comment link")?
        .id
        .clone();
    assert!(note_links.next().is_none());
    let mut link_views = annotated
        .diagrams
        .iter()
        .find(|diagram| diagram.id == "classes")
        .ok_or("missing class diagram after Quick Edit")?
        .views
        .iter()
        .filter(|view| view.subject.as_deref() == Some(note_link.as_str()));
    let link_view = link_views
        .next()
        .ok_or("Quick Edit must create a top-level Comment link View")?;
    assert!(link_views.next().is_none());
    assert!(link_view.children.is_empty());
    assert!(matches!(&link_view.geometry, Geometry::Path { .. }));
    let comment_view = semantic_view(&annotated, &comment)?;
    assert_eq!(
        link_view.endpoints,
        Some(ViewEndpoints {
            source_view_id: Some(selected_view.id.clone()),
            target_view_id: Some(comment_view.id.clone()),
        })
    );
    let Geometry::Rect { x, y, width, .. } = selected_view.geometry else {
        return Err("selected Quick Edit occurrence must have bounds".into());
    };
    assert!(
        matches!(comment_view.geometry, Geometry::Rect { x: comment_x, y: comment_y, .. }
        if comment_x == x + width + 48.0 && comment_y >= y - 16.0)
    );
    for before in duplicate
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
    {
        let after = annotated
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .find(|view| view.id == before.id)
            .ok_or("Quick Edit removed an existing View")?;
        assert_eq!(after, before);
    }
    reopen_both_formats(&annotation_flow.document, &annotated)?;
    let unshown = annotation_flow.apply(Command::RemoveView {
        diagram_id: "classes".into(),
        view_id: link_view.id.clone(),
    })?;
    let mut expected_unshown = annotated.clone();
    expected_unshown
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == "classes")
        .ok_or("missing expected class diagram")?
        .views
        .retain(|view| view.id != link_view.id);
    assert_eq!(unshown, expected_unshown);
    assert!(unshown.diagrams.iter().all(|diagram| {
        diagram
            .all_views()
            .all(|view| view.subject.as_deref() != Some(note_link.as_str()))
    }));
    let hierarchy_bound = annotation_flow.apply(Command::SetExplorerParent {
        subject_id: "classes".into(),
        parent_id: Some(note_link.clone()),
        parent_occurrence: None,
    })?;
    let placement = hierarchy_bound
        .hierarchy
        .get("classes")
        .ok_or("missing diagram placement under the Comment link")?;
    assert_eq!(placement.parent_id.as_deref(), Some(note_link.as_str()));
    assert!(placement.occurrence.is_none());
    reopen_both_formats(&annotation_flow.document, &hierarchy_bound)?;
    let annotation_revision = annotation_flow.revision;
    let rejection = annotation_flow
        .apply(Command::DeleteElement {
            element_id: note_link,
        })
        .err()
        .ok_or("deleting an unshown Comment link must not strand its child diagram")?;
    assert!(
        rejection
            .to_string()
            .contains("surviving hierarchy placement")
    );
    assert!(rejection.to_string().contains("classes"));
    assert_eq!(annotation_flow.revision, annotation_revision);
    assert_eq!(annotation_flow.section()?, hierarchy_bound);
    reopen_both_formats(&annotation_flow.document, &hierarchy_bound)?;

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
