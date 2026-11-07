# Functional MDE API

This directory is the application/document boundary of `bellatrix_mde`. The former kernel API and second UML validation pass have been removed. Graph and presentation integrity checks remain, but do not certify UML conformance.

Cordis is the external communication boundary. Its adapter calls these functions; UI and host components do not retain another semantic model. MDE itself does not depend on Cordis or GPUI.

## State and serialization

The only persistent UML state is the typed `ruml::Registry` inside `bellatrix_mde::contracts::model::ModelingSection`. Command/render algorithms receive temporary `protocol::model_snapshot` records derived from it. A command returns a candidate, and the platform owns revision checks and atomic publication.

Standalone document schema version is 2. The root uses `schemaVersion`, `id`, `name` and `model`; the model payload contains `registry`, presentation, diagrams and source metadata. Generated UML fields use the supplied camelCase contracts. The API envelope remains `{"api_version":1,"model":...}`; its version is independent of the document schema.

JSON serializes the canonical document. BLX2 carries that same JSON document in a protobuf message, rather than defining duplicate wire nodes. The old flattened model and BLX1 format are not supported. Unknown contract fields and unsupported required values cause explicit errors rather than being silently dropped.

`src/api_contract.rs` handles the common versioned envelope and errors. `ModelDocumentParts` contains identity plus canonical section JSON, not a separate model. Read projections cannot be submitted as persisted sections.

## Entry points

- `decode_model_document`, `encode_model_document`, `convert_model_document`: in-memory JSON/BLX2 document conversion. They do not perform filesystem I/O.
- `prepare_model_document`, `validate_model_section`, `encode_model_section`: prepare, inspect presentation integrity, and encode a canonical section for the platform document.
- `import_mdj_document`, `export_mdj_document`, `export_mdj_section`: conservative interchange with retained original source bytes and residual fields. Unchanged supported projections export original bytes. Unsupported edits fail explicitly.
- `apply_model_command`, `reparent_uml_element`: compute atomic candidates. Contract conversion is included in the command so visual-draft fallback sees missing required contract data before publication.
- `project_model_section`: derive the UI snapshot from the same canonical section.
- `project_documentation`, `project_rust_generation`: read projections for documentation or generated output, not another authored semantic state.
- `report_model_document`: headless graph inspection and traversal.
- `validate_graph_model`, `reachable_graph_nodes`, `shortest_graph_path`: graph integrity and queries.
- `analyze_generation_dependencies`, `generate_rust_struct`: generation queries and projections.
- `prepare_artifact_publication`, `publish_artifact_manifest`: artifact preparation and explicitly requested publication.
- `validate_model_section_diagram`: presentation/diagram diagnostics, not a replacement UML validator.

`MdeExecutionTrace` records command/render activity through its sink; it does not publish or retain model state.

## Adapter boundaries

Ownership edges, association endpoint text and annotation links are derived views of contract fields. Changes must update those fields. The reverse adapter rejects stale or conflicting derived values instead of choosing another authority.

Binary associations use actual Property ends. Multiplicity uses lowerValue/upperValue and ValueSpecifications. Non-literal expressions remain preserved and are not overwritten by a textual multiplicity edit.

Template creation uses template, signature, parameteredElement and their owned references. No internal marker or pseudo-relation is persisted as UML.

Presentation labels, documentation, generated-source attachments and decorative stereotype notation are stored separately from UML fields. Decorative notation does not apply a UML Stereotype.

MDJ import currently keeps association/end encodings and unsupported source declarations opaque when a safe two-way mapping is unavailable. This preserves source data, but is not full semantic interchange or full editor support for those objects.

## Verification status

The schema migration intentionally changes persistence. Existing legacy fixtures and tests are not evidence that the new document path works. Removed validation suites are not replaced by a no-op validator. Formatting and patch application are distinct from compilation and runtime validation; report their results separately for each published checkpoint.
