use bellatrix_mde::{
    apply_model_command,
    contracts::{
        Authored, Geometry, InheritedStyleProperty, ModelingSection, PresentationReference,
        RelationshipRouting, StyleInheritance, View, ViewEndpoints,
        protocol::{
            Command, ModelDocumentParts, ModelingElementKind, ModelingRelationKind,
            StructuralDiagramKind, UmlQuickCreate,
            inspector::{PropertyEditor, QuickEditAvailability, QuickEditControl, QuickEditIntent},
            projection::ModelingProjection,
        },
        ruml::{Contract, aggregation_kind::AggregationKind, class::Class},
    },
    decode_model_document, encode_model_section, prepare_model_document, project_model_section,
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, error::Error};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
const DIAGRAM: &str = "classes";
const FEATURE: &str = include_str!("features/native_class_authoring.feature");

#[derive(Clone)]
struct AuthoringSession {
    document: ModelDocumentParts,
    revision: u64,
}

impl AuthoringSession {
    fn blank() -> Result<Self> {
        let envelope = decode_model_document(include_bytes!("../fixtures/blank.json"), "json")?;
        let session = Self {
            document: prepare_model_document(&envelope)?,
            revision: 0,
        };
        let section = session.model()?;
        assert_eq!(section.root_model.as_deref(), Some("model"));
        assert_eq!(section.registry.entries().len(), 1);
        let [diagram] = section.diagrams.as_slice() else {
            return Err("expected one blank diagram".into());
        };
        assert_eq!(diagram.id, DIAGRAM);
        assert_eq!(diagram.uml_di_type, "UMLClassDiagram");
        assert_eq!(diagram.model_elements, ["model"]);
        assert!(diagram.views.is_empty());
        assert_eq!(
            section
                .hierarchy
                .get(DIAGRAM)
                .ok_or("missing context placement")?
                .parent_id
                .as_deref(),
            Some("model")
        );
        Ok(session)
    }

    fn model(&self) -> Result<ModelingSection> {
        Ok(serde_json::from_str(&self.document.section_json)?)
    }

    fn projection(&self) -> Result<ModelingProjection> {
        Ok(project_model_section(
            &self.document.id,
            &self.document.name,
            &self.document.section_json,
        )?)
    }

    fn execute(&mut self, command: Command) -> Result<ModelingSection> {
        let next = self.revision.checked_add(1).ok_or("revision overflow")?;
        let model = apply_model_command(
            &self.document.id,
            &self.document.name,
            &self.document.section_json,
            next,
            command,
        )?;
        let model: ModelingSection = serde_json::from_value(model)?;
        let section_json = serde_json::to_string(&model)?;
        self.document.section_json = section_json;
        self.revision = next;
        Ok(model)
    }

    fn reject(&mut self, command: Command, diagnostic: &str) -> Result {
        let before = self.document.section_json.clone();
        let revision = self.revision;
        let error = self
            .execute(command)
            .err()
            .ok_or("command unexpectedly succeeded")?;
        assert!(error.to_string().contains(diagnostic), "{error}");
        assert_eq!(self.document.section_json, before);
        assert_eq!(self.revision, revision);
        Ok(())
    }

    fn create(&mut self, command: Command, accepts: fn(&Contract) -> bool) -> Result<String> {
        let before = self.model()?;
        let after = self.execute(command)?;
        let mut created = after
            .registry
            .entries()
            .values()
            .filter(|entry| !before.registry.exists(&entry.id) && accepts(&entry.contract));
        let identity = created
            .next()
            .ok_or("command created no matching contract")?
            .id
            .clone();
        assert!(
            created.next().is_none(),
            "creation must identify exactly one contract"
        );
        Ok(identity)
    }

    fn classifier(
        &mut self,
        name: &str,
        interface: bool,
        owner: Option<&str>,
        x: f64,
    ) -> Result<String> {
        self.create(
            Command::CreateElement {
                diagram_id: DIAGRAM.into(),
                owner_id: owner.map(str::to_owned),
                kind: if interface {
                    ModelingElementKind::Interface
                } else {
                    ModelingElementKind::Class
                },
                name: Some(name.into()),
                x,
                y: 100.0,
            },
            |contract| matches!(contract, Contract::Class(_) | Contract::Interface(_)),
        )
    }

    fn action(&self, subject: &str, identity: &str) -> Result<QuickEditIntent> {
        let projection = self.projection()?;
        let element = projection
            .elements
            .iter()
            .find(|element| element.id == subject)
            .ok_or("missing action subject")?;
        let action = element
            .quick_actions
            .iter()
            .find(|action| action.id == identity)
            .ok_or("missing quick action")?;
        assert_eq!(action.availability, QuickEditAvailability::Implemented);
        assert_eq!(action.control, QuickEditControl::Button);
        assert!(
            projection
                .inspect_element(subject)
                .ok_or("missing Inspector")?
                .quick_actions
                .contains(action)
        );
        QuickEditIntent::from_id(action.id).ok_or_else(|| "unknown quick intent".into())
    }

    fn member(&mut self, owner: &str, action: &str) -> Result<String> {
        let QuickEditIntent::AddMember(kind) = self.action(owner, action)? else {
            return Err("expected member action".into());
        };
        self.create(
            Command::AddMember {
                owner_id: owner.into(),
                kind,
            },
            |contract| {
                matches!(
                    contract,
                    Contract::Property(_) | Contract::Operation(_) | Contract::Parameter(_)
                )
            },
        )
    }

    fn property(&mut self, subject: &str, path: &str, value: Value) -> Result<ModelingSection> {
        self.execute(Command::SetProperty {
            element_id: subject.into(),
            path: path.into(),
            value,
        })
    }

