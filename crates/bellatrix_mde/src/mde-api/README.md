# Functional MDE API: version 1

This directory is the explicit application/document boundary of `bellatrix_mde`.
It is separate from [`kernel-api/`](../kernel-api/README.md), which exposes kernel
validation, graph queries and generation analysis. MDE operations coordinate the
existing document codec and application commands; they do not redefine UML rules
or introduce another model.

The external communication boundary is Cordis events. These Rust functions are
the engine entry points used by the Cordis adapter, not a second direct route
for the UI, Zed host, CLI or exporters. The adapter receives integration/project
events and invokes MDE; calls inside MDE remain direct. MDE does not depend on
Cordis, UI or Zed types. Purely visual interaction and presentation state remain
inside the UI, rather than becoming integration events.

The application facade is defined here and reexported from `src/mde.rs`:

- `document.rs`: `decode_model_document`, `encode_model_document`,
  `convert_model_document`, `prepare_model_document`, `validate_model_section`,
  `encode_model_section`.
- `interchange.rs`: `import_mdj_document`, `export_mdj_document`, `export_mdj_section`.
- `commands.rs`: `reparent_uml_element`, `apply_model_command`.
- `projection.rs`: `project_model_section`.
- `presentation.rs`: `project_documentation`.
- `generation.rs`: `project_rust_generation`.
- `headless.rs`: `report_model_document`.
- `artifacts.rs`: `prepare_artifact_publication`, `publish_artifact_manifest`.
- `observability.rs`: `validate_model_section_diagram`, plus the opaque
  `MdeExecutionTrace` struct and its public methods `new`, `set_sink`, `clear_sink`,
  `run_id`, `events`, `dropped` and `emit`.

Implementation modules are private. Input commands and derived read records are
independent DTOs in `bellatrix_mde_protocol`; they do not expose engine model,
graph, compiler or UML types. The protocol depends on neither MDE nor Cordis and
contains no runtime, document store or semantic algorithm. `ModelDocumentParts`
contains identity plus opaque canonical section JSON, rather than a second model.
`src/api_contract.rs` owns the common envelope version, errors and full-document
decoder. Kernel and application APIs use that private helper independently.
The semantic-only projection in `kernel-api/contract.rs` is never a save format.

## Document decoding

`decode_model_document(document: &[u8], format: &str) -> Result<String, String>`
accepts original bytes and an explicit `json` or `protobuf` format (case-sensitive).
Protobuf uses the existing BLX1 header. It returns the version-1 JSON envelope
`{"api_version":1,"model":...}`, directly accepted by the validation functions.
No base64, filesystem access or public Rust model type is required. Existing
codec defaults and normalization are preserved; this is not lossless preservation
of source JSON formatting or unknown fields. Decoding does not validate semantics.
Errors are `UNSUPPORTED_FORMAT`, `INVALID_MODEL`, or
`RESPONSE_SERIALIZATION_FAILED`, with the shared versioned error envelope.

Protobuf can encode non-finite diagram dimensions; JSON cannot. The decoder
returns `RESPONSE_SERIALIZATION_FAILED` for that case instead of silently replacing
numbers with null. The underlying protobuf codec is unchanged.

## Document encoding

`encode_model_document(input_json: &str, format: &str) -> Result<Vec<u8>, String>`
accepts exactly `{"api_version":1,"model":...}` plus the explicit case-sensitive
`json` or `protobuf` format. The result is the existing codec's document bytes:
pretty JSON or BLX1-prefixed Protobuf, without another envelope or base64 layer.
The original model is unchanged. The codec's schema, composition, provenance and
identity checks still apply; no UML validation or full graph validation is added.

Errors use `INVALID_REQUEST`, `UNSUPPORTED_API_VERSION`, `UNSUPPORTED_FORMAT` or
`INVALID_MODEL`. Encoding is not source JSON formatting preservation. Current
authoring knowledge, composition, typed presentation and MDJ provenance use the
same existing JSON/BLX representation rather than a new persisted DTO.

