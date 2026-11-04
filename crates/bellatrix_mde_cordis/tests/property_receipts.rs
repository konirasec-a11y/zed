#[path = "support/events.rs"]
mod events;
use bellatrix_mde_cordis::*;
use bellatrix_project::*;
use cordis::{Context, Plugin, PreparedPlugin, Routing, event::QueryOutcome};
use serde_json::Value;
use std::sync::Arc;

type Receipt = tokio::sync::watch::Receiver<Option<Result<u64, String>>>;

async fn receipt_result(
    mut receipt: Receipt,
) -> Result<Result<u64, String>, Box<dyn std::error::Error>> {
    if receipt.borrow().is_none() {
        tokio::time::timeout(std::time::Duration::from_secs(5), receipt.changed()).await??;
    }
    receipt
        .borrow()
        .clone()
        .ok_or_else(|| "receipt missing".into())
}

#[tokio::test]
async fn property_receipts_are_targeted_revision_guarded_and_acknowledge_noops()
-> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new();
    let service = Arc::new(ProjectService::new());
    let provider = context
        .spawn(PreparedPlugin::from_input(
            ProjectPlugin::new(Arc::clone(&service)),
            Arc::clone(&service),
        ))
        .await?;
    let plugin = MdePlugin;
    plugin.prepare(())?;
    let plugin = context
        .spawn(PreparedPlugin::from_input(plugin, ()))
        .await?;
    let id = ProjectId::new("property-receipts")?;
    let mut model: Value =
        serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json"))?;
    model["id"] = id.as_str().into();
    model["name"] = "Properties".into();
    let document = events::prepare(&context, &model).await?;
    assert!(matches!(
        context
            .query::<ProjectCreate>(Routing::Unscoped, ProjectCreateRequested { document })
            .await?,
        QueryOutcome::Answer(Ok(_))
    ));
    let first = model["elements"]
        .as_array()
        .and_then(|elements| elements.first())
        .and_then(|element| element["id"].as_str())
        .ok_or("first element missing")?;
    let second = model["elements"]
        .as_array()
        .and_then(|elements| elements.get(1))
        .and_then(|element| element["id"].as_str())
        .ok_or("second element missing")?;
    let bridge = MdeUiBridge::start(&context, id.clone())?;
    let submit =
        |element: &str, value: &str, revision| -> Result<Receipt, Box<dyn std::error::Error>> {
            let (completion, receipt) = tokio::sync::watch::channel(None);
            bridge.sender().try_send(MdeUiCommand::SetPropertyTracked {
                request: ModelingSetPropertyRequested {
                    project_id: id.clone(),
                    element_id: element.into(),
                    path: "name".into(),
                    value: value.into(),
                    expected_revision: revision,
                },
                completion,
            })?;
            Ok(receipt)
        };
    assert_eq!(
        receipt_result(submit(first, "First edited", 0)?).await?,
        Ok(1)
    );
    assert_eq!(
        receipt_result(submit(first, "First edited", 1)?).await?,
        Ok(1)
    );
    assert!(receipt_result(submit(second, "stale", 0)?).await?.is_err());
    assert_eq!(service.snapshot(&id)?.revision(), 1);
    // Closing or changing the selected inspector must not redirect an already submitted edit.
    drop(submit(first, "First pending", 1)?);
    assert_eq!(
        receipt_result(submit(second, "Second edited", 2)?).await?,
        Ok(3)
    );
    let section = events::projection(&context, &service, &id).await?.section;
    assert_eq!(
        section
            .elements
            .iter()
            .find(|element| element.id == first)
            .ok_or("first missing")?
            .name,
        "First pending"
    );
    assert_eq!(
        section
            .elements
            .iter()
            .find(|element| element.id == second)
            .ok_or("second missing")?
            .name,
        "Second edited"
    );
    assert!(
        receipt_result(submit("deleted-or-missing", "No target", 3)?)
            .await?
            .is_err()
    );
    assert_eq!(service.snapshot(&id)?.revision(), 3);
    drop(bridge);
    plugin.dispose().await?;
    provider.dispose().await?;
    Ok(())
}