    fn place(&mut self, subject: &str, diagram: &str, x: f64) -> Result<View> {
        let before = self.model()?;
        let previous: BTreeSet<_> = before
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .map(|view| &view.id)
            .collect();
        let after = self.execute(Command::PlaceElementView {
            diagram_id: diagram.into(),
            element_id: subject.into(),
            x,
            y: 400.0,
        })?;
        Ok(after
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .find(|view| view.subject.as_deref() == Some(subject) && !previous.contains(&view.id))
            .ok_or("missing new occurrence")?
            .clone())
    }

    fn load(&self, section: &ModelingSection) -> Result<Self> {
        let bytes = encode_model_section(
            &self.document.id,
            &self.document.name,
            &serde_json::to_string(section)?,
            "json",
        )?;
        let loaded = Self {
            document: prepare_model_document(&decode_model_document(&bytes, "json")?)?,
            revision: self.revision,
        };
        assert_eq!(loaded.model()?, *section);
        loaded.round_trip()?;
        Ok(loaded)
    }

    fn round_trip(&self) -> Result {
        for format in ["json", "protobuf"] {
            let encoded = encode_model_section(
                &self.document.id,
                &self.document.name,
                &self.document.section_json,
                format,
            )?;
            let reopened = prepare_model_document(&decode_model_document(&encoded, format)?)?;
            assert_eq!(
                (&reopened.id, &reopened.name),
                (&self.document.id, &self.document.name)
            );
            assert_eq!(
                serde_json::from_str::<ModelingSection>(&reopened.section_json)?,
                self.model()?,
                "{format}"
            );
        }
        Ok(())
    }
}

fn contract<'a>(model: &'a ModelingSection, identity: &str) -> Result<&'a Contract> {
    Ok(&model
        .registry
        .get_entry(identity)
        .ok_or_else(|| format!("missing contract {identity}"))?
        .contract)
}

fn class<'a>(model: &'a ModelingSection, identity: &str) -> Result<&'a Class> {
    match contract(model, identity)? {
        Contract::Class(class) => Ok(class),
        _ => Err(format!("{identity} must be a Class").into()),
    }
}

fn occurrence<'a>(model: &'a ModelingSection, subject: &str) -> Result<&'a View> {
    let mut occurrences = model
        .diagrams
        .iter()
        .find(|diagram| diagram.id == DIAGRAM)
        .ok_or("missing class diagram")?
        .all_views()
        .filter(|view| view.subject.as_deref() == Some(subject));
    let view = occurrences.next().ok_or("missing semantic occurrence")?;
    assert!(
        occurrences.next().is_none(),
        "expected unique occurrence of {subject}"
    );
    assert!(view.draft.is_none());
    Ok(view)
}

struct ClassModel {
    session: AuthoringSession,
    invoice: String,
    document: String,
    printable: String,
    nested: String,
    attribute: String,
    operation: String,
    parameters: [String; 2],
    association: String,
    generalization: String,
    dependency: String,
    realization: String,
}

impl ClassModel {
    fn author() -> Result<Self> {
        let mut session = AuthoringSession::blank()?;
        let invoice = session.classifier("Invoice", false, None, 100.0)?;
        session.reject(
            Command::DeleteElement {
                element_id: "model".into(),
            },
            "UMLDI_CONTEXT_IN_USE",
        )?;
        let document = session.classifier("Document", false, None, 450.0)?;
        let printable = session.classifier("Printable", true, None, 800.0)?;
        let nested = session.classifier("LineItem", false, Some(&invoice), 100.0)?;
        let attribute = session.member(&invoice, "uml.add-attribute")?;
        let operation = session.member(&invoice, "uml.add-operation")?;
        let parameters = [
            session.member(&operation, "uml.add-parameter")?,
            session.member(&operation, "uml.add-parameter")?,
        ];
        let mut relationship = |kind, owner: &str, from: &str, to: &str| {
            session.create(
                Command::CreateRelation {
                    diagram_id: DIAGRAM.into(),
                    owner_id: Some(owner.into()),
                    kind,
                    from: from.into(),
                    to: to.into(),
                },
                |contract| {
                    matches!(
                        contract,
                        Contract::Association(_)
                            | Contract::Generalization(_)
                            | Contract::Dependency(_)
                            | Contract::InterfaceRealization(_)
                    )
                },
            )
        };
        let association = relationship(
            ModelingRelationKind::Association,
            "model",
            &invoice,
            &document,
        )?;
        let generalization = relationship(
            ModelingRelationKind::Generalization,
            &invoice,
            &invoice,
            &document,
        )?;
        let dependency = relationship(
            ModelingRelationKind::Dependency,
            "model",
            &document,
            &printable,
        )?;
        let realization = relationship(
            ModelingRelationKind::InterfaceRealization,
            &invoice,
            &invoice,
            &printable,
        )?;
        Ok(Self {
            session,
            invoice,
            document,
            printable,
            nested,
            attribute,
            operation,
            parameters,
            association,
            generalization,
            dependency,
            realization,
        })
    }

    fn ends(&self) -> Result<[String; 2]> {
        let model = self.session.model()?;
        let Contract::Association(association) = contract(&model, &self.association)? else {
            return Err("expected Association".into());
        };
        let [source, target] = association.member_end.as_slice() else {
            return Err("expected binary association".into());
        };
        assert_eq!(association.owned_end, association.member_end);
        Ok([source.clone(), target.clone()])
    }
}

