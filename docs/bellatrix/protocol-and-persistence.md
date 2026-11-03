# Zed protocols, persistence and events — reuse contract for Bellatrix

> Investigated against the pinned fork revision in `feat/bellatrix-native-engines`. This is a **source-code audit and migration decision**, not a claim that the integrations below have already been implemented or tested.

## Decision

Bellatrix beta has one canonical model section, `bellatrix.mde`, and one public MDE event prefix, `bellatrix.mde.*.v1`. Old Bellatrix namespaces are unsupported and rejected explicitly; there is no fallback or migration layer. This breaking Bellatrix policy does not authorize changes to Zed native formats or protocols.

**Zed is the source of truth for file/worktree I/O, text buffers, GPUI UI events, Workspace/Pane/Dock persistence and session restoration.** **Cordis-rs remains the source of truth for backend capability lifecycles and public domain events.** Engine-MDE is responsible for semantic validation and versioned model changes. Do **not** implement another file explorer, text persistence engine, UI event bus, dock/session database or RPC framework.

The `proto` crate defines the **remote and collaboration wire protocol** `package zed.messages`, not the application's entire local state model or local SQLite persistence. It is not a generic replacement for the Cordis domain-event system. Keep its Zed message numbering and `reserved` fields stable.

## Source inventory — what already exists

| Capability | Existing Zed contract | Source / remarks |
| --- | --- | --- |
| Remote RPC envelopes | `Envelope { id, responding_to, original_sender_id, ack_id, oneof payload }` | `crates/proto/proto/zed.proto`; `crates/proto/src/proto.rs` explicitly registers `messages!`, `request_messages!`, `entity_messages!` |
| Open/edit/save text buffers | `OpenBufferByPath`, `UpdateBuffer`, `SaveBuffer`, `BufferSaved`, `BufferReloaded`, `SynchronizeBuffers`, `BufferVersion` | `crates/proto/proto/buffer.proto`; `crates/project/src/buffer_store.rs` uses these on remote paths |
| Replicated text operations | `Operation::Edit`, `Operation::Undo`, `VectorClockEntry`, `Transaction` | `crates/proto/proto/buffer.proto` — these are **text-buffer** versions/transactions, not semantic model revisions |
| File/worktree operations | `CreateProjectEntry`, `RenameProjectEntry`, `CopyProjectEntry`, `TrashProjectEntry`, `RestoreProjectEntry`, `UpdateWorktree` | `crates/proto/proto/worktree.proto` and `crates/proto/proto/call.proto` |
| Collaborative project/session | `ShareProject`, `JoinProject`, `UpdateProject`, `JoinProjectResponse` | `crates/proto/proto/call.proto`; `UpdateProject` lists `WorktreeMetadata`, **not** generic domain data |
| Collaborative view state | `UpdateActiveView`, `UpdateView` | `crates/proto/proto/call.proto`; `UpdateView.oneof variant` only includes `Editor`; `View.oneof variant` includes `Editor` and `ChannelView`. It **does not** represent MDE diagram Views |
| User notifications | `GetNotifications`, `AddNotification`, `UpdateNotification`, `MarkNotificationRead` | `crates/proto/proto/notification.proto` — user/app notification contracts, not a general domain event bus |
| Local buffer events | `language::BufferEvent::{Edited, Saved, Reloaded, DirtyChanged, ...}` | `crates/language/src/buffer.rs`, handled via GPUI `EventEmitter` / `cx.subscribe`; distinct from Proto `BufferSaved` |
| Local project events | `project::Event::{WorktreeAdded, WorktreeRemoved, WorktreeUpdatedEntries, DeletedEntry, EntryRenamed, BufferEdited, ...}` | `crates/project/src/project.rs`; local GPUI entity notifications |
| Workspace session database | `WorkspaceDb` with migration-backed `workspaces`, `panes`, `pane_groups`, `items` and dock columns | `crates/workspace/src/persistence.rs` plus `persistence/model.rs`; SQLite, **not protobuf** |
| Persisted/restored Zed Item | `workspace::SerializableItem`, `workspace::register_serializable_item`, `SerializedItem`, `SerializableItemRegistry` | `crates/workspace/src/{item,workspace}.rs`; `crates/editor/src/items.rs` shows the pattern |
| Text editor hot exit | `EditorDb`, `Editor::serialize/deserialize`, unsaved buffer contents and scroll/selections | `crates/editor/src/{items,persistence}.rs`; uses `db::Domain` migrations and Workspace ID / Item ID |
| Application persistence infrastructure | `AppDatabase`, `db::Domain`, `db::kvp::KeyValueStore::scoped` | `crates/db/src/{db,kvp}.rs`; reuse rather than implementing a Bellatrix DB framework |
| Bellatrix semantic document | `bellatrix_project::ProjectDocument` with `ProjectId`, `sections`, `expected_revision` and dirty/saved revision | `crates/bellatrix_project/src/{document,store,events}.rs`; this is **not** Zed `project::Project` |
| Public backend domain notifications | `ProjectOpenedEvent`, `ProjectStateChangedEvent`, `ProjectSavedEvent`, `ProjectClosedEvent` | `crates/bellatrix_project/src/events.rs`; already Cordis events, preserve backend lifecycle guarantees |

## No duplicate event system

There are three distinct event scopes; never confuse them:

1. **GPUI local events**: `cx.emit`, `EventEmitter`, `cx.subscribe` and `Workspace::ItemEvent` keep tabs, buffers and the UI in sync. Do not mirror every mouse event into Cordis.
2. **Zed Proto / RPC messages**: `Envelope` and the typed payload contracts transport operations between peers/remote processes. They are not local persistence themselves. Only add Bellatrix-specific wire messages when real remote semantic MDE collaboration is implemented; do not repurpose `UpdateView` or `UpdateProject`.
3. **Cordis public domain events**: requests, confirmed model changes, completed tasks and Services for **all backend engines**, including headless engines. Use typed events the Bellatrix code already defines; Zed UI is the presentation consumer across a narrow bridge.

A successfully saved **file** (`BufferSaved`) is not by itself proof that a semantic **model mutation** passed MDE validation and optimistic revision checks (`ProjectStateChangedEvent`). Conversely, changing a persisted model in memory should not silently overwrite an unsaved Zed editor buffer. Deliberately define the boundary and consistency strategy.

## Concrete gap in the current fork

`crates/bellatrix_mde_workbench/src/UI/workbench.rs` implements `workspace::Item` and `EventEmitter<()>` for `MdeWorkbenchItem`, but **not** `workspace::SerializableItem`. The integration now registers the native `bellatrix_mde::OpenMdeWorkbench` Workspace action via `cx.observe_new` instead of inserting MDE Items and Panels into every Zed window. Reopening the action focuses an existing MDE tab. This removes the automatic-UI duplication, **but** MDE tab/diagram/selection restoration **still does not use** Zed's standard Item/session mechanism.

Do not invent an alternate session database to fix this. Follow `crates/editor/src/items.rs` and `crates/editor/src/persistence.rs`:

- Register `MdeWorkbenchItem` as `SerializableItem` through `workspace::register_serializable_item`.
- Persist only a **reference to a model document and transient UI state** (active diagram, selected element, current viewport) under Zed's per-Workspace Item lifecycle, using an existing `db::Domain` pattern if a table is needed.
- `deserialize` must resolve the correct Bellatrix document within the real Zed Project/worktree and open exactly that Item, rather than the synthetic startup fixture; avoid duplicate Items per model. The interim `OpenMdeWorkbench` action is deliberately user-activated, not auto-mounted.
- Let the Zed Dock persist panel sizes, visibility, arrangement and focus. Do **not** serialize these again in Bellatrix.
- Preserve old `.bellatrix` document formats and namespaces; session rows are not canonical model storage.

## File persistence — reuse Zed without deleting semantic rules

The current Bellatrix `ProjectService` independently stores `Arc<ProjectDocument>` snapshots, checks `expected_revision`, tracks `saved_revision`, and atomically writes JSON files. Zed already has mature worktree and text-buffer persistence, but **its `BufferVersion` vector clock is not equivalent to a document-wide semantic `expected_revision`**.

Recommended incremental migration:

1. Associate an MDE document with a real `project::ProjectPath` and `language::Buffer` belonging to Zed, so file open/save/rename/reload and local notifications use Zed's existing Project/BufferStore APIs.
2. Parse an immutable MDE snapshot from the serialized model; validate mutations through the MDE domain/Cordis contract, with explicit conflict policy when the underlying buffer changed. Do not perform arbitrary domain mutations from an unvalidated text edit.
3. After round-trip, external-edit, concurrent-write, dirty-state and unsupported-beta-format rejection tests pass, retire the duplicate Bellatrix disk writer/open-file shell **only where Zed is demonstrably authoritative**. Keep semantic model identity, namespace and validation contracts independent from the UI runtime.
4. Headless MDE engines must still work without initializing GPUI/Workspace. Do not make backend Cordis Services depend on Zed UI entities; use an explicit storage adapter/interface if sharing a file representation.

Do not put user project documents into Zed's workspace/session SQLite as the only authoritative copy. The current host still creates a standalone Bellatrix document from `BELLATRIX_PROJECT_FILE`, `BELLATRIX_MODEL_FILE` or a demo fixture at startup; it **does not yet select documents from the active Zed Project**. Retire that behavior only when ProjectPath/Buffer-backed document opening and domain revision tests are operational. That database is scoped to the local application's session/state.

## When a new `mde.proto` would be justified

**Not for local drag, class creation, layout persistence, save-to-disk or notifications.** Those already have suitable local mechanisms.

When native **remote semantic MDE collaboration** is required (multiple editors concurrently manipulating element identities, views, geometry or relations), design versioned MDE wire messages; then register them in Zed's `zed.proto` Envelope with unallocated field numbers, update `crates/proto/src/proto.rs` `messages!` and request/entity maps, and implement routing/authorization/compatibility tests. Zed's `UpdateView` currently cannot carry arbitrary MDE diagrams. Do not allocate field numbers, define unsupported generic payloads or claim collaboration before this use case.

## Acceptance tests for the next change

1. Open two MDE documents as separate Zed Items; close/reopen Workspace and restore only their own Item identities, diagrams, selections and panel arrangement.
2. Save an MDE document via an actual Zed file path; observe local `BufferEvent::Saved` and correctly confirm the MDE domain revision. Test external modification and conflict without losing edits.
3. A headless MDE operation still succeeds with the Zed application completely absent.
4. A new Cordis engine observes `ProjectStateChangedEvent` without depending on GPUI/proto; disposing that engine unregisters listeners and does not discard saved data.
5. Run all of the above in the Windows Zed fork and retain the required Cargo/CI checks. No assertion of completion based solely on a Protobuf message existing.

**Source of truth:** code in this Zed fork at the audited commit. This note is additive to the existing Bellatrix migration, not a request to replace Zed's protocols, Editor, Project, Workspace, DB or Dock.
