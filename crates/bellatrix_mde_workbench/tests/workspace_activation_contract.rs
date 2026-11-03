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
fn ordinary_workspace_gets_an_entry_without_an_imposed_mde_tab() {
    let (before, _, after) = workbench_action();
    assert!(!before.contains("open_in_workspace(workspace,"));
    assert!(!before.contains("panels::attach_panels("));
    let explicit_startup = after
        .find("if auto_open_configured_source {")
        .expect("explicit startup gate");
    assert!(!after[..explicit_startup].contains("open_in_workspace(workspace,"));
    assert!(!after[..explicit_startup].contains("panels::attach_panels("));
    assert!(!after[..explicit_startup].contains("dispatch_action("));
    assert!(before.contains("std::env::var_os(\"BELLATRIX_PROJECT_FILE\").is_some()"));
    assert!(before.contains("std::env::var_os(\"BELLATRIX_MODEL_FILE\").is_some()"));
    assert!(!INTEGRATION.contains("register_project_item"));
    assert!(!INTEGRATION.contains("worktrees("));
}

#[test]
fn status_entry_is_deduplicated_and_uses_visible_text_and_existing_action() {
    let (_, _, after) = workbench_action();
    let gate = after
        .find("if show_status_button {")
        .expect("button visibility gate");
    let dedup = after
        .find("status_bar.item_of_type::<MdeStatusEntry>().is_none()")
        .expect("button deduplication");
    let add = after
        .find("status_bar.add_left_item(button, window, cx)")
        .expect("native status entry");
    assert!(gate < dedup && dedup < add);
    let status = include_str!("../src/status_entry.rs");
    assert!(status.contains("Button::new(\"bellatrix-mde-status-entry\", \"MDE\")"));
    assert!(status.contains("Tooltip::for_action(\"Open MDE Workbench\", &OpenMdeWorkbench, cx)"));
    assert!(status.contains("window.dispatch_action(Box::new(OpenMdeWorkbench), cx)"));
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