fn author_classifiers_and_members() -> Result {
    let mut world = ClassModel::author()?;
    let invoice_view = occurrence(&world.session.model()?, &world.invoice)?
        .id
        .clone();
    for name in ["CustomerInvoice", "SalesInvoice"] {
        for command in [
            Command::Rename {
                element_id: world.invoice.clone(),
                name: name.into(),
            },
            Command::SetProperty {
                element_id: world.invoice.clone(),
                path: "name".into(),
                value: json!(name),
            },
            Command::SetNameExpression {
                element_id: world.invoice.clone(),
                expression: format!("+ {name}"),
            },
        ] {
            let model = world.session.execute(command)?;
            assert_eq!(class(&model, &world.invoice)?.name.as_deref(), Some(name));
            assert_eq!(occurrence(&model, &world.invoice)?.id, invoice_view);
            let projection = world.session.projection()?;
            assert_eq!(
                projection.contract(&world.invoice),
                Some(contract(&model, &world.invoice)?)
            );
            assert_eq!(
                projection
                    .elements
                    .iter()
                    .find(|element| element.id == world.invoice)
                    .ok_or("missing displayed Class")?
                    .name,
                name
            );
            let inspector = projection
                .inspect_element(&world.invoice)
                .ok_or("missing Class Inspector")?;
            let names: Vec<_> = inspector
                .properties
                .iter()
                .filter(|property| property.path == "name" || property.path == "properties.name")
                .collect();
            let [property] = names.as_slice() else {
                return Err("expected one Class name editor".into());
            };
            assert_eq!(property.path, "name");
            assert_eq!(property.value, json!(name));
            assert!(property.editable);
        }
    }
    for (subject, multiplicity) in [
        (&world.attribute, "0..*"),
        (&world.parameters[0], "2..4"),
        (&world.parameters[1], "0..1"),
    ] {
        world
            .session
            .property(subject, "properties.type", json!({"$ref": world.document}))?;
        world
            .session
            .property(subject, "properties.multiplicity", json!(multiplicity))?;
    }
    world.session.property(
        &world.parameters[1],
        "properties.direction",
        json!("return"),
    )?;
    let reordered = world.session.execute(Command::MoveParameter {
        parameter_id: world.parameters[1].clone(),
        up: true,
    })?;
    let Contract::Operation(operation) = contract(&reordered, &world.operation)? else {
        return Err("expected Operation".into());
    };
    assert_eq!(
        operation.owned_parameter,
        [world.parameters[1].clone(), world.parameters[0].clone()]
    );
    world.session.execute(Command::MoveParameter {
        parameter_id: world.parameters[1].clone(),
        up: false,
    })?;
    let model = world.session.model()?;
    let owner = class(&model, &world.invoice)?;
    assert_eq!(owner.owned_attribute, [world.attribute.clone()]);
    assert_eq!(owner.owned_operation, [world.operation.clone()]);
    assert_eq!(owner.nested_classifier, [world.nested.clone()]);
    assert_eq!(owner.generalization, [world.generalization.clone()]);
    assert_eq!(owner.interface_realization, [world.realization.clone()]);
    assert_eq!(
        class(&model, &world.nested)?.name.as_deref(),
        Some("LineItem")
    );
    let Contract::Operation(operation) = contract(&model, &world.operation)? else {
        return Err("expected Operation".into());
    };
    assert_eq!(operation.class.as_deref(), Some(world.invoice.as_str()));
    assert_eq!(operation.owned_parameter, world.parameters);
    let Contract::Model(root) = contract(&model, "model")? else {
        return Err("expected Model".into());
    };
    assert_eq!(
        root.packaged_element.iter().collect::<BTreeSet<_>>(),
        [
            &world.invoice,
            &world.document,
            &world.printable,
            &world.association,
            &world.dependency
        ]
        .into_iter()
        .collect()
    );
    let Contract::Dependency(dependency) = contract(&model, &world.dependency)? else {
        return Err("expected Dependency".into());
    };
    assert_eq!(dependency.client, [world.document.clone()]);
    assert_eq!(dependency.supplier, [world.printable.clone()]);
    for (end, classifier) in world.ends()?.iter().zip([&world.invoice, &world.document]) {
        let Contract::Property(property) = contract(&model, end)? else {
            return Err("expected Property end".into());
        };
        assert_eq!(property.type_.as_deref(), Some(classifier.as_str()));
        assert_eq!(
            property.association.as_deref(),
            Some(world.association.as_str())
        );
        assert_eq!(
            property.owning_association.as_deref(),
            Some(world.association.as_str())
        );
    }
    let projection = world.session.projection()?;
    assert!(
        projection
            .contracts
            .owns_attribute(&world.invoice, &world.attribute)
    );
    for subject in [&world.operation, &world.nested, &world.association] {
        assert!(!projection.contracts.owns_attribute(&world.invoice, subject));
    }
    assert!(
        !projection
            .contracts
            .owns_attribute(&world.document, &world.attribute)
    );
    let inspector = projection
        .inspect_element(&world.invoice)
        .ok_or("missing Class Inspector")?;
    let unavailable = inspector
        .properties
        .iter()
        .find(|property| property.path == "properties.isFinalSpecialization")
        .ok_or("missing unsupported field")?;
    assert_eq!(unavailable.editor, PropertyEditor::ReadOnly);
    assert!(!unavailable.editable);
    assert_eq!(unavailable.value, json!("Unavailable"));
    let generalization = projection
        .inspect_element(&world.generalization)
        .ok_or("missing Generalization Inspector")?;
    assert!(generalization.properties.iter().all(|property| {
        !["properties.name", "properties.sourceMultiplicity"].contains(&property.path.as_str())
    }));
    let original = occurrence(&model, &world.invoice)?.style.clone();
    for visible in [true, false, true] {
        let mut style = original.clone();
        style.uml.show_multiplicity = Some(visible);
        let styled = world.session.execute(Command::SetViewStyle {
            diagram_id: DIAGRAM.into(),
            view_id: invoice_view.clone(),
            style,
        })?;
        assert_eq!(styled.registry, model.registry);
        let projection = world.session.projection()?;
        let view = projection
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .find(|view| view.id == invoice_view)
            .ok_or("missing projected occurrence")?;
        let attribute = view
            .member_labels
            .get(&world.attribute)
            .ok_or("missing attribute label")?;
        let operation = view
            .member_labels
            .get(&world.operation)
            .ok_or("missing operation label")?;
        if visible {
            assert!(attribute.contains("[0..*]"));
            assert!(operation.contains("[2..4]"));
            assert!(operation.ends_with(": Document [0..1]"));
        } else {
            assert!(!attribute.contains('['));
            assert!(!operation.contains('['));
        }
    }
    world.session.round_trip()
}

