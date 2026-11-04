#[path = "support/events.rs"]
mod events;
use serde_json::{Value, json};
use bellatrix_mde_cordis::{StructuralDiagramKind, ModelingValidateDiagram, ModelingValidateDiagramRequested};
use bellatrix_mde_cordis::{
    MODELING_NAMESPACE, MdePlugin, ModelingCreateDiagram, ModelingCreateDiagramRequested,
    ModelingDeleteDiagram, ModelingDeleteDiagramRequested, ModelingRenameDiagram,
    ModelingRenameDiagramRequested,
};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectId, ProjectPlugin,
    ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::sync::Arc;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn admit<P: Plugin>(context: &Context, plugin: P) -> TestResult<cordis::FiberHandle>
where
    P::Config: Default,
    P::PrepareError: std::fmt::Debug,
{
    let input = plugin
        .prepare(P::Config::default())
        .map_err(|error| format!("{error:?}"))?;
    Ok(context
        .spawn(PreparedPlugin::from_input(plugin, input))
        .await
        .map_err(|error| format!("{error:?}"))?)
}

#[tokio::test]
async fn diagram_lifecycle_is_revision_checked_and_preserves_semantic_model() -> TestResult {
    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let platform = admit(&context, ProjectPlugin::new(Arc::clone(&service))).await?;
    let mde = admit(&context, MdePlugin).await?;
    let model: Value =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    let id = ProjectId::new(model["id"].as_str().ok_or("id missing")?)?;
    let document = events::prepare(&context, &model).await?;
    let baseline = document.section(MODELING_NAMESPACE).ok_or("section missing")?.clone();
    let result = context
        .query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
        .await?;
    assert!(matches!(result, QueryOutcome::Answer(Ok(_))));
    let create = |kind, revision| ModelingCreateDiagramRequested {
        project_id: id.clone(),
        kind,
        name: "New diagram".into(),
        model_elements: Vec::new(),
        expected_revision: revision,
    };
    let created = context
        .query::<ModelingCreateDiagram>(Routing::Unscoped, create(StructuralDiagramKind::Class, 0))
        .await?;
    assert!(matches!(created, QueryOutcome::Answer(Ok(1))));
    let snapshot = service.snapshot(&id)?;
    let section = events::projection(&context, &service, &id).await?.section;
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.name == "New diagram")
        .ok_or("created diagram missing")?;
    let diagram_id = diagram.id.clone();
    assert_eq!(diagram.uml_di_type.as_str(), "UMLClassDiagram");
    assert_eq!(snapshot.section(MODELING_NAMESPACE).ok_or("section missing")?["elements"], baseline["elements"]);
    assert_eq!(snapshot.section(MODELING_NAMESPACE).ok_or("section missing")?["relations"], baseline["relations"]);

    let stale = context
        .query::<ModelingCreateDiagram>(Routing::Unscoped, create(StructuralDiagramKind::Class, 0))
        .await?;
    assert!(matches!(stale, QueryOutcome::Answer(Err(_))));
    let rename = |revision| ModelingRenameDiagramRequested {
        project_id: id.clone(),
        diagram_id: diagram_id.clone(),
        name: "Renamed diagram".into(),
        expected_revision: revision,
    };
    assert!(matches!(
        context
            .query::<ModelingRenameDiagram>(Routing::Unscoped, rename(0))
            .await?,
        QueryOutcome::Answer(Err(_))
    ));
    assert!(matches!(
        context
            .query::<ModelingRenameDiagram>(Routing::Unscoped, rename(1))
            .await?,
        QueryOutcome::Answer(Ok(2))
    ));
    let renamed = service.snapshot(&id)?;
    let renamed_section = events::projection(&context, &service, &id).await?.section;
    assert!(
        renamed_section
            .diagrams
            .iter()
            .any(|diagram| diagram.id == diagram_id && diagram.name == "Renamed diagram")
    );
    assert_eq!(renamed.id(), &id);
    assert_eq!(renamed.name(), model["name"].as_str().ok_or("name missing")?);

    let delete = |revision| ModelingDeleteDiagramRequested {
        project_id: id.clone(),
        diagram_id: diagram_id.clone(),
        expected_revision: revision,
    };
    assert!(matches!(
        context
            .query::<ModelingDeleteDiagram>(Routing::Unscoped, delete(1))
            .await?,
        QueryOutcome::Answer(Err(_))
    ));
    assert!(matches!(
        context
            .query::<ModelingDeleteDiagram>(Routing::Unscoped, delete(2))
            .await?,
        QueryOutcome::Answer(Ok(3))
    ));
    let deleted = service.snapshot(&id)?;
    let final_section = events::section(&service, &id)?;
    assert_eq!(final_section, baseline);
    assert!(matches!(
        context
            .query::<ModelingCreateDiagram>(
                Routing::Unscoped,
                create(StructuralDiagramKind::CompositeStructure, 3)
            )
            .await?,
        QueryOutcome::Answer(Ok(4))
    ));
    let unverified = service.snapshot(&id)?;
    let section = events::projection(&context, &service, &id).await?.section;
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.uml_di_type == "UMLCompositeStructureDiagram")
        .ok_or("unverified diagram missing")?;
    let validation = match context.query::<ModelingValidateDiagram>(Routing::Unscoped,
        ModelingValidateDiagramRequested { project_id: id.clone(), diagram_id: diagram.id.clone(),
            expected_revision: unverified.revision() }).await? {
        QueryOutcome::Answer(Ok(validation)) => validation,
        other => return Err(format!("unexpected validation response: {other:?}").into()),
    };
    assert!(validation.diagnostics.iter()
        .any(|diagnostic| diagnostic.code == "UMLDI_CONTEXT_MULTIPLICITY"));
    mde.dispose().await?;
    assert!(matches!(
        context
            .query::<ModelingCreateDiagram>(
                Routing::Unscoped,
                create(StructuralDiagramKind::Class, 4)
            )
            .await?,
        QueryOutcome::Miss
    ));
    platform.dispose().await?;
    Ok(())
}

