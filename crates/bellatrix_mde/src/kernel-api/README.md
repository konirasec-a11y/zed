# Functional kernel API: version 1

The initial public entry point is `bellatrix_mde::validate_uml_model`:

```rust
pub fn validate_uml_model(input_json: &str) -> Result<String, String>
```

Implementation lives in this literal `kernel-api/` directory. Its Rust module,
request/response types and helpers are private. Only the function is reexported;
no new public structs, enums, modules, handles or registries are introduced.

## Input contract

A JSON object with exactly these fields:

```json
{
  "api_version": 1,
  "model": {
    "schema_version": 1,
    "id": "example",
    "name": "Example",
    "elements": [],
    "relations": [],
    "diagrams": []
  }
}
```

`api_version` is a required unsigned integer and must equal 1. Unknown envelope
fields are rejected. `model` uses the existing Bellatrix document representation;
its model `schema_version` is independent of the API version and currently must
be 1 (the existing codec defaults an omitted model version to 1). This boundary
exposes a documented data format, not the internal Rust structs.

## Successful call

`Ok` contains a JSON object with:

- `api_version`: 1.
- `coverage`: `"implemented_rules_only"`.
- `has_errors`: whether any reported diagnostic has severity `"error"`.
- `diagnostics`: an array of objects with `severity` (`"error"` or `"warning"`),
  `code`, `subject_id` and `message`, all strings.

Diagnostics are ordered by `(code, subject_id, severity, message)`. Input array
order is preserved when invoking existing semantics because ordered UML members
are meaningful. A well-formed invalid model returns `Ok` with diagnostics.

Graph integrity runs first. If it fails, the response contains one
`GRAPH_INTEGRITY` error and skips semantic validation. Otherwise the existing
structural-model validation runs. This does not run each diagram-specific rule,
prove complete UML 2.5.1 conformance, or certify a project can be generated.
Warnings and unsupported-capability diagnostics remain visible in the result.

## Contract errors

`Err` contains JSON, not an unstructured panic message:

```json
{
  "api_version": 1,
  "error": {"code": "UNSUPPORTED_API_VERSION", "message": "expected api_version 1, received 2"}
}
```

Codes are `INVALID_REQUEST` (JSON/envelope), `UNSUPPORTED_API_VERSION`,
`INVALID_MODEL` (document decoding/version), and
`RESPONSE_SERIALIZATION_FAILED` (response encoding). Message text explains the
failure but is not a substitute for the stable code.

## Effects and transition

No external effects: no filesystem, network, process, clock, sink or external
callback. It does not mutate the input or retain caller state. Returned bytes
are repeatable for the same input and implementation, including A-B-A calls.

This is a functional migration slice, not completed encapsulation. Existing
legacy exports remain available so current consumers keep compiling. Their
removal and migration to this boundary are separate steps; the presence of this
function must not be advertised as having made every kernel internal private.

## Graph queries

Two additional functions use the same `&str -> Result<String, String>` boundary:

- `reachable_graph_nodes`: request fields `api_version`, `model`, `roots` (array
  of IDs) and `relation_kinds` (array of strings). Returns
  `{"api_version":1,"node_ids":[...]}` with sorted unique IDs, including roots.
  Traversal is outgoing and follows only listed kinds. Empty roots return no
  nodes; empty kinds return only roots. Unknown kinds match no edges.
- `shortest_graph_path`: request fields `api_version`, `model`, `from`, `to`,
  optional `relation_kind` (string or null). Absent/null selects all kinds.
  Returns `{"api_version":1,"relation_ids":[...]}` in traversal order, null
  when unreachable, and an empty array for identical existing endpoints.
  Ties use the existing index ordering by next node ID, kind and relation ID.