fn quick_associations() -> Result {
    for (action, aggregation, label) in [
        ("uml.add-associated-class", AggregationKind::None, "none"),
        (
            "uml.add-aggregated-class",
            AggregationKind::Shared,
            "shared",
        ),
        (
            "uml.add-composited-class",
            AggregationKind::Composite,
            "composite",
        ),
    ] {
        let mut session = AuthoringSession::blank()?;
        let owner = session.classifier("Invoice", false, None, 100.0)?;
        let view = occurrence(&session.model()?, &owner)?.id.clone();
        let QuickEditIntent::QuickCreate(action) = session.action(&owner, action)? else {
            return Err("expected quick creation".into());
        };
        let relationship = session.create(
            Command::QuickEdit {
                diagram_id: DIAGRAM.into(),
                element_id: owner.clone(),
                view_id: Some(view),
                action,
            },
            |contract| matches!(contract, Contract::Association(_)),
        )?;
        let model = session.model()?;
        let Contract::Association(association) = contract(&model, &relationship)? else {
            return Err("expected Association".into());
        };
        let [source, target] = association.member_end.as_slice() else {
            return Err("expected binary ends".into());
        };
        assert_eq!(association.owned_end, association.member_end);
        let (Contract::Property(source), Contract::Property(target)) =
            (contract(&model, source)?, contract(&model, target)?)
        else {
            return Err("expected Property ends".into());
        };
        assert_eq!(source.type_.as_deref(), Some(owner.as_str()));
        assert_eq!(source.aggregation, AggregationKind::None);
        assert_eq!(target.aggregation, aggregation);
        class(
            &model,
            target.type_.as_deref().ok_or("missing target type")?,
        )?;
        let inspector = session
            .projection()?
            .inspect_element(&target.id)
            .ok_or("missing target Inspector")?;
        assert_eq!(
            inspector
                .properties
                .iter()
                .find(|property| property.path == "properties.aggregation")
                .ok_or("missing aggregation editor")?
                .value,
            json!(label)
        );
        session.round_trip()?;
    }
    Ok(())
}

fn endpoint_presentation(
    session: &AuthoringSession,
    relationship: &str,
    original: &View,
    source: &str,
    target: &str,
) -> Result {
    let model = session.model()?;
    let resolve = |subject: &str| {
        let views: Vec<_> = model
            .diagrams
            .iter()
            .filter(|diagram| diagram.id == DIAGRAM)
            .flat_map(|diagram| diagram.all_views())
            .filter(|view| view.subject.as_deref() == Some(subject))
            .collect();
        match views.as_slice() {
            [view] => Some(view.id.clone()),
            _ => None,
        }
    };
    let mut expected = original.clone();
    expected.endpoints = Some(ViewEndpoints {
        source_view_id: resolve(source),
        target_view_id: resolve(target),
    });
    assert_eq!(occurrence(&model, relationship)?, &expected);
    let projection = session.projection()?;
    assert_eq!(
        projection.contract(relationship),
        Some(contract(&model, relationship)?)
    );
    let relation = projection
        .relations
        .iter()
        .find(|relation| relation.id == relationship)
        .ok_or("missing projected relationship")?;
    assert_eq!(
        (relation.from.as_str(), relation.to.as_str()),
        (source, target)
    );
    let view = projection
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
        .find(|view| view.id == original.id)
        .ok_or("missing projected route")?;
    assert_eq!(view.endpoints, expected.endpoints);
    let attached = resolve(source).is_some() && resolve(target).is_some();
    assert_eq!(view.routed_geometry.is_some(), attached);
    let resolved = view
        .relationship
        .as_ref()
        .ok_or("missing resolved relationship")?;
    assert_eq!(
        resolved
            .diagnostics
            .iter()
            .any(|message| message.contains("unattached")),
        !attached
    );
    Ok(())
}

fn realization_contract(world: &ClassModel, source: &str, target: &str) -> Result {
    let model = world.session.model()?;
    let Contract::InterfaceRealization(realization) = contract(&model, &world.realization)? else {
        return Err("expected InterfaceRealization".into());
    };
    assert_eq!(realization.name.as_deref(), Some("PrintableContract"));
    assert_eq!(realization.implementing_classifier, source);
    assert_eq!(realization.contract, target);
    assert_eq!(realization.client, [source]);
    assert_eq!(realization.supplier, [target]);
    for owner in [&world.invoice, &world.document, &world.nested] {
        let expected = if owner == source {
            vec![world.realization.clone()]
        } else {
            Vec::new()
        };
        assert_eq!(class(&model, owner)?.interface_realization, expected);
    }
    Ok(())
}

