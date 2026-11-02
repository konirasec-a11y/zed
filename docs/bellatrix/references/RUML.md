# RUML: consulta, não dependência

Origem: https://github.com/javapak/ruml-wasm

Snapshot de referência: `5c90d7399acbce4c37847b272759ed85d677ba92`.

`ruml-wasm/` conserva, sem alterações, o lote parcial de fontes que já existia em `crates/bellatrix_uml_metamodel/upstream`. Não é uma cópia completa do repositório upstream. Está fora do workspace, não é dependência, não é compilado e não é fonte de estado da Engine MDE. O manifesto original está arquivado ao lado exclusivamente para rastreabilidade; sua contagem planejada de tipos não comprova a cobertura implementada.

As declarações nativas aproveitadas são mantidas em `crates/bellatrix_mde/src/uml`. Não há geração automática durante o build, inclusão de fonte upstream, registry global JavaScript ou runtime WASM. O `Cargo.toml` upstream declara MIT; os avisos de origem foram preservados.

A referência normativa para o desenvolvimento é a especificação UML 2.5.1 da OMG e seus documentos XMI. Os arquivos RUML são referência de implementação e não comprovação de conformidade. Os quatro XMI existentes permanecem em `docs/bellatrix/standards/uml/20161101/`.
