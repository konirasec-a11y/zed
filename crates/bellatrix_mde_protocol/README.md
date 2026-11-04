# Modeling protocol compatibility facade

This crate reexports `bellatrix_mde_contracts::protocol` without defining another set of types. Existing imports remain compatible; new consumers should depend on `bellatrix_mde_contracts` directly.

Canonical model records, command values, presentation inputs and UML schema descriptors live in the shared contracts crate. Read projections are derived values, not persisted document state. Runtime events, snapshot leases, revision checks and publication remain in Cordis and the host integration.