#[tokio::test]
async fn repeated_validation_reports_hidden_semantics_without_mutating_or_bumping_revision()
-> TestResult {
    use bellatrix_mde_cordis::{ModelingValidateDiagram, ModelingValidateDiagramRequested};
    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let platform = admit(&context, ProjectPlugin::new(Arc::clone(&service))).await?;
    let mde = admit(&context, MdePlugin).await?;
    let mut model: Value =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    for diagram in model["diagrams"].as_array_mut().ok_or("diagrams missing")? {
        if diagram["id"] == "diagram-tasks" {
            diagram["uml_di_type"] = "UMLClassDiagram".into();
        }
    }
    model["relations"].as_array_mut().ok_or("relations missing")?.push(json!({
        "id": "hidden-self-generalization", "kind": "uml.generalization",
        "from": "type-list", "to": "type-list", "properties": {},
    }));
    let id = ProjectId::new(model["id"].as_str().ok_or("id missing")?)?;
    let document = events::prepare(&context, &model).await?;
    assert!(matches!(
        context
            .query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
            .await?,
        QueryOutcome::Answer(Ok(_))
    ));
    let before = service.snapshot(&id)?;
    for _ in 0..2 {
        let result = context
            .query::<ModelingValidateDiagram>(
                Routing::Unscoped,
                ModelingValidateDiagramRequested {
                    project_id: id.clone(),
                    diagram_id: "diagram-tasks".into(),
                    expected_revision: before.revision(),
                },
            )
            .await?;
        let validation = match result {
            QueryOutcome::Answer(Ok(result)) => result,
            other => return Err(format!("unexpected validation response: {other:?}").into()),
        };
        assert_eq!(validation.revision, before.revision());
        assert_eq!(validation.diagram_id, "diagram-tasks");
        let diagnostics = validation.diagnostics;
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "UML_GENERALIZATION_CYCLE")
        );
        // Validation uses the persisted Class type.
        assert!(
            !diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "UMLDI_CONTEXT_MULTIPLICITY")
        );
        let after = service.snapshot(&id)?;
        assert_eq!(after.revision(), before.revision());
        assert_eq!(
            after.section(MODELING_NAMESPACE),
            before.section(MODELING_NAMESPACE)
        );
    }
    let stale = context
        .query::<ModelingValidateDiagram>(
            Routing::Unscoped,
            ModelingValidateDiagramRequested {
                project_id: id.clone(),
                diagram_id: "diagram-tasks".into(),
                expected_revision: before.revision() + 1,
            },
        )
        .await?;
    assert!(matches!(stale, QueryOutcome::Answer(Err(_))));
    let missing = context
        .query::<ModelingValidateDiagram>(
            Routing::Unscoped,
            ModelingValidateDiagramRequested {
                project_id: id.clone(),
                diagram_id: "missing".into(),
                expected_revision: before.revision(),
            },
        )
        .await?;
    assert!(matches!(missing, QueryOutcome::Answer(Err(_))));
    assert_eq!(service.snapshot(&id)?.revision(), 0);
    mde.dispose().await?;
    platform.dispose().await?;
    Ok(())
}
