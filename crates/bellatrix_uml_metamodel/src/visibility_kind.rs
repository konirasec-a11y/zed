// Original declaration from javapak/ruml-wasm (MIT), native Bellatrix adaptation.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisibilityKind {
    Public,
    Private,
    Protected,
    Package,
}