fn retarget_relationships() -> Result {
    let mut world = ClassModel::author()?;
    let exportable = world.session.create(
        Command::CreateModelElement {
            owner_id: Some("model".into()),
            kind: ModelingElementKind::Interface,
            name: Some("Exportable".into()),
        },
        |contract| matches!(contract, Contract::Interface(_)),
    )?;
    world.session.property(
        &world.realization,
        "properties.name",
        json!("PrintableContract"),
    )?;
    for relationship in [
        &world.association,
        &world.generalization,
        &world.realization,
    ] {
        let view = occurrence(&world.session.model()?, relationship)?.clone();
        let mut style = view.style.clone();
        style.stroke = Some("#4169e1".into());
        style.stroke_width = Some(2.5);
        world.session.execute(Command::SetViewStyle {
            diagram_id: DIAGRAM.into(),
            view_id: view.id.clone(),
            style,
        })?;
        world.session.execute(Command::SetViewLabel {
            view_id: view.id,
            label: Some("authored relationship label".into()),
        })?;
    }
    let before = world.session.model()?;
    let association_view = occurrence(&before, &world.association)?.clone();
    let generalization_view = occurrence(&before, &world.generalization)?.clone();
    let realization_view = occurrence(&before, &world.realization)?.clone();
    let ends = world.ends()?;
    for name in ["documentReference", "sourceDocument"] {
        let model = world
            .session
            .property(&world.association, "properties.name", json!(name))?;
        let Contract::Association(association) = contract(&model, &world.association)? else {
            return Err("expected Association".into());
        };
        assert_eq!(association.name.as_deref(), Some(name));
        assert_eq!(
            occurrence(&model, &world.association)?.id,
            association_view.id
        );
        let projection = world.session.projection()?;
        assert_eq!(
            projection.contract(&world.association),
            Some(contract(&model, &world.association)?)
        );
        let inspector = projection
            .inspect_element(&world.association)
            .ok_or("missing Association Inspector")?;
        let name_field = inspector
            .properties
            .iter()
            .find(|property| property.path == "properties.name")
            .ok_or("missing Association name")?;
        assert_eq!(name_field.value, json!(name));
        assert!(name_field.editable);
    }
    for source in [&world.document, &world.invoice] {
        world
            .session
            .property(&ends[0], "properties.type", json!({"$ref": source}))?;
        endpoint_presentation(
            &world.session,
            &world.association,
            &association_view,
            source,
            &world.document,
        )?;
    }
    for (path, value, specific, general) in [
        ("general", &world.nested, &world.invoice, &world.nested),
        ("general", &world.document, &world.invoice, &world.document),
        ("specific", &world.nested, &world.nested, &world.document),
        ("specific", &world.invoice, &world.invoice, &world.document),
    ] {
        let model = world.session.property(
            &world.generalization,
            &format!("properties.{path}"),
            json!({"$ref": value}),
        )?;
        let Contract::Generalization(generalization) = contract(&model, &world.generalization)?
        else {
            return Err("expected Generalization".into());
        };
        assert_eq!(
            (&generalization.specific, &generalization.general),
            (specific, general)
        );
        for owner in [&world.invoice, &world.document, &world.nested] {
            assert_eq!(
                class(&model, owner)?.generalization,
                if owner == specific {
                    vec![world.generalization.clone()]
                } else {
                    Vec::new()
                }
            );
        }
        endpoint_presentation(
            &world.session,
            &world.generalization,
            &generalization_view,
            specific,
            general,
        )?;
    }
    for count in 0..=2 {
        if count > 0 {
            world
                .session
                .place(&exportable, DIAGRAM, 1000.0 + f64::from(count) * 200.0)?;
        }
        for target in [&exportable, &world.printable] {
            world.session.property(
                &world.realization,
                "properties.contract",
                json!({"$ref": target}),
            )?;
            realization_contract(&world, &world.invoice, target)?;
            endpoint_presentation(
                &world.session,
                &world.realization,
                &realization_view,
                &world.invoice,
                target,
            )?;
            world.session.round_trip()?;
        }
        for target in [&exportable, &world.document] {
            let model =
                world
                    .session
                    .property(&ends[1], "properties.type", json!({"$ref": target}))?;
            let Contract::Association(association) = contract(&model, &world.association)? else {
                return Err("expected Association".into());
            };
            assert_eq!(association.member_end, ends);
            endpoint_presentation(
                &world.session,
                &world.association,
                &association_view,
                &world.invoice,
                target,
            )?;
            world.session.round_trip()?;
        }
    }
    for source in [world.document.clone(), world.invoice.clone()] {
        world.session.property(
            &world.realization,
            "properties.implementingClassifier",
            json!({"$ref": source}),
        )?;
        realization_contract(&world, &source, &world.printable)?;
        endpoint_presentation(
            &world.session,
            &world.realization,
            &realization_view,
            &source,
            &world.printable,
        )?;
    }
    for (subject, path, invalid) in [
        (&world.generalization, "general", &world.attribute),
        (&world.generalization, "specific", &world.attribute),
        (&world.realization, "contract", &world.invoice),
        (
            &world.realization,
            "implementingClassifier",
            &world.printable,
        ),
    ] {
        world.session.reject(
            Command::SetProperty {
                element_id: subject.clone(),
                path: format!("properties.{path}"),
                value: json!({"$ref": invalid}),
            },
            "",
        )?;
    }
    for (relationship, subject, path, original) in [
        (
            &world.association,
            &ends[1],
            "properties.type",
            association_view,
        ),
        (
            &world.realization,
            &world.realization,
            "properties.contract",
            realization_view,
        ),
    ] {
        let projection = world.session.projection()?;
        let geometry = projection
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .find(|view| view.id == original.id)
            .and_then(|view| view.routed_geometry.clone())
            .ok_or("missing routed geometry")?;
        let mut expected = original.clone();
        expected.geometry = geometry.clone();
        expected.style.routing = Some(RelationshipRouting::Manual);
        let model = world.session.execute(Command::SetViewPath {
            diagram_id: DIAGRAM.into(),
            view_id: original.id,
            geometry,
        })?;
        assert_eq!(occurrence(&model, relationship)?, &expected);
        world.session.reject(
            Command::SetProperty {
                element_id: subject.clone(),
                path: path.into(),
                value: json!({"$ref": exportable}),
            },
            "manual",
        )?;
        assert_eq!(
            occurrence(&world.session.model()?, relationship)?,
            &expected
        );
    }
    world.session.round_trip()
}

