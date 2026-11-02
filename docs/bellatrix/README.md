# Bellatrix on Zed

Bellatrix is implemented directly within the **Zed fork**. Zed owns the GPUI runtime, editor, workspace, docking, layouts, project worktrees and the `zed.messages` protocol. Bellatrix extends the Zed application with its own bounded backend domains.

**Engine-MDE Inspector:** [especificação com diagramas e critérios mínimos](inspector.md). O Inspector reflete o elemento selecionado; o Property Editor constrói controles tipados e o Project Panel do Zed continua responsável por arquivos.\n\n## Runtime boundaries

- `crates/bellatrix_project`: ProjectDocument, identities, revision control and persisted Bellatrix domain sections. It **does not replace** the Zed `project` crate.
- `crates/bellatrix_mde_{model,modeling,cordis,rust_generator,html_exporter,cli}`: headless Model-Driven Engineering domain and its typed Cordis events/services.
- `crates/bellatrix_mde_workbench`: native Zed Workspace Item, Dock Panels, Canvas, Toolbox, Inspector, and a typed bridge to MDE; opened on demand through the `bellatrix_mde::OpenMdeWorkbench` action.
- `crates/bellatrix_zed_host`: installs Project/MDE Cordis plugins inside the existing Zed initialization; no separate GUI event loop.
- `crates/bellatrix_design_system` and `crates/bellatrix_ui_*`: presentation and symbol vocabulary on Zed's pinned GPUI.
- `crates/bellatrix_mde_model/fixtures` and `schema`: MDE fixtures and its versioned wire format, separate from Zed RPC/proto.

See [Zed protocol and persistence reuse audit](protocol-and-persistence.md) for the exact separation between Zed's Protobuf RPC, GPUI local events, SQLite Workspace/Item state and Cordis domain events. In particular, MDE Items are not yet registered with Zed's `SerializableItem` restoration lifecycle; this is the first integration gap to close.

Do not build an alternate `EngineManager`, EventBus, Zed Project manager, or Dock. Cordis controls backend dependency/lifecycle composition and events; Zed controls visual/document editing. Unloading an MDE plugin must not delete persisted project state.

## Running

Use Zed's normal Rust toolchain and workflow from the fork root:

```powershell
cargo check -p bellatrix_mde_cordis
cargo test -p bellatrix_mde_cordis
cargo check -p bellatrix_mde_workbench
cargo check -p zed
cargo run -p zed
```

Use Zed's command palette action `bellatrix_mde::OpenMdeWorkbench` to open or refocus the MDE Workbench in the current Zed Workspace. It no longer inserts a tab or Dock panels into unrelated windows automatically. Set `BELLATRIX_PROJECT_FILE` to load a persisted Bellatrix project or `BELLATRIX_MODEL_FILE` for an MDE JSON model. If neither is supplied, the MDE host uses `crates/bellatrix_mde_model/fixtures/todo.json`. Zed's own Workspace and project flows remain available.

## Compatibility

The Bellatrix MDE model and its existing persisted namespaces remain independent of the names of the source directories. The code originates from the Bellatrix main repository at commit `f1362fd6ee45e894428dda58a56e08e24fa8949a`. This migration does not automatically convert saved projects into Zed worktrees or Zed's collaboration protocol. The current demo-backed MDE host still initializes a standalone domain document at startup; the next integration is to open an MDE document from an actual Zed `ProjectPath`/`Buffer` and then register `SerializableItem` restoration.

Licensing and build: Zed crates use GPL-3.0-or-later (some GPUI subcrates are Apache-2.0). Before redistribution, verify license notices and Cordis's MIT obligations. Validate on Windows, including the C++ Spectre-mitigated MSVC libraries needed by Zed.
