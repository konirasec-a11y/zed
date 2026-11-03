# Bellatrix MDE Workbench

`bellatrix_mde_workbench` contributes a native `workspace::Item` and Zed Dock `Panel` implementations to the application in this fork. Zed owns the editor, Workspace, GPUI lifecycle, focus, layout, themes and status bar. Bellatrix supplies model commands and transient Canvas interaction.

## Integration

The built-in `bellatrix_zed_host` is installed directly in `crates/zed/src/main.rs` after `initialize_workspace(app_state.clone(), cx)`. No separate GPUI runtime, third-party overlay or `scripts/enable_zed_desktop.py` is required.

The host owns the Cordis Project/MDE plugins and a `MdeUiBridge`. Every Workspace has a visible **MDE** text button in the native status bar by default. Click it to open the MDE Workbench and its panels, or invoke `bellatrix_mde::OpenMdeWorkbench` from the command palette. Repeating the action focuses the existing tab; reopening a closed tab reuses its Item without attaching duplicate panels. Registering the button does not open a tab or change focus in an ordinary repository.

Setting `BELLATRIX_PROJECT_FILE` or `BELLATRIX_MODEL_FILE` before launching explicitly selects a startup source and automatically opens its MDE Workbench. With neither variable set, the button opens the host's existing demo source. It does **not** automatically load the JSON file selected in the native editor. There is no empty-workspace or restored-session auto-open heuristic. To hide only this status-bar entry, set `BELLATRIX_MDE_STATUS_BUTTON=false` before launch; the default is visible, and the command-palette action remains available. This Bellatrix-specific startup option does not change native editor/status-bar settings.

The adjacent **Open project** dropdown offers **Open MDE project (read-only preview)**. It uses the native file dialog, Project/worktree and shared text Buffer, validates the selected Bellatrix document, and renders its captured model through the existing Workbench. The preview has its own diagram chooser and pure validation control; selection and pan/zoom remain available. It does not enable model editing or native MDE save. Changes to the source text, file identity, dirty/conflict state or imported semantic snapshot mark the preview stale without overwriting source text. Close and reopen after resolving source changes. A document ID already owned by a configured/demo source is rejected explicitly, without substituting the fixture or cloning its ID.

While a preview is the active Item, existing editing panels show a notice naming that preview and the previous Workbench instead of showing the wrong model's controls. Their drafts are retained, and their mutation sources are suspended only in that Workspace. Returning to an ordinary Item restores the previous context; the primary **MDE** button intentionally returns to the configured/demo Workbench. This is a bounded preview path, not full active-document panel rebinding or editable persistence.

The bridge relays typed Cordis commands and observes model revisions. The interim host still supplies one configured model or demo fixture; per-document `ProjectPath`/Buffer integration, native save acknowledgement and `SerializableItem` restoration remain incomplete. The visible-entry recovery does not claim those later integration gates passed.

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

Prefer a few feature-level behavior scenarios that exercise complete flows across
public module boundaries, rather than a test for each helper or method. Assert
observable results and preserved state so internal refactors do not require
rewriting the scenarios. The focused `property_receipts` scenario covers editing,
no-op acknowledgement, stale-revision rejection and independent targets through
the public Cordis bridge. Extend a relevant flow for a concrete regression;
do not grow a large suite of overlapping implementation tests.

## Existing-design frontend integration

The visual source is `../bellatrix_design_system/docs/design/src/components/panels/`,
with reference captures in that package's `preview/` directory. Its React controls
are layout specimens, not permission to introduce unsupported model operations.

The native workbench uses compact rows of native Zed controls and
`bellatrix_ui_panels::PaneHeader` for panel disclosure/actions. Inspector text,
number and documentation edits are transient drafts keyed by project, element
and property, not another model store. Changed fields show **Not applied** with
explicit **Apply** and **Cancel** controls. Cancel reloads the current canonical
value. Clean fields follow model updates; a changed canonical value preserves a
dirty draft, shows **Conflict**, and requires **Reload** before applying again.

Apply captures the target and revision and submits the existing validated MDE
command. **Applying…** remains visible until its typed receipt and corresponding
snapshot arrive. Success, including a no-op, acknowledges only the submitted
edit; later typing is retained. Rejection keeps the draft and displays its error.
Changing selection cannot redirect a captured edit to the new selection.
Boolean and choice controls submit immediately through the same target and
revision guard, with failures reported as feedback. ID and type remain read-only.
**Name** is the canonical diagram name; **Label metadata (not diagram name)**
remains stored and editable under Advanced. Documentation and less-common
properties stay reachable in their sections; opening the Inspector does not
mutate the model.

The toolbox presents supported quick tools, searchable categories and existing
symbol glyphs. Collapse state belongs to the panel, not the semantic model.
Zed still owns dock widths and persisted placement; this increment does not
reset a user's saved workspace layout or introduce a separate docking system.

Native acceptance remains required for this increment:

- Compare light/dark and compact/wide windows with the existing design captures.
- Collapse and reopen every panel section; retain selection and split sizes.
- Search tools, clear the query, select a tool and create through the canvas.
- Select classes, members and relations from the canvas and Model Explorer.
- Edit and apply text, toggle booleans and choose enums; inspect revision feedback.
- Open Documentation/Advanced and ensure all schema properties are reachable.
- Confirm full figure bounds, context menus, resize and toolbar zoom still work.

Pure-helper tests, Cargo metadata and source checks do not establish native GPUI
compilation or successful interaction. Keep that distinction in release reports.

### View styles and routing

Select a concrete canvas View to edit Styles. Fill, stroke, text color, font,
font size, stroke width and opacity use the existing View style contract; blank
fields inherit the theme. Apply uses the revision captured with the draft.
Conflicting revisions preserve the draft and require explicit reload instead of
silently overwriting a newer style. Selecting only a semantic model element does
not choose an arbitrary representation to modify.

Relationship Views offer Straight, Automatic and Curve routing. Curve authors a
cubic path and moving an endpoint reanchors that representation without changing
other Views of the same semantic relation. Orthogonal/rounded routing and direct
control-point dragging remain pending. Straight self-relations retain a visible
loop. UML arrowheads and relationship dashing remain derived from semantics.

Documentation uses the native multiline editor; semantic properties still use
the existing Inspector command boundary. These controls require the native
acceptance scenarios above, independently of headless command tests.
