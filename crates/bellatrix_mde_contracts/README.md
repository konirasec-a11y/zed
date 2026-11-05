# Universal MDE contracts

`bellatrix_mde_contracts` contains the shared contracts used by persistence, Cordis and the UI. Its default features do not depend on the engine, GPUI or project services. The optional `cordis` feature adds event bindings and asynchronous completion channels.

- `ruml`: the supplied UML structs and enums, with camelCase serialization and an explicit document-local typed `Registry`. Generated accessors use this registry; they do not retain global process state.
- `model`: canonical `Project` and `ModelingSection`, whose UML payload is that registry. Presentation, composition, provenance and authoring metadata reference its identities. They do not replace contract fields.
- `protocol`: command intents and disposable read projections. `model_snapshot::Element` and `Relation` support existing query/render algorithms only. They are not another persistent UML representation.
- `events`: shared requests, responses, immutable snapshots and UI command messages.
- `project`: the platform document envelope, revision, lifecycle and save capabilities. This is separate from the UML Model and from the standalone MDE document.
- `uml`: source-qualified descriptors and catalog metadata used for display, queries and adapters. These descriptors do not override a supplied ruml field or reintroduce the removed second UML validation pass.

Standalone documents use schema version 2 with a camelCase root envelope. JSON and BLX2 transport the same typed document; BLX2 wraps its JSON bytes in protobuf. There is no required compatibility with the old Element/Relation save format or BLX1.

The engine owns command execution, disposable projections, codecs and interchange conversion. Conversion failures are explicit and atomic. Unsupported MDJ objects and fields remain in source provenance rather than becoming invented UML contracts; edits without a lossless mapping are rejected.

`bellatrix_mde_protocol` remains a reexport-only facade. New consumers should depend directly on this crate.