`convert_model_document(document: &[u8], source_format: &str,
destination_format: &str) -> Result<Vec<u8>, String>` converts through the codec
without a JSON envelope. It preserves non-finite binary diagram dimensions for
binary output; JSON output fails with `RESPONSE_SERIALIZATION_FAILED` rather than
replacing them with null. This distinction is necessary for byte-oriented codec
consumers. Other errors are `UNSUPPORTED_FORMAT` and `INVALID_MODEL`. Output uses
the existing codec's normalization; this is not original source-byte replay.

## MDJ interchange

`import_mdj_document(document: &[u8]) -> Result<String, String>` passes the original
bytes to the existing MDJ converter and returns
`{"api_version":1,"model":...,"diagnostics":[{"path":"...","message":"..."}]}`.
Diagnostics retain their adapter order and text, including unsupported mapping
and unresolved-reference notices. They are separate response information, not a
second stored model or a UML-validation result. The model contains the original
source bytes in existing provenance, plus source residue, composition and current
authoring knowledge. Import errors use `MDJ_IMPORT_FAILED`; response encoding
errors use `RESPONSE_SERIALIZATION_FAILED`.

`export_mdj_document(input_json: &str) -> Result<Vec<u8>, String>` accepts exactly
`{"api_version":1,"model":...}` and returns MDJ bytes. Import diagnostics are not
accepted as an extra request field. The full document decoder runs its existing
schema/integrity checks, then the existing MDJ exporter requires provenance.
MDJ export is experimental: only supported mappings are emitted, unsupported
edits fail explicitly, and full compatibility with StarUML is not guaranteed.
An unchanged projection returns original source bytes. For supported edits, the
exporter reimports its actual output and compares the complete supported
projection. Unsupported edits fail instead of dropping data or replaying stale
source. Errors use `INVALID_REQUEST`, `UNSUPPORTED_API_VERSION`, `INVALID_MODEL`
or `MDJ_EXPORT_FAILED`, with the existing adapter explanation for export failures.
This introduces no new MDJ mappings and does not certify UML or StarUML parity.

The import response and model request are deliberately different envelopes.
To validate, encode or export an imported model, retain `diagnostics` separately
and construct the request from the response's `api_version` and complete `model`.
Do not route this extraction through an IEEE-754-only JSON object parser, which
can round large integer/residue values. In Rust, deserialize the response's
`model` field as `Box<serde_json::value::RawValue>` and insert its raw JSON text
verbatim as the request's `model` value, without quoting it as a JSON string.
Alternatively, a JSON representation must preserve arbitrary-precision numbers
and reject duplicate fields rather than silently overwrite them. The facade
itself keeps request models raw until the existing document decoder consumes
them. Do not remove or reconstruct provenance, source residue or unfamiliar
properties while extracting the model. The public original MDJ byte slice is
not re-encoded before the adapter captures it.

All these operations are in-memory. Exported/encoded bytes do not reserve a file,
write a destination, publish a revision or acknowledge a save. The host must keep
its snapshot/revision checks and write/acknowledgement protocol.

## Read-only documentation projection

`project_documentation(input_json: &str) -> Result<String, String>` accepts exactly
`{"api_version":1,"model":...}`. It applies the existing graph validation gate
and returns `{"api_version":1,"document":...}` with display text, links and
diagram primitives. UML labels and relationship stroke patterns are resolved
inside MDE. The Cordis export handler supplies this neutral projection to the
HTML renderer, which does not depend on MDE or query UML semantics.

The result is not a persisted model, mutation input or source backup. It omits
source provenance and current authoring knowledge and cannot replace the model.
Text is not HTML-escaped; escaping belongs to the output renderer. No UI events,
GPUI objects, filesystem effects or retained renderer state enter MDE.

## Cordis document integration

The current integration events live in `crates/bellatrix_mde_cordis/src/`:

