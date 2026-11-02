// Adapted from javapak/ruml-wasm (MIT); native UML/Ecore type.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectorKind {
    Assembly,
    Delegation,
}
