# Bellatrix MDE on Zed

**Bellatrix** is a multipurpose platform organized around spatial/temporal composability. Model-Driven Engineering is one bounded backend context of Bellatrix, rather than a new editor framework.

The Zed fork supplies the application lifecycle, Workspace, Dock, project worktrees, code editor and UI conventions. The MDE engine contributes its own model validation, diagram semantics, commands and generators through Cordis-rs. Its persistent ProjectDocument is not a replacement for Zed's `project::Project`.

## Crates and ownership

| Crate | Responsibility |
| --- | --- |
| `bellatrix_project` | Domain document storage, identities and revisions |
| `bellatrix_mde` | Single domain: elements, diagrams, UML semantics, commands, graph/index, validation and wire schema |
| `bellatrix_mde_cordis` | Typed Cordis events, Service injection and lifecycle |
| `bellatrix_mde_rust_generator` | Generated Rust artifacts |
| `bellatrix_mde_html_exporter` | Generated HTML/SVG documentation |
| `bellatrix_mde_cli` | Optional headless commands, not part of Zed's GUI shell |
| `bellatrix_mde_workbench` | Zed Workspace Item and Dock Panels |

The model fixtures and schema live at `crates/bellatrix_mde/{fixtures,schema}/`.

```powershell
cargo run -p bellatrix_mde_cli -- check crates/bellatrix_mde/fixtures/todo.json
cargo run -p bellatrix_mde_cli -- analyze crates/bellatrix_mde/fixtures/todo.json
cargo run -p bellatrix_mde_cli -- generate crates/bellatrix_mde/fixtures/todo.json --output generated-application
cargo run -p bellatrix_mde_cli -- export crates/bellatrix_mde/fixtures/todo.json --format html --output generated-docs
```

For an end-to-end backend smoke run, use `script/bellatrix_mde/smoke.ps1` on Windows.

## Beta format and composition

The canonical MDE section namespace is `bellatrix.mde`, defined once by `bellatrix_mde::MODELING_NAMESPACE`. Public MDE events use `bellatrix.mde.*.v1`. During beta, Bellatrix does not retain legacy namespace aliases or migrate old project envelopes: documents without the canonical section are rejected explicitly rather than read as an empty model. This policy does not change Zed formats or APIs.

The Cordis runtime provides public events, services, coeffects and lifecycle management; no second event bus or `EngineManager` belongs in Bellatrix. Unloading the engine must release runtime resources while preserving user-authored project data.

**Quick Edit contextual — prioridade da Engine-MDE:** [ações por tipo de elemento](quick-edit.md), incluindo nome, visibilidade, Add Attribute/Operation e demais ações de classe/atributo/operação. O Quick Edit não é substituído pelo Property Editor.\n\n**Inspector / Property Editor:** [contrato normativo ilustrado](inspector.md) — descritores de elementos base/especializados, navegação de atributos/operações e edição tipada pelo Cordis. O Property Editor não é o Project Panel do Zed.\n\nThe MDE Canvas is a Zed `workspace::Item`; its Toolbox, Inspector and related panes are Zed `workspace::dock::Panel` contributors. Use Zed's layout, shortcuts and UX patterns. StarUML remains a behavior study for MDE only; proprietary source must not be copied.
