//! Acceptance: the StarUML class-diagram tools create typed persisted nodes
//! and relationships through the same public Cordis revision boundary.
#[path = "support/events.rs"]
mod events;
use bellatrix_mde_cordis::{
    DiagnosticSeverity, Geometry, MODELING_NAMESPACE, MdePlugin, ModelingConvertDocument,
    ModelingConvertDocumentRequested, ModelingCreateElement, ModelingCreateElementRequested,
    ModelingCreateRelation, ModelingCreateRelationRequested, ModelingDeleteElement,
    ModelingDeleteElementRequested, ModelingDocumentFormat, ModelingElementKind, ModelingMoveView,
    ModelingMoveViewRequested, ModelingPrepareDocument, ModelingPrepareDocumentRequested,
    ModelingReadProjection, ModelingReadProjectionRequested, ModelingRelationKind,
    ModelingSetProperty, ModelingSetPropertyRequested, ModelingValidateDiagram,
    ModelingValidateDiagramRequested, ViewEndpoints,
};
use bellatrix_mde_cordis::project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectImport,
    ProjectImportRequested, ProjectPlugin, ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::sync::Arc;

fn relation_properties<'a>(document: &'a ProjectDocument, id: &str) -> &'a serde_json::Value {
    &document.section(MODELING_NAMESPACE).unwrap()["relations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|relation| relation["id"] == id)
        .expect("persisted relation")["properties"]
}

fn source_aggregation<'a>(document: &'a ProjectDocument, id: &str) -> Option<&'a str> {
    let end_id = relation_properties(document, id)
        .get("memberEnd")?
        .as_array()?
        .get(1)?
        .get("$ref")?
        .as_str()?;
    document
        .section(MODELING_NAMESPACE)?
        .get("elements")?
        .as_array()?
        .iter()
        .find(|element| element.get("id").and_then(serde_json::Value::as_str) == Some(end_id))?
        .get("properties")?
        .get("aggregation")?
        .as_str()
}