fn navigation_knowledge() -> Result {
    let mut world = ClassModel::author()?;
    let ends = world.ends()?;
    let view = occurrence(&world.session.model()?, &world.association)?
        .id
        .clone();
    for navigation in ["navigable", "notNavigable", "unverified", "navigable"] {
        let model = world
            .session
            .property(&ends[0], "authoring.navigation", json!(navigation))?;
        let Contract::Association(association) = contract(&model, &world.association)? else {
            return Err("expected Association".into());
        };
        assert_eq!(association.member_end, ends);
        assert_eq!(
            association.navigable_owned_end,
            if navigation == "navigable" {
                vec![ends[0].clone()]
            } else {
                Vec::new()
            }
        );
        assert_eq!(
            model
                .authoring_knowledge
                .associations
                .get(&world.association)
                .is_some_and(|knowledge| knowledge.navigation_unverified.contains(&ends[0])),
            navigation == "unverified"
        );
        assert_eq!(occurrence(&model, &world.association)?.id, view);
        let projection = world.session.projection()?;
        let association = projection
            .inspect_element(&world.association)
            .ok_or("missing Association Inspector")?;
        assert_eq!(
            association
                .association_ends
                .iter()
                .map(|member| member.id.clone())
                .collect::<Vec<_>>(),
            ends
        );
        assert!(
            association
                .properties
                .iter()
                .all(|property| property.path != "properties.navigable")
        );
        assert!(matches!(
            projection.contract(&ends[0]),
            Some(Contract::Property(_))
        ));
        let inspector = projection
            .inspect_element(&ends[0])
            .ok_or("missing Property Inspector")?;
        let field = inspector
            .properties
            .iter()
            .find(|property| property.path == "authoring.navigation")
            .ok_or("missing navigation editor")?;
        assert!(field.editable);
        assert_eq!(field.value, json!(navigation));
        assert_eq!(
            field.editor,
            PropertyEditor::Choice(vec![
                "navigable".into(),
                "notNavigable".into(),
                "unverified".into()
            ])
        );
        let derived = inspector
            .properties
            .iter()
            .find(|property| property.path == "uml.isNavigable")
            .ok_or("missing derived navigation")?;
        assert!(!derived.editable);
        assert_eq!(derived.editor, PropertyEditor::ReadOnly);
        if navigation == "unverified" {
            assert!(
                derived
                    .value
                    .as_str()
                    .is_some_and(|value| value.contains("unverified"))
            );
        } else {
            assert_eq!(derived.value, json!(navigation == "navigable"));
        }
        world.session.round_trip()?;
    }
    Ok(())
}

fn explorer_occurrences() -> Result {
    let mut world = ClassModel::author()?;
    let initial = world.session.projection()?;
    let rows = &initial.explorer.added;
    assert_eq!(
        rows.first().ok_or("missing Explorer root")?.element_id,
        "model"
    );
    assert!(
        rows.iter()
            .any(|row| row.element_id == DIAGRAM && row.depth == 1)
    );
    let owner = rows
        .iter()
        .find(|row| row.key == format!("semantic:{}", world.invoice))
        .ok_or("missing owner row")?;
    let nested = rows
        .iter()
        .find(|row| row.key == format!("semantic:{}", world.nested))
        .ok_or("missing nested row")?;
    assert_eq!(nested.depth, owner.depth + 1);
    assert!(
        rows.iter()
            .any(|row| row.element_id == world.invoice && row.view.is_some() && row.depth == 2)
    );
    let model = world.session.execute(Command::CreateDiagram {
        kind: StructuralDiagramKind::Class,
        name: "Other diagram".into(),
        model_elements: vec!["model".into()],
    })?;
    let other = model
        .diagrams
        .iter()
        .find(|diagram| diagram.id != DIAGRAM)
        .ok_or("missing second diagram")?
        .id
        .clone();
    world.session.place(&world.invoice, &other, 20.0)?;
    let same_name = world.session.create(
        Command::CreateElement {
            diagram_id: other,
            owner_id: None,
            kind: ModelingElementKind::Class,
            name: Some("Invoice".into()),
            x: 350.0,
            y: 20.0,
        },
        |contract| matches!(contract, Contract::Class(_)),
    )?;
    assert_ne!(same_name, world.invoice);
    let before = world.session.document.section_json.clone();
    let projection = world.session.projection()?;
    let mut key_sets = Vec::new();
    for rows in [&projection.explorer.added, &projection.explorer.name] {
        let keys: BTreeSet<_> = rows.iter().map(|row| row.key.clone()).collect();
        assert_eq!(keys.len(), rows.len(), "Explorer keys must be unique");
        key_sets.push(keys);
        for subject in [&world.invoice, &same_name] {
            assert_eq!(
                rows.iter()
                    .filter(|row| row.key == format!("semantic:{subject}"))
                    .count(),
                1
            );
            let expected = if subject == &world.invoice { 2 } else { 1 };
            assert_eq!(
                rows.iter()
                    .filter(|row| row.element_id == *subject && row.view.is_some())
                    .count(),
                expected
            );
            assert_eq!(
                projection
                    .diagrams
                    .iter()
                    .flat_map(|diagram| diagram.all_views())
                    .filter(|view| view.subject.as_deref() == Some(subject.as_str()))
                    .count(),
                expected
            );
        }
    }
    assert_eq!(key_sets.first(), key_sets.last());
    assert_eq!(world.session.document.section_json, before);
    world.session.round_trip()
}