Unknown envelope fields are rejected. Both functions first run graph integrity
validation, returning `GRAPH_INTEGRITY` rather than silently dropping broken
edges. This is not structural UML conformance validation. Unknown queried nodes
return `UNKNOWN_NODE`, including unknown reachability roots (unlike the legacy
index's permissive helper). Graph vertices include model elements and semantic
UML relationships recognized by the index; diagram/view/document IDs are not
vertices. Request array order is preserved; output node IDs are sorted explicitly.
Neither function exposes internal graph objects or relation structs. The same
no-effects and A-B-A determinism contract applies. Legacy exports still remain.

Private `contract.rs` shares envelope version/error formatting and document
codec checks with validation; it is not an additional public interface.

Graph-query integrity deliberately excludes numeric presentation constraints
(diagram dimensions, geometry, style and timing), so a visual defect cannot block
an unrelated topology query. It still checks global IDs, semantic root, relation
endpoints, ownership/declaration/physical-parent constraints, hierarchy cycles,
and view subject references. Existing full validation retains presentation
checks. Schema decoding still requires well-formed presentation data when present;
this is not a lenient document recovery API. Neither path claims checks beyond
the existing graph validator (for example, it does not add view endpoint checks).

Request/document decoding errors use boundary-owned messages instead of exposing
Serde's internal Rust type names. Request errors include JSON line/column;
document errors identify unsupported versions or an invalid document schema.

## Document decoding and legacy graph validation

`decode_model_document(document: &[u8], format: &str) -> Result<String, String>`
accepts original bytes and an explicit `json` or `protobuf` format (case-sensitive).
Protobuf uses the existing BLX1 header. It returns the version-1 JSON envelope
`{"api_version":1,"model":...}`, directly accepted by the validation functions.
No base64, filesystem access or public Rust model type is required. Existing
codec defaults and normalization are preserved; this is not lossless preservation
of source JSON formatting or unknown fields. Decoding does not validate semantics.
Errors are `UNSUPPORTED_FORMAT`, `INVALID_MODEL`, or
`RESPONSE_SERIALIZATION_FAILED`, with the existing versioned error envelope.
The output version is explicit; incompatible future codec boundary behavior must
use a new function/version rather than silently changing version-1 semantics.

`validate_graph_model(input_json: &str) -> Result<String, String>` accepts the
same envelope as `validate_uml_model`. It returns the same diagnostic shape with
coverage `graph_integrity_and_presentation`, preserving full legacy
`Graph::validate` behavior (including visual geometry/style validation) for future
CLI migration. It does not run structural UML rules. Invalid decoded graphs return
`Ok` with `has_errors:true` and the first `GRAPH_INTEGRITY` diagnostic, so consumers
must check that flag before proceeding. Contract errors remain `Err`.

Both functions are pure in-memory transformations with no I/O, clock, callback,
sink or retained state. The CLI has not been changed by this slice, and legacy
exports are still exposed. Topology queries retain their narrower presentation-
independent integrity check; that is intentionally not the CLI compatibility gate.

Protobuf can encode non-finite diagram dimensions; JSON cannot. The decoder
returns `RESPONSE_SERIALIZATION_FAILED` for that case instead of silently replacing
numbers with null. This representation limitation must be handled explicitly by
future CLI migration; the legacy protobuf decoder itself is unchanged.

## Generation dependency analysis

`analyze_generation_dependencies(input_json: &str) -> Result<String, String>`
accepts exactly `api_version:1`, `model`, and `selected_ids` (source element IDs).
It reuses the existing pure dependency analysis without exposing its Rust IR.
Selection is exact, not a scope-expansion request: duplicate IDs collapse, empty
selection yields no units, and unknown IDs (including relation/diagram IDs) return
`UNKNOWN_SELECTION`. No dependency or ownership closure is added automatically.

The response contains `api_version:1`, coverage `generation_dependency_analysis`,
and `units` in prerequisite-first order; independent ties are ordered by source
ID. A unit has `id`, `kind`, `name`, and `requires`, whose entries have `id`,
`kind`, `name`, and `relation_kind`. Requirements retain external dependencies
outside the selection, sorted and deduplicated by `(id, relation_kind)`.

Ordering edges are the existing `depends_on`, `uml.dependency`, `uml.usage`,
`uml.generalization`, `uml.interface_realization`, `uml.realization`,
`uml.template_binding`, and `build.depends_on`. Source edges point from client to
supplier; scheduling reverses that direction. Association, ownership and physical
containment do not become compilation dependencies. Only selected-to-selected
edges impose order, so cycles outside that subgraph are not errors for this call.
Selected dependency cycles return `GENERATION_CYCLE` with a stable message, not
an insertion-order-dependent internal cycle witness.

The function requires reference/identity/hierarchy graph integrity, excluding
numeric presentation checks. It does not run structural UML rules or certify a
model can generate a target project. Other errors use the established
`INVALID_REQUEST`, `UNSUPPORTED_API_VERSION`, `INVALID_MODEL`, `GRAPH_INTEGRITY`
contract. No files, paths, templates, processes, clocks, sinks or callbacks are
used; all state is invocation-local and input is unchanged. Tests cover A-B-A,
concurrent calls, reordered inputs, selection errors, cycles and external
requirements. This is dependency analysis only, not complete target-graph,
trace-graph, DPO/SPO/TGG transformation, artifact planning or publication. See
`docs/bellatrix/MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md` for the remaining stages.

A graph-valid ordering dependency may target a semantic UML relationship identity
rather than a source Element. The current generation IR cannot represent that
requirement. For selected clients, this returns `GENERATION_UNSUPPORTED_TARGET`
(including external targets outside the selection), rather than silently omitting
it from `requires`. The shared analyzer enforces this rule for legacy consumers
as well. Unsupported dependencies of unselected clients do not expand the scope.

## Diagram validation

`validate_uml_diagram(input_json: &str) -> Result<String, String>` accepts exactly
`api_version:1`, `model`, `diagram_id` (string) and `diagram_kind` (string).
The explicit requested kind prevents a stored diagram from silently being checked
as a different category. Accepted kind names are `class`, `object`, `package`,
`component`, `composite_structure`, `deployment`, `profile`, `use_case`, `activity`,
`state_machine`, `sequence`, `communication`, `interaction_overview` and `timing`.
Names are case-sensitive; other names return `UNSUPPORTED_DIAGRAM_KIND` in the
versioned error envelope. API/model decoding is checked before the kind name.

The successful response uses the validation diagnostic shape with coverage
`implemented_model_and_diagram_rules_only`. Full graph integrity and presentation
checks run first; a failure returns one `GRAPH_INTEGRITY` error and skips semantic
and projection validation. Otherwise existing whole-model semantics and the
selected diagram's projection rules run. A missing diagram produces
`UMLDI_DIAGRAM_MISSING`; an incompatible stored category or interaction kind
produces `UMLDI_DIAGRAM_KIND_MISMATCH`. Unknown stored categories retain the
existing `UMLDI_UNSUPPORTED_DIAGRAM_TYPE` diagnostic.

The projection checks include context type/multiplicity, shape/edge geometry and
visible endpoints under existing editor policy. Visual draft and unsupported
coverage warnings are preserved. Whole-model errors may concern elements outside
the selected diagram. Other diagrams receive graph/presentation integrity checks,
but their projection rules do not run. This is not complete UML/UMLDI conformance
or generation certification. No new normative rules are claimed by this adapter.

Like the other boundary functions, this function has no external effects or
retained state and returns sorted diagnostics. The original model-only and graph
validation contracts remain unchanged. No UI or CLI migration is included.
