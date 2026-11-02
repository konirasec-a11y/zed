//! Acceptance: the StarUML class-diagram tools create typed persisted nodes
//! and relationships through the same public Cordis revision boundary.
use bellatrix_mde_cordis::{
    MdePlugin, ModelingCreateElement, ModelingCreateElementRequested,
    ModelingCreateRelation, ModelingCreateRelationRequested,
    ModelingElementKind, ModelingRelationKind, ModelingDeleteElement,
    ModelingDeleteElementRequested, with_model,
};
use bellatrix_mde_model::{Geometry, ModelingSection, Project};
use bellatrix_project::{
    ProjectCreate, ProjectCreateRequested, ProjectDocument, ProjectId, ProjectPlugin, ProjectService,
};
use cordis::event::QueryOutcome;
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::sync::Arc;

#[tokio::test]
async fn complete_class_diagram_toolset_is_revisioned_and_isolated() {
    let cx=Context::new();
    let service=Arc::new(ProjectService::new());
    let provider=ProjectPlugin::new(Arc::clone(&service));
    let provider=cx.spawn(PreparedPlugin::from_input(provider,Arc::clone(&service))).await.unwrap();
    let plugin=MdePlugin;
    let input=plugin.prepare(()).unwrap();
    let plugin=cx.spawn(PreparedPlugin::from_input(plugin,input)).await.unwrap();
    let id=ProjectId::new("uml-acceptance").unwrap();
    let mut model:Project=serde_json::from_str(
        include_str!("../../bellatrix_mde_model/fixtures/todo.json")
    ).unwrap();
    model.id=id.as_str().into();
    model.name="UML acceptance".into();
    let document=with_model(ProjectDocument::new(id.clone(),model.name.clone()).unwrap(),&model).unwrap();
    assert!(matches!(
        cx.query::<ProjectCreate>(Routing::Unscoped,ProjectCreateRequested {document})
            .await.unwrap(),QueryOutcome::Answer(Ok(_))
    ));
    let diagram_id=model.diagrams.first().unwrap().id.clone();
    let nodes=ModelingElementKind::BASIC.iter().chain(ModelingElementKind::ADVANCED)
        .chain(ModelingElementKind::PACKAGES).chain(ModelingElementKind::ANNOTATIONS);
    let mut revision=0_u64;
    for (index,kind) in nodes.copied().enumerate() {
        let request=ModelingCreateElementRequested {
            project_id:id.clone(),diagram_id:diagram_id.clone(),kind,
            name:Some(format!("Uml_{}",kind.label().replace(' ',"_"))),
            x: 40.0+index as f64*8.0,y:30.0+index as f64*12.0,
            expected_revision:revision,
        };
        revision+=1;
        assert!(matches!(
            cx.query::<ModelingCreateElement>(Routing::Unscoped,request).await.unwrap(),
            QueryOutcome::Answer(Ok(n)) if n==revision
        ));
    }
    let snapshot=service.snapshot(&id).unwrap();
    let section:ModelingSection=serde_json::from_value(
        snapshot.section("bellatrix.bellatrix-mde-modeling").unwrap().clone()
    ).unwrap();
    let node=|kind:ModelingElementKind|section.elements.iter()
        .find(|e|e.kind==kind.model_kind()&&e.id.starts_with("mde-element-"))
        .unwrap().id.clone();
    let class=node(ModelingElementKind::Class);
    let interface=node(ModelingElementKind::Interface);
    let package=node(ModelingElementKind::Package);
    let template=node(ModelingElementKind::Template);
    let types=ModelingRelationKind::BASIC.iter().chain(ModelingRelationKind::ADVANCED)
        .chain(ModelingRelationKind::PACKAGES);
    let mut connector_ids=Vec::new();
    for kind in types.copied() {
        let (from,to)=match kind {
            ModelingRelationKind::Containment=>(package.clone(),class.clone()),
            ModelingRelationKind::TemplateBinding=>(template.clone(),class.clone()),
            _=>(class.clone(),interface.clone()),
        };
        revision+=1;
        assert!(matches!(
            cx.query::<ModelingCreateRelation>(Routing::Unscoped,ModelingCreateRelationRequested{
                project_id:id.clone(),diagram_id:diagram_id.clone(),kind,
                from,to,expected_revision:revision-1,
            }).await.unwrap(),QueryOutcome::Answer(Ok(n)) if n==revision
        ));
        connector_ids.push(kind.model_kind().to_owned());
    }
    let final_snapshot=service.snapshot(&id).unwrap();
    let result:ModelingSection=serde_json::from_value(
        final_snapshot.section("bellatrix.bellatrix-mde-modeling").unwrap().clone()
    ).unwrap();
    assert_eq!(final_snapshot.revision(),24);
    for kind in connector_ids {
        let relation=result.relations.iter().find(|r|r.kind==kind).unwrap();
        assert!(result.diagrams.iter().flat_map(|d| &d.views)
            .any(|v| v.subject.as_deref()==Some(&relation.id)
                && matches!(v.geometry,Geometry::Path{..})));
    }
    // Deleting a connector must not cascade through its endpoint classes.
    let target=result.relations.iter().find(|r|r.kind=="uml.aggregation").unwrap().id.clone();
    assert!(matches!(
        cx.query::<ModelingDeleteElement>(Routing::Unscoped,ModelingDeleteElementRequested {
            project_id:id.clone(),element_id:target,expected_revision:revision,
        }).await.unwrap(),QueryOutcome::Answer(Ok(25))
    ));
    let final_doc=service.snapshot(&id).unwrap();
    let section:ModelingSection=serde_json::from_value(
        final_doc.section("bellatrix.bellatrix-mde-modeling").unwrap().clone()
    ).unwrap();
    assert!(section.elements.iter().any(|e|e.id==class));
    assert!(section.elements.iter().any(|e|e.id==interface));
    plugin.dispose().await.unwrap();
    provider.dispose().await.unwrap();
}
