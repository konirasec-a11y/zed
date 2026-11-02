# RUML: consulta, não dependência

Origem: https://github.com/javapak/ruml-wasm

Snapshot de referência: `5c90d7399acbce4c37847b272759ed85d677ba92`.

`ruml-wasm/` conserva, sem alterações, o lote parcial de fontes que já existia em `crates/bellatrix_uml_metamodel/upstream`. Não é uma cópia completa do repositório upstream. Está fora do workspace, não é dependência, não é compilado e não é fonte de estado da Engine MDE. O manifesto original está arquivado ao lado exclusivamente para rastreabilidade; sua contagem planejada de tipos não comprova a cobertura implementada.

As declarações DTO importadas, sem consumidores no código ativo, foram removidas de `crates/bellatrix_mde/src/uml`. O estado do domínio permanece no grafo canônico `Element`/`Graph`; o catálogo `uml::metamodel`, gerado a partir do XMI oficial, fornece a hierarquia de metaclasses usada pelas regras. Isso não constitui tipagem UML completa nem certificação de conformidade. Não há geração automática durante o build, inclusão de fonte upstream, registry global JavaScript ou runtime WASM. O `Cargo.toml` upstream declara MIT; as fontes de consulta e seus avisos de origem foram preservados.

A referência normativa para o desenvolvimento é a especificação UML 2.5.1 da OMG e seus documentos XMI. Os arquivos RUML são referência de implementação e não comprovação de conformidade. Os quatro XMI existentes permanecem em `docs/bellatrix/standards/uml/20161101/`.

Useful missing structural contracts are inventoried in [structural-contracts-to-reuse.md](structural-contracts-to-reuse.md), with official-source and Git provenance for the next Object/Deployment work. They are not a second runtime model.
