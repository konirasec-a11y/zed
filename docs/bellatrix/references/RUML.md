# RUML: reutilização adaptada, sem runtime paralelo

Origem: https://github.com/javapak/ruml-wasm

Snapshot de referência: `5c90d7399acbce4c37847b272759ed85d677ba92`.

`ruml-wasm/` conserva, sem alterações, o lote parcial de fontes que já existia em `crates/bellatrix_uml_metamodel/upstream`. Não é uma cópia completa do repositório upstream. Está fora do workspace, não é dependência, não é compilado e não é fonte de estado da Engine MDE. O manifesto original está arquivado ao lado exclusivamente para rastreabilidade; sua contagem planejada de tipos não comprova a cobertura implementada.

As declarações DTO importadas, sem consumidores no código ativo, foram removidas de `crates/bellatrix_mde/src/uml`. O estado do domínio permanece no grafo canônico `Element`/`Graph`; o catálogo `uml::metamodel`, gerado a partir do XMI oficial, fornece a hierarquia de metaclasses usada pelas regras. Isso não constitui tipagem UML completa nem certificação de conformidade. Não há geração automática durante o build, inclusão de fonte upstream, registry global JavaScript ou runtime WASM. O `Cargo.toml` upstream declara MIT; as fontes de consulta e seus avisos de origem foram preservados.

A referência normativa para o desenvolvimento é a especificação UML 2.5.1 da OMG e seus documentos XMI. Os arquivos RUML são referência de implementação e não comprovação de conformidade. Os quatro XMI existentes permanecem em `docs/bellatrix/standards/uml/20161101/`.

Useful missing structural contracts are inventoried in [structural-contracts-to-reuse.md](structural-contracts-to-reuse.md), with official-source and Git provenance for the next Object/Deployment work. They are not a second runtime model.

## Reutilização executável: proprietário de Slot

`crates/bellatrix_mde/src/KERNEL/uml/semantics/instances.rs` e o caminho existente
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

A atribuição e movimentação de um Slot existente usa
`properties.definingFeature` como referência `{ "$ref": "id" }` de uma
StructuralFeature elegível em um dos Classifiers referenciados no array
`properties.classifier` da instância. A elegibilidade agora consome o grafo
compartilhado de Classifiers, incluindo atributos herdados privados e
redefinições diretas. A representação e a resolução usam o
mesmo `uml::references::ReferenceIndex` das demais referências canônicas.
Rejeita proprietário ausente ou incompatível, referência inválida, duplicação de
definingFeature e armazenamento paralelo de `slot`/`owningInstance`.
Falhas não alteram o projeto de entrada nem seu proprietário anterior; repetir a
mesma atribuição é idempotente. O teste público em `tests/graph_rules.rs` percorre
comandos, consultas e serialização JSON/Protobuf.

Features são obtidas pela união normativa dos membros StructuralFeature com
atributos dos ancestrais, inclusive privados, filtrando as redefinições diretas.
A resolução usa o mesmo motor de Namespace; capacidades de membership ainda
não resolvidas e redefinições ambíguas em ancestrais produzem diagnóstico de
capacidade não suportada, não uma falsa proibição da UML. A documentação em
[uml4net/README.md](uml4net/README.md) registra esse limite. Não foram
transplantados os erros silenciosos nem os métodos remove/clear inconsistentes
do upstream. Este incremento não implementa valores de Slots, toda consistência
de redefinições, criação visual de instâncias ou suporte completo ao diagrama
de objetos. Referências de classifiers são validadas mesmo em instâncias
sem Slots; `specification` e valores de Slots populados continuam produzindo
diagnósticos explícitos de capacidade não suportada.

## Operation Parameters: one canonical ordered graph

The executable slice in `uml/operations.rs` uses the existing `Element` records
and `owns` edges. `Operation-ownedParameter` redefines the ordered composite
`BehavioralFeature-ownedParameter`; `Parameter-operation` is its opposite. Their
only persisted representation is the sequence of `owns` records in
`Project.relations`, filtered by Operation and Parameter. No `ownedParameter`
array, `operation` field, position cache or inverse list is stored. Duplicate
ownership edges fail closed. Reordering swaps the two relevant edge positions,
leaving unrelated records and their relative order unchanged. JSON and Protobuf
preserve this vector order.

The official contracts are `Operation-at_most_one_return`,
`Operation-returnResult`, `Operation-type.1`, `Parameter-direction`,
`ParameterDirectionKind`, and inherited `TypedElement-type` in the pinned
`20161101/UML.xmi`. The corresponding UML 2.5.1 PDF sections are 9.4.3–4, 9.6,
9.9.2, 9.9.11 and 9.9.13–14. Direction choices and the absent-value `in` default
come from the generated XMI schema, not another handwritten enum. Parameter
Types are optional local `{ "$ref": "id" }` references, resolved by the shared
reference index. `Operation.type` is a read-only projection of its sole return
Parameter's Type, or null. Existing authored Rust `signature` text is preserved
as metadata and is never parsed to infer Parameter identities or Type references.
Stored historical Operation `type` data is shown separately and ignored by the
derivation. Signature notation uses canonical Parameters in their owned order,
with the return Parameter's Type after the parentheses; raw text stays separate.

The upstream recipes reviewed were
[`BehavioralFeature::add_owned_parameter` / `remove_owned_parameter`, lines 213–243](https://github.com/javapak/ruml-wasm/blob/5c90d7399acbce4c37847b272759ed85d677ba92/behavioral_feature.rs#L213-L243),
[`Parameter::set_direction`, lines 198–204](https://github.com/javapak/ruml-wasm/blob/5c90d7399acbce4c37847b272759ed85d677ba92/parameter.rs#L198-L204),
and the Operation creation fields in that same pinned snapshot. Their append,
uniqueness, remove and typed-direction ideas map to the existing member,
deletion and property-command machinery. No upstream source was copied for this
slice: its registry-backed DTO lists would duplicate the graph, and its setters
do not enforce the one-return invariant. The adapted workflow instead validates
one candidate graph before the existing revisioned commit, so a second return,
invalid Type, stale request or malformed direction cannot partially publish.
The retained upstream files remain inspection-only and are not executed.

The existing Class Quick Edit integration flow now creates real Parameters,
renames and retypes them, derives the return Type, rejects a second return
atomically, reorders and deletes Parameters, and reopens both JSON and Protobuf.
The native Inspector navigates from classifiers to Operations to Parameters,
including members without diagram views, and uses the existing realtime editing
queue and captured-target/read-only guards. Parameters are also ordered in the
semantic tree. Rooted and rootless model diagnostics use the same operations
validator. Local Parameter direction/effect/exception constraints are checked;
this is **not** complete Operation or BehavioralFeature conformance. Complete
signature distinguishability, redefinition/pre/post/body constraints, parameter
sets, Connector/Behavior streaming constraints, multiplicity/default-value
ValueSpecifications and derived return multiplicity/order/uniqueness remain
outside this slice. The notation omits these unsupported return details rather
than treating old independent Operation fields as canonical results.
