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