- `mdj_interchange.rs`: `ModelingPrepareDocument` accepts bytes and
  `ModelingDocumentFormat::{ProjectJson, ModelJson, ModelProtobuf, StarUml}`. It
  returns `PreparedModelingDocument` containing the platform `ProjectDocument`
  and diagnostic strings. Preparation runs the existing presentation safety
  checks, not UML validation, and does not publish, open a session or save.
  These request/response records contain no MDE model or UI types.
- `document_codec.rs`: `ModelingConvertDocument` accepts bytes plus source and
  destination format strings and returns converted bytes. Its handler delegates
  to the byte-to-byte codec facade without an intermediate JSON model envelope.
- `mdj_interchange.rs`: `ModelingExportMdj` retains the exact-snapshot save
  preparation contract. `ModelingApplyMdjEdit` retains the inverse-export gate
  before atomic snapshot publication. Their existing snapshot/revision guards
  are not replaced by the stateless document preparation event.

The Zed host dispatches preparation through Cordis and injects an asynchronous
preparation callback into the native-document UI adapter. The UI passes source
bytes and retains its buffer guards, local interaction state and messages.
`native_document.rs` no longer parses the imported MDE model or calls MDE
import/graph functions. `ProjectDocument` belongs to the platform project crate, which does not
depend on MDE. The Zed host has no direct Cargo dependency on MDE.

The workbench now receives `ModelingReadProjection` results through its Cordis
bridge. Read results are tied to the exact immutable `ProjectDocument` Arc and
revision; an old or reopened source cannot replace the current projection.
The canonical section remains in the platform document. Derived inspector,
hierarchy, tool catalog and notation records are rebuilt inside MDE, not edited
or persisted. Local selection, gestures, drafts and painting remain in the UI.

`ModelingApplyMdjEdit` carries the neutral `MdjEditCommand`, translated only inside
the MDE command facade. `apply_model_command` computes a candidate section; Cordis
retains compare-and-swap publication and the native MDJ exact-snapshot guard.
For MDJ sessions the engine requires imported provenance and successful verified
export before returning a candidate. Preparation and export preserve original
MDJ bytes in provenance; unsupported edits still fail without publication.

`prepare_model_document` extracts identity and canonical section JSON from a model
envelope after presentation checks. `validate_model_section` applies hierarchy
and presentation checks before opening a native platform document.
`encode_model_section` and `export_mdj_section` reconstruct the same universal
contract from section JSON plus platform identity, without leaking typed model
internals to Cordis. JSON numbers retain arbitrary precision. Non-finite binary
dimensions use the byte-to-byte codec path; JSON cannot carry them.

`ModelingPreviewMove` evaluates the same command facade without publishing, and
`ModelingValidateSnapshot` validates a stored diagram with before/after snapshot
guards. These are integration queries, not UI-only pointer or selection events.

## Read projection, generation and publication

`project_model_section` returns `protocol::projection::ModelingProjection`, a
read-only projection containing labels, inspector data, hierarchy, tool metadata,
view geometry and resolved notation. Private `projection_*.rs` producers reuse
engine semantics. The projection omits source/provenance and is not accepted as
a canonical model or mutation input.

`project_rust_generation` returns a versioned emission document consumed by the
independent Rust renderer. `report_model_document` handles the existing headless
check/inspect/analyze reports. `prepare_artifact_publication` validates a renderer
manifest and returns canonical artifact information and counts.
`publish_artifact_manifest` is the explicitly effectful API: it uses the existing
safe path/ownership publication implementation at the caller-selected directory.
Cordis owns orchestration and destination selection; renderers do not query MDE.

`MdeExecutionTrace` hides the engine trace implementation behind neutral events.
Unlike pure projection/codec functions it reads timing and invokes an explicitly
registered sink; it does not expose mutable model state. Diagram validation accepts
an optional trace handle and returns neutral diagnostics, not a UML certificate.

## Semantic reparenting candidate

