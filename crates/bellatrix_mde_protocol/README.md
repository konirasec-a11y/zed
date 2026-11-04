# Modeling protocol

Neutral command inputs and revision-bound read projections shared by the MDE API and its Cordis adapter. This crate has no dependency on the engine, Cordis, Zed, GPUI or platform project services.

These values are messages and derived views, not a second persisted model. The universal document in `bellatrix_mde` remains authoritative. Projections omit provenance and cannot be submitted as document state. Mutations remain commands and are validated atomically inside the engine.

Runtime events, project identity, snapshot leases, revision checks, file I/O and publication stay in Cordis and the host integration. Rendering and local interaction state stay in the UI.
