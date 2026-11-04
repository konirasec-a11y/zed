# Universal MDE contracts

`bellatrix_mde_contracts` is the shared language used by the MDE engine, persistence, UML validation, Cordis and UI consumers. Its default feature set uses value/serialization libraries, never the engine, GPUI or project services. The optional `cordis` feature adds event bindings and asynchronous completion channels; handlers and runtime services remain outside.

- `model`: canonical `Project`, `ModelingSection`, `Element`, `Relation`, `Diagram`, `View`, geometry, style, composition, provenance and authoring knowledge. These exact types are used for state, JSON serialization and validation; command geometry/style are reexports, not copies.
- `protocol`: commands, read projections, tool descriptions and trace events. Commands are the same types executed by the engine. Read projections contain calculated display data and cannot be written back as persisted state.
- `events`: shared MDE request/response payloads, event identities, immutable snapshots and UI command messages.
- `project`: platform document identity/envelope, error and lifecycle/save events. Save tickets carry opaque capabilities issued and verified by the project runtime; they do not expose reservation state.
- `uml`: source-qualified OMG descriptors and generated catalog, primitive literals, structural diagnostics and pure schema queries. The engine retains graph queries, semantic validation and editing policy.

`Project` identifies a standalone MDE model. `ModelingSection` is its payload inside the platform's `bellatrix.mde` section. The platform `ProjectDocument` is a separate shared contract under `project`, reexported by `bellatrix_project`: its identity, revision, other sections and ephemeral session are not an MDE model or a UML Model. Project services and storage stay in `bellatrix_project`.

The engine owns JSON/Protobuf codecs, internal `Wire*` encoding records, mutations and validation. The existing Protobuf schema encodes the standalone MDE model, not the entire platform envelope. Moving contracts does not change field names, wire tags, missing/null semantics, duplicate-key rejection or retained source provenance.

`bellatrix_mde_protocol` remains a compatibility facade that only reexports this crate's `protocol` module. New consumers should depend directly on `bellatrix_mde_contracts`.