`reparent_uml_element(input_json: &str) -> Result<String, String>` accepts exactly
`api_version:1`, `model`, `element_id` and `owner_id`. The IDs are required strings;
null/omitted owners are rejected rather than silently detaching an element. The
result is `{"api_version":1,"model":...}`, suitable for subsequent validation.

This pure function reuses the existing transactional `SetOwner` command, not an
Explorer placement override. It first requires graph/presentation integrity,
then applies the supported UML composite ownership rules, root/derived-owner
restrictions, cycle checks and the command's semantic change-validation policy.
A rejected candidate returns no partial model and never changes the input.
Existing ownership relation identity is retained when reparenting; a new ownership
relation receives the existing deterministic, collision-checked model-derived ID.
Other normalization performed by the command, including association projection
synchronization, remains in force; this is not a raw single-edge rewrite.

Errors use the shared envelope: `INVALID_REQUEST`, `UNSUPPORTED_API_VERSION`,
`INVALID_MODEL`, `GRAPH_INTEGRITY`, `OWNERSHIP_REPARENT_REJECTED`, or
`RESPONSE_SERIALIZATION_FAILED`. The rejection message carries the existing
command reason. Existing semantic errors may remain under the command's delta
policy: success is not full UML conformance certification. Full validation remains
a separate operation and should inform the caller's publication policy.

Existing Explorer overrides are preserved, so an old presentation override can
still visually mask the new semantic owner. Reconciliation of those placements
requires an explicit application policy; semantic reparenting does not silently
delete presentation information or reinterpret diagram placement as UML ownership.

## Shared contract and effects

`api_version` is independent of the document's `schema_version`; both currently
use version 1. Request envelope fields are explicit and unknown fields are
rejected. Nested model input remains raw JSON until it reaches the document
decoder, preserving duplicate-key rejection for typed authoring knowledge.
The single document contract remains in `contracts/`, including composition,
source provenance and current authoring knowledge. This facade does not create
a second DTO model or reinterpret source-specific fields as normative UML.

Versioned envelope functions return JSON error strings with `api_version` and
an `error` object containing `code` and `message`. Typed in-process section,
command and observation functions return explanatory `String` errors; callers
should not treat those messages as stable error codes.
Codec, projection and command-candidate functions are in-memory and do not
publish, advance revisions or mark documents clean. Publication and observation
have the explicit effects described above. The application must compare the
source snapshot/revision and publish atomically; a client-supplied revision alone
cannot provide that guarantee.

## Public surface and production boundary

Production consumers use Cordis events, and Cordis invokes the explicit MDE
application facade. MDE has no Cordis, UI, Zed or platform-project dependency.
CLI and both renderer crates have no direct MDE dependency. UI records and
interaction/presentation rules remain outside the engine; they are not UML
validity verdicts. Implemented UML validity is decided by the kernel and its
reported coverage, not inferred from successful editing or rendering.

The crate root now exposes exactly 24 free functions and one opaque struct.
The source inventory below covers every external root reexport in `src/mde.rs`.
All facade implementation modules remain private. No engine model, graph,
compiler, command implementation or UML module is exported as another route.

### Public free functions

Paths in this table are relative to `crates/bellatrix_mde/src/`.

