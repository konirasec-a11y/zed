//! Source-contract checks; these deliberately do not claim GPUI runtime coverage.
//! Can run independently with `rustc --test` without linking the GUI workspace.

const INTEGRATION: &str = include_str!("../src/integration.rs");

fn workbench_action() -> (&'static str, &'static str, &'static str) {
    let start = INTEGRATION
        .find("workspace.register_action(move |workspace, _: &OpenMdeWorkbench, window, cx| {")
        .expect("explicit workbench action must remain registered");
    let end = start
        + INTEGRATION[start..]
            .find("\n            });")
            .expect("workbench action terminator");
    (
        &INTEGRATION[..start],
        &INTEGRATION[start..end],
        &INTEGRATION[end..],
    )
}

#[test]
fn workspace_registration_does_not_mount_mde_or_defer_a_startup_mount() {
    let (before, _, after) = workbench_action();
    for registration in [before, after] {
        assert!(!registration.contains("open_in_workspace(workspace,"));
        assert!(!registration.contains("panels::attach_panels("));
        assert!(!registration.contains("defer_in("));
        assert!(!registration.contains("dispatch_action("));
    }
    assert!(!INTEGRATION.contains("register_project_item"));
}

#[test]
fn explicit_action_opens_once_and_reuses_existing_or_closed_item() {
    let (_, action, _) = workbench_action();
    let existing = action
        .find("workspace.item_of_type::<MdeWorkbenchItem>(cx)")
        .expect("existing item lookup");
    let cached = action
        .find("action_item.borrow().clone()")
        .expect("closed item lookup");
    let create = action
        .find("open_in_workspace(workspace,")
        .expect("explicit open");
    let attach = action
        .find("panels::attach_panels(")
        .expect("first-open panels");
    assert!(existing < cached && cached < create && create < attach);
    assert!(action[existing..cached].contains("workspace.activate_item("));
    assert!(action[existing..cached].contains("return;"));
    assert!(action[cached..create].contains("workspace.add_item_to_active_pane("));
    assert!(action[cached..create].contains("return;"));
    assert_eq!(action.matches("open_in_workspace(workspace,").count(), 1);
    assert_eq!(action.matches("panels::attach_panels(").count(), 1);
    assert!(action[attach..].contains("*action_item.borrow_mut() = Some(item)"));
}

#[test]
fn modeling_delete_bindings_remain_scoped_away_from_native_editor() {
    assert_eq!(
        INTEGRATION
            .matches("Some(\"BellatrixMde && !Editor\")")
            .count(),
        3
    );
}
