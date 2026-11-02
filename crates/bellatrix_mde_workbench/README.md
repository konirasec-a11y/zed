# Engine-MDE — Zed-native Workbench

This crate now implements a Zed Workspace Item plus Zed Dock Panels **inside the
Zed fork**, using Zed's native workspace dependencies. The UI
does not import GPUI Kit and does not create a second Dock or event bus.

## What is implemented

- Zed `workspace::item::Item` with view selection and diagram switching.
- GPUI mouse movement preview, move and rectangle resize on mouse release.
- Zed Toolbox with click-to-select and native GPUI drag-and-drop onto the Canvas.
- Zed Dock Panels for Working Diagrams/Toolbox, Explorer/Inspector and bottom
  auxiliary tabs. Left/right Panels internally compose the simultaneous stacks
  required by the React v3 design; Zed owns placement and focus.
- Class/Interface creation, rename, move, resize and save requests cross the
  typed `MdeUiBridge` (owned by the host) into Cordis responders and the
  versioned `ProjectService`. All persistent changes go through MDE validation.
- Project snapshots observe existing and legacy namespaces without rewriting
  saved projects, and reject stale edits.
- Dark and Light semantic colors follow `docs/design/src/styles/tokens.css`
  and the current Zed theme appearance.

## Application integration contract

The Zed-based Bellatrix launcher, after installing its Zed services
and Cordis Project/MDE plugins, calls:

```rust
// On the Tokio host thread, while Project/MDE plugins are active:
let bridge = bellatrix_mde_cordis::MdeUiBridge::start(&context, project_id)?;
let source = bellatrix_mde_workbench::MdeUiSource::from_bridge(
    projects.clone(), &bridge,
);
// Keep the bridge handle alive until after the window is closed.

// During the Zed GPUI App initialization, after editor::init:
bellatrix_mde_workbench::register_mde_workspace(cx, source);
```

This registers the workbench in each newly created Zed Workspace via
`App::observe_new`, adds it as an Item and registers the panels with
`Workspace::add_panel`. `editor::init` is required first because Zed
`ui_input::InputField` uses its editor factory.

**The Zed-based launcher is wired to this hook** through the reproducible
`scripts/enable_zed_desktop.py` overlay and `apps/zed-host`.
Do not claim the native UI has passed end-to-end acceptance until the
Windows build and interaction tests complete.

## Reference and design contract

- React: `docs/design/src/styles/tokens.css`,
  `docs/design/docs/{DESIGN_SYSTEM,GPUI_PORT_MAP,UX_ACCEPTANCE}.md`.
- StarUML: `docs/ref/staruml/src/ui/diagram-editor.js`, **behavioral
  reference only**; copyright notices prohibit copying source.
- Zed: `crates/workspace/src/{item,dock,workspace}.rs`,
  `crates/ui`, `crates/ui_input` and
  `crates/gpui/examples/drag_drop.rs`.
- Composition: Cordis-rs is the only backend lifecycle and public-event
  infrastructure. UI subscription state is ephemeral.

## Windows validation

```powershell
rustup toolchain install 1.98.1 --profile minimal --component rustfmt,clippy
cargo +1.98.1 check -p bellatrix-mde-cordis
cargo +1.98.1 test -p bellatrix-mde-cordis
cargo +1.98.1 check -p bellatrix-mde-workbench
cargo +1.98.1 test -p bellatrix-mde-workbench
cargo +1.98.1 clippy -p bellatrix-mde-workbench --all-targets -- -D warnings
```

The new UI packages are GPL-3.0-or-later as indicated in Zed's Cargo
manifests. Review distribution obligations before release.
