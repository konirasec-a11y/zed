# Engine-MDE — criação de aplicações Low-Code/No-Code baseada em MDE

**Engine-MDE** é uma engine de criação de aplicações da plataforma Bellatrix.
Usa modelos semânticos, diagramas e geração de artefatos para permitir autoria
progressiva de aplicações, inclusive fluxos Low-Code/No-Code. **Modeling é uma
capacidade interna, não a identidade da engine.**

## Capacidade e responsabilidades

- `bellatrix-mde-model`: `Project`, `Element`, `Relation`, `Diagram`, `View` e suas representações.
- `bellatrix-mde-modeling`: grafo, validação, escopo, planejamento e publicação de artefatos.
- `bellatrix-mde-rust-generator`: geração de projetos e código Rust.
- `bellatrix-mde-html-exporter`: documentação HTML e SVG sem dependência de GPUI.
- `bellatrix-mde-cordis`: `Plugin`, `InjectSpec` e eventos tipados para integração ao backend Bellatrix.
- `bellatrix-mde-cli`: interface headless; a engine não depende do Desktop.
- `fixtures/`, `schema/`, `scripts/`: validação da engine.

`../../platform/bellatrix-project` pertence à plataforma; não à engine. O namespace
persistido `bellatrix.modeling` é preservado **por compatibilidade de documentos**.
Descarregar a engine remove contribuições do runtime Cordis, nunca o trabalho
persistido do usuário.

## CLI headless

```powershell
cargo run -p bellatrix-mde-cli -- check engines/mde/fixtures/todo.json
cargo run -p bellatrix-mde-cli -- analyze engines/mde/fixtures/todo.json
cargo run -p bellatrix-mde-cli -- generate engines/mde/fixtures/todo.json --output generated-application
cargo run -p bellatrix-mde-cli -- export engines/mde/fixtures/todo.json --format html --output generated-docs
```

Modelagem, geração, exportação e compilação são operações distintas. O modelo
pode ser parcial sem produzir implementações fictícias.

Os contratos arquiteturais e o glossário ubíquo estão em `../../ARCHITECTURE.md`.
As antigas especificações em `docs/history/specs/` são referências históricas,
não a ordem de implementação.

**Validação:** `scripts/verify-project.ps1` executa os gates do workspace.

## Engine de referência para o SDK

A Engine-MDE moderniza o StarUML como estudo de referência comportamental e amplia o seu papel com a criação de aplicações MDE Low-Code/No-Code. É o primeiro caso real para validar o futuro SDK Bellatrix: integração Cordis, eventos públicos, projetos e contribuições de interface. Os eventos do domínio são visíveis a qualquer engine interessada através do runtime Cordis; não existem buses privados por engine. Os nomes versionados atuais permanecem compatíveis até migração formal.
