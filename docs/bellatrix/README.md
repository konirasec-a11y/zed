# Bellatrix — plataforma componível sobre Zed e Cordis-rs

**Plano mestre de execução MDE:** [MDE_IMPLEMENTATION_PLAN.md](MDE_IMPLEMENTATION_PLAN.md).
Os documentos técnicos abaixo detalham esse plano, sem substituí-lo.

**Gerador M2T com Tera 2.4.0:** [TERA_GENERATION_PIPELINE.md](TERA_GENERATION_PIPELINE.md).

**Metamodelos e transformação de grafos (M2M→M2T):**
[MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md](MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md).
**Contrato único das relações UML:** [GRAPH_RELATIONSHIPS.md](GRAPH_RELATIONSHIPS.md).

**Fonte de verdade para missão, composibilidade, SDK e referências de design:** [PLATFORM_VISION.md](PLATFORM_VISION.md). Bellatrix é uma plataforma generalista para DevSecOps, computação científica e aplicações criadas pelos usuários; MDE é uma engine core, não a plataforma.

Bellatrix is implemented directly within the **Zed fork**. Zed owns the GPUI runtime, editor, workspace, docking, layouts, project worktrees and the `zed.messages` protocol. Bellatrix extends the Zed application with its own bounded backend domains.

**Quick Edit contextual:** [opções de cada elemento e estado de implementação](quick-edit.md) — contrato prioritário derivado da referência HTML/JSON StarUML.\n\n**Engine-MDE Inspector:** [especificação com diagramas e critérios mínimos](inspector.md). O Inspector reflete o elemento selecionado; o Property Editor constrói controles tipados e o Project Panel do Zed continua responsável por arquivos.\n\n## Runtime boundaries

- `crates/bellatrix_project`: ProjectDocument, identities, revision control and persisted Bellatrix domain sections. It **does not replace** the Zed `project` crate.
- `crates/bellatrix_mde` and `crates/bellatrix_mde_{cordis,rust_generator,html_exporter,cli}`: headless Model-Driven Engineering domain and its typed Cordis events/services.
- `crates/bellatrix_mde_workbench`: native Zed Workspace Item, Dock Panels, Canvas, Toolbox, Inspector, and a typed bridge to MDE; shown as a native Workspace Item with a focus/reopen action (`bellatrix_mde::OpenMdeWorkbench`).
- `crates/bellatrix_zed_host`: installs Project/MDE Cordis plugins inside the existing Zed initialization; no separate GUI event loop.
- `crates/bellatrix_design_system` and `crates/bellatrix_ui_*`: presentation and symbol vocabulary on Zed's pinned GPUI.
- `crates/bellatrix_mde/fixtures`: MDE fixtures. The engine owns persistence codecs in `src/persistence`, separate from Zed RPC/proto.

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

O Zed registra o Workbench MDE como Item e contribuições de Dock nativos; a ação `bellatrix_mde::OpenMdeWorkbench` o reabre ou foca. Sua inicialização é parte da instalação de capacidades do fork, não um segundo runtime de GUI. Set `BELLATRIX_PROJECT_FILE` to load a persisted Bellatrix project or `BELLATRIX_MODEL_FILE` for an MDE JSON model. If neither is supplied, the MDE host uses `crates/bellatrix_mde/fixtures/todo.json`. Zed's own Workspace and project flows remain available.

## Beta format policy

Bellatrix beta uses only the canonical `bellatrix.mde` document section and `bellatrix.mde.*.v1` public events. Obsolete namespaces are rejected, not migrated or silently replaced; no compatibility aliases are retained. Zed native formats and APIs are unaffected. See the [current MDE contract](mde.md); historical migration reports describe earlier checkpoints and do not override this policy. The code originates from the Bellatrix main repository at commit `f1362fd6ee45e894428dda58a56e08e24fa8949a`. This migration does not automatically convert saved projects into Zed worktrees or Zed's collaboration protocol. The current demo-backed MDE host still initializes a standalone domain document at startup; the next integration is to open an MDE document from an actual Zed `ProjectPath`/`Buffer` and then register `SerializableItem` restoration.

Licensing and build: Zed crates use GPL-3.0-or-later (some GPUI subcrates are Apache-2.0). Before redistribution, verify license notices and Cordis's MIT obligations. Validate on Windows, including the C++ Spectre-mitigated MSVC libraries needed by Zed.
