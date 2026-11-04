# Behavioral acceptance fixtures

Each `*.valid.json` / `*.invalid.json` is a standalone Bellatrix document with
one UMLDI diagram, a designated Model root, local references, canonical ownership,
semantic relationships and attached edge views. Load the document, then wrap it
in the version-1 `validate_uml_diagram` request. `cases.json` supplies the requested
kind, diagram ID and expected diagnostic witnesses; warnings are not success
certificates and must remain visible in reports.

- **Use case:** customer specialization, payment actor, system subject, four use
  cases, two includes, an extension point and extend, and two associations with
  materialized Property ends, derived association links and synchronized role
  projections. Named relationships remain distinguishable in their namespaces.
  The negative case empties
  `discount-extension.extensionLocation`, violating `Extend::extensionLocation`
  multiplicity 1..* (`UML_PROPERTY_CONTRACT`).
- **Activity:** initial/final nodes, decision/merge, fork/join, five opaque actions,
  warehouse partition, typed input/output pins, object flow, control flows and
  owned Boolean guards with explicitly absent visibility because their edge
  owners are not Namespaces. The negative case changes the authored `flow-1.target`
  reference to the Class `order` instead of an ActivityNode
  (`UML_PROPERTY_CONTRACT`). All IDs still resolve; this is not a missing-ID test.
- **State machine:** initial, ordinary and final vertices, nested composite state
  with its own Region, cancellation branch, and seven explicitly contained
  named transitions. The negative case moves `complete` to originate at `done` and end
  at `pending`, updating semantic endpoint properties and View attachments.
  `FinalState-no_outgoing_transitions` must report
  `UML_FINAL_STATE_NO_OUTGOING_TRANSITIONS` for `done`.

Sources: the pinned `docs/bellatrix/standards/uml/20161101/UML.xmi` and UMLDI
catalog, `src/model/{graph,diagram,project}.rs`, and
`src/KERNEL/uml/{metamodel,semantics,validation}`. Names identify fixture content,
not runtime action implementations. The Activity's Boolean guards select the
stock-available path deterministically; no expression evaluator is implied.

Coverage is intentionally `implemented_model_and_diagram_rules_only`. These
fixtures do not demonstrate execution of actions, token scheduling, event
processing, use-case execution, behavioral code generation, or every normative
UML constraint. Include/Extend and activity-edge endpoint properties are authored
explicitly because their complete canonical role adapters are not implied by
binary `from`/`to` fields. They must not be replaced by visual drafts to hide
unsupported rules. Baselines exercise the supported representation, not a claim
that every feature of each UML family is exhaustively covered.
