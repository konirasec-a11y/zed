# Functional MDE API: version 1

This directory is the explicit application/document boundary of `bellatrix_mde`.
It is separate from [`kernel-api/`](../kernel-api/README.md), which exposes kernel
validation, graph queries and generation analysis. MDE operations coordinate the
existing document codec and application commands; they do not redefine UML rules
or introduce another model.

The current public functions are reexported directly from the crate root:

- `bellatrix_mde::decode_model_document`, implemented in `document.rs`.
- `bellatrix_mde::reparent_uml_element`, implemented in `commands.rs`.

Their implementation modules and request types are private. There is no second
public namespace, compatibility alias, public command enum or registry for this
boundary. `src/api_contract.rs` privately owns the shared envelope version, errors
and full-document decoder. Both API directories depend on that neutral helper;
neither API depends on the other. The semantic-only UML decoding projection stays
in `kernel-api/contract.rs` and must not be used to save or replace a document.

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

Errors are JSON strings with `api_version` and an `error` object containing
`code` and `message`. Codes identify the failure; message text is explanatory.
Neither function performs I/O, reads clocks, invokes callbacks or retains state.
Results depend only on the input and implementation. They do not publish, save,
advance revisions or mark documents clean. The application must compare the
source snapshot/revision and publish atomically; a client-supplied revision alone
cannot provide that guarantee.

## Existing exposure and remaining migration

This is the first physical separation, not complete public API encapsulation.
`mde.rs` still exposes `contracts`, `commands`, `mdj`, `mdj_conversion` and `uml`
modules, together with typed graph, compiler, persistence and projection exports.
CLI, Cordis and workbench consumers still use those types and, in some cases,
UML internals. They must migrate before the corresponding visibility can close.
Those exports are not duplicated under this facade.

MDJ import/export still lives in `persistence/mdj_conversion.rs`, exposed through
the existing typed module. No new MDJ endpoint or general command-serialization
protocol is introduced here. Future document/interchange and application-command
entry points belong in this directory, with explicit requests and without leaking
the internal `Command` enum. They must preserve source residue/provenance and
current authoring knowledge, and retain the inverse-export gate before publishing
an MDJ edit. Snapshot comparison and save acknowledgement remain in the host
integration rather than a second session manager here.

The existing `validate_graph_model` and `validate_uml_diagram` kernel facades still
include presentation checks. Their behavior and placement are unchanged in this
slice; this directory split does not claim that all existing dependency or
validation boundaries are already isolated. UML rule implementations remain in
`KERNEL/uml/`. This beta does not guarantee compatibility with earlier API or
document versions. This relocation preserves current function contracts without
retaining old private module aliases.
