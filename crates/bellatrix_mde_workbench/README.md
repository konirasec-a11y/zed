# Bellatrix MDE Workbench

`bellatrix_mde_workbench` contributes a native `workspace::Item` and Zed Dock `Panel` implementations to the application in this fork. Zed owns the editor, Workspace, GPUI lifecycle, focus, layout, themes and status bar. Bellatrix supplies model commands and transient Canvas interaction.

## Integration

The built-in `bellatrix_zed_host` is installed directly in `crates/zed/src/main.rs` after `initialize_workspace(app_state.clone(), cx)`. No separate GPUI runtime, third-party overlay or `scripts/enable_zed_desktop.py` is required.

The host owns the Cordis Project/MDE plugins and a `MdeUiBridge`. `register_mde_workspace` observes each new native Zed Workspace, opens the MDE `Item` and attaches its Dock `Panel` contributions once GPUI has finished constructing the Workspace. The `bellatrix_mde::OpenMdeWorkbench` action remains available to refocus the tab or reopen it after closing, without adding duplicate panels. The first-launch branch skips the upstream Zed onboarding window so the Bellatrix Workbench is visible. The bridge relays user operations as typed Cordis commands and observes model revision changes. The interim host still loads a single model from a file or demo fixture; adopting Zed `ProjectPath`/BufferStore and `SerializableItem` is the next step, not yet complete.

Implemented operation paths include Class/Interface creation, rename, view move/resize, project save and a drag-and-drop Toolbox. These use the MDE graph validation and versioned `bellatrix_project` service, rather than writing model data in the GUI.

## UI conventions

Follow the Zed Workspace and Editor UX. Use existing Zed UI components and docks. Do not implement a second Dock manager, another theme system or a parallel project tree. Bellatrix presentation components in `crates/bellatrix_ui_*` provide MDE-specific symbols and diagrams without owning backend state.

## Validation

```powershell
cargo check -p bellatrix_mde_workbench -p bellatrix_zed_host
cargo test -p bellatrix_mde_cordis
cargo clippy -p bellatrix_mde_workbench --all-targets -- -D warnings
cargo check -p zed
cargo run -p zed
```

The native code must still pass Windows build and interaction tests before the new Workbench is considered fully validated.