fn annotate_occurrence() -> Result {
    let mut world = ClassModel::author()?;
    let selected = world.session.place(&world.invoice, DIAGRAM, 1200.0)?;
    let before = world.session.model()?;
    let QuickEditIntent::QuickCreate(action) =
        world.session.action(&world.invoice, "uml.add-note")?
    else {
        return Err("expected note action".into());
    };
    assert_eq!(action, UmlQuickCreate::Note);
    for view in [None, Some("missing-view".into())] {
        world.session.reject(
            Command::QuickEdit {
                diagram_id: DIAGRAM.into(),
                element_id: world.invoice.clone(),
                view_id: view,
                action,
            },
            "",
        )?;
    }
    let comment = world.session.create(
        Command::QuickEdit {
            diagram_id: DIAGRAM.into(),
            element_id: world.invoice.clone(),
            view_id: Some(selected.id.clone()),
            action,
        },
        |contract| matches!(contract, Contract::Comment(_)),
    )?;
    let model = world.session.model()?;
    let Contract::Comment(note) = contract(&model, &comment)? else {
        return Err("expected Comment".into());
    };
    assert_eq!(note.annotated_element, [world.invoice.clone()]);
    let projection = world.session.projection()?;
    let links: Vec<_> = projection
        .relations
        .iter()
        .filter(|relation| {
            relation.kind == "uml.note_link"
                && relation.from == world.invoice
                && relation.to == comment
        })
        .collect();
    let [link] = links.as_slice() else {
        return Err("expected exactly one Comment link".into());
    };
    let link_id = link.id.clone();
    let link_view = occurrence(&model, &link_id)?.clone();
    let note_view = occurrence(&model, &comment)?;
    assert!(
        model
            .diagrams
            .iter()
            .find(|diagram| diagram.id == DIAGRAM)
            .ok_or("missing class diagram")?
            .views
            .contains(&link_view)
    );
    assert!(link_view.children.is_empty());
    assert!(matches!(link_view.geometry, Geometry::Path { .. }));
    assert_eq!(
        link_view.endpoints,
        Some(ViewEndpoints {
            source_view_id: Some(selected.id.clone()),
            target_view_id: Some(note_view.id.clone())
        })
    );
    let Geometry::Rect { x, y, width, .. } = selected.geometry else {
        return Err("expected selected rectangle".into());
    };
    assert!(
        matches!(note_view.geometry, Geometry::Rect { x: note_x, y: note_y, .. } if note_x == x + width + 48.0 && note_y >= y - 16.0)
    );
    for old in before
        .diagrams
        .iter()
        .flat_map(|diagram| diagram.all_views())
    {
        assert_eq!(
            model
                .diagrams
                .iter()
                .flat_map(|diagram| diagram.all_views())
                .find(|view| view.id == old.id),
            Some(old)
        );
    }
    world.session.round_trip()?;
    let removed = world.session.execute(Command::RemoveView {
        diagram_id: DIAGRAM.into(),
        view_id: link_view.id.clone(),
    })?;
    let mut expected = model;
    expected
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == DIAGRAM)
        .ok_or("missing expected diagram")?
        .views
        .retain(|view| view.id != link_view.id);
    assert_eq!(removed, expected);
    assert!(
        world
            .session
            .projection()?
            .relations
            .iter()
            .any(|relation| relation.id == link_id)
    );
    let placed = world.session.execute(Command::SetExplorerParent {
        subject_id: DIAGRAM.into(),
        parent_id: Some(link_id.clone()),
        parent_occurrence: None,
    })?;
    let placement = placed
        .hierarchy
        .get(DIAGRAM)
        .ok_or("missing link child placement")?;
    assert_eq!(placement.parent_id.as_deref(), Some(link_id.as_str()));
    assert!(placement.occurrence.is_none());
    world.session.reject(
        Command::DeleteElement {
            element_id: link_id,
        },
        "surviving hierarchy placement",
    )?;
    world.session.round_trip()
}

fn visual(identity: &str, x: f64, fill: &str) -> Result<View> {
    Ok(serde_json::from_value(json!({
        "id": identity, "subject": null,
        "geometry": { "kind": "rect", "x": x, "y": 25.0, "width": 110.0, "height": 75.0 },
        "style": { "fill": fill, "stroke": "#334455", "stroke_width": 2.0 }
    }))?)
}

