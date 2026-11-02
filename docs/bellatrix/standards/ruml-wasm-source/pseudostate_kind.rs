// ============================================================================
// Generated Rust Code
// ============================================================================
//
// Type:           PseudostateKind (enum)
// Source Package: uml
// Package URI:    http://www.eclipse.org/uml2/2.1.0/UML
// Generated:      2025-11-26 14:56:46
// Generator:      EcoreToRustGenerator v0.1.0
//
// Generation Options:
//   - WASM:       enabled
//   - Tsify:      enabled
//   - Serde:      enabled
//   - Builders:   disabled
//   - References: String IDs
//
// WARNING: This file is auto-generated. Manual changes will be overwritten.
// ============================================================================

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use tsify::Tsify;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[wasm_bindgen]
pub enum PseudostateKind {
    Initial,
    Deephistory,
    Shallowhistory,
    Join,
    Fork,
    Junction,
    Choice,
    Entrypoint,
    Exitpoint,
    Terminate,
}