async fn reopen(
    context: &Context,
    document: &ProjectDocument,
    format: &str,
    diagram_id: &str,
) -> (
    ProjectDocument,
    bellatrix_mde_cordis::ModelingSnapshot,
    Vec<bellatrix_mde_cordis::ModelingDiagnostic>,
) {
    let bytes = serde_json::to_vec(&events::model_document(document).unwrap()).unwrap();
    let bytes = match context
        .query::<ModelingConvertDocument>(
            Routing::Unscoped,
            ModelingConvertDocumentRequested {
                bytes,
                source_format: "json".into(),
                destination_format: format.into(),
            },
        )
        .await
        .unwrap()
    {
        QueryOutcome::Answer(Ok(bytes)) => bytes,
        other => panic!("conversion unavailable: {other:?}"),
    };
    let prepared = match context
        .query::<ModelingPrepareDocument>(
            Routing::Unscoped,
            ModelingPrepareDocumentRequested {
                bytes,
                format: if format == "json" {
                    ModelingDocumentFormat::ModelJson
                } else {
                    ModelingDocumentFormat::ModelProtobuf
                },
            },
        )
        .await
        .unwrap()
    {
        QueryOutcome::Answer(Ok(prepared)) => prepared,
        other => panic!("reopen unavailable: {other:?}"),
    };
    let document = prepared.document;
    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let provider = context
        .spawn(PreparedPlugin::from_input(
            ProjectPlugin::new(service.clone()),
            service,
        ))
        .await
        .unwrap();
    let plugin = context
        .spawn(PreparedPlugin::from_input(MdePlugin, ()))
        .await
        .unwrap();
    let imported = match context
        .query::<ProjectImport>(
            Routing::Unscoped,
            ProjectImportRequested {
                document: document.clone(),
                is_saved: true,
            },
        )
        .await
        .unwrap()
    {
        QueryOutcome::Answer(Ok(document)) => document,
        other => panic!("import unavailable: {other:?}"),
    };
    let projection = match context
        .query::<ModelingReadProjection>(
            Routing::Unscoped,
            ModelingReadProjectionRequested {
                document: imported.clone(),
            },
        )
        .await
        .unwrap()
    {
        QueryOutcome::Answer(Ok(projection)) => projection,
        other => panic!("projection unavailable: {other:?}"),
    };
    let diagnostics = match context
        .query::<ModelingValidateDiagram>(
            Routing::Unscoped,
            ModelingValidateDiagramRequested {
                project_id: imported.id().clone(),
                diagram_id: diagram_id.into(),
                expected_revision: imported.revision(),
            },
        )
        .await
        .unwrap()
    {
        QueryOutcome::Answer(Ok(validation)) => validation.diagnostics,
        other => panic!("validation unavailable: {other:?}"),
    };
    plugin.dispose().await.unwrap();
    provider.dispose().await.unwrap();
    (document, projection, diagnostics)
}

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
    let mut model: serde_json::Value =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap();
    model["id"] = id.as_str().into();
    model["name"] = "UML acceptance".into();
    let diagram_id = model["diagrams"][0]["id"].as_str().unwrap().to_owned();
    let document = events::prepare(&cx, &model).await.unwrap();
    assert!(matches!(
        cx.query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
            .await
            .unwrap(),
        QueryOutcome::Answer(Ok(_))
    ));
    let catalog = events::projection(&cx, &service, &id)
        .await
        .unwrap()
        .section
        .tools;
    let nodes = [
        ModelingElementKind::Class,
        ModelingElementKind::Interface,
        ModelingElementKind::Signal,
        ModelingElementKind::DataType,
        ModelingElementKind::PrimitiveType,
        ModelingElementKind::Enumeration,
        ModelingElementKind::Frame,
        ModelingElementKind::NaryAssociation,
        ModelingElementKind::AssociationClass,
        ModelingElementKind::Template,
        ModelingElementKind::Package,
        ModelingElementKind::Model,
        ModelingElementKind::Subsystem,
        ModelingElementKind::Comment,
    ];
    let mut revision = 0_u64;
    for (index, kind) in nodes.into_iter().enumerate() {
        let request = ModelingCreateElementRequested {
            owner_id: None,
            project_id: id.clone(),
            diagram_id: diagram_id.clone(),
            kind,
            name: Some(format!(
                "Uml_{}",
                catalog.element(kind).unwrap().label.replace(' ', "_")
            )),
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
    let section = events::projection(&cx, &service, &id)
        .await
        .unwrap()
        .section;
    let node = |kind: ModelingElementKind| {
        section
            .elements
            .iter()
            .find(|e| {
                e.kind == catalog.element(kind).unwrap().model_kind
                    && e.id.starts_with("mde-element-")
            })
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
            ModelingCreateRelationRequested {
                owner_id: None,
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
        unchanged.section(MODELING_NAMESPACE),
        snapshot.section(MODELING_NAMESPACE)
    );
    let types = [
        (ModelingRelationKind::Association, "uml.association", None),
        (
            ModelingRelationKind::DirectedAssociation,
            "uml.directed_association",
            None,
        ),
        (
            ModelingRelationKind::Aggregation,
            "uml.association",
            Some("shared"),
        ),
        (
            ModelingRelationKind::Composition,
            "uml.association",
            Some("composite"),
        ),
        (ModelingRelationKind::Dependency, "uml.dependency", None),
        (
            ModelingRelationKind::Generalization,
            "uml.generalization",
            None,
        ),
        (
            ModelingRelationKind::InterfaceRealization,
            "uml.interface_realization",
            None,
        ),
        (
            ModelingRelationKind::TemplateBinding,
            "uml.template_binding",
            None,
        ),
        (ModelingRelationKind::Realization, "uml.realization", None),
        (ModelingRelationKind::Containment, "uml.containment", None),
    ];
    let mut connector_ids = Vec::new();
    for (kind, model_kind, aggregation) in types {
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
        connector_ids.push((model_kind, aggregation));
    }
    let final_snapshot = service.snapshot(&id).unwrap();
    let result = events::projection(&cx, &service, &id)
        .await
        .unwrap()
        .section;
    assert_eq!(final_snapshot.revision(), 24);
    for (kind, aggregation) in connector_ids {
        let relation = result
            .relations
            .iter()
            .find(|relation| {
                relation.kind == kind
                    && source_aggregation(&final_snapshot, &relation.id) == aggregation
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
                && source_aggregation(&final_snapshot, &relation.id) == Some("shared")
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
    let section = events::projection(&cx, &service, &id)
        .await
        .unwrap()
        .section;
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
    let after = events::projection(&cx, &service, &id)
        .await
        .unwrap()
        .section;
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
                && source_aggregation(&updated, &relation.id).is_none()
        })
        .expect("authored association");
    let association_id = association.id.clone();
    assert_eq!(association.presentation.target_labels.multiplicity, None);
    let huge = "999999999999999999999999999999999999999999999999";
    revision = updated.revision();
    for (end, notation, expected_issue) in [
        ("end1Multiplicity", " 01 .. 0002 ".to_owned(), None),
        ("end2Multiplicity", format!("{huge}..*"), None),
        (
            "end2Multiplicity",
            "0..limit".to_owned(),
            Some((DiagnosticSeverity::Warning, "MDE_MULTIPLICITY_UNSUPPORTED")),
        ),
        (
            "end2Multiplicity",
            "3..2".to_owned(),
            Some((DiagnosticSeverity::Error, "UML_MULTIPLICITY_BOUNDS")),
        ),
        ("end2Multiplicity", "*".to_owned(), None),
        ("end1Multiplicity", "-0".to_owned(), None),
    ] {
        let result = cx
            .query::<ModelingSetProperty>(
                Routing::Unscoped,
                ModelingSetPropertyRequested {
                    project_id: id.clone(),
                    element_id: association_id.clone(),
                    path: format!("properties.{}", end),
                    value: serde_json::json!(notation),
                    expected_revision: revision,
                },
            )
            .await
            .expect("multiplicity edit response");
        if notation == "3..2" {
            assert!(
                matches!(result, QueryOutcome::Answer(Err(ref error)) if error.to_string().contains("multiplicity"))
            );
            assert_eq!(
                service
                    .snapshot(&id)
                    .expect("rejected edit snapshot")
                    .revision(),
                revision
            );
            continue;
        }
        revision += 1;
        assert!(matches!(result, QueryOutcome::Answer(Ok(value)) if value == revision));
        let persisted = service.snapshot(&id).expect("edited association");
        for format in ["json", "protobuf"] {
            let (reopened_document, reopened, diagnostics) =
                reopen(&cx, &persisted, format, &diagram_id).await;
            let relation = reopened
                .section
                .relations
                .iter()
                .find(|relation| relation.id == association_id)
                .expect("same association identity after reopen");
            assert_eq!(
                relation_properties(&reopened_document, &association_id)
                    .get(end)
                    .and_then(serde_json::Value::as_str),
                Some(notation.as_str())
            );
            let presentation = &relation.presentation;
            let label = match end {
                "end1Multiplicity" => presentation.source_labels.multiplicity.clone(),
                "end2Multiplicity" => presentation.target_labels.multiplicity.clone(),
                _ => unreachable!("fixture multiplicity end"),
            };
            assert_eq!(label.as_deref(), Some(notation.as_str()));
            let diagnostics = diagnostics
                .into_iter()
                .filter(|issue| {
                    issue.subject_id == association_id && issue.code.contains("MULTIPLICITY")
                })
                .map(|issue| (issue.severity, issue.code))
                .collect::<Vec<_>>();
            assert_eq!(
                diagnostics,
                expected_issue
                    .into_iter()
                    .map(|(severity, code)| (severity, code.to_owned()))
                    .collect::<Vec<_>>()
            );
        }
    }
    let before_visual = service.snapshot(&id).unwrap();
    let before_section = events::projection(&cx, &service, &id)
        .await
        .unwrap()
        .section;
    let diagram = before_section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .unwrap();
    let source_view = diagram
        .views
        .iter()
        .find(|view| view.subject.as_deref() == Some(class.as_str()))
        .unwrap();
    let target_view = diagram
        .views
        .iter()
        .find(|view| view.subject.as_deref() == Some(interface.as_str()))
        .unwrap();
    let exact_endpoints = ViewEndpoints {
        source_view_id: Some(source_view.id.clone()),
        target_view_id: Some(target_view.id.clone()),
    };
    assert!(
        matches!(cx.query::<bellatrix_mde_cordis::ModelingCreateRelationAtViewsOrVisualDraft>(Routing::Unscoped,
        bellatrix_mde_cordis::ModelingCreateRelationAtViewsRequested { owner_id: None,
            project_id: id.clone(), diagram_id: diagram_id.clone(), kind: ModelingRelationKind::AssociationClassAttachment,
            source_view_id: source_view.id.clone(), target_view_id: target_view.id.clone(),
            from: Some(class.clone()), to: Some(interface.clone()), expected_revision: revision,
        }).await.unwrap(), QueryOutcome::Answer(Ok(value)) if value == revision + 1)
    );
    revision += 1;
    let after_visual = service.snapshot(&id).unwrap();
    let after_section = events::projection(&cx, &service, &id)
        .await
        .unwrap()
        .section;
    assert_eq!(
        after_visual.section(MODELING_NAMESPACE).unwrap()["elements"],
        before_visual.section(MODELING_NAMESPACE).unwrap()["elements"]
    );
    assert_eq!(
        after_visual.section(MODELING_NAMESPACE).unwrap()["relations"],
        before_visual.section(MODELING_NAMESPACE).unwrap()["relations"]
    );
    let attachment = after_section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .unwrap()
        .views
        .last()
        .unwrap();
    assert!(attachment.subject.is_none());
    assert_eq!(
        attachment.draft.as_ref().unwrap().notation_kind,
        "view.association_class_attachment"
    );
    assert_eq!(attachment.endpoints.as_ref(), Some(&exact_endpoints));
    assert!(
        matches!(&attachment.geometry, Geometry::Path { segments, .. } if !segments.is_empty())
    );
    assert_eq!(service.snapshot(&id).unwrap().revision(), revision);
    plugin.dispose().await.unwrap();
    provider.dispose().await.unwrap();
}
