use bellatrix_mde_cordis::{
    MdePlugin, ModelingAddMember, ModelingAddMemberRequested, ModelingMemberKind, with_model,
    ModelingQuickEdit, ModelingQuickEditRequested, UmlQuickCommand,
};
use bellatrix_mde_model::{ModelingSection, Project};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectPlugin, ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::sync::Arc;

async fn admit<P: Plugin>(ctx:&Context, plugin:P) -> cordis::FiberHandle
where P::Config: Default, P::PrepareError:std::fmt::Debug
{
    let input=plugin.prepare(P::Config::default()).unwrap();
    ctx.spawn(PreparedPlugin::from_input(plugin,input)).await.unwrap()
}
fn model_document(id:&str)->ProjectDocument {
    let mut model:Project=serde_json::from_str(
        include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
    model.id=id.into();
    model.name=id.into();
    with_model(ProjectDocument::new(ProjectId::new(id).unwrap(),id).unwrap(),&model).unwrap()
}

#[tokio::test]
async fn class_quick_edit_creates_owned_attribute_and_operation_through_cordis() {
    let ctx=Context::new();
    let projects=Arc::new(ProjectService::new());
    let storage=admit(&ctx,ProjectPlugin::new(Arc::clone(&projects))).await;
    let mde=admit(&ctx,MdePlugin).await;
    for id in ["quick-edit-a","quick-edit-b"] {
        assert!(matches!(ctx.query::<ProjectCreate>(Routing::Unscoped,
            ProjectCreateRequested{document:model_document(id)}).await.unwrap(),
            QueryOutcome::Answer(Ok(_))));
    }
    let a=ProjectId::new("quick-edit-a").unwrap();
    let b=ProjectId::new("quick-edit-b").unwrap();
    let request=|kind,rev|ModelingAddMemberRequested{
        project_id:a.clone(), owner_id:"type-list".into(),kind,expected_revision:rev,
    };
    let attr_req=request(ModelingMemberKind::Attribute,0);
    assert!(matches!(ctx.query::<ModelingAddMember>(Routing::Unscoped,attr_req.clone())
        .await.unwrap(),QueryOutcome::Answer(Ok(1))));
    assert!(matches!(ctx.query::<ModelingAddMember>(Routing::Unscoped,attr_req)
        .await.unwrap(),QueryOutcome::Answer(Err(_))));
    assert!(matches!(ctx.query::<ModelingAddMember>(Routing::Unscoped,
        request(ModelingMemberKind::Operation,1)).await.unwrap(),
        QueryOutcome::Answer(Ok(2))));
    let snapshot=projects.snapshot(&a).unwrap();
    assert_eq!(snapshot.revision(),2);
    assert_eq!(projects.snapshot(&b).unwrap().revision(),0);
    let section:ModelingSection=serde_json::from_value(snapshot
        .section("bellatrix.bellatrix-mde-modeling").unwrap().clone()).unwrap();
    for (name,kind) in [("Attribute1","rust.field"),("Operation1","rust.function")] {
        let element=section.elements.iter().find(|e|e.name==name && e.kind==kind).unwrap();
        assert!(section.relations.iter().any(|r|r.kind=="owns" &&
            r.from=="type-list" && r.to==element.id));
    }
    assert!(section.elements.iter().any(|e|e.id=="file-task"));
    mde.dispose().await.unwrap();
    assert!(matches!(ctx.query::<ModelingAddMember>(Routing::Unscoped,
        ModelingAddMemberRequested{project_id:b,owner_id:"type-list".into(),
            kind:ModelingMemberKind::Attribute,expected_revision:0})
        .await.unwrap(),QueryOutcome::Miss));
    storage.dispose().await.unwrap();
}

#[tokio::test]
async fn class_specializations_use_the_same_versioned_cordis_event() {
    let context=Context::new();
    let projects=Arc::new(ProjectService::new());
    let storage=admit(&context,ProjectPlugin::new(Arc::clone(&projects))).await;
    let engine=admit(&context,MdePlugin).await;
    let project_id=ProjectId::new("quick-edit-specialized-class").unwrap();
    assert!(matches!(
        context.query::<ProjectCreate>(Routing::Unscoped,ProjectCreateRequested {
            document:model_document(project_id.as_str()),
        }).await.unwrap(),QueryOutcome::Answer(Ok(_))
    ));
    let section:ModelingSection=serde_json::from_value(projects.snapshot(&project_id).unwrap()
        .section("bellatrix.bellatrix-mde-modeling").unwrap().clone()).unwrap();
    let diagram_id=section.diagrams[0].id.clone();
    let actions=[
        UmlQuickCommand::AddTemplateParameter,UmlQuickCommand::AddProvidedInterface,
        UmlQuickCommand::AddRequiredInterface,UmlQuickCommand::AddAssociatedClass,
        UmlQuickCommand::AddAggregatedClass,UmlQuickCommand::AddCompositedClass,
        UmlQuickCommand::AddPort,UmlQuickCommand::AddPart,
    ];
    for (index,action) in actions.into_iter().enumerate() {
        let revision=index as u64;
        assert!(matches!(
            context.query::<ModelingQuickEdit>(Routing::Unscoped,ModelingQuickEditRequested {
                project_id:project_id.clone(),diagram_id:diagram_id.clone(),
                element_id:"type-task".into(),action,expected_revision:revision,
            }).await.unwrap(),
            QueryOutcome::Answer(Ok(value)) if value==revision+1
        ),"failed {action:?}");
    }
    let document=projects.snapshot(&project_id).unwrap();
    assert_eq!(document.revision(),8);
    let section:ModelingSection=serde_json::from_value(document
        .section("bellatrix.bellatrix-mde-modeling").unwrap().clone()).unwrap();
    for kind in ["uml.template_parameter","uml.port","uml.part"] {
        let node=section.elements.iter().find(|e|e.kind==kind).unwrap();
        assert!(section.relations.iter().any(|r|
            r.kind=="owns" && r.from=="type-task" && r.to==node.id));
    }
    for kind in ["uml.association","uml.aggregation","uml.composition",
                 "uml.interface_realization","uml.dependency"] {
        assert!(section.relations.iter().any(|r|r.kind==kind),"{kind}");
    }
    assert_eq!(section.elements.iter().filter(|e|e.kind=="rust.trait"
        && e.id.starts_with("mde-")).count(),2);
    engine.dispose().await.unwrap();
    storage.dispose().await.unwrap();
}
