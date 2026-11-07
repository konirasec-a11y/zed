# Bellatrix — plataforma componível sobre Zed e Cordis-rs

**Plano mestre de execução MDE:** [MDE_IMPLEMENTATION_PLAN.md](MDE_IMPLEMENTATION_PLAN.md).
Os documentos técnicos abaixo detalham esse plano, sem substituí-lo.

**Gerador M2T com Tera 2.4.0:** [TERA_GENERATION_PIPELINE.md](TERA_GENERATION_PIPELINE.md).

**Metamodelos e transformação de grafos (M2M→M2T):**
[MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md](MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md).
**Contrato único das relações UML:** [GRAPH_RELATIONSHIPS.md](GRAPH_RELATIONSHIPS.md).

**Fonte de verdade para missão, composibilidade, SDK e referências de design:** [PLATFORM_VISION.md](PLATFORM_VISION.md). Bellatrix é uma plataforma generalista para DevSecOps, computação científica e aplicações criadas pelos usuários; MDE é uma engine core, não a plataforma.

Bellatrix is implemented directly within the **Zed fork**. Zed owns the GPUI runtime, editor, workspace, docking, layouts, project worktrees and the `zed.messages` protocol. Bellatrix extends the Zed application with its own bounded backend domains.

**Quick Edit contextual:** [opções de cada elemento e estado de implementação](quick-edit.md) — contrato prioritário derivado da referência HTML/JSON StarUML.

**Engine-MDE Inspector:** [especificação com diagramas e critérios mínimos](inspector.md). O Inspector reflete o elemento selecionado; o Property Editor constrói controles tipados e o Project Panel do Zed continua responsável por arquivos.

## Runtime boundaries

Bellatrix has four crates. Earlier plans and migration reports may describe
removed crates; these boundaries describe the current layout.

- `crates/bellatrix_mde`: headless MDE models, contracts and protocol types in
  `src/contracts`, persistence codecs, Rust generation and HTML export.
  Public APIs include `bellatrix_mde::contracts`,
  `bellatrix_mde::rust_renderer` and `bellatrix_mde::html_renderer`.
  Tera templates are owned by the engine's generation implementation.
- `crates/bellatrix_mde_cordis`: typed Cordis events/services and project
  document lifecycle in `bellatrix_mde_cordis::project`. It does not replace
  Zed's `project` crate.
- `crates/bellatrix_mde_ui`: native Workspace Item, Dock Panels, Canvas,
  Toolbox, Inspector and shared presentation on Zed's pinned GPUI. The action
  `bellatrix_mde::OpenMdeWorkbench` focuses or reopens the workbench.
- `crates/bellatrix_zed_host`: installs Project/MDE Cordis plugins during
  existing Zed initialization, without a separate GUI event loop.

The former standalone CLI, contracts, protocol, templates, generator, exporter
and project crates are removed. They are not compatibility packages. MDE
fixtures remain in `crates/bellatrix_mde/fixtures`.

See [Zed protocol and persistence reuse audit](protocol-and-persistence.md) for the exact separation between Zed's Protobuf RPC, GPUI local events, SQLite Workspace/Item state and Cordis domain events. In particular, MDE Items are not yet registered with Zed's `SerializableItem` restoration lifecycle; this is the first integration gap to close.

Do not build an alternate `EngineManager`, EventBus, Zed Project manager, or Dock. Cordis controls backend dependency/lifecycle composition and events; Zed controls visual/document editing. Unloading an MDE plugin must not delete persisted project state.

## Running

Commit and push before running checks, builds or tests. Then use Zed's normal
Rust toolchain from the fork root. The selected tests are existing BDD
integration targets, not unit tests:

```powershell
cargo check -p bellatrix_mde -p bellatrix_mde_cordis -p bellatrix_mde_ui -p bellatrix_zed_host
./script/bellatrix_mde/smoke.ps1
cargo check -p zed
cargo run -p zed
```

O Zed registra o Workbench MDE como Item e contribuições de Dock nativos; a ação `bellatrix_mde::OpenMdeWorkbench` o reabre ou foca. Sua inicialização é parte da instalação de capacidades do fork, não um segundo runtime de GUI. Set `BELLATRIX_PROJECT_FILE` to load a persisted Bellatrix project or `BELLATRIX_MODEL_FILE` for an MDE JSON model. If neither is supplied, the MDE host uses `crates/bellatrix_mde/fixtures/todo.json`. Zed's own Workspace and project flows remain available.

## Beta format policy

Bellatrix beta uses only the canonical `bellatrix.mde` document section and `bellatrix.mde.*.v1` public events. Obsolete namespaces are rejected, not migrated or silently replaced; no compatibility aliases are retained. Zed native formats and APIs are unaffected. See the [current MDE contract](mde.md); historical migration reports describe earlier checkpoints and do not override this policy. The code originates from the Bellatrix main repository at commit `f1362fd6ee45e894428dda58a56e08e24fa8949a`. This migration does not automatically convert saved projects into Zed worktrees or Zed's collaboration protocol. The current demo-backed MDE host still initializes a standalone domain document at startup; the next integration is to open an MDE document from an actual Zed `ProjectPath`/`Buffer` and then register `SerializableItem` restoration.

Licensing and build: Zed crates use GPL-3.0-or-later (some GPUI subcrates are Apache-2.0). Before redistribution, verify license notices and Cordis's MIT obligations. Validate on Windows, including the C++ Spectre-mitigated MSVC libraries needed by Zed.