| Function names | Definition file | Boundary |
| --- | --- | --- |
| `decode_model_document`, `encode_model_document`, `convert_model_document`, `prepare_model_document`, `validate_model_section`, `encode_model_section` | `mde-api/document.rs` | Document codec and platform-section preparation |
| `import_mdj_document`, `export_mdj_document`, `export_mdj_section` | `mde-api/interchange.rs` | MDJ import and experimental export of supported mappings |
| `reparent_uml_element`, `apply_model_command` | `mde-api/commands.rs` | Atomic application-command candidates |
| `project_model_section` | `mde-api/projection.rs` | Transient read projection |
| `project_documentation` | `mde-api/presentation.rs` | Neutral documentation projection |
| `project_rust_generation` | `mde-api/generation.rs` | Neutral Rust emission projection |
| `report_model_document` | `mde-api/headless.rs` | Headless reports with explicit validation scope |
| `prepare_artifact_publication`, `publish_artifact_manifest` | `mde-api/artifacts.rs` | Manifest preparation and explicit filesystem publication |
| `validate_model_section_diagram` | `mde-api/observability.rs` | Diagram diagnostics with optional trace observation |
| `validate_uml_model`, `validate_uml_diagram`, `validate_graph_model` | `kernel-api/validation.rs` | Kernel validation adapters |
| `reachable_graph_nodes`, `shortest_graph_path` | `kernel-api/graph.rs` | Kernel graph queries |
| `analyze_generation_dependencies` | `kernel-api/generation.rs` | Kernel generation dependency analysis |

There are 18 application/document functions and six kernel functions. Their
signatures use primitives, standard-library values, JSON/bytes and independent
`bellatrix_mde_protocol` records. The protocol contains neutral commands,
projections, diagnostics and events, not a second persisted domain model.
`ModelDocumentParts` carries identity and canonical section JSON. The original
universal document remains the only authoritative engine state.

### Public struct and methods

`MdeExecutionTrace` is defined in `mde-api/observability.rs` and reexported from
`mde.rs`. Its engine-trace field is private. Its complete public inherent-method
list is `new`, `set_sink`, `clear_sink`, `run_id`, `events`, `dropped` and `emit`.
It implements `Clone`. Observation uses protocol `ExecutionEvent`, `TracePhase`
and `TraceOutcome` records; no internal trace type is part of this interface.
`set_sink` invokes a caller callback, and `publish_artifact_manifest` writes files:
the public surface must not be described as uniformly pure or side-effect-free.

### Internal implementations no longer exported

`contracts`, `commands`, `uml`, `MODELING_NAMESPACE` and the former typed root
reexports are now crate-visible, not public APIs. They remain available for
direct calls inside MDE without wrappers or duplicated state. This includes
`Project`, `ModelingSection`, `Graph`, `GraphIndex`, `Plan`, `Scope`, the old
`Format`/`decode`/`encode` route, typed commands and raw UML queries. Their defining
files and implementations remain in place; visibility closure does not delete
semantic functionality or alter the JSON/BLX/MDJ document formats.

The internal locations remain `contracts/`, `application/commands/`,
`application/edits/`, `persistence/`, `view-model/`, `observability/`,
`KERNEL/graph/`, `KERNEL/compiler/` and `KERNEL/uml/`. MDJ implementations remain
private in `persistence/mdj.rs` and `persistence/mdj_conversion.rs`. There is no
compatibility alias, public glob export or public submodule exposing these
implementations. Integration uses the existing facade functions only.

The current `validate_graph_model` and `validate_uml_diagram` kernel adapters
still include their existing presentation checks. This visibility change does
not move those checks, remove legitimate UI interaction/presentation rules, or
introduce a UML rule implementation outside the kernel. It does not establish
full UML coverage or StarUML parity.

### Preserved test work and verification limits

Legacy integration tests that still import the now-private engine internals
will no longer compile until their authorized migration resumes. Their existing
files and in-progress changes are preserved untouched; this production closure
does not remove workflows, invent test-only APIs or claim those tests passed.
Some old semantic-edit rejection expectations also predate the current draft
editing policy. They are not silently weakened or used to restore a validation
gate during this change.

Verification for this checkpoint is source inspection of public signatures,
production imports and Cargo dependencies, plus whitespace checking. No build,
test run or test edit was performed for visibility closure. Production-source
boundary closure and the unfinished legacy-test migration are separate results.
Restricting former exports can expose `unused_imports`/`dead_code` warnings for
internal capabilities without current callers, such as `execution_trace_jsonl`
and `python_construction_script`. The CI configuration promotes warnings to
errors. This remains an explicit verification risk; no blanket lint suppression
or artificial public endpoint has been added to conceal it.
