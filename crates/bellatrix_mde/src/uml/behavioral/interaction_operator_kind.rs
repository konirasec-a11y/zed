// Adapted from javapak/ruml-wasm (MIT); native type.
#![allow(unused_imports)]
use serde::{Serialize, Deserialize};
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionOperatorKind {
    Seq,
    Alt,
    Opt,
    Break,
    Par,
    Strict,
    Loop,
    Critical,
    Neg,
    Assert,
    Ignore,
    Consider,
}
