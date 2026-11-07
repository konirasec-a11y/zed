# Interaction authored project corpus

## Canonical persistence acceptance

The document files are explicit schema-2 projects: `schemaVersion`, project identity,
and `model.registry.entries`. The registry contains typed ruml contracts; it is the
only persisted UML state. No schema-1 loader, projection-to-storage adapter or
legacy compatibility fixture is part of the executable BDD.

`tests/bdd_diagram_flows.rs` imports each document through the public document API,
prepares its canonical section, saves to JSON and BLX2, closes and reopens both
files, and compares the complete typed project and canonical API envelope. It
also persists the prepared section through the public section API. The manifests
supply explicit authored witnesses checked before and after persistence. Every
Gherkin example must have exactly one manifest entry. Of the 40 preserved
requirements, 32 have executable canonical documents and eight are explicitly
pending because their original intent is not representable by the current
contract. Pending cases have no substitute JSON fixture.

Names ending in `valid` or `invalid` are historical scenario identifiers, not a
claim of UML certification. `historical_diagnostics` and
`historical_forbidden_diagnostics` preserve the original requirement context but
are not evaluated by these persistence flows. Semantically questionable contracts
must remain inspectable and survive persistence; no removed validator is revived.

Multiplicities are authored as `lowerValue`/`upperValue` references to
LiteralInteger/LiteralUnlimitedNatural contracts. Ownership is encoded in UML
composite fields, not persisted `owns` edge records. Required scalar fields use
the current contract's enum spelling and string representation. Diagram geometry,
styles, view endpoints and identities remain part of the equality assertion.

The eight requirements cover sequence, communication, timing and interaction
overview pairs. The two interaction-overview examples have canonical documents
and preserve the InteractionUse.refersTo mutation.

### Pending requirements, without substitute fixtures

`sequence-valid`, `sequence-invalid`, `communication-valid`,
`communication-invalid`, `timing-valid` and `timing-invalid` are explicitly ignored
by the integration harness. Their authored Message.signature has no field in the
current generated Message contract. MessageOccurrenceSpecification also requires
an event that the authored models do not specify. Moving an operation reference
to a newly invented SendOperationEvent/ReceiveOperationEvent has not been shown
equivalent and is not used as a substitute.

These six JSON fixtures have been removed. Each manifest entry retains its
requirement, historical diagnostic context and `pending_reason`, but no `file`.
The ignored bodies return an explicit error if forced with `--ignored`, never a
fictitious success. The original Message.receiveEvent, Lifeline.represents and
DurationConstraint.firstEvent requirements remain pending, not certified or
silently dropped.

## Historical semantic requirements

The following original notes preserve the intended domain scenarios and normative
references. Mentions of old validation executables, legacy paths or additional
lifecycles describe historical requirements, not coverage of the current runner.

# Interações isoladas

Cada `.valid.json` é um cenário positivo de projeto Bellatrix com exatamente um diagrama; cada `.invalid.json` é sua cópia com exatamente uma propriedade alterada, conforme `mapping.json`. Os modelos contêm referências locais tipadas e ownership, não rascunhos visuais. São entradas BDD, não certificados de conformidade completa nem testes unitários.

- Sequence: três participantes tipados, chamadas síncronas/respostas, ocorrências de envio/recepção, execução, fragmento opt, operando e guarda. Defeito: `submit.receiveEvent` referencia Lifeline, que não é MessageEnd.
- Communication: os mesmos papéis em disposição espacial e mensagens numeradas. Defeito: `service.represents` referencia Class, que não é ConnectableElement. A numeração é rótulo de apresentação, não propriedade inventada do metamodelo.
- Timing: estados e valores autorados, mensagens com ocorrências e DurationConstraint com intervalo 0..20 ms. Defeito: dois constrainedElement e somente um firstEvent. As amostras `view.timing` são apresentação; não calculam tempo UML, nem demonstram satisfação do intervalo. `ms` é convenção textual do cenário.
- Interaction Overview: contexto Interaction, dois InteractionUse, comportamentos reutilizados e Activity de apoio com início, chamadas, decisão, guardas, final e fluxos. Defeito: `use-authenticate.refersTo` referencia Class, que não é Interaction. Existe somente um diagrama; as interações referidas são elementos locais auxiliares, não diagramas extras. As ações CallBehaviorAction têm Shapes próprios como endpoints dos fluxos; as caixas InteractionUse aparecem ao lado e referenciam os mesmos comportamentos. Não há conversor formal Interaction/Activity ou equivalência de execução demonstrada pelo kernel.

Fontes normativas disponíveis no repositório: `src/KERNEL/uml/metamodel/metamodel.rs`, descritores UML 2.5.1 `Message-receiveEvent`, `Lifeline-represents`, `InteractionUse-refersTo`, `DurationConstraint-first_event`; UMLDI usa `UMLInteractionDiagram` e kind sequence/communication/timing/overview com um contexto Interaction. Contratos observáveis: `src/model/{project,graph,diagram}.rs`, `src/KERNEL/uml/semantics/property_contracts.rs`, `src/KERNEL/uml/validation/structural_diagrams.rs` e `tests/duration_first_event.rs`.

Limites: os cenários não enumeram todos os elementos opcionais de UML, nem todos os operadores de interação. Ordenação temporal, correspondência de assinaturas/argumentos, semântica das guardas, traces, execução de comportamentos e notação completa precisam de cobertura adicional. Um resultado sem erros nos contratos implementados não significa aprovação integral UML 2.5.1. A verificação executada e suas lacunas devem ser registradas pelo runner BDD, não presumidas a partir dos nomes `valid`.

Guardas e ValueSpecifications sem namespace declaram `visibility: null` explicitamente: omitir a propriedade usa o default UML herdado, que não é válido nesses owners. Message continua sendo um NamedElement no modelo, com Path de apresentação nos diagramas de interação; não é convertido artificialmente em Relationship.
