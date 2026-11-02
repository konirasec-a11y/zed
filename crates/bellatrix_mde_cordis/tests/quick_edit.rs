use bellatrix_mde_cordis::{
    MdePlugin, ModelingAddMember, ModelingAddMemberRequested, ModelingMemberKind, with_model,
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