fn delete_owned_content() -> Result {
    let mut world = ClassModel::author()?;
    let duplicate = world.session.place(&world.printable, DIAGRAM, 1000.0)?;
    let mut independent = world.session.model()?;
    independent.hierarchy.remove(DIAGRAM);
    independent
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == DIAGRAM)
        .ok_or("missing independent diagram")?
        .model_elements
        .clear();
    let mut plain = world.session.load(&independent)?;
    let deleted = plain.execute(Command::DeleteElement {
        element_id: "model".into(),
    })?;
    assert!(deleted.root_model.is_none());
    assert!(deleted.registry.entries().is_empty());
    let [diagram] = deleted.diagrams.as_slice() else {
        return Err("independent diagram must survive".into());
    };
    assert_eq!(diagram.id, DIAGRAM);
    assert_eq!(diagram.uml_di_type, "UMLClassDiagram");
    assert!(diagram.views.is_empty());
    assert!(diagram.model_elements.is_empty());
    plain.round_trip()?;

    let diagram = independent
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == DIAGRAM)
        .ok_or("missing class diagram")?;
    let mut layout = diagram
        .views
        .iter()
        .find(|view| {
            view.subject.as_deref() == Some(world.printable.as_str()) && view.id != duplicate.id
        })
        .ok_or("missing original occurrence")?
        .clone();
    assert!(layout.children.is_empty());
    assert!(layout.draft.is_none());
    layout.geometry = Geometry::Layout {
        bounds: Default::default(),
    };
    assert_eq!(
        serde_json::to_value(&layout.geometry)?,
        json!({"kind": "layout", "bounds": {}})
    );
    assert!(matches!(duplicate.geometry, Geometry::Rect { .. }));
    diagram
        .views
        .retain(|view| view.id != layout.id && view.id != duplicate.id);
    let before = visual("surviving-sibling-before", 15.0, "#dceeff")?;
    let after = visual("surviving-sibling-after", 350.0, "#ffeedd")?;
    let mut container = visual("surviving-visual-container", 10.0, "#f8f8f8")?;
    container.children = vec![
        before.clone(),
        layout.clone(),
        duplicate.clone(),
        after.clone(),
    ];
    diagram.views.push(container.clone());
    let mut expected_diagram = diagram.clone();
    container.children = vec![before, after];
    expected_diagram.views = vec![container];
    let mut session = world.session.load(&independent)?;
    let mut only_occurrence = session.clone();
    let removed = only_occurrence.execute(Command::RemoveView {
        diagram_id: DIAGRAM.into(),
        view_id: duplicate.id.clone(),
    })?;
    let mut expected = independent.clone();
    expected
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == DIAGRAM)
        .ok_or("missing nested diagram")?
        .views
        .iter_mut()
        .find(|view| view.id == "surviving-visual-container")
        .ok_or("missing container")?
        .children
        .retain(|view| view.id != duplicate.id);
    assert_eq!(removed, expected);
    only_occurrence.round_trip()?;

    let mut style_reference = independent.clone();
    let mut dependent = visual("surviving-style-dependent", 1500.0, "#ffffff")?;
    dependent.presentation.style_inheritance = Some(StyleInheritance {
        source: Authored::Value(layout.id),
        properties: BTreeSet::from([InheritedStyleProperty::Fill]),
    });
    style_reference
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == DIAGRAM)
        .ok_or("missing style diagram")?
        .views
        .push(dependent);
    let mut guarded = session.load(&style_reference)?;
    guarded.reject(
        Command::DeleteElement {
            element_id: "model".into(),
        },
        "surviving-style-dependent",
    )?;
    guarded.round_trip()?;

    assert!(
        independent
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.all_views())
            .all(|view| view.subject.as_deref() != Some(world.attribute.as_str()))
    );
    let mut model_reference = independent.clone();
    model_reference
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == DIAGRAM)
        .ok_or("missing reference diagram")?
        .views
        .iter_mut()
        .find(|view| view.id == "surviving-visual-container")
        .ok_or("missing reference container")?
        .presentation
        .references
        .insert(
            "owned-property-reference".into(),
            Authored::Value(PresentationReference::Model(world.attribute.clone())),
        );
    let mut guarded = session.load(&model_reference)?;
    guarded.reject(
        Command::DeleteElement {
            element_id: "model".into(),
        },
        "surviving-visual-container",
    )?;
    guarded.round_trip()?;

    let deleted = session.execute(Command::DeleteElement {
        element_id: "model".into(),
    })?;
    assert!(deleted.root_model.is_none());
    assert!(deleted.registry.entries().is_empty());
    assert_eq!(deleted.diagrams, [expected_diagram]);
    session.round_trip()
}

#[test]
fn native_class_authoring_feature() -> Result {
    let scenarios: [(&str, fn() -> Result); 7] = [
        (
            "Author classifiers and ordered members from the native blank",
            author_classifiers_and_members,
        ),
        (
            "Quick actions create associations with concrete Property ends",
            quick_associations,
        ),
        (
            "Retarget relationships without losing their presentation",
            retarget_relationships,
        ),
        (
            "Edit association-end navigation knowledge",
            navigation_knowledge,
        ),
        (
            "Distinguish semantic ownership from diagram occurrences",
            explorer_occurrences,
        ),
        ("Add a note to the selected occurrence", annotate_occurrence),
        (
            "Delete owned content while preserving independent visual containers",
            delete_owned_content,
        ),
    ];
    let mut executed = BTreeSet::new();
    for title in FEATURE
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Scenario: "))
    {
        let (_, run) = scenarios
            .iter()
            .find(|(name, _)| *name == title)
            .ok_or_else(|| format!("unimplemented scenario: {title}"))?;
        assert!(executed.insert(title), "duplicate scenario: {title}");
        run().map_err(|error| format!("Scenario: {title}: {error}"))?;
    }
    assert_eq!(
        executed.len(),
        scenarios.len(),
        "each scenario must be declared in the feature"
    );
    Ok(())
}
