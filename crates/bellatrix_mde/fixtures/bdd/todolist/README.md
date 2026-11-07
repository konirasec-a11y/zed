# Todolist authored project corpus

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

Both examples retain all six diagrams: requirements, architecture, classes,
create-request activity, lifecycle state machine and deployment. They share one
complete canonical registry. The only difference is
`/model/registry/entries/open/data/submachine`: null becomes
`"lifecycle-recovery"`. The harness asserts that exact value change and equality
of everything else. `mutation.json` and `single-error.diff` describe this current
canonical mutation, not a positional legacy elements-array offset.

## Historical semantic requirements

The following original notes preserve the intended domain scenarios and normative
references. Mentions of old validation executables, legacy paths or additional
lifecycles describe historical requirements, not coverage of the current runner.

# TodoList multi-diagram acceptance project

`valid.json` models a TodoList Web API with 111 semantic elements, canonical
ownership and 184 relationships. Six diagrams cover user capabilities, component
architecture, layered classes, request processing, task lifecycle and deployment.
`cases.json` validates the complete model and each of these diagrams through the
public API, then preserves model contents and diagnostics across JSON and
protobuf save/close/reopen flows.

`invalid.json` is the same project with exactly one semantic field changed:
`/elements/88/properties/submachine` on the ordinary State `open` changes from
`null` to a reference to the existing StateMachine `lifecycle-recovery`.
`mutation.json` records the normative source, OCL and expected diagnostic
`UML_STATE_SUBMACHINE_STATES`. `single-error.diff` is the complete
textual difference between the documents. No missing IDs, malformed serialization
or disconnected diagram views manufacture the negative case.

Package-owned PackageableElements have public visibility. Owned values and
relationships without a UML Namespace retain explicitly absent visibility.
Use-case, component, class and deployment diagrams have no UMLDI modelElement
context; the activity and state-machine diagrams retain their respective context.

These fixtures exercise implemented model and diagram rules only. Unsupported
metaclasses, property contracts and expression evaluation remain visible warnings,
not proof of complete UML conformance or executable application behavior.

The negative case follows the published `State-submachine_states` OCL literally:
`isSubmachineState implies connection->notEmpty( )`. The State has no connection
point references. The accompanying specification prose describes the converse;
this discrepancy is recorded explicitly rather than changing kernel semantics.
