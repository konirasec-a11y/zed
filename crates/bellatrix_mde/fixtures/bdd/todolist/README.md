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
