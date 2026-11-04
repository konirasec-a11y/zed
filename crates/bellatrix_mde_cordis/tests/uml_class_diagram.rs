//! Acceptance: the StarUML class-diagram tools create typed persisted nodes
//! and relationships through the same public Cordis revision boundary.
use bellatrix_mde::uml::types::{
    Integer, MultiplicityError, MultiplicityRange, RelationEnd, RelationEndMultiplicity,
    StandardPrimitiveType, UnlimitedNatural,
};
use bellatrix_mde::{
    Format, Geometry, ModelingSection, Project, StructuralSeverity, decode, encode,
    relation_kind_for_creation, relationship_presentation, validate_structural_model,
};
use bellatrix_mde_cordis::{
    MdePlugin, ModelingCreateElement, ModelingCreateElementRequested, ModelingCreateRelation,
    ModelingCreateRelationRequested, ModelingDeleteElement, ModelingDeleteElementRequested,
    ModelingElementKind, ModelingMoveView, ModelingMoveViewRequested, ModelingRelationKind,
    ModelingSetProperty, ModelingSetPropertyRequested, with_model,
};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectPlugin,
    ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::sync::Arc;

#[tokio::test]
async fn complete_class_diagram_toolset_is_revisioned_and_editable() {
    let cx = Context::new();
    let service = Arc::new(ProjectService::new());
    let provider = ProjectPlugin::new(Arc::clone(&service));
    let provider = cx
        .spawn(PreparedPlugin::from_input(provider, Arc::clone(&service)))
        .await
        .unwrap();
    let plugin = MdePlugin;
    plugin.prepare(()).unwrap();
    let plugin = cx
        .spawn(PreparedPlugin::from_input(plugin, ()))
        .await
        .unwrap();
    let id = ProjectId::new("uml-acceptance").unwrap();
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap();
    model.id = id.as_str().into();
    model.name = "UML acceptance".into();
    let document = with_model(
        ProjectDocument::new(id.clone(), model.name.clone()).unwrap(),
        &model,
    )
    .unwrap();
    assert!(matches!(
        cx.query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
            .await
            .unwrap(),
        QueryOutcome::Answer(Ok(_))
    ));
    let diagram_id = model.diagrams.first().unwrap().id.clone();
    let nodes = ModelingElementKind::BASIC
        .iter()
        .chain(ModelingElementKind::ADVANCED)
        .chain(ModelingElementKind::PACKAGES)
        .chain(ModelingElementKind::ANNOTATIONS);
    let mut revision = 0_u64;
    for (index, kind) in nodes.copied().enumerate() {
        let request = ModelingCreateElementRequested {
            owner_id: None,
            project_id: id.clone(),
            diagram_id: diagram_id.clone(),
            kind,
            name: Some(format!("Uml_{}", kind.label().replace(' ', "_"))),
            x: 40.0 + index as f64 * 8.0,
            y: 30.0 + index as f64 * 12.0,
            expected_revision: revision,
        };
        revision += 1;
        assert!(matches!(
            cx.query::<ModelingCreateElement>(Routing::Unscoped,request).await.unwrap(),
            QueryOutcome::Answer(Ok(n)) if n==revision
        ));
    }
    let snapshot = service.snapshot(&id).unwrap();
    let section: ModelingSection = serde_json::from_value(
        snapshot
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    let node = |kind: ModelingElementKind| {
        section
            .elements
            .iter()
            .find(|e| e.kind == kind.model_kind() && e.id.starts_with("mde-element-"))
            .unwrap()
            .id
            .clone()
    };
    let class = node(ModelingElementKind::Class);
    let interface = node(ModelingElementKind::Interface);
    let package = node(ModelingElementKind::Package);
    let template = node(ModelingElementKind::Template);
    let rejected = cx
        .query::<ModelingCreateRelation>(
            Routing::Unscoped,
            ModelingCreateRelationRequested { owner_id: None,
                project_id: id.clone(),
                diagram_id: diagram_id.clone(),
                kind: ModelingRelationKind::Generalization,
                from: class.clone(),
                to: interface.clone(),
                expected_revision: revision,
            },
        )
        .await
        .unwrap();
    assert!(
        matches!(rejected, QueryOutcome::Answer(Err(ref error)) if error.contains("UML_GENERALIZATION_TYPE"))
    );
    let unchanged = service.snapshot(&id).unwrap();
    assert_eq!(unchanged.revision(), revision);
    assert_eq!(
        unchanged.section(bellatrix_mde::MODELING_NAMESPACE),
        snapshot.section(bellatrix_mde::MODELING_NAMESPACE)
    );
    let types = ModelingRelationKind::BASIC
        .iter()
        .chain(ModelingRelationKind::ADVANCED)
        .chain(ModelingRelationKind::PACKAGES);
    let mut connector_ids = Vec::new();
    for kind in types.copied().filter(|kind| !kind.is_visual_only()) {
        let (from, to) = match kind {
            ModelingRelationKind::Generalization => (class.clone(), "type-task".into()),
            ModelingRelationKind::InterfaceRealization => (class.clone(), interface.clone()),
            ModelingRelationKind::Containment => (package.clone(), class.clone()),
            ModelingRelationKind::TemplateBinding => (template.clone(), class.clone()),
            _ => (class.clone(), interface.clone()),
        };
        revision += 1;
        assert!(matches!(
            cx.query::<ModelingCreateRelation>(Routing::Unscoped,ModelingCreateRelationRequested{ owner_id: None,
                project_id:id.clone(),diagram_id:diagram_id.clone(),kind,
                from,to,expected_revision:revision-1,
            }).await.unwrap(),QueryOutcome::Answer(Ok(n)) if n==revision
        ));
        connector_ids.push(relation_kind_for_creation(kind));
    }
    let final_snapshot = service.snapshot(&id).unwrap();
    let result: ModelingSection = serde_json::from_value(
        final_snapshot
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert_eq!(final_snapshot.revision(), 24);
    for (kind, aggregation) in connector_ids {
        let relation = result
            .relations
            .iter()
            .find(|relation| {
                relation.kind == kind
                    && relation
                        .properties
                        .get("end1Aggregation")
                        .and_then(serde_json::Value::as_str)
                        == aggregation
            })
            .unwrap();
        assert!(
            result
                .diagrams
                .iter()
                .flat_map(|d| &d.views)
                .any(|v| v.subject.as_deref() == Some(relation.id.as_str())
                    && matches!(&v.geometry, Geometry::Path { .. }))
        );
    }
    // Deleting a connector must not cascade through its endpoint classes.
    let target = result
        .relations
        .iter()
        .find(|relation| {
            relation.kind == "uml.association"
                && relation
                    .properties
                    .get("end1Aggregation")
                    .and_then(serde_json::Value::as_str)
                    == Some("shared")
        })
        .unwrap()
        .id
        .clone();
    assert!(matches!(
        cx.query::<ModelingDeleteElement>(
            Routing::Unscoped,
            ModelingDeleteElementRequested {
                project_id: id.clone(),
                element_id: target,
                expected_revision: revision,
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(25))
    ));
    let final_doc = service.snapshot(&id).unwrap();
    let section: ModelingSection = serde_json::from_value(
        final_doc
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert!(section.elements.iter().any(|e| e.id == class));
    assert!(section.elements.iter().any(|e| e.id == interface));
    let class_view = section
        .diagrams
        .iter()
        .flat_map(|d| &d.views)
        .find(|v| v.subject.as_deref() == Some(class.as_str()))
        .unwrap()
        .id
        .clone();
    let edge_id = section
        .relations
        .iter()
        .find(|r| r.kind == "uml.generalization")
        .unwrap()
        .id
        .clone();
    let previous = section
        .diagrams
        .iter()
        .flat_map(|d| &d.views)
        .find(|v| v.subject.as_deref() == Some(edge_id.as_str()))
        .unwrap()
        .geometry
        .clone();
    assert!(matches!(
        cx.query::<ModelingMoveView>(
            Routing::Unscoped,
            ModelingMoveViewRequested {
                project_id: id.clone(),
                view_id: class_view,
                dx: 45.0,
                dy: 25.0,
                expected_revision: 25,
            }
        )
        .await
        .unwrap(),
        QueryOutcome::Answer(Ok(26))
    ));
    let updated = service.snapshot(&id).unwrap();
    let after: ModelingSection = serde_json::from_value(
        updated
            .section(bellatrix_mde::MODELING_NAMESPACE)
            .unwrap()
            .clone(),
    )
    .unwrap();
    let rerouted = after
        .diagrams
        .iter()
        .flat_map(|d| &d.views)
        .find(|v| v.subject.as_deref() == Some(edge_id.as_str()))
        .unwrap();
    assert_ne!(
        previous, rerouted.geometry,
        "auto-routed UML connector must follow its node"
    );
    assert_eq!(updated.revision(), 26);
    let association = after
        .relations
        .iter()
        .find(|relation| {
            relation.kind == "uml.association"
                && !relation.properties.contains_key("end1Aggregation")
        })
        .expect("authored association");
    let association_id = association.id.clone();
    let default_range = RelationEndMultiplicity::new(association, RelationEnd::Target)
        .effective()
        .expect("absent multiplicity defaults to one");
    assert_eq!(default_range.lower_bound(), &Integer::from(1));
    assert_eq!(default_range.upper_bound(), &UnlimitedNatural::from(1));
    assert!(!default_range.is_multivalued());
    assert_eq!(
        relationship_presentation(association)
            .target_labels
            .multiplicity,
        None
    );
    assert_eq!(Integer::PRIMITIVE_TYPE, StandardPrimitiveType::Integer);
    assert_eq!(
        UnlimitedNatural::PRIMITIVE_TYPE,
        StandardPrimitiveType::UnlimitedNatural
    );
    let unbounded: MultiplicityRange = "*".parse().expect("unlimited alias");
    assert!(unbounded.includes_multiplicity(&default_range));
    assert!(default_range.compatible_with(&unbounded));
    assert!(!default_range.includes_multiplicity(&unbounded));
    assert_eq!(
        MultiplicityRange::from_bounds(None, None).unwrap(),
        default_range
    );
    assert_eq!(
        MultiplicityRange::from_bounds(Some(0.into()), None)
            .unwrap()
            .upper_bound(),
        &UnlimitedNatural::from(1)
    );
    assert_eq!(
        MultiplicityRange::from_bounds(None, Some(2.into()))
            .unwrap()
            .lower_bound(),
        &Integer::from(1)
    );
    let huge = "999999999999999999999999999999999999999999999999";
    revision = updated.revision();
    for (end, notation, expected_issue) in [
        (RelationEnd::Source, " 01 .. 0002 ".to_owned(), None),
        (RelationEnd::Target, format!("{huge}..*"), None),
        (
            RelationEnd::Target,
            "0..limit".to_owned(),
            Some((StructuralSeverity::Warning, "MDE_MULTIPLICITY_UNSUPPORTED")),
        ),
        (
            RelationEnd::Target,
            "3..2".to_owned(),
            Some((StructuralSeverity::Error, "UML_MULTIPLICITY_BOUNDS")),
        ),
        (RelationEnd::Target, "*".to_owned(), None),
        (RelationEnd::Source, "-0".to_owned(), None),
    ] {
        let result = cx
            .query::<ModelingSetProperty>(
                Routing::Unscoped,
                ModelingSetPropertyRequested {
                    project_id: id.clone(),
                    element_id: association_id.clone(),
                    path: format!("properties.{}", end.multiplicity_key()),
                    value: serde_json::json!(notation),
                    expected_revision: revision,
                },
            )
            .await
            .expect("multiplicity edit response");
        if notation == "3..2" {
            assert!(matches!(result, QueryOutcome::Answer(Err(ref error)) if error.to_string().contains("multiplicity")));
            assert_eq!(service.snapshot(&id).expect("rejected edit snapshot").revision(), revision);
            continue;
        }
        revision += 1;
        assert!(matches!(result, QueryOutcome::Answer(Ok(value)) if value == revision));
        let persisted = service.snapshot(&id).expect("edited association");
        let section: ModelingSection = serde_json::from_value(
            persisted
                .section(bellatrix_mde::MODELING_NAMESPACE)
                .unwrap()
                .clone(),
        )
        .expect("persisted model");
        let model = section.to_model(id.as_str(), "UML acceptance");
        for format in [Format::Json, Format::Protobuf] {
            let reopened = decode(&encode(&model, format).expect("serialize model"), format)
                .expect("reopen model");
            let relation = reopened
                .relations
                .iter()
                .find(|relation| relation.id == association_id)
                .expect("same association identity after reopen");
            let multiplicity = RelationEndMultiplicity::new(relation, end);
            assert_eq!(multiplicity.authored_notation(), Some(notation.as_str()));
            let presentation = relationship_presentation(relation);
            let label = match end {
                RelationEnd::Source => presentation.source_labels.multiplicity,
                RelationEnd::Target => presentation.target_labels.multiplicity,
            };
            assert_eq!(label.as_deref(), Some(notation.as_str()));
            let diagnostics = validate_structural_model(&reopened)
                .into_iter()
                .filter(|issue| {
                    issue.subject_id == association_id && issue.code.contains("MULTIPLICITY")
                })
                .map(|issue| (issue.severity, issue.code))
                .collect::<Vec<_>>();
            assert_eq!(diagnostics, expected_issue.into_iter().collect::<Vec<_>>());
            match notation.as_str() {
                "0..limit" => assert_eq!(
                    multiplicity.effective(),
                    Err(MultiplicityError::UnsupportedValueSpecification)
                ),
                "3..2" => assert_eq!(
                    multiplicity.effective(),
                    Err(MultiplicityError::InvalidBounds)
                ),
                " 01 .. 0002 " => {
                    let range = multiplicity.effective().expect("finite interval");
                    assert_eq!(range.lower_bound(), &Integer::from(1));
                    assert_eq!(range.upper_bound(), &UnlimitedNatural::from(2));
                    assert!(range.is_multivalued());
                    assert!(range.includes_cardinality(&Integer::from(2)));
                    assert!(!range.includes_cardinality(&Integer::from(3)));
                }
                "-0" => {
                    let range = multiplicity.effective().expect("zero cardinality");
                    assert_eq!(range.lower_bound(), &Integer::from(0));
                    assert_eq!(range.upper_bound(), &UnlimitedNatural::from(0));
                    assert!(!range.is_multivalued());
                }
                _ => {
                    let range = multiplicity.effective().expect("unlimited interval");
                    assert!(range.upper_bound().is_unlimited());
                    assert!(range.is_multivalued());
                    assert!(range.includes_cardinality(&huge.parse().unwrap()));
                    assert_eq!(
                        range.lower_bound().to_string(),
                        if notation == "*" { "0" } else { huge }
                    );
                }
            }
        }
    }
    let before_visual = service.snapshot(&id).unwrap();
    let before_section: ModelingSection = serde_json::from_value(before_visual.section(bellatrix_mde::MODELING_NAMESPACE).unwrap().clone()).unwrap();
    let diagram = before_section.diagrams.iter().find(|diagram| diagram.id == diagram_id).unwrap();
    let source_view = diagram.views.iter().find(|view| view.subject.as_deref() == Some(class.as_str())).unwrap();
    let target_view = diagram.views.iter().find(|view| view.subject.as_deref() == Some(interface.as_str())).unwrap();
    let exact_endpoints = bellatrix_mde::ViewEndpoints { source_view_id: Some(source_view.id.clone()), target_view_id: Some(target_view.id.clone()) };
    assert!(matches!(cx.query::<bellatrix_mde_cordis::ModelingCreateRelationAtViewsOrVisualDraft>(Routing::Unscoped,
        bellatrix_mde_cordis::ModelingCreateRelationAtViewsRequested { owner_id: None,
            project_id: id.clone(), diagram_id: diagram_id.clone(), kind: ModelingRelationKind::AssociationClassAttachment,
            source_view_id: source_view.id.clone(), target_view_id: target_view.id.clone(),
            from: Some(class.clone()), to: Some(interface.clone()), expected_revision: revision,
        }).await.unwrap(), QueryOutcome::Answer(Ok(value)) if value == revision + 1));
    revision += 1;
    let after_visual = service.snapshot(&id).unwrap();
    let after_section: ModelingSection = serde_json::from_value(after_visual.section(bellatrix_mde::MODELING_NAMESPACE).unwrap().clone()).unwrap();
    assert_eq!(after_section.elements, before_section.elements);
    assert_eq!(after_section.relations, before_section.relations);
    let attachment = after_section.diagrams.iter().find(|diagram| diagram.id == diagram_id).unwrap().views.last().unwrap();
    assert!(attachment.subject.is_none());
    assert_eq!(attachment.draft.as_ref().unwrap().notation_kind, "view.association_class_attachment");
    assert_eq!(attachment.endpoints.as_ref(), Some(&exact_endpoints));
    assert!(matches!(&attachment.geometry, Geometry::Path { segments, .. } if !segments.is_empty()));
    assert_eq!(service.snapshot(&id).unwrap().revision(), revision);
    plugin.dispose().await.unwrap();
    provider.dispose().await.unwrap();
}
