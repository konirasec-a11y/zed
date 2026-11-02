use bellatrix_mde::{ModelingSection, Project, StructuralDiagramKind};
use bellatrix_mde_cordis::{
    MODELING_NAMESPACE, MdePlugin, ModelingCreateDiagram, ModelingCreateDiagramRequested,
    ModelingDeleteDiagram, ModelingDeleteDiagramRequested, ModelingRenameDiagram,
    ModelingRenameDiagramRequested, with_model,
};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectPlugin,
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
    let model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    let id = ProjectId::new(model.id.clone())?;
    let document = with_model(
        ProjectDocument::new(id.clone(), model.name.clone())?,
        &model,
    )?;
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
    let section: ModelingSection = serde_json::from_value(
        snapshot
            .section(MODELING_NAMESPACE)
            .ok_or("section missing")?
            .clone(),
    )?;
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.name == "New diagram")
        .ok_or("created diagram missing")?;
    let diagram_id = diagram.id.clone();
    assert_eq!(diagram.uml_di_type.as_str(), "UMLClassDiagram");
    assert_eq!(section.elements, model.elements);
    assert_eq!(section.relations, model.relations);

    let stale = context
        .query::<ModelingCreateDiagram>(Routing::Unscoped, create(StructuralDiagramKind::Class, 0))
        .await?;
    assert!(matches!(stale, QueryOutcome::Answer(Err(_))));
    let invalid = context
        .query::<ModelingCreateDiagram>(
            Routing::Unscoped,
            create(StructuralDiagramKind::CompositeStructure, 1),
        )
        .await?;
    assert!(matches!(invalid, QueryOutcome::Answer(Err(_))));
    let unchanged = service.snapshot(&id)?;
    assert_eq!(unchanged.revision(), 1);
    assert_eq!(
        unchanged.section(MODELING_NAMESPACE),
        snapshot.section(MODELING_NAMESPACE)
    );

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
    let renamed_section: ModelingSection = serde_json::from_value(
        renamed
            .section(MODELING_NAMESPACE)
            .ok_or("section missing")?
            .clone(),
    )?;
    assert!(
        renamed_section
            .diagrams
            .iter()
            .any(|diagram| diagram.id == diagram_id && diagram.name == "Renamed diagram")
    );
    assert_eq!(renamed.id(), &id);
    assert_eq!(renamed.name(), model.name);

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
    let final_section: ModelingSection = serde_json::from_value(
        deleted
            .section(MODELING_NAMESPACE)
            .ok_or("section missing")?
            .clone(),
    )?;
    assert_eq!(final_section, ModelingSection::from_model(&model));
    mde.dispose().await?;
    assert!(matches!(
        context
            .query::<ModelingCreateDiagram>(
                Routing::Unscoped,
                create(StructuralDiagramKind::Class, 3)
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
    let mut model: Project =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    for diagram in &mut model.diagrams {
        if diagram.id == "diagram-tasks" {
            diagram.uml_di_type = "UMLClassDiagram".into();
        }
    }
    model.relations.push(bellatrix_mde::Relation {
        id: "hidden-self-generalization".into(),
        kind: "uml.generalization".into(),
        from: "type-list".into(),
        to: "type-list".into(),
        properties: Default::default(),
    });
    let id = ProjectId::new(model.id.clone())?;
    let document = with_model(
        ProjectDocument::new(id.clone(), model.name.clone())?,
        &model,
    )?;
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
                },
            )
            .await?;
        let diagnostics = match result {
            QueryOutcome::Answer(Ok(diagnostics)) => diagnostics,
            other => return Err(format!("unexpected validation response: {other:?}").into()),
        };
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
    let missing = context
        .query::<ModelingValidateDiagram>(
            Routing::Unscoped,
            ModelingValidateDiagramRequested {
                project_id: id.clone(),
                diagram_id: "missing".into(),
            },
        )
        .await?;
    assert!(matches!(missing, QueryOutcome::Answer(Err(_))));
    assert_eq!(service.snapshot(&id)?.revision(), 0);
    mde.dispose().await?;
    platform.dispose().await?;
    Ok(())
}
