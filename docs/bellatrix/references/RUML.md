# RUML: reutilização adaptada, sem runtime paralelo

Origem: https://github.com/javapak/ruml-wasm

Snapshot de referência: `5c90d7399acbce4c37847b272759ed85d677ba92`.

`ruml-wasm/` conserva, sem alterações, o lote parcial de fontes que já existia em `crates/bellatrix_uml_metamodel/upstream`. Não é uma cópia completa do repositório upstream. Está fora do workspace, não é dependência, não é compilado e não é fonte de estado da Engine MDE. O manifesto original está arquivado ao lado exclusivamente para rastreabilidade; sua contagem planejada de tipos não comprova a cobertura implementada.

As declarações DTO importadas, sem consumidores no código ativo, foram removidas de `crates/bellatrix_mde/src/uml`. O estado do domínio permanece no grafo canônico `Element`/`Graph`; o catálogo `uml::metamodel`, gerado a partir do XMI oficial, fornece a hierarquia de metaclasses usada pelas regras. Isso não constitui tipagem UML completa nem certificação de conformidade. Não há geração automática durante o build, inclusão de fonte upstream, registry global JavaScript ou runtime WASM. O `Cargo.toml` upstream declara MIT; as fontes de consulta e seus avisos de origem foram preservados.

A referência normativa para o desenvolvimento é a especificação UML 2.5.1 da OMG e seus documentos XMI. Os arquivos RUML são referência de implementação e não comprovação de conformidade. Os quatro XMI existentes permanecem em `docs/bellatrix/standards/uml/20161101/`.

Useful missing structural contracts are inventoried in [structural-contracts-to-reuse.md](structural-contracts-to-reuse.md), with official-source and Git provenance for the next Object/Deployment work. They are not a second runtime model.

## Reutilização executável: proprietário de Slot

`crates/bellatrix_mde/src/uml/instances.rs` e o caminho existente
`Command::SetOwner` → `uml::root::reparent` adaptam a transição de
[`Slot::set_owning_instance`, linhas 167–188](https://github.com/javapak/ruml-wasm/blob/5c90d7399acbce4c37847b272759ed85d677ba92/slot.rs#L167-L188).
O algoritmo upstream remove a referência do proprietário anterior, atribui o novo
proprietário e insere a referência inversa sem duplicação. Na adaptação, uma única
aresta `owns` realiza essa transição; `InstanceSpecificationRef::slots` e
`SlotRef::owning_instance` consultam a mesma aresta. Não existem listas inversas
persistidas, DTOs duplicados ou registry WASM. As consultas são consumidas pela
validação de comandos e do modelo, não apenas mantidas como documentação.

O upstream declara licença **MIT** no `Cargo.toml` preservado em `ruml-wasm/`.
O arquivo original `slot.rs` nesse commit contém os avisos “Slot - Generated UML
Class” e “Uses unified registry with type filtering”, mas não contém aviso de
copyright ou texto adicional de licença; o repositório fixado não contém um
arquivo LICENSE. O código adaptado identifica origem, função, commit e licença
declarada. O gerador citado nos arquivos upstream não faz parte desse snapshot.

O contrato normativo foi conferido em `UML.xmi`: `InstanceSpecification-slot`,
`Slot-owningInstance`, `Slot-definingFeature`,
`InstanceSpecification-structural_feature`,
`InstanceSpecification-defining_feature` e `Classifier-allSlottableFeatures`.
As seções correspondentes do PDF UML 2.5.1 são 9.9.9, 9.9.20 e 9.9.4.7.

Este incremento suporta a atribuição e movimentação de um Slot existente com
`properties.definingFeature` como referência `{ "$ref": "id" }` de uma Property
diretamente possuída por um dos Classifiers referenciados no array
`properties.classifier` da instância. A representação e a resolução usam o
mesmo `uml::references::ReferenceIndex` das demais referências canônicas.
Rejeita proprietário ausente ou incompatível, referência inválida, duplicação de
definingFeature e armazenamento paralelo de `slot`/`owningInstance`.
Falhas não alteram o projeto de entrada nem seu proprietário anterior; repetir a
mesma atribuição é idempotente. O teste público em `tests/graph_rules.rs` percorre
comandos, consultas e serialização JSON/Protobuf.

Classifiers com generalizações, propriedades redefinidas e features herdadas,
importadas ou memberEnds de associações ainda exigem resolução adicional. São
reportados como capacidade não suportada, não como proibições da UML. Não foram
transplantados os erros silenciosos nem os métodos remove/clear inconsistentes
do upstream. Este incremento não implementa valores de Slots, todo
`allSlottableFeatures`, criação visual de instâncias ou suporte completo ao
diagrama de objetos. Referências de classifiers são validadas mesmo em instâncias
sem Slots; `specification` e valores de Slots populados continuam produzindo
diagnósticos explícitos de capacidade não suportada.
